use adjust::{controller::Controller, response::NonJsonHttpResult};
use axum::{extract::{Path, Query, State, WebSocketUpgrade}, response::IntoResponse, routing::{get, post}, Json, Router};
use serde::Deserialize;
use serde_json::Value;
use crate::{service::wss::WssService, AppState};

#[derive(Deserialize)]
struct QueryEventType {
  r#type: String
}

#[derive(Deserialize)]
struct BroadcastBody {
  ids: Vec<i32>,
  body: Value
}

pub struct WssController;

impl WssController {
  // подключение к wss
  async fn connect(
    wss: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(id): Path<i32>,
  ) -> impl IntoResponse {
    WssService::upgrade(state.redis, wss, id)
  }

  // secure, internal
  // отправка ивента в подключение wss
  async fn send(
    Path(id): Path<i32>,
    Query(query): Query<QueryEventType>,
    Json(payload): Json<Value>
  ) -> NonJsonHttpResult<()> {
    WssService::send(
      id,
      &WssService::serialize(query.r#type, payload)?
    ).await?;

    Ok(())
  }

  // secure, internal
  // отправка ивента подключениям wss
  async fn broadcast(
    Query(query): Query<QueryEventType>,
    Json(payload): Json<BroadcastBody>
  ) -> NonJsonHttpResult<()> {
    WssService::broadcast(payload.ids, query.r#type, payload.body)
      .await?;

    Ok(())
  }
}

impl Controller<AppState> for WssController {
  fn new() -> anyhow::Result<Box<Self>> {
    Ok(Box::new(Self))
  }

  fn register(&self, router: Router<AppState>) -> Router<AppState> {
    router
      .route("/{id}", get(Self::connect))
      .route("/send/{id}", post(Self::send))
      .route("/broadcast", post(Self::broadcast))
  }
}