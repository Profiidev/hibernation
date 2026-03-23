use axum::{Router, routing::get};
use centaurus::{db::init::Connection, error::Result};

use crate::{
  auth::cli_auth::CliAuth,
  cache::{push::PushState, storage::FileStorage},
  config::Config,
};

mod cleanup;
mod management;
mod push;
mod storage;

pub fn router() -> Router {
  Router::new()
    .nest("/management", management::router())
    .nest("/push", push::router())
    .route("/test", get(test))
}

pub async fn state(router: Router, db: Connection, config: &Config) -> Router {
  let storage = FileStorage::init(&config.storage)
    .await
    .expect("Failed to init FileStorage");
  let push_state = PushState::new();

  cleanup::start(db, storage.clone());

  router
    .layer(axum::Extension(storage))
    .layer(axum::Extension(push_state))
}

async fn test(auth: CliAuth) -> Result<String> {
  Ok(auth.user_id.to_string())
}
