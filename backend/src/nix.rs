use axum::{
  Router,
  body::Body,
  extract::{FromRequestParts, Path},
  routing::{get, head},
};
use centaurus::{bail, db::init::Connection, error::Result};
use http::{HeaderMap, HeaderName, StatusCode, header};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
  auth::cli_auth::CliAuth,
  cache::storage::FileStorage,
  db::{DBTrait, nar::NarInfoData},
};

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

async fn nix_cache_info(
  db: Connection,
  path: CachePath,
  auth: Option<CliAuth>,
) -> Result<(HeaderMap, String)> {
  let Some(cache) = db.cache().by_id(path.uuid).await? else {
    bail!(NOT_FOUND, "Cache not found");
  };

  if !cache.public {
    let Some(auth) = auth else {
      bail!(UNAUTHORIZED, "Authentication required");
    };

    if db
      .cache()
      .cache_user_access(auth.user_id, cache.id)
      .await?
      .is_none()
    {
      bail!(FORBIDDEN, "Access denied");
    }
  }

  let mut headers = HeaderMap::new();
  headers.insert("Content-Type", "text/x-nix-cache-info".parse().unwrap());

  Ok((
    headers,
    format!(
      "StoreDir: /nix/store
WantMassQuery: 1
Priority: {}
",
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

async fn get_data(
  db: &Connection,
  path: NarInfoPath,
  auth: Option<CliAuth>,
) -> Result<NarInfoData> {
  let Some(hash) = path.path.strip_suffix(".narinfo") else {
    bail!(NOT_FOUND, "Invalid narinfo path");
  };

  if !db.cache().is_public(path.uuid).await? {
    let Some(auth) = auth else {
      bail!(UNAUTHORIZED, "Authentication required");
    };

    if db
      .cache()
      .cache_user_access(auth.user_id, path.uuid)
      .await?
      .is_none()
    {
      bail!(FORBIDDEN, "Access denied");
    }
  }

  let Some(data) = db.nar().nar_info_data(path.uuid, hash).await? else {
    bail!(NOT_FOUND, "Narinfo not found");
  };

  Ok(data)
}

async fn head_nar_info(
  db: Connection,
  path: NarInfoPath,
  auth: Option<CliAuth>,
) -> Result<HeaderMap> {
  get_data(&db, path, auth).await?;

  let mut headers = HeaderMap::new();
  headers.insert("Content-Type", "text/x-nix-narinfo".parse().unwrap());

  Ok(headers)
}

async fn nar_info(
  db: Connection,
  path: NarInfoPath,
  auth: Option<CliAuth>,
) -> Result<(HeaderMap, String)> {
  let data = get_data(&db, path, auth).await?;
  let references = db.nar().nar_info_references(data.id).await?;

  let mut headers = HeaderMap::new();
  headers.insert("Content-Type", "text/x-nix-narinfo".parse().unwrap());

  let compression = match data.compression.as_str() {
    "zst" => "zstd",
    _ => bail!(NOT_FOUND, "Unsupported compression format"),
  };

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
Sig: {}
",
      data.store_path,
      data.hash,
      data.compression,
      compression,
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

#[derive(FromRequestParts, Deserialize)]
#[from_request(via(Path))]
struct NarPath {
  uuid: Uuid,
  hash: String,
}

async fn nar(
  db: Connection,
  path: NarPath,
  storage: FileStorage,
  auth: Option<CliAuth>,
) -> Result<(StatusCode, [(HeaderName, String); 2], Body)> {
  // parse hash as <hash>.nar.<compression>
  let Some((hash, compression)) = path.hash.split_once(".nar.") else {
    bail!(NOT_FOUND, "Invalid nar path");
  };

  if !db.cache().is_public(path.uuid).await? {
    let Some(auth) = auth else {
      bail!(UNAUTHORIZED, "Authentication required");
    };

    if db
      .cache()
      .cache_user_access(auth.user_id, path.uuid)
      .await?
      .is_none()
    {
      bail!(FORBIDDEN, "Access denied");
    }
  }

  let Some((nar_id, file_size)) = db.nar().get_nar(path.uuid, hash, compression).await? else {
    bail!(NOT_FOUND, "Nar not found");
  };
  tracing::info!("Serving nar {} for cache {}", nar_id, path.uuid);

  let body = storage.get_file(nar_id).await?;

  let headers = [
    (header::CONTENT_TYPE, "application/x-nix-nar".into()),
    (header::CONTENT_LENGTH, file_size.to_string()),
  ];

  Ok((StatusCode::OK, headers, body))
}
