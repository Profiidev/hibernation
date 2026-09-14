use aide::axum::routing::{post_with, put_with};
use std::{
  io::Write,
  sync::Arc,
  time::{Duration, Instant},
};

use aide::{OperationIo, axum::ApiRouter};
use axum::{
  Extension, Json,
  extract::{DefaultBodyLimit, FromRequestParts, Path, Request},
};
use centaurus::{
  bail,
  db::init::Connection,
  error::{ErrorReportStatusExt, Result},
  eyre::Context,
  storage::FileStorage,
};
use dashmap::DashMap;
use entity::sea_orm_active_enums::AccessType;
use futures_util::{StreamExt, TryStreamExt};
use harmonia_store_core::store_path::StorePath;
use http::StatusCode;
use schemars::JsonSchema;
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
  io::{self, AsyncRead, AsyncReadExt},
  sync::Semaphore,
  time::sleep,
};
use tokio_util::io::{InspectReader, StreamReader};
use url::Url;
use uuid::Uuid;

use crate::{
  auth::cli_auth::CliAuth, cache::state::CacheEvictionState, db::DBTrait, utils::client,
};

pub fn router() -> ApiRouter {
  ApiRouter::new()
    .api_route("/info", post_with(upload_info, |op| op.id("uploadInfo")))
    .api_route("/", post_with(upload_path, |op| op.id("uploadPath")))
    .api_route(
      "/{uuid}",
      post_with(upload_nar, |op| op.id("uploadNar")).layer(DefaultBodyLimit::disable()),
    )
    .api_route(
      "/{uuid}",
      put_with(upload_finish, |op| op.id("uploadFinish")),
    )
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

struct NarHasher {
  hasher: Sha256,
  size: u64,
  max_size: u64,
}

impl Write for NarHasher {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.size += buf.len() as u64;
    if self.size > self.max_size {
      return Err(std::io::Error::other(
        "Decompressed NAR exceeds declared size",
      ));
    }
    self.hasher.update(buf);
    Ok(buf.len())
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

#[derive(FromRequestParts, Clone, OperationIo)]
#[from_request(via(Extension))]
pub struct PushState {
  pending_uploads: Arc<DashMap<Uuid, (UploadPathRequest, i64, Instant)>>,
  pending_finish: Arc<DashMap<Uuid, (UploadFinishData, Instant)>>,
  upload_limit: Arc<Semaphore>,
}

impl PushState {
  pub fn new(max_concurrent_uploads: usize) -> Self {
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
      upload_limit: Arc::new(Semaphore::new(max_concurrent_uploads)),
    }
  }
}

async fn upload_info(
  auth: CliAuth,
  db: Connection,
  Json(req): Json<UploadInfoRequest>,
) -> Result<Json<UploadInfoResponse>> {
  let paths = req
    .paths
    .iter()
    .filter_map(|p| StorePath::from_base_path(p).ok())
    .collect::<Vec<_>>();
  if paths.len() != req.paths.len() {
    bail!(UNPROCESSABLE_ENTITY, "One or more invalid store paths");
  }

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

  let mut missing_paths = db.nar().missing_paths(cache.id, paths).await?;
  if missing_paths.is_empty() {
    bail!(NO_CONTENT, "All paths are already present in the cache");
  }

  if req.force {
    return Ok(Json(UploadInfoResponse {
      paths: missing_paths.into_iter().map(|p| p.to_string()).collect(),
      cache: cache.id,
    }));
  }

  let mut downstream_caches = db.cache().downstream_caches(cache.id).await?;
  let client = client();
  while let Some(downstream) = downstream_caches.pop() {
    let mut futures = Vec::new();

    for path in missing_paths {
      let url = Url::parse(&downstream.url)
        .unwrap()
        .join(&format!("{}.narinfo", path.hash()))?;
      let req = client.head(url).build()?;
      let res_future = client.execute(req);

      futures.push(async move {
        let Ok(res) = res_future.await else {
          return (false, path);
        };
        let Ok(res) = res.error_for_status() else {
          return (false, path);
        };

        (res.status() == reqwest::StatusCode::OK, path)
      });
    }

    let results = FuturePool::new(futures).run().await;
    let mut remaining_missing = Vec::new();

    for result in results.into_iter() {
      if let Ok((exists, path)) = result
        && !exists
      {
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
    paths: missing_paths.into_iter().map(|p| p.to_string()).collect(),
    cache: cache.id,
  }))
}

async fn upload_path(
  auth: CliAuth,
  db: Connection,
  state: PushState,
  Json(req): Json<UploadPathRequest>,
) -> Result<Json<UploadPathResponse>> {
  let Ok(store_path) = StorePath::from_base_path(&req.store_path) else {
    bail!(UNPROCESSABLE_ENTITY, "Invalid store path format");
  };
  let references = req
    .references
    .iter()
    .flat_map(|r| StorePath::from_base_path(r).ok())
    .collect::<Vec<_>>();
  if references.len() != req.references.len() {
    bail!(
      UNPROCESSABLE_ENTITY,
      "One or more invalid reference store paths"
    );
  }

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
    &store_path,
    &req.nar_hash,
    req.nar_size,
    &references,
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
    let client = client();
    while let Some(downstream) = downstream_caches.pop() {
      let url = Url::parse(&downstream.url)
        .unwrap()
        .join(&format!("{}.narinfo", store_path.hash()))?;
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

#[derive(Deserialize, Clone, Copy, JsonSchema)]
struct UploadNarPath {
  uuid: Uuid,
}

async fn upload_nar(
  _auth: CliAuth,
  state: PushState,
  Path(path): Path<UploadNarPath>,
  storage: FileStorage,
  db: Connection,
  body: Request,
) -> Result<()> {
  let Some((_, (info, quota, _))) = state.pending_uploads.remove(&path.uuid) else {
    bail!("Invalid upload session");
  };
  let _permit = state
    .upload_limit
    .acquire()
    .await
    .context("Upload limiter closed")?;

  let nar_id = Uuid::now_v7();
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
      let body = StreamReader::new(
        body
          .into_body()
          .into_data_stream()
          .map_err(io::Error::other),
      );
      let (file_hash, file_size) = store_nar(
        &storage,
        body,
        &format!("{}.nar", nar_id),
        quota as u64,
        &info.nar_hash,
        info.nar_size,
      )
      .await?;

      (
        info.nar_hash.clone(),
        info.nar_size,
        file_hash,
        file_size,
        false,
      )
    }
  };

  state.pending_finish.insert(
    path.uuid,
    (
      UploadFinishData {
        cache: info.cache,
        store_path: info.store_path.to_string(),
        store_path_hash: StorePath::from_base_path(&info.store_path)
          .unwrap()
          .hash()
          .to_string(),
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

async fn store_nar<R: AsyncRead + Unpin + Send>(
  storage: &FileStorage,
  body: R,
  name: &str,
  quota: u64,
  nar_hash: &str,
  nar_size: u64,
) -> Result<(String, u64)> {
  let mut file_hasher = Sha256::new();
  let mut file_size = 0;
  let mut nar = zstd::stream::write::Decoder::new(NarHasher {
    hasher: Sha256::new(),
    size: 0,
    max_size: nar_size,
  })?;
  let mut decode_err = None;

  let mut reader = InspectReader::new(body.take(quota.saturating_add(1)), |buf| {
    file_hasher.update(buf);
    file_size += buf.len() as u64;
    if decode_err.is_none()
      && let Err(e) = nar.write_all(buf)
    {
      decode_err = Some(e);
    }
  });
  storage.save_file(&mut reader, name).await?;
  drop(reader);

  let error = if file_size > quota {
    Some("File size exceeds cache quota".to_string())
  } else if let Some(e) = decode_err.or_else(|| nar.flush().err()) {
    Some(format!("Failed to decompress NAR: {e}"))
  } else {
    let decoded = nar.into_inner();
    (to_nix_base32(&decoded.hasher.finalize()) != nar_hash || decoded.size != nar_size)
      .then(|| "NAR hash or size mismatch".to_string())
  };

  if let Some(error) = error {
    storage.delete_file(name).await?;
    bail!("{error}");
  }

  Ok((to_nix_base32(&file_hasher.finalize()), file_size))
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

#[cfg(test)]
mod tests {
  use super::*;

  struct TestStorage {
    storage: FileStorage,
    dir: std::path::PathBuf,
  }

  impl TestStorage {
    fn new() -> Self {
      let dir = std::env::temp_dir().join(format!("hibernation-push-{}", Uuid::new_v4()));
      Self {
        storage: FileStorage::Local(dir.clone()),
        dir,
      }
    }
  }

  impl Drop for TestStorage {
    fn drop(&mut self) {
      let _ = std::fs::remove_dir_all(&self.dir);
    }
  }

  fn nar() -> Vec<u8> {
    (0..200_000u32)
      .flat_map(|i| (i % 251).to_le_bytes())
      .collect()
  }

  fn hash(data: &[u8]) -> String {
    to_nix_base32(&Sha256::digest(data))
  }

  #[tokio::test]
  async fn stores_valid_nar() {
    let s = TestStorage::new();
    let nar = nar();
    let file = zstd::encode_all(&nar[..], 3).unwrap();

    let (file_hash, file_size) = store_nar(
      &s.storage,
      &file[..],
      "a.nar",
      u64::MAX,
      &hash(&nar),
      nar.len() as u64,
    )
    .await
    .unwrap();

    assert_eq!(file_hash, hash(&file));
    assert_eq!(file_size, file.len() as u64);
    assert_eq!(std::fs::read(s.dir.join("a.nar")).unwrap(), file);
  }

  async fn assert_rejected(file: &[u8], quota: u64, nar_hash: &str, nar_size: u64) {
    let s = TestStorage::new();
    let res = store_nar(&s.storage, file, "a.nar", quota, nar_hash, nar_size).await;
    assert!(res.is_err());
    assert!(!s.storage.exists("a.nar").await.unwrap());
  }

  #[tokio::test]
  async fn rejects_over_quota() {
    let nar = nar();
    let file = zstd::encode_all(&nar[..], 3).unwrap();
    assert_rejected(&file, file.len() as u64 - 1, &hash(&nar), nar.len() as u64).await;
  }

  #[tokio::test]
  async fn accepts_exactly_quota() {
    let s = TestStorage::new();
    let nar = nar();
    let file = zstd::encode_all(&nar[..], 3).unwrap();
    let res = store_nar(
      &s.storage,
      &file[..],
      "a.nar",
      file.len() as u64,
      &hash(&nar),
      nar.len() as u64,
    )
    .await;
    assert!(res.is_ok());
  }

  #[tokio::test]
  async fn rejects_invalid_zstd() {
    let nar = nar();
    assert_rejected(&nar, u64::MAX, &hash(&nar), nar.len() as u64).await;
  }

  #[tokio::test]
  async fn rejects_truncated_zstd() {
    let nar = nar();
    let file = zstd::encode_all(&nar[..], 3).unwrap();
    assert_rejected(
      &file[..file.len() / 2],
      u64::MAX,
      &hash(&nar),
      nar.len() as u64,
    )
    .await;
  }

  #[tokio::test]
  async fn rejects_hash_mismatch() {
    let nar = nar();
    let file = zstd::encode_all(&nar[..], 3).unwrap();
    assert_rejected(&file, u64::MAX, &hash(b"other"), nar.len() as u64).await;
  }

  #[tokio::test]
  async fn rejects_larger_than_declared_size() {
    let nar = nar();
    let file = zstd::encode_all(&nar[..], 3).unwrap();
    assert_rejected(&file, u64::MAX, &hash(&nar), nar.len() as u64 - 1).await;
  }

  #[tokio::test]
  async fn rejects_smaller_than_declared_size() {
    let nar = nar();
    let file = zstd::encode_all(&nar[..], 3).unwrap();
    assert_rejected(&file, u64::MAX, &hash(&nar), nar.len() as u64 + 1).await;
  }

  #[tokio::test]
  async fn limits_concurrent_uploads() {
    let state = PushState::new(3);
    let permits = (0..3)
      .map(|_| state.upload_limit.clone().try_acquire_owned().unwrap())
      .collect::<Vec<_>>();
    assert!(state.upload_limit.try_acquire().is_err());
    drop(permits);
    assert!(state.upload_limit.try_acquire().is_ok());
  }
}
