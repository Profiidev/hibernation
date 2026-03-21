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
use centaurus::{bail, db::init::Connection, error::Result, eyre::Context};
use dashmap::DashMap;
use entity::sea_orm_active_enums::AccessType;
use futures_util::StreamExt;
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
};
use tokio::{
  io::{self, AsyncReadExt, AsyncWriteExt, BufReader},
  time::sleep,
};
use tracing::warn;
use url::Url;
use uuid::Uuid;

use crate::{auth::cli_auth::CliAuth, db::DBTrait};

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

pub fn state(router: Router) -> Router {
  router.layer(Extension(PushState::new()))
}

struct UploadFinishData {
  cache: Uuid,
  store_path: String,
  nar_hash: String,
  nar_size: u64,
  file_hash: String,
  file_size: u64,
  deriver: Option<String>,
  references: Vec<String>,
  signature: String,
}

#[derive(FromRequestParts, Clone)]
#[from_request(via(Extension))]
struct PushState {
  pending_uploads: Arc<DashMap<Uuid, (UploadPathRequest, Instant)>>,
  pending_finish: Arc<DashMap<Uuid, (UploadFinishData, Instant)>>,
}

impl PushState {
  fn new() -> Self {
    let pending_uploads = Arc::new(DashMap::new());
    let pending_finish = Arc::new(DashMap::new());

    tokio::spawn({
      let pending_uploads = pending_uploads.clone();
      let pending_finish = pending_finish.clone();
      async move {
        loop {
          sleep(Duration::from_secs(60)).await;
          let now = Instant::now();
          pending_uploads
            .retain(|_, (_, uploaded)| now.duration_since(*uploaded) < Duration::from_secs(300));
          pending_finish
            .retain(|_, (_, uploaded)| now.duration_since(*uploaded) < Duration::from_secs(300));
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

  let mut missing_paths = db.cache().missing_paths(cache.id, req.paths).await?;
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

  if !req.force
    && db
      .cache()
      .is_store_path_in_cache(cache.id, &req.store_path.to_string())
      .await?
  {
    bail!("Store path is already in the cache");
  }

  let upload_id = Uuid::new_v4();
  state
    .pending_uploads
    .insert(upload_id, (req, Instant::now()));

  Ok(Json(UploadPathResponse { uuid: upload_id }))
}

#[derive(Deserialize, FromRequestParts)]
#[from_request(via(Path))]
struct UploadNarPath {
  uuid: Uuid,
}

async fn upload_nar(
  _auth: CliAuth,
  state: PushState,
  path: UploadNarPath,
  body: Request,
) -> Result<()> {
  let Some((_, (info, _))) = state.pending_uploads.remove(&path.uuid) else {
    bail!("Invalid upload session");
  };

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

  let mut raw_hasher = Sha256::new();
  let mut file_size = 0;
  let mut stream = body.into_body().into_data_stream();

  while let Some(chunk) = stream.next().await {
    let chunk = chunk.context("Failed to read request body")?;

    file_size += chunk.len() as u64;
    raw_hasher.update(&chunk);
    pipe_writer.write_all(&chunk).await?;
  }
  pipe_writer.shutdown().await?; // Ensure all data is flushed to the decoder
  drop(pipe_writer); // Close the writer to signal EOF to the decoder

  let file_hash = to_nix_base32(&raw_hasher.finalize());
  let (nar_hash, nar_size) = hashing_task.await.context("Hashing task failed")?;

  if nar_hash != info.nar_hash || nar_size != info.nar_size {
    bail!("NAR hash or size mismatch");
  }

  state.pending_finish.insert(
    path.uuid,
    (
      UploadFinishData {
        cache: info.cache,
        store_path: info.store_path.to_string(),
        nar_hash,
        nar_size,
        file_hash,
        file_size,
        signature: String::new(), // TODO: calculate real signature
        deriver: info.deriver.map(|d| d.to_string()),
        references: info.references.into_iter().map(|r| r.to_string()).collect(),
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
  Path(uuid): Path<Uuid>,
  Json(req): Json<UploadFinishRequest>,
) -> Result<()> {
  let Some((_, (data, _))) = state.pending_finish.remove(&uuid) else {
    bail!("Invalid upload session");
  };

  if data.file_hash != req.file_hash || data.file_size != req.file_size {
    bail!("File hash or size mismatch");
  }
  if data.signature != req.signature {
    bail!("Signature mismatch");
  }

  db.cache()
    .create_path(
      data.cache,
      data.store_path,
      data.nar_hash,
      data.nar_size,
      data.file_hash,
      data.file_size,
      data.deriver,
      data.signature,
      data.references,
    )
    .await?;

  Ok(())
}
