use std::{convert::Infallible, path::PathBuf};

use centaurus::{
  error::{ErrorReportExt, Result},
  eyre::ContextCompat,
};
use clap::{Parser, Subcommand};
use shared::sig::KeyPair;
use tracing::{error, info};
use url::Url;

use crate::{api::ApiClient, config::Config, push};

const SIGING_KEY_ENV_VAR: &str = "HIBERNATION_SIGNING_KEY";

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
  /// The URL of the Hibernation server to connect to.
  #[arg(long, short, global = true)]
  pub url: Option<Url>,

  /// The config file to use
  #[arg(long, short, global = true)]
  pub config: Option<PathBuf>,

  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
  /// Set the URL of the Hibernation server to connect to.
  SetUrl {
    /// The URL to use for future connections to the Hibernation server.
    url: Url,
  },
  /// Authenticate with a Hibernation server.
  Auth {
    /// The token to use for authentication
    token: Option<String>,
  },
  #[command(flatten)]
  AuthenticatedCommand(AuthenticatedCommand),
}

#[derive(Subcommand)]
pub enum AuthenticatedCommand {
  /// A test command to check if the authentication works.
  Test,
  /// Push the given paths and all their dependencies to the given cache on the Hibernation server.
  Push {
    /// The cache to push to.
    cache: String,
    /// The paths to push.
    paths: Vec<String>,
    /// Only upload the given paths and not its dependencies.
    #[arg(long)]
    no_deps: bool,
    /// Push paths even if they already exist in a downstream cache. The cache needs to be configured to allow this.
    #[arg(long)]
    force: bool,
    /// Path of the key to use for signing.
    #[arg(long)]
    key: Option<PathBuf>,
  },
}

impl Cli {
  pub async fn run(self, config: Option<Config>, url: Option<Url>) -> Result<Infallible> {
    match self.command {
      Commands::SetUrl { url } => {
        let mut config = config.unwrap_or_else(|| Config::new(url.clone(), None));
        config.app_url = url;
        config
          .save(self.config)
          .await
          .context("Failed to save config")?;
      }
      Commands::Auth { token } => {
        let Some(url) = url else {
          error!(
            "No URL specified. Please provide a URL using --url or set it using the set-url command."
          );
          std::process::exit(1);
        };

        if let Some(token) = token {
          let mut config = config.unwrap_or_else(|| Config::new(url.clone(), Some(token.clone())));
          config.token = Some(token);
          config
            .save(self.config)
            .await
            .context("Failed to save config")?;
          info!("Token saved successfully.");
        }
      }
      Commands::AuthenticatedCommand(cmd) => cmd.run(config, url, self.config).await,
    }

    // Tokio does not exit when other tasks are still running
    std::process::exit(0);
  }
}

impl AuthenticatedCommand {
  async fn run(self, config: Option<Config>, url: Option<Url>, config_path: Option<PathBuf>) {
    let client = ApiClient::build(config, url, config_path).await;

    match self {
      AuthenticatedCommand::Test => {
        if let Err(e) = client.test().await {
          error!("Test request failed: {:?}", e);
          std::process::exit(1);
        } else {
          info!("Test request succeeded.");
        }
      }
      AuthenticatedCommand::Push {
        cache,
        paths,
        no_deps,
        force,
        key,
      } => {
        let signing_key = get_signing_key(key).await;
        push::push_paths(client, cache, &paths, no_deps, force, signing_key).await;
      }
    }
  }
}

async fn get_signing_key(path: Option<PathBuf>) -> KeyPair {
  match load_signing_key(path).await {
    Ok(key) => key,
    Err(e) => {
      error!("Failed to load signing key: {:?}", e);
      std::process::exit(1);
    }
  }
}

async fn load_signing_key(path: Option<PathBuf>) -> Result<KeyPair> {
  if let Some(path) = path {
    let key_data = tokio::fs::read_to_string(path).await?;
    Ok(KeyPair::from_string(&key_data).context("Invalid signing key format in file")?)
  } else {
    let key_data = std::env::var(SIGING_KEY_ENV_VAR).ok().context(format!(
      "Signing key not found in environment variable {SIGING_KEY_ENV_VAR}"
    ))?;
    Ok(KeyPair::from_string(&key_data).context("Invalid signing key format from env")?)
  }
}
