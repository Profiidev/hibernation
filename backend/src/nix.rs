use std::{sync::Arc, time::Instant};

use aide::{OperationIo, axum::ApiRouter};
use axum::{
  Extension,
  body::Body,
  extract::{FromRequestParts, Path},
  routing::{get, head},
};
use centaurus::{bail, db::init::Connection, error::Result, storage::FileStorage};
use dashmap::DashMap;
use entity::cache;
use http::{HeaderMap, HeaderValue, StatusCode, header};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
  auth::cli_auth::CliAuth,
  db::{DBTrait, nar::NarInfoData},
  utils::fresh,
};

/// Recent cache lookups and access checks, so bursts of narinfo requests only query the narinfo itself
#[derive(Clone, Default, FromRequestParts, OperationIo)]
#[from_request(via(Extension))]
pub struct NixCache {
  caches: Arc<DashMap<String, (Instant, cache::Model)>>,
  access: Arc<DashMap<(Uuid, Uuid), (Instant, bool)>>,
}

impl NixCache {
  pub fn clear(&self) {
    self.caches.clear();
    self.access.clear();
  }
}

/// Resolves the cache by name and checks that the request may read it
async fn readable_cache(
  db: &Connection,
  nix_cache: &NixCache,
  name: String,
  auth: Option<CliAuth>,
) -> Result<cache::Model> {
  let name = name.to_lowercase();
  let cache = match fresh(&nix_cache.caches, &name) {
    Some(cache) => cache,
    None => {
      let Some(cache) = db.cache().by_name(name.clone()).await? else {
        bail!(NOT_FOUND, "Cache not found");
      };
      nix_cache
        .caches
        .insert(name, (Instant::now(), cache.clone()));
      cache
    }
  };

  if cache.public {
    return Ok(cache);
  }

  let Some(auth) = auth else {
    bail!(UNAUTHORIZED, "Authentication required");
  };

  let key = (auth.user_id, cache.id);
  let allowed = match fresh(&nix_cache.access, &key) {
    Some(allowed) => allowed,
    None => {
      let allowed = db
        .cache()
        .cache_user_access(auth.user_id, cache.id)
        .await?
        .is_some();
      nix_cache.access.insert(key, (Instant::now(), allowed));
      allowed
    }
  };

  if !allowed {
    bail!(FORBIDDEN, "Access denied");
  }

  Ok(cache)
}

const CACHE_INFO_MIME: HeaderValue = HeaderValue::from_static("text/x-nix-cache-info");
const NAR_INFO_MIME: HeaderValue = HeaderValue::from_static("text/x-nix-narinfo");
const NAR_MIME: HeaderValue = HeaderValue::from_static("application/x-nix-nar");
const ACCEPT_RANGES: HeaderValue = HeaderValue::from_static("bytes");

/// https://fzakaria.github.io/nix-http-binary-cache-api-spec/#/default
pub fn router() -> ApiRouter {
  ApiRouter::new()
    .route("/{name}/nix-cache-info", get(nix_cache_info))
    .route("/{name}/{path}", head(head_nar_info))
    .route("/{name}/{path}", get(nar_info))
    .route("/{name}/nar/{hash}", get(nar))
}

#[derive(Deserialize, JsonSchema)]
struct CachePath {
  name: String,
}

