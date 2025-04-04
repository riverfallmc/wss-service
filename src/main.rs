use adjust::{controller::Controller, controllers, database::{redis::Redis, Pool}, main, service::Service};
use controllers::wss::WssController;

mod controllers;
mod service;

#[derive(Default, Clone)]
struct AppState {
  redis: Pool<Redis>
}

#[main]
async fn main() -> Service<'_, AppState> {
  adjust::server::WebServer::enviroment();

  Service {
    name: "Wss",
    state: AppState::default(),
    controllers: controllers![WssController],
    port: Some(1401),
    ..Default::default()
  }
}