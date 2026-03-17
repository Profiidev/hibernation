use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use axum::{
  Extension, Json, Router,
  extract::{FromRequestParts, Query},
  routing::{get, post},
};
use centaurus::{auth::pw::PasswordState, bail, db::init::Connection, error::Result};
use chrono::Utc;
use dashmap::DashMap;
use rand::{RngExt, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use tokio::{spawn, time::sleep};
use tower_governor::GovernorLayer;
use uuid::Uuid;

use crate::{
  auth::{cli_auth::CLI_TOKEN_LEN, jwt_auth::JwtAuth, jwt_state::JwtState},
  db::DBTrait,
  rate_limit::RateLimiter,
};

pub fn router(rate_limiter: &mut RateLimiter) -> Router {
  Router::new()
    .route("/", get(get_token))
    .layer(GovernorLayer::new(rate_limiter.create_limiter()))
    .route("/", post(new_code))
}

pub fn state(router: Router) -> Router {
  let cli_state = CliState {
    codes: Arc::new(DashMap::new()),
  };

  spawn({
    let codes = cli_state.codes.clone();

    async move {
      loop {
        let now = Instant::now();
        codes.retain(|_, (time, _)| now.duration_since(*time).as_secs() < 300);
        sleep(Duration::from_secs(600)).await;
      }
    }
  });

  router.layer(Extension(cli_state))
}

#[derive(FromRequestParts, Clone)]
#[from_request(via(Extension))]
struct CliState {
  codes: Arc<DashMap<Uuid, (Instant, Uuid)>>,
}

#[derive(Serialize)]
struct CodeResponse {
  code: String,
}

async fn new_code(auth: JwtAuth, state: CliState) -> Json<CodeResponse> {
  let code = Uuid::new_v4();
  state.codes.insert(code, (Instant::now(), auth.user_id));
  Json(CodeResponse {
    code: code.to_string(),
  })
}

#[derive(FromRequestParts, Clone, Deserialize)]
#[from_request(via(Query))]
struct TokenReq {
  code: Uuid,
}

async fn get_token(
  token_req: TokenReq,
  state: CliState,
  db: Connection,
  jwt: JwtState,
  pw: PasswordState,
) -> Result<String> {
  let Some((instant, user)) = state.codes.get(&token_req.code).map(|entry| *entry.value()) else {
    bail!("Invalid code");
  };

  state.codes.remove(&token_req.code);
  if Instant::now().duration_since(instant).as_secs() > 300 {
    bail!("Invalid user or code");
  }

  let token: String = {
    let mut rng = rand::rng();
    (0..CLI_TOKEN_LEN)
      .map(|_| rng.sample(Alphanumeric) as char)
      .collect()
  };

  let Some(exp) = Utc::now().checked_add_signed(chrono::Duration::seconds(jwt.exp)) else {
    bail!(INTERNAL_SERVER_ERROR, "Failed to create token");
  };

  let hash = pw.pw_hash_raw("", &token)?;

  db.token()
    .insert(user, format!("Cli-{}", Utc::now()), hash, exp.naive_utc())
    .await?;

  Ok(token)
}
