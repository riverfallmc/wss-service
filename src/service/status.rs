use std::sync::LazyLock;
use adjust::{database::{redis::Redis, Database}, load_env, redis::Commands, response::{HttpError, NonJsonHttpResult}};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

static CLIENT: LazyLock<Client> = LazyLock::new(Client::default);

load_env!(USER_URL);

pub struct StatusService;

#[derive(Serialize, Deserialize)]
enum UserStatus {
  Online,
  Offline
}

impl StatusService {
  fn update(
    redis: &mut Database<Redis>,
    user_id: i32,
    payload: Value
  ) -> NonJsonHttpResult<()> {
    redis.set::<_, _, ()>(format!("status:{user_id}"), payload.to_string())
      .map_err(|_| HttpError::new("Не получилось обновить статус в Redis", None))
  }

  pub fn online(
    redis: &mut Database<Redis>,
    user_id: i32
  ) {
    #[allow(unused)]
    Self::update(redis, user_id, json!({
      "status": UserStatus::Online
    }));

    tokio::spawn(async move {
      #[allow(unused)]
      CLIENT.post(format!("http://{}/status/{user_id}", *USER_URL))
        .send()
        .await;
    });
  }

  pub fn playing(
    redis: &mut Database<Redis>,
    user_id: i32,
    server_id: i32
  ) {
    let payload = if server_id == 0 {
      json!({
        "status": UserStatus::Online,
      })
    } else {
      json!({
        "status": UserStatus::Online,
        "server": server_id
      })
    };

    #[allow(unused)]
    Self::update(redis, user_id, payload);

    tokio::spawn(async move {
      #[allow(unused)]
      CLIENT.patch(format!("http://{}/status/{user_id}?server_id={server_id}", *USER_URL))
        .send()
        .await;
    });
  }

  pub fn offline(
    redis: &mut Database<Redis>,
    user_id: i32
  ) {
    #[allow(unused)]
    Self::update(redis, user_id, json!({
      "status": UserStatus::Offline,
      "last_seen_at": Utc::now().naive_utc()
    }));

    tokio::spawn(async move {
      #[allow(unused)]
      CLIENT.delete(format!("http://{}/status/{user_id}", *USER_URL))
        .send()
        .await;
    });
  }
}