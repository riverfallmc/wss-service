use adjust::{controller::Controller, controllers, main, service::Service};
use controllers::wss::WssController;

mod controllers;
mod service;

#[derive(Default, Clone)]
struct AppState {}

#[main]
async fn main() -> Service<'_, AppState> {
  Service {
    name: "Wss",
    state: AppState::default(),
    controllers: controllers![WssController],
    ..Default::default()
  }
}