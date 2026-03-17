use std::{
  io::{IsTerminal, Write},
  path::PathBuf,
};

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
  io::{self, AsyncBufReadExt, BufReader},
  select,
};
use url::Url;

use crate::{
  api::ApiClient,
  auth::CodeServer,
  cli::{Cli, Commands},
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

  match &cli.command {
    Commands::SetUrl { url } => {
      let mut config = config.unwrap_or_else(|| Config::new(url.clone(), None));
      config.app_url = url.clone();
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
        config.token = Some(token.clone());
        config
          .save(cli.config)
          .await
          .context("Failed to save config")?;
        println!("Token saved successfully.");
      }
    }
    cmd => handle_command(cmd, config, url, cli.config).await,
  }

  Ok(())
}

async fn handle_command(
  cmd: &Commands,
  config: Option<Config>,
  url: Option<Url>,
  config_path: Option<PathBuf>,
) {
  let tty = std::io::stdin().is_terminal();

  let client = if let Some(config) = &config
    && let Some(token) = &config.token
  {
    ApiClient::new(token.clone(), url.unwrap_or(config.app_url.clone()))
  } else if tty {
    let mut config = if let Some(config) = config {
      config
    } else if let Some(url) = url {
      Config::new(url, None)
    } else {
      let url = loop {
        println!("Please enter the URL of the hibernation instance.");
        print!("URL: ");
        std::io::stdout().flush().unwrap();
        let mut lines = BufReader::new(io::stdin()).lines();
        let Some(url_input) = lines.next_line().await.ok().flatten() else {
          eprintln!("Failed to read URL from input");
          continue;
        };
        let url_input = url_input.trim();
        let Ok(url) = Url::parse(url_input) else {
          eprintln!("Invalid URL format");
          continue;
        };

        break url;
      };

      Config::new(url, None)
    };

    println!(
      "Opening browser for authentication: {}auth/cli",
      config.app_url
    );
    if opener::open(format!("{}auth/cli", config.app_url)).is_err() {
      eprintln!("Failed to open browser for authentication.");
    }

    println!("Waiting for authentication...");

    let mut server = match CodeServer::new(config.app_url.clone()).await {
      Ok(server) => {
        println!("If the authentication fails, please enter the code manually.");
        server
      }
      Err(e) => {
        eprintln!(
          "Failed to start authentication server. Please enter the code manually. Error: {}",
          e
        );
        return;
      }
    };

    let token = loop {
      print!("Code: ");
      std::io::stdout().flush().unwrap();
      let mut lines = BufReader::new(io::stdin()).lines();

      select! {
        Some(token) = server.wait_for_code() => {
          println!("\nGot token from authentication server.");
          break token;
        }
        Ok(Some(code)) = lines.next_line() => {
          let code = code.trim().to_string();
          if let Ok(token) = ApiClient::request_token(config.app_url.clone(), &code).await {
            break token;
          } else {
            eprintln!("Invalid code. Please try again.");
          }
        }
        else => {
          eprintln!("Failed to read code from input");
        }
      }
    };

    server.cleanup();
    config.token = Some(token.clone());
    if config.save(config_path).await.is_err() {
      eprintln!("Failed to save config");
    }
    println!("Authenticated successfully.");

    ApiClient::new(token, config.app_url)
  } else {
    if url.is_some() {
      eprintln!("Token not set. Please use \"hibernation auth\" first");
    } else {
      eprintln!("Cli not configured. Please login with \"hibernation auth --url <url>\" first");
    }
    return;
  };

  match cmd {
    Commands::Test => {
      if let Err(e) = client.test().await {
        eprintln!("Test request failed: {:?}", e);
      } else {
        println!("Test request succeeded.");
      }
    }
    _ => unreachable!(),
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
