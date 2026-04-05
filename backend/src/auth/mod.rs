use aide::axum::ApiRouter;
use centaurus::backend::{auth, middleware::rate_limiter::RateLimiter};

pub mod cli_auth;
mod test_token;

pub fn router(rate_limiter: &mut RateLimiter) -> ApiRouter {
  ApiRouter::new()
    .nest("/test_token", test_token::router())
    .merge(auth::router(rate_limiter))
}
