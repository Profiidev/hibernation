use aide::axum::{
  ApiRouter,
  routing::{delete, get, post, put},
};
use axum::{Json, extract::Path};
use centaurus::{auth::pw::PasswordState, bail, db::init::Connection, error::Result};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  auth::jwt_auth::JwtAuth,
  cli,
  db::DBTrait,
  ws::state::{UpdateMessage, Updater},
};

pub fn router() -> ApiRouter {
  ApiRouter::new()
    .api_route("/", get(list_tokens))
    .api_route("/", post(create_token))
    .api_route("/", delete(delete_token))
    .api_route("/", put(edit_token))
    .api_route("/{uuid}", get(token_info))
    .api_route("/{uuid}", post(token_regenerate))
}

#[derive(Serialize, JsonSchema)]
struct TokenInfo {
  uuid: Uuid,
  name: String,
  exp: DateTime<Utc>,
  last_used: Option<DateTime<Utc>>,
}

impl From<entity::token::Model> for TokenInfo {
  fn from(value: entity::token::Model) -> Self {
    Self {
      uuid: value.id,
      name: value.name,
      exp: DateTime::from_naive_utc_and_offset(value.exp, Utc),
      last_used: value
        .last_used
        .map(|d| DateTime::from_naive_utc_and_offset(d, Utc)),
    }
  }
}

async fn list_tokens(auth: JwtAuth, db: Connection) -> Result<Json<Vec<TokenInfo>>> {
  let tokens = db.token().get_by_user(auth.user_id).await?;
  let token_info = tokens.into_iter().map(|t| t.into()).collect();
  Ok(Json(token_info))
}

#[derive(Deserialize, JsonSchema)]
struct TokenViewPath {
  uuid: Uuid,
}

async fn token_info(
  auth: JwtAuth,
  db: Connection,
  Path(path): Path<TokenViewPath>,
) -> Result<Json<TokenInfo>> {
  let info = db.token().by_id_user(path.uuid, auth.user_id).await?;
  let Some(info) = info else {
    bail!(NOT_FOUND, "Token not found");
  };
  Ok(Json(info.into()))
}

#[derive(Deserialize, JsonSchema)]
struct CreateTokenRequest {
  name: String,
  exp: DateTime<Utc>,
}

#[derive(Serialize, JsonSchema)]
struct CreateTokenResponse {
  token: String,
  uuid: Uuid,
}

async fn create_token(
  auth: JwtAuth,
  db: Connection,
  pw: PasswordState,
  updater: Updater,
  Json(req): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>> {
  if req.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Token name cannot be empty");
  }

  if db
    .token()
    .get_by_name(auth.user_id, &req.name)
    .await
    .is_ok()
  {
    bail!(CONFLICT, "A token with this name already exists");
  }

  let token = cli::gen_token();
  let hash = pw.pw_hash_raw("", &token)?;

  let uuid = db
    .token()
    .insert(auth.user_id, req.name, hash, req.exp.naive_utc())
    .await?
    .id;
  updater.broadcast(UpdateMessage::Token { uuid }).await;

  Ok(Json(CreateTokenResponse { token, uuid }))
}

#[derive(Deserialize, JsonSchema)]
struct DeleteTokenRequest {
  uuid: Uuid,
}

async fn delete_token(
  auth: JwtAuth,
  db: Connection,
  updater: Updater,
  Json(req): Json<DeleteTokenRequest>,
) -> Result<()> {
  db.token().invalidate(auth.user_id, req.uuid).await?;
  updater
    .broadcast(UpdateMessage::Token { uuid: req.uuid })
    .await;
  Ok(())
}

#[derive(Deserialize, JsonSchema)]
struct EditTokenRequest {
  uuid: Uuid,
  name: String,
  exp: DateTime<Utc>,
}

async fn edit_token(
  auth: JwtAuth,
  db: Connection,
  updater: Updater,
  Json(req): Json<EditTokenRequest>,
) -> Result<()> {
  if req.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Token name cannot be empty");
  }

  if let Ok(token) = db.token().get_by_name(auth.user_id, &req.name).await
    && token.id != req.uuid
  {
    bail!(CONFLICT, "A token with this name already exists");
  }

  db.token()
    .update(auth.user_id, req.uuid, req.name, req.exp.naive_utc())
    .await?;
  updater
    .broadcast(UpdateMessage::Token { uuid: req.uuid })
    .await;
  Ok(())
}

#[derive(Serialize, JsonSchema)]
struct TokenRegenerateResponse {
  token: String,
}

async fn token_regenerate(
  auth: JwtAuth,
  db: Connection,
  pw: PasswordState,
  Path(path): Path<TokenViewPath>,
) -> Result<Json<TokenRegenerateResponse>> {
  let token = cli::gen_token();
  let hash = pw.pw_hash_raw("", &token)?;

  db.token().replace(auth.user_id, path.uuid, hash).await?;

  Ok(Json(TokenRegenerateResponse { token }))
}
