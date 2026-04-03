use aide::axum::{
  ApiRouter,
  routing::{get, post},
};
use axum::Json;
use axum_extra::extract::CookieJar;
use centaurus::{auth::pw::PasswordState, bail, db::init::Connection, error::Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tower_governor::GovernorLayer;
use tracing::debug;
use uuid::Uuid;

use crate::{
  auth::{jwt_state::JwtState, res::TokenRes},
  db::DBTrait,
  rate_limit::RateLimiter,
};

pub fn router(rate_limiter: &mut RateLimiter) -> ApiRouter {
  ApiRouter::new()
    .api_route("/", post(authenticate))
    .layer(GovernorLayer::new(rate_limiter.create_limiter()))
    .api_route("/", get(key))
}

#[derive(Serialize, JsonSchema)]
struct KeyRes {
  key: String,
}

async fn key(state: PasswordState) -> Json<KeyRes> {
  Json(KeyRes { key: state.pub_key })
}

#[derive(Deserialize, JsonSchema)]
struct LoginReq {
  email: String,
  password: String,
}

#[derive(Serialize, Debug, JsonSchema)]
struct LoginResponse {
  user: Uuid,
}

async fn authenticate(
  state: PasswordState,
  jwt: JwtState,
  db: Connection,
  mut cookies: CookieJar,
  Json(req): Json<LoginReq>,
) -> Result<(CookieJar, TokenRes<LoginResponse>)> {
  let user = db.user().get_user_by_email(&req.email).await?;
  let hash = state.pw_hash(&user.salt, &req.password)?;

  if hash != user.password {
    bail!(UNAUTHORIZED, "Invalid email or password");
  }

  let cookie = jwt.create_token(user.id)?;
  cookies = cookies.add(cookie);
  debug!("User logged in: {}", user.id);

  Ok((cookies, TokenRes(LoginResponse { user: user.id })))
}
