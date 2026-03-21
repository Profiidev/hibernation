use harmonia_store_core::store_path::StorePath;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct UploadInfoRequest {
  pub cache: String,
  pub paths: Vec<StorePath>,
  pub force: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UploadInfoResponse {
  pub paths: Vec<StorePath>,
  pub cache: Uuid,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UploadPathRequest {
  pub cache: Uuid,
  pub force: bool,
  pub store_path: StorePath,
  pub nar_hash: String,
  pub nar_size: u64,
  pub deriver: Option<StorePath>,
  pub references: Vec<StorePath>,
}

#[derive(Serialize, Deserialize)]
pub struct UploadPathResponse {
  pub uuid: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct UploadFinishRequest {
  pub file_hash: String,
  pub file_size: u64,
  pub signature: String,
}
