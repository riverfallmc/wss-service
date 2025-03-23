use std::{collections::HashMap, sync::OnceLock};
use adjust::response::NonJsonHttpResult;
use anyhow::anyhow;
use axum::{
  extract::{ws::{Message, WebSocket}, WebSocketUpgrade},
  response::IntoResponse
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc::{unbounded_channel, UnboundedSender}, Mutex};
use futures_util::{StreamExt, SinkExt};

struct WssConnection {
  #[allow(unused)]
  sender: UnboundedSender<Message>
}

#[derive(Serialize, Deserialize)]
struct WssEvent {
  r#type: String,
  data: Value
}

type WssConnections = Vec<WssConnection>;

static CONNECTIONS: OnceLock<Mutex<HashMap<usize, WssConnections>>> = OnceLock::new();

pub struct WssService;

impl WssService {
  /// апгрейдит соединение с http на websocket
  pub fn upgrade(
    wss: WebSocketUpgrade,
    id: usize
  ) -> impl IntoResponse {
    wss.on_upgrade(move |socket| Self::socket_handler(socket, id))
  }

  async fn socket_handler(
    wss: WebSocket,
    id: usize
  ) {
    let (mut tx, mut rx) = wss.split();
    let (sender, mut receiver) = unbounded_channel();

    let connection = WssConnection { sender };

    let id_in_vec = Self::connect(id, connection).await;

    tokio::spawn(async move {
      while let Some(msg) = receiver.recv().await {
        let _ = tx.send(msg).await;
      }
    });

    while let Some(msg) = rx.next().await {
      if msg.is_err() {
        break;
      }
    }

    Self::disconnect(id, id_in_vec).await;
  }

  pub async fn send(
    id: usize,
    r#type: String,
    data: Value,
  ) -> NonJsonHttpResult<()> {
    let connections = CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let lock = connections.lock().await;
    if let Some(vec) = lock.get(&id) {
      let event = WssEvent {r#type, data};
      let json = serde_json::to_string(&event)
        .map_err(|e| anyhow!("Не получилось десериализовать WssEvent: {e}"))?;
      let msg = Message::Text(json.to_string().into());

      for conn in vec {
        let _ = conn.sender.send(msg.clone());
      }
    }

    Ok(())
  }

  async fn connect(id: usize, connection: WssConnection) -> usize {
    let connections = CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lock = connections.lock().await;
    let vec = lock.entry(id).or_default();
    vec.push(connection);

    vec.len() - 1
  }

  async fn disconnect(id: usize, id_in_vec: usize) {
    let connections = CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lock = connections.lock().await;
    if let Some(vec) = lock.get_mut(&id) {
      if id_in_vec < vec.len() {
        vec.remove(id_in_vec);
      }
      if vec.is_empty() {
        lock.remove(&id);
      }
    }
  }
}