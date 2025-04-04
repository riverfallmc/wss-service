use std::{collections::HashMap, sync::{Arc, OnceLock}};
use adjust::{database::{redis::Redis, Database, Pool}, response::NonJsonHttpResult};
use anyhow::{anyhow, bail, Result};
use axum::{
  extract::{ws::{Message, WebSocket}, WebSocketUpgrade},
  response::IntoResponse
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc::{unbounded_channel, UnboundedSender}, Mutex};
use futures_util::{StreamExt, SinkExt};
use super::status::StatusService;

struct WssConnection {
  #[allow(unused)]
  sender: UnboundedSender<Message>
}

#[derive(Serialize, Deserialize)]
struct WssEvent {
  r#type: String,
  data: Value
}

#[derive(Serialize, Deserialize)]
struct EventPlay {
  server: i32
}

/// Сообщение от клиента
#[derive(Serialize, Deserialize)]
struct WssMessage {
  r#type: String,
  data: Value
}

type WssConnections = Vec<WssConnection>;

static CONNECTIONS: OnceLock<Mutex<HashMap<i32, WssConnections>>> = OnceLock::new();

fn handle_message(redis: &mut Database<Redis>, user_id: i32, message: Message) -> Result<()> {
  match message {
    Message::Text(bytes) => {
      let message = serde_json::from_str::<WssMessage>(&bytes)?;

      match message.r#type.to_lowercase().as_str() {
        "play" => {
          let body = serde_json::from_value::<EventPlay>(message.data)?;

          StatusService::playing(redis, user_id, body.server);
        },
        _ => {}
      }
    },
    Message::Close(_) => bail!("Closed"),
    _ => {},
  };

  Ok(())
}

pub struct WssService;

impl WssService {
  /// апгрейдит соединение с http на websocket
  pub fn upgrade(
    redis: Pool<Redis>,
    wss: WebSocketUpgrade,
    id: i32
  ) -> impl IntoResponse {
    wss.on_upgrade(move |socket| Self::socket_handler(redis, socket, id))
  }

  async fn socket_handler(
    redis: Pool<Redis>,
    wss: WebSocket,
    id: i32
  ) {
    let (tx, mut rx) = wss.split();
    let tx = Arc::new(Mutex::new(tx));
    let (sender, mut receiver) = unbounded_channel();

    let connection = WssConnection { sender };

    let mut id_in_vec = 0;

    if let Ok(mut redis) = redis.get() {
      id_in_vec = Self::connect(&mut redis, id, connection).await;
    }

    let tx_clone = Arc::clone(&tx);

    tokio::spawn(async move {
      while let Some(msg) = receiver.recv().await {
        let _ = tx_clone.lock().await.send(msg).await;
      }
    });

    while let Some(msg) = rx.next().await {
      let mut redis = redis.get().unwrap(); // damn it

      match msg {
        Ok(message) => {
          if handle_message(&mut redis, id, message).is_err() {
            break
          }
        },
        Err(_) => {}
      }
    }

    let _ = tx.lock().await.close().await;

    if let Ok(mut redis) = redis.get() {
      Self::disconnect(&mut redis, id, id_in_vec).await;
    }
  }

  pub fn serialize(
    r#type: String,
    data: Value
  ) -> NonJsonHttpResult<Message> {
    let data = Self::fix_json_value(data);

    let event = WssEvent {r#type, data};

    let json = serde_json::to_string(&event)
      .map_err(|e| anyhow!("Не получилось сериализовать WssEvent: {e}"))?;

    Ok(Message::Text(json.into()))
  }

  pub async fn send(
    id: i32,
    message: &Message
  ) -> NonJsonHttpResult<()> {
    let connections = CONNECTIONS
      .get_or_init(Mutex::default);

    let guard = connections.lock()
      .await;

    if let Some(vec) = guard.get(&id) {
      for conn in vec {
        let _ = conn.sender.send(message.to_owned());
      }
    }

    Ok(())
  }

  pub async fn broadcast(
    ids: Vec<i32>,
    r#type: String,
    data: Value
  ) -> NonJsonHttpResult<()> {
    let message = Self::serialize(r#type, data)?;

    for id in ids {
      Self::send(id, &message)
        .await?; // тут await потому-что мютекс лочится в функции
    }

    Ok(())
  }

  async fn connect(redis: &mut Database<Redis>, id: i32, connection: WssConnection) -> usize {
    let connections = CONNECTIONS
      .get_or_init(Mutex::default);

    let mut guard = connections.lock()
      .await;

    let vec = guard.entry(id).or_default();

    vec.push(connection);

    StatusService::online(redis, id);

    vec.len() - 1
  }

  async fn disconnect(redis: &mut Database<Redis>, id: i32, id_in_vec: usize) {
    let connections = CONNECTIONS
      .get_or_init(Mutex::default);

    let mut guard = connections.lock()
      .await;

    if let Some(vec) = guard.get_mut(&id) {
      if id_in_vec < vec.len() {
        vec.remove(id_in_vec);
      }

      if vec.is_empty() {
        guard.remove(&id);
      }
    }

    StatusService::offline(redis, id);
  }

  fn fix_json_value(val: Value) -> Value {
    match val {
      Value::String(s) => serde_json::from_str(&s)
        .unwrap_or(Value::String(s)),
      _ => val,
    }
  }
}