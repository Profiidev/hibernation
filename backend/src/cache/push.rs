use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use async_compression::tokio::bufread::ZstdDecoder;
use axum::{
  Extension, Json, Router,
  extract::{DefaultBodyLimit, FromRequestParts, Path, Request},
  routing::{post, put},
};
use centaurus::{
  bail,
  db::init::Connection,
  error::{ErrorReportStatusExt, Result},
  eyre::Context,
};
use dashmap::DashMap;
use entity::sea_orm_active_enums::AccessType;
use futures_util::StreamExt;
use http::StatusCode;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use shared::{
  api::push::{
    UploadFinishRequest, UploadInfoRequest, UploadInfoResponse, UploadPathRequest,
    UploadPathResponse,
  },
  hash::to_nix_base32,
  pool::FuturePool,
  sig::PublicKey,
};
use tokio::{
  io::{self, AsyncReadExt, AsyncWriteExt, BufReader},
  time::sleep,
};
use tracing::warn;
use url::Url;
use uuid::Uuid;

use crate::{
  auth::cli_auth::CliAuth,
  cache::{state::CacheEvictionState, storage::FileStorage},
  db::DBTrait,
};

pub fn router() -> Router {
  Router::new()
    .route("/info", post(upload_info))
    .route("/", post(upload_path))
    .route(
      "/{uuid}",
      post(upload_nar).layer(DefaultBodyLimit::disable()),
    )
    .route("/{uuid}", put(upload_finish))
}

struct UploadFinishData {
  cache: Uuid,
  store_path: String,
  store_path_hash: String,
  nar_hash: String,
  nar_size: u64,
  file_hash: String,
  file_size: u64,
  deriver: Option<String>,
  references: Vec<String>,
  signature: String,
  nar_id: Uuid,
  nar_found: bool,
}

#[derive(FromRequestParts, Clone)]
#[from_request(via(Extension))]
pub struct PushState {
  pending_uploads: Arc<DashMap<Uuid, (UploadPathRequest, i64, Instant)>>,
  pending_finish: Arc<DashMap<Uuid, (UploadFinishData, Instant)>>,
}

impl PushState {
  pub fn new() -> Self {
    let pending_uploads = Arc::new(DashMap::new());
    let pending_finish: Arc<DashMap<Uuid, (UploadFinishData, Instant)>> = Arc::new(DashMap::new());

    tokio::spawn({
      let pending_uploads = pending_uploads.clone();
      let pending_finish = pending_finish.clone();
      async move {
        loop {
          sleep(Duration::from_secs(60)).await;
          let now = Instant::now();
          pending_uploads
            .retain(|_, (_, _, uploaded)| now.duration_since(*uploaded) < Duration::from_secs(300));
          pending_finish
            .retain(|_, (_, uploaded)| now.duration_since(*uploaded) < Duration::from_secs(120));
        }
      }
    });

    Self {
      pending_uploads,
      pending_finish,
    }
  }
}

