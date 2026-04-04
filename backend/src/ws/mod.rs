use aide::axum::ApiRouter;
use axum::Extension;

use crate::ws::state::{UpdateState, Updater};

pub mod state;
mod updater;

pub fn router() -> ApiRouter {
  ApiRouter::new().merge(updater::router())
}

pub async fn state(router: ApiRouter) -> (ApiRouter, Updater) {
  let (state, updater) = UpdateState::init().await;

  (
    router
      .layer(Extension(state))
      .layer(Extension(updater.clone())),
    updater,
  )
}
