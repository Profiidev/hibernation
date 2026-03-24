use axum::{
  Json, Router,
  extract::{FromRequest, FromRequestParts, Path},
  routing::{delete, get, post},
};
use centaurus::{bail, db::init::Connection, error::Result};
use chrono::{DateTime, Utc};
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
};

pub fn router() -> Router {
  Router::new()
    .route("/", get(list_caches))
    .route("/", post(create_cache))
    .route("/", delete(delete_cache))
    .route("/{uuid}", get(cache_details))
    .route("/{uuid}/search", post(search_store_paths))
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
  req: CreateCacheRequest,
) -> Result<Json<CreateCacheResponse>> {
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

  Ok(Json(CreateCacheResponse { uuid }))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct DeleteCacheRequest {
  uuid: Uuid,
}

async fn delete_cache(auth: JwtAuth, db: Connection, req: DeleteCacheRequest) -> Result<()> {
  db.cache().delete_cache(req.uuid, auth.user_id).await?;
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

  let paths = db
    .cache()
    .search_store_paths(path.uuid, auth.user_id, req.query, req.order, req.sort)
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
