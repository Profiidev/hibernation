use aide::axum::routing::{post_with, put_with};
use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use aide::{OperationIo, axum::ApiRouter};
use axum::{
  Extension, Json,
  extract::{FromRequestParts, Query},
};
use centaurus::{
  backend::{
    auth::{jwt_auth::JwtAuth, jwt_state::JwtState, pw_state::PasswordState},
    middleware::rate_limiter::RateLimiter,
  },
  bail,
  db::init::Connection,
  error::Result,
};
use chrono::Utc;
use dashmap::DashMap;
use rand::{RngExt, distr::Alphanumeric};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::{spawn, time::sleep};
use tower_governor::GovernorLayer;
use uuid::Uuid;

use crate::{
  auth::cli_auth::CLI_TOKEN_LEN,
  db::DBTrait,
  updater::{UpdateMessage, Updater},
};

pub fn router(rate_limiter: &mut RateLimiter) -> ApiRouter {
  ApiRouter::new()
    .api_route("/", put_with(get_token, |op| op.id("getToken")))
    .layer(GovernorLayer::new(rate_limiter.create_limiter()))
    .api_route("/", post_with(new_code, |op| op.id("newCode")))
}

pub fn state(router: ApiRouter) -> ApiRouter {
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

#[derive(FromRequestParts, Clone, OperationIo)]
#[from_request(via(Extension))]
struct CliState {
  codes: Arc<DashMap<Uuid, (Instant, Uuid)>>,
}

#[derive(Serialize, JsonSchema)]
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

#[derive(Clone, Deserialize, JsonSchema)]
struct TokenReq {
  code: Uuid,
}

async fn get_token(
  Query(token_req): Query<TokenReq>,
  state: CliState,
  db: Connection,
  jwt: JwtState,
  pw: PasswordState,
  updater: Updater,
) -> Result<String> {
  let Some((instant, user)) = state.codes.get(&token_req.code).map(|entry| *entry.value()) else {
    bail!("Invalid code");
  };

  state.codes.remove(&token_req.code);
  if Instant::now().duration_since(instant).as_secs() > 300 {
    bail!("Invalid user or code");
  }

  let Some(exp) = Utc::now().checked_add_signed(chrono::Duration::seconds(jwt.exp)) else {
    bail!(INTERNAL_SERVER_ERROR, "Failed to create token");
  };

  let token: String = gen_token();
  let hash = pw.pw_hash_raw("", &token)?;

  let token_model = db
    .token()
    .insert(
      user,
      format!("Cli-{}", Utc::now().to_rfc3339()),
      hash,
      exp.naive_utc(),
    )
    .await?;
  updater
    .send_to(
      user,
      UpdateMessage::Token {
        uuid: token_model.id,
      },
    )
    .await;

  Ok(token)
}

pub fn gen_token() -> String {
  let mut rng = rand::rng();
  (0..CLI_TOKEN_LEN)
    .map(|_| rng.sample(Alphanumeric) as char)
    .collect()
}
