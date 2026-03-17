use std::path::PathBuf;

use async_compression::tokio::bufread::ZstdEncoder;
use centaurus::{
  error::{ErrorReportExt, Result},
  init::logging::init_logging,
};
use clap::Parser;
use harmonia_store_core::store_path::{StoreDir, StorePath};
use harmonia_store_remote::{DaemonClient, DaemonStore};
use tokio::{
  fs::File,
  io::{self},
};
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
        eprintln!(
          "No URL specified. Please provide a URL using --url or set it using the set-url command."
        );
        return Ok(());
      };

      if let Some(token) = token {
        let mut config = config.unwrap_or_else(|| Config::new(url.clone(), Some(token.clone())));
        config.token = Some(token);
        config
          .save(cli.config)
          .await
          .context("Failed to save config")?;
        println!("Token saved successfully.");
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
  let Some(client) = ApiClient::build(config, url, config_path).await else {
    return;
  };

  match cmd {
    AuthenticatedCommand::Test => {
      if let Err(e) = client.test().await {
        eprintln!("Test request failed: {:?}", e);
      } else {
        println!("Test request succeeded.");
      }
    }
  }
}

async fn _test() {
  let store_dir = StoreDir::default();
  let path: StorePath = store_dir
    .parse("/nix/store/hlxw2q9qansq7bn52xvlb5badw3z1v8s-coreutils-9.10")
    .unwrap();

  let mut conn = DaemonClient::builder().connect_daemon().await.unwrap();
  let info = conn.query_path_info(&path).await.unwrap().unwrap();

  println!("Daemon info: {:?}", info);

  let nar = conn.nar_from_path(&path).await.unwrap();
  let mut encoder = ZstdEncoder::new(nar);

  let mut file = File::create("output.nar.zst").await.unwrap();
  io::copy(&mut encoder, &mut file).await.unwrap();
}
