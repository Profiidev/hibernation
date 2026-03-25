use axum::{
  Json, Router,
  extract::{FromRequest, FromRequestParts, Path},
  routing::{delete, get, post},
};
use centaurus::{bail, db::init::Connection, error::Result};
use chrono::{DateTime, Utc};
use entity::sea_orm_active_enums::AccessType;
use serde::{Deserialize, Serialize};
use shared::sig::PublicKey;
use uuid::Uuid;

use crate::{
  auth::jwt_auth::JwtAuth,
  db::{
    DBTrait,
    cache::{CacheDetails, CacheInfo, SearchOrder, SearchSort},
  },
  permissions::CacheCreate,
  ws::state::{UpdateMessage, Updater},
};

pub fn router() -> Router {
  Router::new()
    .route("/", get(list_caches))
    .route("/", post(create_cache))
    .route("/", delete(delete_cache))
    .route("/{uuid}", get(cache_details))
    .route("/{uuid}/search", post(search_store_paths))
    .route("/{uuid}", post(edit_cache))
    .route("/{uuid}", delete(clear_cache))
}

async fn list_caches(auth: JwtAuth, db: Connection) -> Result<Json<Vec<CacheInfo>>> {
  Ok(Json(db.cache().list_caches(auth.user_id).await?))
}

#[derive(Deserialize, FromRequestParts)]
#[from_request(via(Path))]
struct CachePath {
  uuid: Uuid,
}

async fn cache_details(
  auth: JwtAuth,
  path: CachePath,
  db: Connection,
) -> Result<Json<CacheDetails>> {
  let Some(details) = db.cache().cache_details(path.uuid, auth.user_id).await? else {
    bail!(NOT_FOUND, "Cache not found");
  };

  Ok(Json(details))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct CreateCacheRequest {
  name: String,
  public: bool,
  quota: i64,
  sig_key: String,
}

#[derive(Serialize)]
struct CreateCacheResponse {
  uuid: Uuid,
}

async fn create_cache(
  auth: JwtAuth<CacheCreate>,
  db: Connection,
  updater: Updater,
  req: CreateCacheRequest,
) -> Result<Json<CreateCacheResponse>> {
  if req.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Cache name cannot be empty");
  }

  if db.cache().by_name(req.name.clone()).await?.is_some() {
    bail!(CONFLICT, "Cache with this name already exists");
  }

  if PublicKey::from_string(&req.sig_key).is_none() {
    bail!(NOT_ACCEPTABLE, "Invalid signature key format");
  }

  let quota = req.quota.max(0) * 1024 * 1024; // Convert from MiB to bytes, ensuring non-negative
  let uuid = db
    .cache()
    .create_cache(req.name, req.public, quota, req.sig_key, auth.user_id)
    .await?;
  updater.broadcast(UpdateMessage::Cache { uuid }).await;

  Ok(Json(CreateCacheResponse { uuid }))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct DeleteCacheRequest {
  uuid: Uuid,
}

async fn delete_cache(
  auth: JwtAuth,
  db: Connection,
  updater: Updater,
  req: DeleteCacheRequest,
) -> Result<()> {
  if db.cache().cache_user_access(auth.user_id, req.uuid).await? != Some(AccessType::Edit) {
    bail!(FORBIDDEN, "Insufficient permissions");
  }

  db.cache().delete_cache(req.uuid).await?;
  updater
    .broadcast(UpdateMessage::Cache { uuid: req.uuid })
    .await;
  Ok(())
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct SearchStorePathsRequest {
  query: String,
  sort: SearchSort,
  order: SearchOrder,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
  store_path: String,
  created_at: DateTime<Utc>,
  last_accessed_at: Option<DateTime<Utc>>,
  size: i64,
  accessed: i64,
}

async fn search_store_paths(
  auth: JwtAuth,
  path: CachePath,
  db: Connection,
  req: SearchStorePathsRequest,
) -> Result<Json<Vec<SearchResult>>> {
  if req.query.trim().is_empty() {
    bail!(BAD_REQUEST, "Query cannot be empty");
  }

  if db
    .cache()
    .cache_user_access(auth.user_id, path.uuid)
    .await?
    != Some(AccessType::Edit)
  {
    bail!(FORBIDDEN, "Insufficient permissions");
  }

  let paths = db
    .cache()
    .search_store_paths(path.uuid, req.query, req.order, req.sort)
    .await?
    .into_iter()
    .map(|entry| SearchResult {
      store_path: entry.store_path,
      created_at: DateTime::from_naive_utc_and_offset(entry.created_at, Utc),
      last_accessed_at: entry
        .last_accessed_at
        .map(|val| DateTime::from_naive_utc_and_offset(val, Utc)),
      size: entry.size,
      accessed: entry.accessed,
    })
    .collect();

  Ok(Json(paths))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct EditCacheRequest {
  name: String,
  priority: i32,
  public: bool,
  quota: i64,
  sig_key: String,
  allow_force_push: bool,
}

async fn edit_cache(
  auth: JwtAuth,
  path: CachePath,
  db: Connection,
  updater: Updater,
  req: EditCacheRequest,
) -> Result<()> {
  if req.priority < 0 {
    bail!(BAD_REQUEST, "Priority must be non-negative");
  }

  if PublicKey::from_string(&req.sig_key).is_none() {
    bail!(NOT_ACCEPTABLE, "Invalid signature key format");
  }

  if req.quota < 0 {
    bail!(BAD_REQUEST, "Quota must be non-negative");
  }

  if db
    .cache()
    .cache_user_access(auth.user_id, path.uuid)
    .await?
    != Some(AccessType::Edit)
  {
    bail!(FORBIDDEN, "Insufficient permissions");
  }

  if db
    .cache()
    .by_name(req.name.clone())
    .await?
    .map_or_else(|| false, |c| c.id != path.uuid)
  {
    bail!(CONFLICT, "Cache with this name already exists");
  }

  db.cache()
    .edit_cache(
      req.name,
      req.public,
      req.quota,
      req.sig_key,
      req.priority,
      req.allow_force_push,
      path.uuid,
    )
    .await?;
  updater
    .broadcast(UpdateMessage::Cache { uuid: path.uuid })
    .await;

  Ok(())
}

async fn clear_cache(auth: JwtAuth, path: CachePath, db: Connection) -> Result<()> {
  if db
    .cache()
    .cache_user_access(auth.user_id, path.uuid)
    .await?
    != Some(AccessType::Edit)
  {
    bail!(FORBIDDEN, "Insufficient permissions");
  }

  db.cache().clear_cache(path.uuid).await?;
  Ok(())
}
