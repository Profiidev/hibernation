use argon2::password_hash::SaltString;
use axum::{
  Json, Router,
  extract::{FromRequest, FromRequestParts, Path},
  routing::{delete, get, post, put},
};
use base64::prelude::*;
use centaurus::{
  auth::pw::PasswordState,
  bail,
  db::init::Connection,
  error::{ErrorReportStatusExt, Result},
};
use http::StatusCode;
use rand::RngExt;
use rsa::rand_core::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  auth::jwt_auth::JwtAuth,
  config::Config,
  db::{
    DBTrait,
    cache::SimpleCacheInfo,
    group::CacheMapping,
    user::{DetailUserInfo, SimpleGroupInfo, UserInfo},
  },
  mail::{state::Mailer, templates},
  permissions::{CacheEdit, Permission, UserEdit, UserView},
  ws::state::{UpdateMessage, Updater},
};

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789)(*&^%$#@!~";

pub fn router() -> Router {
  Router::new()
    .route("/", get(list_users))
    .route("/", post(create_user))
    .route("/", delete(delete_user))
    .route("/", put(edit_user))
    .route("/{uuid}", get(user_info))
    .route("/mail", get(mail_active))
    .route("/groups", get(list_groups_simple))
    .route("/caches", get(list_caches_simple))
    .route("/avatar", delete(reset_user_avatar))
    .route("/password", put(reset_user_password))
}

async fn list_users(_auth: JwtAuth<UserView>, db: Connection) -> Result<Json<Vec<UserInfo>>> {
  let users = db.user().list_users().await?;
  Ok(Json(users))
}

#[derive(Deserialize, FromRequestParts)]
#[from_request(via(Path))]
struct UserViewPath {
  uuid: Uuid,
}

async fn user_info(
  _auth: JwtAuth<UserView>,
  db: Connection,
  path: UserViewPath,
) -> Result<Json<DetailUserInfo>> {
  let info = db.user().user_info(path.uuid).await?;
  let Some(info) = info else {
    bail!(NOT_FOUND, "User not found");
  };
  Ok(Json(info))
}

#[derive(Serialize)]
struct MailActiveResponse {
  active: bool,
}

async fn mail_active(_auth: JwtAuth<UserView>, mailer: Mailer) -> Result<Json<MailActiveResponse>> {
  let active = mailer.is_active().await;
  Ok(Json(MailActiveResponse { active }))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct CreateUser {
  name: String,
  email: String,
  password: Option<String>,
}

#[derive(Serialize)]
struct CreateUserResponse {
  uuid: Uuid,
}

async fn create_user(
  _auth: JwtAuth<UserEdit>,
  db: Connection,
  updater: Updater,
  mailer: Mailer,
  state: PasswordState,
  config: Config,
  req: CreateUser,
) -> Result<Json<CreateUserResponse>> {
  if req.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Name cannot be empty");
  }

  if req.email.trim().is_empty() {
    bail!(BAD_REQUEST, "Email cannot be empty");
  }

  if db.user().try_get_user_by_email(&req.email).await?.is_some() {
    bail!(CONFLICT, "User with this email already exists");
  }

  let password = if mailer.is_active().await {
    let mut rng = rand::rng();
    (0..12)
      .map(|_| {
        let idx = rng.random_range(0..CHARSET.len());
        CHARSET[idx] as char
      })
      .collect::<String>()
  } else if let Some(pw) = req.password {
    let bytes = BASE64_STANDARD.decode(pw).status(StatusCode::BAD_REQUEST)?;
    let pw_bytes = state.decrypt(&bytes).status(StatusCode::BAD_REQUEST)?;
    String::from_utf8_lossy(&pw_bytes).to_string()
  } else {
    bail!(
      BAD_REQUEST,
      "Password must be provided when mail service is not active"
    );
  };

  let salt = SaltString::generate(OsRng {}).to_string();
  let password_hash = state.pw_hash_raw(&salt, &password)?;

  let user_id = db
    .user()
    .create_user(req.name.clone(), req.email.clone(), password_hash, salt)
    .await?;
  if mailer.is_active().await {
    let subject = "Your new account";
    mailer
      .send_mail(
        req.name,
        req.email,
        subject.to_string(),
        templates::init_password(config.site_url.as_str(), &password),
      )
      .await?;
  }
  updater
    .broadcast(UpdateMessage::User { uuid: user_id })
    .await;

  Ok(Json(CreateUserResponse { uuid: user_id }))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct DeleteUserRequest {
  uuid: Uuid,
}

async fn delete_user(
  auth: JwtAuth<UserEdit>,
  db: Connection,
  updater: Updater,
  data: DeleteUserRequest,
) -> Result<()> {
  let Some(admin_group) = db.setup().get_admin_group_id().await? else {
    bail!(INTERNAL_SERVER_ERROR, "Admin group is not set up");
  };

  if db.group().is_last_admin(admin_group, data.uuid).await? {
    bail!(CONFLICT, "Cannot delete the last user from the admin group");
  }

  if db.group().is_in_group(admin_group, data.uuid).await?
    && !db.group().is_in_group(admin_group, auth.user_id).await?
  {
    bail!(
      FORBIDDEN,
      "User cannot delete another user with higher permissions"
    );
  }

  db.user().delete_user(data.uuid).await?;
  updater
    .broadcast(UpdateMessage::User { uuid: data.uuid })
    .await;

  Ok(())
}

async fn list_groups_simple(
  _auth: JwtAuth<UserView>,
  db: Connection,
) -> Result<Json<Vec<SimpleGroupInfo>>> {
  let groups = db.group().list_groups_simple().await?;
  Ok(Json(groups))
}

async fn list_caches_simple(
  auth: JwtAuth<UserView>,
  db: Connection,
) -> Result<Json<Vec<SimpleCacheInfo>>> {
  let users = db.cache().list_caches_simple(auth.user_id).await?;
  Ok(Json(users))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct UserEditReq {
  uuid: Uuid,
  name: String,
  groups: Vec<Uuid>,
  caches: Vec<CacheMapping>,
}

async fn edit_user(
  auth: JwtAuth<UserEdit>,
  db: Connection,
  updater: Updater,
  req: UserEditReq,
) -> Result<()> {
  if req.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Name cannot be empty");
  }

  let self_permissions = db.group().get_user_permissions(auth.user_id).await?;
  let target_permissions = db
    .group()
    .get_groups_permissions(req.groups.clone())
    .await?;
  let current_user_permissions = db.group().get_user_permissions(req.uuid).await?;

  if target_permissions
    .iter()
    .any(|p| !self_permissions.contains(p))
    || current_user_permissions
      .iter()
      .any(|p| !self_permissions.contains(p))
  {
    bail!(
      FORBIDDEN,
      "Cannot assign permissions that the editor does not have"
    );
  }

  let Some(admin_group) = db.setup().get_admin_group_id().await? else {
    bail!(INTERNAL_SERVER_ERROR, "Admin group is not set up");
  };

  if !req.groups.contains(&admin_group) && db.group().is_last_admin(admin_group, req.uuid).await? {
    bail!(CONFLICT, "Cannot remove the last user from the admin group");
  }

  let Some(user) = db.user().user_info(req.uuid).await? else {
    bail!(NOT_FOUND, "User not found");
  };

  // only allow changing cache mappings if the user has the general cache edit permission
  if user.caches != req.caches && !self_permissions.contains(&CacheEdit::name().to_string()) {
    bail!(
      FORBIDDEN,
      "Cannot edit cache mappings via the group edit endpoint"
    );
  }

  db.user().edit_user(req.uuid, req.name, req.groups).await?;
  db.user()
    .update_cache_mappings(req.uuid, user.caches, req.caches)
    .await?;
  updater
    .broadcast(UpdateMessage::User { uuid: req.uuid })
    .await;

  Ok(())
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct UserAvatarResetRequest {
  uuid: Uuid,
}

async fn reset_user_avatar(
  _auth: JwtAuth<UserEdit>,
  db: Connection,
  updater: Updater,
  req: UserAvatarResetRequest,
) -> Result<()> {
  db.user().reset_avatar(req.uuid).await?;
  updater
    .broadcast(UpdateMessage::User { uuid: req.uuid })
    .await;

  Ok(())
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct ResetUserPassword {
  uuid: Uuid,
  new_password: String,
}

async fn reset_user_password(
  auth: JwtAuth<UserEdit>,
  db: Connection,
  state: PasswordState,
  mailer: Mailer,
  req: ResetUserPassword,
) -> Result<()> {
  if mailer.is_active().await {
    bail!(
      BAD_REQUEST,
      "Cannot reset password when mail service is active"
    );
  }

  let self_permissions = db.group().get_user_permissions(auth.user_id).await?;
  let target_permissions = db.group().get_user_permissions(req.uuid).await?;

  if target_permissions
    .iter()
    .any(|p| !self_permissions.contains(p))
  {
    bail!(
      FORBIDDEN,
      "Cannot reset password for a user with higher permissions"
    );
  }

  let user = db.user().get_user_by_id(req.uuid).await?;
  let hash = state.pw_hash(&user.salt, &req.new_password)?;
  db.user().update_user_password(req.uuid, hash).await?;

  Ok(())
}
