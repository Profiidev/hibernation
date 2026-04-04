use aide::axum::ApiRouter;
use axum::Extension;
use centaurus::{backend::rate_limiter::RateLimiter, db::init::Connection};
use tower_governor::GovernorLayer;

use crate::mail::state::{Mailer, ResetPasswordState};

mod reset;
pub mod state;
pub mod templates;
mod test;

pub fn router(rate_limiter: &mut RateLimiter) -> ApiRouter {
  ApiRouter::new()
    .nest("/reset", reset::router())
    .nest("/test", test::router())
    .layer(GovernorLayer::new(rate_limiter.create_limiter()))
}

pub async fn state(router: ApiRouter, db: &Connection) -> ApiRouter {
  let mailer = Mailer::new(db).await;
  let password_reset_state = ResetPasswordState::default();

  router
    .layer(Extension(mailer))
    .layer(Extension(password_reset_state))
}