async fn upload_info(
  auth: CliAuth,
  db: Connection,
  Json(req): Json<UploadInfoRequest>,
) -> Result<Json<UploadInfoResponse>> {
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

  let mut missing_paths = db.nar().missing_paths(cache.id, req.paths).await?;
  if missing_paths.is_empty() {
    bail!(NO_CONTENT, "All paths are already present in the cache");
  }

  if req.force {
    return Ok(Json(UploadInfoResponse {
      paths: missing_paths,
      cache: cache.id,
    }));
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

  Ok(Json(UploadInfoResponse {
    paths: missing_paths,
    cache: cache.id,
  }))
}

async fn upload_path(
  auth: CliAuth,
  db: Connection,
  state: PushState,
  Json(req): Json<UploadPathRequest>,
) -> Result<Json<UploadPathResponse>> {
  let Some(cache) = db
    .cache()
    .by_id_filtered(req.cache, auth.user_id, AccessType::Edit)
    .await?
  else {
    bail!("Cache not found or access denied");
  };

  if !cache.allow_force_push && req.force {
    bail!("Force push is not allowed for this cache");
  }

  let pk =
    PublicKey::from_string(&cache.public_signing_key).status(StatusCode::INTERNAL_SERVER_ERROR)?;
  if !pk.verify(
    &req.signature,
    &req.store_path,
    &req.nar_hash,
    req.nar_size,
    &req.references,
  ) {
    bail!("Invalid signature");
  }

  if db
    .nar()
    .is_store_path_in_cache(cache.id, &req.store_path.to_string())
    .await?
  {
    bail!("Store path is already in the cache");
  }

  if !req.force {
    let mut downstream_caches = db.cache().downstream_caches(cache.id).await?;
    let client = Client::new();
    while let Some(downstream) = downstream_caches.pop() {
      let url = Url::parse(&downstream.url)
        .unwrap()
        .join(&format!("{}.narinfo", req.store_path.hash()))?;
      let req = client.head(url).build()?;
      if let Ok(res) = client.execute(req).await
        && let Ok(res) = res.error_for_status()
        && res.status() == reqwest::StatusCode::OK
      {
        bail!("Store path is already in the downstream cache");
      }
    }
  }

  let upload_id = Uuid::new_v4();
  state
    .pending_uploads
    .insert(upload_id, (req, cache.quota, Instant::now()));

  Ok(Json(UploadPathResponse { uuid: upload_id }))
}

#[derive(Deserialize, FromRequestParts, Clone, Copy)]
#[from_request(via(Path))]
struct UploadNarPath {
  uuid: Uuid,
}

async fn upload_nar(
  _auth: CliAuth,
  state: PushState,
  path: UploadNarPath,
  storage: FileStorage,
  db: Connection,
  body: Request,
) -> Result<()> {
  let Some((_, (info, quota, _))) = state.pending_uploads.remove(&path.uuid) else {
    bail!("Invalid upload session");
  };

  let nar_id = Uuid::new_v4();
  let (nar_hash, nar_size, file_hash, file_size, nar_found) = match db
    .nar()
    .create_nar(nar_id, &info.nar_hash, info.nar_size)
    .await?
  {
    Some(existing) => {
      // Just consume the body to avoid client issues, but ignore the content since we already have the NAR
      let mut stream = body.into_body().into_data_stream();
      while let Some(_chunk) = stream.next().await {}

      (
        existing.nar_hash,
        existing.nar_size as u64,
        existing.hash,
        existing.size as u64,
        true,
      )
    }
    None => {
      let (mut pipe_writer, pipe_reader) = io::duplex(64 * 1024);
      let hashing_task = tokio::spawn(async move {
        let mut decompressed_hasher = Sha256::new();
        let mut decoder = ZstdDecoder::new(BufReader::new(pipe_reader));
        let mut buffer = [0u8; 8192];
        let mut size = 0;

        loop {
          let n = match decoder.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
              warn!("Error reading from decoder: {:?}", e);
              return (String::new(), 0);
            }
          };

          size += n as u64;
          decompressed_hasher.update(&buffer[..n]);
        }

        (to_nix_base32(&decompressed_hasher.finalize()), size)
      });

      let (mut storage_writer, mut storage_reader) = io::duplex(64 * 1024);
      let storage_task =
        tokio::spawn(async move { storage.save_file(&mut storage_reader, nar_id).await });

      let mut raw_hasher = Sha256::new();
      let mut file_size = 0;
      let mut stream = body.into_body().into_data_stream();

      while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read request body")?;

        file_size += chunk.len() as u64;
        raw_hasher.update(&chunk);
        pipe_writer.write_all(&chunk).await?;
        storage_writer.write_all(&chunk).await?;

        if file_size > quota as u64 {
          bail!("File size exceeds cache quota");
        }
      }
      pipe_writer.shutdown().await?; // Ensure all data is flushed to the decoder
      storage_writer.shutdown().await?; // Ensure all data is flushed to storage
      drop(pipe_writer); // Close the writer to signal EOF to the decoder
      drop(storage_writer); // Close the writer to signal EOF to storage

      storage_task.await.context("Storage task failed")??;

      let file_hash = to_nix_base32(&raw_hasher.finalize());
      let (nar_hash, nar_size) = hashing_task.await.context("Hashing task failed")?;

      if nar_hash != info.nar_hash || nar_size != info.nar_size {
        bail!("NAR hash or size mismatch");
      }

      (nar_hash, nar_size, file_hash, file_size, false)
    }
  };

  state.pending_finish.insert(
    path.uuid,
    (
      UploadFinishData {
        cache: info.cache,
        store_path: info.store_path.to_string(),
        store_path_hash: info.store_path.hash().to_string(),
        nar_hash,
        nar_size,
        file_hash,
        file_size,
        signature: info.signature,
        deriver: info.deriver.map(|d| d.to_string()),
        references: info.references.into_iter().map(|r| r.to_string()).collect(),
        nar_id,
        nar_found,
      },
      Instant::now(),
    ),
  );

  Ok(())
}

async fn upload_finish(
  _auth: CliAuth,
  db: Connection,
  state: PushState,
  lock: CacheEvictionState,
  Path(uuid): Path<Uuid>,
  Json(req): Json<UploadFinishRequest>,
) -> Result<()> {
  let Some((_, (data, _))) = state.pending_finish.remove(&uuid) else {
    bail!("Invalid upload session");
  };

  if !data.nar_found && (data.file_hash != req.file_hash || data.file_size != req.file_size) {
    bail!("File hash or size mismatch");
  }

  let lock = lock.lock_cache(data.cache).await;
  let Some(info) = db.cache().by_id(data.cache).await? else {
    bail!("Cache not found");
  };

  let Some(size) = db.cache().cache_size(data.cache).await? else {
    bail!("Cache not found");
  };

  if info.quota < data.file_size as i64 {
    bail!("File size exceeds cache quota");
  }

  let diff = (size + data.file_size as i64) - info.quota;
  if diff > 0 {
    db.cache()
      .evict(data.cache, diff, info.eviction_policy)
      .await?;
  }

  db.nar()
    .create_path(
      data.nar_id,
      data.cache,
      data.store_path,
      data.store_path_hash,
      data.nar_hash,
      data.nar_size,
      data.file_hash,
      data.file_size,
      data.deriver,
      data.signature,
      data.references,
    )
    .await?;

  drop(lock); // Release the cache lock as soon as possible

  Ok(())
}
