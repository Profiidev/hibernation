use aide::axum::ApiRouter;
use aide::axum::routing::{delete_with, get_with, post_with, put_with};
use axum::{Json, extract::Path};
use centaurus::backend::auth::jwt_auth::JwtAuth;
use centaurus::backend::auth::permission::{GroupEdit, GroupView, Permission};
use centaurus::db::tables::ConnectionExt;
use centaurus::db::tables::group::{GroupInfo, SimpleUserInfo};
use centaurus::{bail, db::init::Connection, error::Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::updater::{UpdateMessage, Updater};
use crate::{
  db::{
    DBTrait,
    cache::SimpleCacheInfo,
    group::{CacheMapping, GroupDetails},
  },
  permissions::CacheEdit,
};

pub fn router() -> ApiRouter {
  ApiRouter::new()
    .api_route("/", get_with(list_groups, |op| op.id("listGroups")))
    .api_route("/", post_with(create_group, |op| op.id("createGroup")))
    .api_route("/", delete_with(delete_group, |op| op.id("deleteGroup")))
    .api_route("/", put_with(edit_group, |op| op.id("editGroup")))
    .api_route("/{uuid}", get_with(group_info, |op| op.id("groupInfo")))
    .api_route(
      "/users",
      get_with(list_users_simple, |op| op.id("listUsersSimple")),
    )
    .api_route(
      "/caches",
      get_with(list_caches_simple, |op| op.id("listCachesSimple")),
    )
}

#[derive(Serialize, JsonSchema)]
struct ListGroupResponse {
  groups: Vec<GroupInfo>,
  admin_group: Option<Uuid>,
}

async fn list_groups(_auth: JwtAuth<GroupView>, db: Connection) -> Result<Json<ListGroupResponse>> {
  let groups = db.group().list_groups().await?;
  let admin_group = db.setup().get_admin_group_id().await?;
  Ok(Json(ListGroupResponse {
    groups,
    admin_group,
  }))
}

#[derive(Deserialize, JsonSchema)]
struct GroupViewPath {
  uuid: Uuid,
}

async fn group_info(
  _auth: JwtAuth<GroupView>,
  db: Connection,
  Path(path): Path<GroupViewPath>,
) -> Result<Json<GroupDetails>> {
  let info = db.group_ext().group_info(path.uuid).await?;
  let Some(info) = info else {
    bail!(NOT_FOUND, "Group not found");
  };
  Ok(Json(info))
}

#[derive(Deserialize, JsonSchema)]
struct CreateGroupRequest {
  name: String,
}

#[derive(Serialize, JsonSchema)]
struct GroupCreateResponse {
  uuid: Uuid,
}

async fn create_group(
  _auth: JwtAuth<GroupEdit>,
  db: Connection,
  updater: Updater,
  Json(data): Json<CreateGroupRequest>,
) -> Result<Json<GroupCreateResponse>> {
  if data.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Group name cannot be empty");
  }

  if db.group().find_group_by_name(&data.name).await?.is_some() {
    bail!(CONFLICT, "A group with this name already exists");
  }

  let group_id = db.group().create_group(data.name).await?;
  updater
    .broadcast(UpdateMessage::Group { uuid: group_id })
    .await;

  Ok(Json(GroupCreateResponse { uuid: group_id }))
}

#[derive(Deserialize, JsonSchema)]
struct DeleteGroupRequest {
  uuid: Uuid,
}

async fn delete_group(
  _auth: JwtAuth<GroupEdit>,
  db: Connection,
  updater: Updater,
  Json(data): Json<DeleteGroupRequest>,
) -> Result<()> {
  if let Some(admin_group) = db.setup().get_admin_group_id().await?
    && admin_group == data.uuid
  {
    bail!(BAD_REQUEST, "Cannot delete the admin group");
  }

  let users = db.group().get_group_users_ids(data.uuid).await?;
  db.group().delete_group(data.uuid).await?;

  updater
    .broadcast(UpdateMessage::Group { uuid: data.uuid })
    .await;
  for user_id in users {
    updater
      .send_to(user_id, UpdateMessage::UserPermissions)
      .await;
  }

  Ok(())
}

#[derive(Deserialize, JsonSchema)]
struct EditGroupRequest {
  uuid: Uuid,
  name: String,
  permissions: Vec<String>,
  users: Vec<Uuid>,
  caches: Vec<CacheMapping>,
}

async fn edit_group(
  auth: JwtAuth<GroupEdit>,
  db: Connection,
  updater: Updater,
  Json(data): Json<EditGroupRequest>,
) -> Result<()> {
  if data.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Group name cannot be empty");
  }

  if let Some(admin_group) = db.setup().get_admin_group_id().await?
    && admin_group == data.uuid
  {
    bail!(BAD_REQUEST, "Cannot edit the admin group");
  }

  if let Some(existing_group) = db.group().find_group_by_name(&data.name).await?
    && existing_group != data.uuid
  {
    bail!(CONFLICT, "A group with this name already exists");
  }

  let Some(group) = db.group_ext().group_info(data.uuid).await? else {
    bail!(NOT_FOUND, "Group not found");
  };

  let user_permissions = db.group().get_user_permissions(auth.user_id).await?;
  if group
    .permissions
    .iter()
    .any(|perm| !user_permissions.contains(perm))
  {
    bail!(
      FORBIDDEN,
      "Cannot edit a group with permissions you do not have"
    );
  }
  if data
    .permissions
    .iter()
    .any(|perm| !user_permissions.contains(perm))
  {
    bail!(
      FORBIDDEN,
      "Cannot assign permissions you do not have to a group"
    );
  }

  // only allow changing cache mappings if the user has the general cache edit permission
  if group.caches != data.caches && !user_permissions.contains(&CacheEdit::name().to_string()) {
    bail!(
      FORBIDDEN,
      "Cannot edit cache mappings via the group edit endpoint"
    );
  }

  let old_users = db.group().get_group_users_ids(data.uuid).await?;

  db.group()
    .edit_group(
      data.uuid,
      data.name,
      data.permissions.clone(),
      data.users.clone(),
    )
    .await?;
  db.group_ext()
    .update_cache_mappings(data.uuid, group.caches, data.caches)
    .await?;

  updater
    .broadcast(UpdateMessage::Group { uuid: data.uuid })
    .await;

  let permissions_changed = group.permissions.len() != data.permissions.len()
    || group
      .permissions
      .iter()
      .any(|perm| !data.permissions.contains(perm));

  let mut users_to_notify = old_users.clone();
  users_to_notify.extend(data.users.clone());
  users_to_notify.sort_unstable();
  users_to_notify.dedup();

  // Only notify users that where added or removed
  if !permissions_changed {
    users_to_notify.retain(|user_id| {
      let in_old = old_users.contains(user_id);
      let in_new = data.users.contains(user_id);
      in_old != in_new
    });
  }

  for user_id in users_to_notify {
    updater
      .send_to(user_id, UpdateMessage::UserPermissions)
      .await;
  }

  Ok(())
}

async fn list_users_simple(
  _auth: JwtAuth<GroupView>,
  db: Connection,
) -> Result<Json<Vec<SimpleUserInfo>>> {
  let users = db.user().list_users_simple().await?;
  Ok(Json(users))
}

async fn list_caches_simple(
  auth: JwtAuth<GroupView>,
  db: Connection,
) -> Result<Json<Vec<SimpleCacheInfo>>> {
  let users = db.cache().list_caches_simple(auth.user_id).await?;
  Ok(Json(users))
}
