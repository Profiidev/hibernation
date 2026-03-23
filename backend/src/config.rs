use centaurus::{
  config::{BaseConfig, MetricsConfig},
  db::config::DBConfig,
};
use figment::{
  Figment,
  providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
  #[serde(flatten)]
  pub base: BaseConfig,
  #[serde(flatten)]
  pub db: DBConfig,
  #[serde(flatten)]
  pub metrics: MetricsConfig,
  #[serde(flatten)]
  pub storage: StorageConfig,

  pub db_url: String,

  pub auth_pepper: String,
  pub auth_issuer: String,
  pub auth_jwt_expiration: i64,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      base: BaseConfig::default(),
      db: DBConfig::default(),
      db_url: "".to_string(),
      metrics: MetricsConfig {
        metrics_name: "hibernation".to_string(),
        ..Default::default()
      },
      storage: StorageConfig::default(),
      auth_pepper: "__HIBERNATION_PEPPER__".to_string(),
      auth_issuer: "hibernation_auth".to_string(),
      auth_jwt_expiration: 60 * 60 * 24 * 7, // 7 days
    }
  }
}

impl Config {
  #[instrument]
  pub fn parse() -> Self {
    let config = Figment::new()
      .merge(Serialized::defaults(Self::default()))
      .merge(Env::raw().global());

    let config: Self = config.extract().expect("Failed to parse configuration");

    if config.db_url.is_empty() {
      panic!("DB_URL is not set");
    }

    config.storage.validate();

    config
  }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct StorageConfig {
  pub storage_path: String,
  pub s3_bucket: Option<String>,
  pub s3_region: Option<String>,
  pub s3_host: Option<String>,
  pub s3_access_key: Option<String>,
  pub s3_secret_key: Option<String>,
  pub s3_force_path_style: bool,
}

impl StorageConfig {
  fn validate(&self) {
    if (self.s3_bucket.is_some()
      || self.s3_region.is_some()
      || self.s3_access_key.is_some()
      || self.s3_secret_key.is_some()
      || self.s3_host.is_some())
      && !self.use_s3()
    {
      warn!(
        "Only some S3 config options are set: Bucket: {}, Region: {}, Host: {}, Access Key: {}, Secret Key: {}",
        self.s3_bucket.is_some(),
        self.s3_region.is_some(),
        self.s3_host.is_some(),
        self.s3_access_key.is_some(),
        self.s3_secret_key.is_some()
      );
    }

    if !self.use_s3() && self.storage_path.is_empty() {
      panic!("STORAGE_PATH is not set and S3 config is incomplete");
    }
  }

  pub fn use_s3(&self) -> bool {
    self.s3_bucket.is_some()
      && self.s3_region.is_some()
      && self.s3_access_key.is_some()
      && self.s3_secret_key.is_some()
      && self.s3_host.is_some()
  }
}
