use std::{env, path::PathBuf, str::FromStr};

use centaurus::{error::Result, eyre::ContextCompat};
use dirs::data_dir;
use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;
use url::Url;

pub fn log_level() -> LevelFilter {
  for key in ["LOG_LEVEL", "RUST_LOG"] {
    if let Ok(raw_level) = env::var(key)
      && let Ok(level) = LevelFilter::from_str(&raw_level)
    {
      return level;
    }
  }
  LevelFilter::INFO
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub app_url: Url,
  pub token: Option<String>,
}

impl Config {
  fn path() -> Result<PathBuf> {
    let data_dir = data_dir().context("Could not determine data directory")?;
    Ok(data_dir.join("hibernation").join("config.json"))
  }

  pub fn new(app_url: Url, token: Option<String>) -> Self {
    Self { app_url, token }
  }

  pub async fn load(path: Option<PathBuf>) -> Result<Option<Self>> {
    let path = path.map_or_else(Self::path, Ok)?;
    if !path.exists() {
      return Ok(None);
    }

    let data = tokio::fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&data)?)
  }

  pub async fn save(&self, path: Option<PathBuf>) -> Result<()> {
    let path = path.map_or_else(Self::path, Ok)?;
    if let Some(parent) = path.parent() {
      tokio::fs::create_dir_all(parent).await?;
    }
    let data = serde_json::to_string_pretty(self)?;
    tokio::fs::write(path, data).await?;
    Ok(())
  }
}
