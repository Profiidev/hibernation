use axum::{
  Json, Router,
  extract::FromRequest,
  routing::{delete, get, post, put},
};
use centaurus::{auth::pw::PasswordState, bail, db::init::Connection, error::Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::jwt_auth::JwtAuth, cli, db::DBTrait};

pub fn router() -> Router {
  Router::new()
    .route("/", get(list_tokens))
    .route("/", post(create_toke))
    .route("/", delete(delete_token))
    .route("/", put(edit_token))
}

#[derive(Serialize)]
struct TokenInfo {
  id: Uuid,
  name: String,
  exp: DateTime<Utc>,
  last_used: Option<DateTime<Utc>>,
}

async fn list_tokens(auth: JwtAuth, db: Connection) -> Result<Json<Vec<TokenInfo>>> {
  let tokens = db.token().get_by_user(auth.user_id).await?;
  let token_info = tokens
    .into_iter()
    .map(|t| TokenInfo {
      id: t.id,
      name: t.name,
      exp: DateTime::from_naive_utc_and_offset(t.exp, Utc),
      last_used: t
        .last_used
        .map(|d| DateTime::from_naive_utc_and_offset(d, Utc)),
    })
    .collect();
  Ok(Json(token_info))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct CreateTokenRequest {
  name: String,
  exp: DateTime<Utc>,
}

#[derive(Serialize)]
struct CreateTokenResponse {
  token: String,
}

async fn create_toke(
  auth: JwtAuth,
  db: Connection,
  pw: PasswordState,
  req: CreateTokenRequest,
) -> Result<Json<CreateTokenResponse>> {
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

  db.token()
    .insert(auth.user_id, req.name, hash, req.exp.naive_utc())
    .await?;

  Ok(Json(CreateTokenResponse { token }))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct DeleteTokenRequest {
  id: Uuid,
}

async fn delete_token(auth: JwtAuth, db: Connection, req: DeleteTokenRequest) -> Result<()> {
  db.token().invalidate(auth.user_id, req.id).await?;
  Ok(())
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct EditTokenRequest {
  id: Uuid,
  name: String,
  exp: DateTime<Utc>,
}

async fn edit_token(auth: JwtAuth, db: Connection, req: EditTokenRequest) -> Result<()> {
  if db
    .token()
    .get_by_name(auth.user_id, &req.name)
    .await
    .is_ok()
  {
    bail!(CONFLICT, "A token with this name already exists");
  }

  db.token()
    .update(auth.user_id, req.id, req.name, req.exp.naive_utc())
    .await?;
  Ok(())
}
