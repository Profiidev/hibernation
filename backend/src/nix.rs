use axum::{
  Router,
  extract::{FromRequestParts, Path},
  routing::{get, head},
};
use centaurus::{bail, db::init::Connection, error::Result};
use http::HeaderMap;
use serde::Deserialize;
use uuid::Uuid;

use crate::db::{DBTrait, nar::NarInfoData};

/// https://fzakaria.github.io/nix-http-binary-cache-api-spec/#/default
pub fn router() -> Router {
  Router::new()
    .route("/{uuid}/nix-cache-info", get(nix_cache_info))
    .route("/{uuid}/{path}", head(head_nar_info))
    .route("/{uuid}/{path}", get(nar_info))
    .route("/{uuid}/nar/{hash}", get(nar))
}

#[derive(FromRequestParts, Deserialize)]
#[from_request(via(Path))]
struct CachePath {
  uuid: Uuid,
}

async fn nix_cache_info(db: Connection, path: CachePath) -> Result<(HeaderMap, String)> {
  let Some(cache) = db.cache().by_id(path.uuid).await? else {
    bail!(NOT_FOUND, "Cache not found");
  };

  let mut headers = HeaderMap::new();
  headers.insert("Content-Type", "text/x-nix-cache-info".parse().unwrap());

  Ok((
    headers,
    format!(
      "StoreDir: /nix/store
WantMassQuery: 1
Priority: {}",
      cache.priority
    ),
  ))
}

#[derive(FromRequestParts, Deserialize)]
#[from_request(via(Path))]
struct NarInfoPath {
  uuid: Uuid,
  path: String,
}

async fn get_data(db: &Connection, path: NarInfoPath) -> Result<NarInfoData> {
  let Some(hash) = path.path.strip_suffix(".narinfo") else {
    bail!(NOT_FOUND, "Invalid narinfo path");
  };

  let Some(data) = db.nar().nar_info_data(path.uuid, hash).await? else {
    bail!(NOT_FOUND, "Narinfo not found");
  };

  Ok(data)
}

async fn head_nar_info(db: Connection, path: NarInfoPath) -> Result<HeaderMap> {
  get_data(&db, path).await?;

  let mut headers = HeaderMap::new();
  headers.insert("Content-Type", "text/x-nix-narinfo".parse().unwrap());

  Ok(headers)
}

async fn nar_info(db: Connection, path: NarInfoPath) -> Result<(HeaderMap, String)> {
  let data = get_data(&db, path).await?;
  let references = db.nar().nar_info_references(data.id).await?;

  let mut headers = HeaderMap::new();
  headers.insert("Content-Type", "text/x-nix-narinfo".parse().unwrap());

  Ok((
    headers,
    format!(
      "StorePath: /nix/store/{}
URL: nar/{}.nar.{}
Compression: {}
FileHash: sha256:{}
FileSize: {}
NarHash: sha256:{}
NarSize: {}
References: {}{}
Sig: {}",
      data.store_path,
      data.hash,
      data.compression,
      data.compression,
      data.hash,
      data.size,
      data.nar_hash,
      data.nar_size,
      references.join(" "),
      data
        .deriver
        .map(|d| format!("\nDeriver: {}", d))
        .unwrap_or_default(),
      data.signature
    ),
  ))
}

async fn nar() {}