async fn nix_cache_info(
  db: Connection,
  nix_cache: NixCache,
  Path(path): Path<CachePath>,
  auth: Option<CliAuth>,
) -> Result<(HeaderMap, String)> {
  let cache = readable_cache(&db, &nix_cache, path.name, auth).await?;

  let mut headers = HeaderMap::new();
  headers.insert(header::CONTENT_TYPE, CACHE_INFO_MIME.clone());

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

#[derive(Deserialize, JsonSchema)]
struct NarInfoPath {
  name: String,
  path: String,
}

async fn get_data(
  db: &Connection,
  nix_cache: &NixCache,
  path: NarInfoPath,
  auth: Option<CliAuth>,
) -> Result<NarInfoData> {
  let Some(hash) = path.path.strip_suffix(".narinfo") else {
    bail!(NOT_FOUND, "Invalid narinfo path");
  };

  let cache = readable_cache(db, nix_cache, path.name, auth).await?;

  let Some(data) = db.nar().nar_info_data(cache.id, hash).await? else {
    bail!(NOT_FOUND, "Narinfo not found");
  };

  Ok(data)
}

async fn head_nar_info(
  db: Connection,
  nix_cache: NixCache,
  Path(path): Path<NarInfoPath>,
  auth: Option<CliAuth>,
) -> Result<HeaderMap> {
  get_data(&db, &nix_cache, path, auth).await?;

  let mut headers = HeaderMap::new();
  headers.insert(header::CONTENT_TYPE, NAR_INFO_MIME.clone());

  Ok(headers)
}

async fn nar_info(
  db: Connection,
  nix_cache: NixCache,
  Path(path): Path<NarInfoPath>,
  auth: Option<CliAuth>,
) -> Result<(HeaderMap, String)> {
  let data = get_data(&db, &nix_cache, path, auth).await?;
  let references = db.nar().nar_info_references(data.id).await?;

  let mut headers = HeaderMap::new();
  headers.insert(header::CONTENT_TYPE, NAR_INFO_MIME.clone());

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

#[derive(Deserialize, JsonSchema)]
struct NarPath {
  name: String,
  hash: String,
}

async fn nar(
  db: Connection,
  nix_cache: NixCache,
  Path(path): Path<NarPath>,
  storage: FileStorage,
  auth: Option<CliAuth>,
  headers: HeaderMap,
) -> Result<(StatusCode, HeaderMap, Body)> {
  // parse hash as <hash>.nar.<compression>
  let Some((hash, compression)) = path.hash.split_once(".nar.") else {
    bail!(NOT_FOUND, "Invalid nar path");
  };

  let cache = readable_cache(&db, &nix_cache, path.name, auth).await?;

  let Some((nar_id, file_size)) = db.nar().get_nar(cache.id, hash, compression).await? else {
    bail!(NOT_FOUND, "Nar not found");
  };
  tracing::info!("Serving nar {} for cache {}", nar_id, cache.id);

  db.nar().nar_accessed(nar_id, cache.id).await?;

  let range = headers
    .get(http::header::RANGE)
    .and_then(|h| h.to_str().ok())
    .and_then(|s| parse_range(s, file_size as u64));

  let status = if range.is_some() {
    StatusCode::PARTIAL_CONTENT
  } else {
    StatusCode::OK
  };

  let nar_name = format!("{}.nar", nar_id);
  let body = storage.get_file(&nar_name.into(), range).await?;

  let mut headers = HeaderMap::new();
  headers.insert(header::CONTENT_TYPE, NAR_MIME.clone());
  headers.insert(header::ACCEPT_RANGES, ACCEPT_RANGES.clone());

  if let Some((start, end)) = range {
    headers.insert(
      header::CONTENT_RANGE,
      HeaderValue::from_str(&format!("bytes {}-{}/{}", start, end, file_size)).unwrap(),
    );
    headers.insert(
      header::CONTENT_LENGTH,
      HeaderValue::from_str(&((end - start + 1).to_string())).unwrap(),
    );
  } else {
    headers.insert(header::CONTENT_LENGTH, file_size.into());
  }

  Ok((status, headers, body))
}

fn parse_range(range: &str, file_size: u64) -> Option<(u64, u64)> {
  let parts: Vec<&str> = range.strip_prefix("bytes=")?.split('-').collect();
  let start = parts.first()?.parse::<u64>().ok()?;
  let end = parts
    .get(1)
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(file_size - 1);

  if start < file_size {
    Some((start, end.min(file_size - 1)))
  } else {
    None
  }
}
