use std::path::PathBuf;

use centaurus::{
  error::{ErrorReportExt, Result},
  init::logging::init_logging,
};
use clap::Parser;
use tracing::{error, info};
use url::Url;

use crate::{
  api::ApiClient,
  cli::{AuthenticatedCommand, Cli, Commands},
  config::Config,
};

mod api;
mod auth;
mod cli;
mod config;
mod push;

#[tokio::main]
async fn main() -> Result<()> {
  let level = config::log_level();
  init_logging(level);

  let cli = Cli::parse();
  let config = Config::load(cli.config.clone())
    .await
    .context("Failed to load config")?;

  let url = cli
    .url
    .or_else(|| config.as_ref().map(|c| c.app_url.clone()));

  match cli.command {
    Commands::SetUrl { url } => {
      let mut config = config.unwrap_or_else(|| Config::new(url.clone(), None));
      config.app_url = url;
      config
        .save(cli.config)
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
          .save(cli.config)
          .await
          .context("Failed to save config")?;
        info!("Token saved successfully.");
      }
    }
    Commands::AuthenticatedCommand(cmd) => handle_auth_command(cmd, config, url, cli.config).await,
  }

  // Tokio does not exit when other tasks are still running
  std::process::exit(0);
}

async fn handle_auth_command(
  cmd: AuthenticatedCommand,
  config: Option<Config>,
  url: Option<Url>,
  config_path: Option<PathBuf>,
) {
  let client = ApiClient::build(config, url, config_path).await;

  match cmd {
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
    } => {
      push::push_paths(client, cache, &paths, no_deps, force).await;
    }
  }
}
