use axum::{Router, routing::get};
use centaurus::error::Result;

use crate::auth::cli_auth::CliAuth;

pub fn router() -> Router {
  Router::new().route("/test", get(test))
}

async fn test(_auth: CliAuth) -> Result<()> {
  Ok(())
}
