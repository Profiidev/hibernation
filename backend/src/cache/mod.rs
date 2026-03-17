use axum::{Router, routing::get};
use centaurus::error::Result;

use crate::{
  auth::jwt_auth::{CliToken, JwtAuth},
  permissions::NoPerm,
};

pub fn router() -> Router {
  Router::new().route("/test", get(test))
}

async fn test(_auth: JwtAuth<NoPerm, CliToken>) -> Result<()> {
  Ok(())
}
