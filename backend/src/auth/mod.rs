use aide::axum::ApiRouter;
use centaurus::backend::{
  auth::{self, config, logout, oidc, password},
  middleware::rate_limiter::RateLimiter,
};

pub use auth::state;

use crate::utils::UpdateMessage;

pub mod cli_auth;
mod test_token;

pub fn router(rate_limiter: &mut RateLimiter) -> ApiRouter {
  ApiRouter::new()
    .nest("/test_token", test_token::router())
    .nest("/password", password::router(rate_limiter))
    .nest("/logout", logout::router())
    .nest("/oidc", oidc::router::<UpdateMessage>(rate_limiter))
    .nest("/config", config::router())
}
