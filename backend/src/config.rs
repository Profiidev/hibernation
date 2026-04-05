use aide::OperationIo;
use axum::{Extension, extract::FromRequestParts};
use centaurus::{
  Config,
  backend::{
    auth::settings::AuthConfig,
    config::{BaseConfig, MetricsConfig, SiteConfig},
  },
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
  #[site]
  #[serde(flatten)]
  pub site: SiteConfig,
  #[serde(flatten)]
  pub auth: AuthConfig,

  pub db_url: String,
  pub site_url: Url,
  pub virtual_host_routing: bool,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      base: BaseConfig::default(),
      db: DBConfig::default(),
      site: SiteConfig::default(),
      db_url: "".to_string(),
      site_url: Url::parse("http://localhost:8000").unwrap(),
      virtual_host_routing: false,
      metrics: MetricsConfig {
        metrics_name: "hibernation".to_string(),
        ..Default::default()
      },
      storage: StorageConfig::default(),
      auth: AuthConfig {
        auth_pepper: "__HIBERNATION_PEPPER__".to_string(),
        ..Default::default()
      },
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
      config.db.validate_sqlite();
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
