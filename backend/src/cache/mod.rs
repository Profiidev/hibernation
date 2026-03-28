use axum::{Router, routing::get};
use centaurus::{db::init::Connection, error::Result};

use crate::{
  auth::cli_auth::CliAuth,
  cache::{
    management::CacheRegex, push::PushState, state::CacheEvictionState, storage::FileStorage,
  },
  config::Config,
};

mod cleanup;
mod dedupe;
mod management;
mod push;
mod state;
pub mod storage;

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

  cleanup::start(db.clone(), storage.clone());
  dedupe::start(db);

  router
    .layer(axum::Extension(storage))
    .layer(axum::Extension(push_state))
    .layer(axum::Extension(CacheEvictionState::new()))
    .layer(axum::Extension(CacheRegex::new()))
}

async fn test(auth: CliAuth) -> Result<String> {
  Ok(auth.user_id.to_string())
}
