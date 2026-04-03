use aide::axum::{ApiRouter, routing::get};
use axum::Json;
use centaurus::{db::init::Connection, error::Result};
use schemars::JsonSchema;
use serde::Serialize;
use uuid::Uuid;

use crate::{auth::jwt_auth::JwtAuth, db::DBTrait};

pub fn router() -> ApiRouter {
  ApiRouter::new().api_route("/", get(info))
}

#[derive(Serialize, JsonSchema)]
struct UserInfo {
  uuid: Uuid,
  name: String,
  email: String,
  permissions: Vec<String>,
  avatar: Option<String>,
}

async fn info(auth: JwtAuth, db: Connection) -> Result<Json<UserInfo>> {
  let user = db.user().get_user_by_id(auth.user_id).await?;
  let permissions = db.group().get_user_permissions(auth.user_id).await?;

  Ok(Json(UserInfo {
    uuid: user.id,
    name: user.name,
    email: user.email,
    permissions,
    avatar: user.avatar,
  }))
}
