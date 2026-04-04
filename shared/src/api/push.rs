use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct UploadInfoRequest {
  pub cache: String,
  pub paths: Vec<String>,
  pub force: bool,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct UploadInfoResponse {
  pub paths: Vec<String>,
  pub cache: Uuid,
}

#[derive(Deserialize, Serialize, Debug, JsonSchema)]
pub struct UploadPathRequest {
  pub cache: Uuid,
  pub force: bool,
  pub store_path: String,
  pub nar_hash: String,
  pub nar_size: u64,
  pub deriver: Option<String>,
  pub references: Vec<String>,
  pub signature: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct UploadPathResponse {
  pub uuid: Uuid,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct UploadFinishRequest {
  pub file_hash: String,
  pub file_size: u64,
}
