use aide::OperationIo;
use axum::{Extension, extract::FromRequestParts};
use centaurus::{
  Config,
  backend::config::{BaseConfig, MetricsConfig},
  db::config::DBConfig,
};
use figment::{
  Figment,
  providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use url::Url;

#[derive(Deserialize, Serialize, Clone, FromRequestParts, OperationIo, Config)]
#[from_request(via(Extension))]
pub struct Config {
  #[base]
  #[serde(flatten)]
  pub base: BaseConfig,
  #[serde(flatten)]
  pub db: DBConfig,
  #[metrics]
  #[serde(flatten)]
  pub metrics: MetricsConfig,
  #[serde(flatten)]
  pub storage: StorageConfig,

  pub db_url: String,
  pub site_url: Url,
  pub virtual_host_routing: bool,

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
      site_url: Url::parse("http://localhost:8000").unwrap(),
      virtual_host_routing: false,
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

    let mut config: Self = config.extract().expect("Failed to parse configuration");

    if config.db_url.is_empty() {
      panic!("DB_URL is not set");
    }

    if config.db_url.starts_with("sqlite") {
      if config.db.database_max_connections > 1 {
        config.db.database_max_connections = 1;
        if config.db.database_max_connections != DBConfig::default().database_max_connections {
          warn!(
            "SQLite does not work properly with multiple connections. Setting DATABASE_MAX_CONNECTIONS to 1."
          );
        }
      }

      if config.db.database_min_connections > 1 {
        config.db.database_min_connections = 1;
        if config.db.database_min_connections != DBConfig::default().database_min_connections {
          warn!(
            "SQLite does not work properly with multiple connections. Setting DATABASE_MIN_CONNECTIONS to 1."
          );
        }
      }
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
