use axum::{Json, Router, extract::FromRequest, routing::post};
use centaurus::{bail, db::init::Connection, error::Result};
use entity::sea_orm_active_enums::AccessType;
use harmonia_store_core::store_path::StorePath;
use reqwest::Client;
use serde::Deserialize;
use url::Url;

use crate::{auth::cli_auth::CliAuth, cache::pool::FuturePool, db::DBTrait};

pub fn router() -> Router {
  Router::new().route("/info", post(upload_info))
}

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json))]
struct UploadInfoRequest {
  cache: String,
  paths: Vec<StorePath>,
  force: bool,
}

async fn upload_info(
  auth: CliAuth,
  db: Connection,
  req: UploadInfoRequest,
) -> Result<Json<Vec<StorePath>>> {
  let Some(cache) = db
    .cache()
    .by_name_filtered(req.cache, auth.user_id, AccessType::Edit)
    .await?
  else {
    bail!(NOT_FOUND, "Cache not found or access denied");
  };

  if !cache.allow_force_push && req.force {
    bail!(NOT_ACCEPTABLE, "Force push is not allowed for this cache");
  }

  let mut missing_paths = db.cache().missing_paths(cache.id, req.paths).await?;
  if missing_paths.is_empty() {
    bail!(NO_CONTENT, "All paths are already present in the cache");
  }

  let mut downstream_caches = db.cache().downstream_caches(cache.id).await?;
  let client = Client::new();
  while let Some(downstream) = downstream_caches.pop() {
    let mut futures = Vec::new();

    for path in &missing_paths {
      let url = Url::parse(&downstream.url)
        .unwrap()
        .join(&format!("{}.narinfo", path.hash()))?;
      let req = client.head(url).build()?;
      let res_future = client.execute(req);

      futures.push(async move {
        let Ok(res) = res_future.await else {
          return false;
        };
        let Ok(res) = res.error_for_status() else {
          return false;
        };

        res.status() == reqwest::StatusCode::OK
      });
    }

    let results = FuturePool::new(futures).run().await;
    let mut remaining_missing = Vec::new();

    for (path, exists) in missing_paths.into_iter().zip(results) {
      if !exists.unwrap_or(false) {
        remaining_missing.push(path);
      }
    }

    if remaining_missing.is_empty() {
      bail!(
        NO_CONTENT,
        "All paths are already present in the downstream cache"
      );
    }
    missing_paths = remaining_missing;
  }

  Ok(Json(missing_paths))
}
