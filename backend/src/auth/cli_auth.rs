use std::marker::PhantomData;

use aide::OperationIo;
use axum::{
  RequestPartsExt,
  extract::{FromRequestParts, OptionalFromRequestParts},
};
use axum_extra::{
  TypedHeader,
  headers::{Authorization, authorization::Basic},
};
use centaurus::{
  auth::{jwt::jwt_from_request, pw::PasswordState},
  bail,
  db::init::Connection,
  error::ErrorReport,
  state::extract::StateExtractExt,
};
use chrono::Utc;
use http::request::Parts;
use uuid::Uuid;

use crate::{
  auth::{jwt_auth, jwt_state::JWT_COOKIE_NAME},
  db::DBTrait,
  permissions::{NoPerm, Permission},
  ws::state::{UpdateMessage, Updater},
};

pub const CLI_TOKEN_LEN: usize = 32;

#[derive(Debug, OperationIo)]
pub struct CliAuth<P: Permission = NoPerm> {
  pub user_id: Uuid,
  _perm: PhantomData<P>,
}

impl<S: Sync, P: Permission> FromRequestParts<S> for CliAuth<P> {
  type Rejection = ErrorReport;

  async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
    let token = match jwt_from_request(parts, JWT_COOKIE_NAME).await {
      Ok(token) => token,
      Err(e) => {
        let Some(token) = parts
          .extract::<TypedHeader<Authorization<Basic>>>()
          .await
          .ok()
          .map(|TypedHeader(Authorization(basic))| basic.password().to_string())
        else {
          return Err(e);
        };

        token
      }
    };

    let db = parts.extract_state::<Connection>().await;
    let user = if token.len() == CLI_TOKEN_LEN {
      check_token(&db, parts, token).await?
    } else {
      jwt_auth::check_jwt(&db, parts, token).await?.sub
    };

    jwt_auth::check_user::<P>(&db, user).await?;

    Ok(CliAuth {
      user_id: user,
      _perm: PhantomData,
    })
  }
}

impl<S: Sync, P: Permission> OptionalFromRequestParts<S> for CliAuth<P> {
  type Rejection = ErrorReport;

  async fn from_request_parts(
    parts: &mut Parts,
    state: &S,
  ) -> Result<Option<Self>, Self::Rejection> {
    match <Self as FromRequestParts<S>>::from_request_parts(parts, state).await {
      Ok(auth) => Ok(Some(auth)),
      Err(_) => Ok(None),
    }
  }
}

async fn check_token(
  db: &Connection,
  parts: &mut Parts,
  token: String,
) -> Result<Uuid, ErrorReport> {
  let pw = parts.extract_state::<PasswordState>().await;
  let hash = pw.pw_hash_raw("", &token)?;
  let record = db.token().get_by_token(&hash).await?;

  if record.exp < Utc::now().naive_utc() {
    bail!("CLI token expired");
  }

  db.token().token_used(record.id).await?;
  let updater = parts.extract_state::<Updater>().await;
  updater
    .send_to(record.user_id, UpdateMessage::Token { uuid: record.id })
    .await;

  Ok(record.user_id)
}
