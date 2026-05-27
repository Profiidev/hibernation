use aide::OperationIo;
use axum::{Extension, extract::FromRequestParts};
use centaurus::{
  Config,
  backend::{
    auth::settings::{AuthConfig, UserSettings},
    config::{BaseConfig, MetricsConfig, SiteConfig},
  },
  db::config::DBConfig,
  mail::MailSettings,
  storage::StorageConfig,
};
use figment::{
  Figment,
  providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};

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
  #[auth]
  #[serde(flatten)]
  pub auth: AuthConfig,
  #[mail]
  #[serde(flatten)]
  pub mail: MailSettings,
  #[oidc]
  #[serde(flatten)]
  pub oidc: UserSettings,

  pub db_url: String,
  pub virtual_host_routing: bool,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      base: BaseConfig::default(),
      db: DBConfig::default(),
      site: SiteConfig::default(),
      mail: MailSettings::default(),
      oidc: UserSettings::default(),
      db_url: "".to_string(),
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
