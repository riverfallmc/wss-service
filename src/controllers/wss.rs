use adjust::{controller::Controller, response::{HttpMessage, HttpResult}};
use axum::{extract::{Path, Query, WebSocketUpgrade}, response::IntoResponse, routing::{get, post}, Json, Router};
use serde::Deserialize;
use serde_json::Value;
use crate::{service::wss::WssService, AppState};

#[derive(Deserialize)]
struct QueryEventType {
  r#type: String
}

pub struct WssController;

impl WssController {
  // подключение к wss
  async fn connect(
    wss: WebSocketUpgrade,
    Path(id): Path<usize>,
  ) -> impl IntoResponse {
    WssService::upgrade(wss, id)
  }

  // secure, internal
  // отправка ивента в подключение wss
  async fn send(
    Path(id): Path<usize>,
    Query(query): Query<QueryEventType>,
    Json(payload): Json<Value>
  ) -> HttpResult<HttpMessage> {
    WssService::send(id, query.r#type, payload)
      .await?;

    Ok(Json(
      HttpMessage::new("Ивент был отправлен")
    ))
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
  }
}