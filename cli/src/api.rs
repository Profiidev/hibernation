use std::{io::IsTerminal, path::PathBuf};

use centaurus::{error::Result, eyre::Context};
use reqwest::{Client, Method, RequestBuilder, Response};
use shared::HIBERNATION_VERSION_HEADER;
use tracing::{error, warn};
use url::Url;

use crate::config::Config;

pub struct ApiClient {
  client: Client,
  token: String,
  url: Url,
}

impl ApiClient {
  fn new(token: String, url: Url) -> Self {
    Self {
      token,
      url,
      client: Client::new(),
    }
  }

  pub async fn build(
    config: Option<Config>,
    url: Option<Url>,
    config_path: Option<PathBuf>,
  ) -> Self {
    if let Some(config) = &config
      && let Some(token) = &config.token
    {
      let client = ApiClient::new(token.clone(), url.clone().unwrap_or(config.app_url.clone()));

      if let Ok(valid) = client.test_token().await {
        if valid {
          return client;
        } else {
          warn!("Token is invalid.");
        }
      } else {
        warn!("Failed to validate token.");
      }
    }

    let tty = std::io::stdin().is_terminal();
    if !tty {
      if url.is_some() {
        error!("Token not set. Please use \"hibernation auth\" first");
      } else {
        error!("Cli not configured. Please login with \"hibernation auth --url <url>\" first");
      }
      std::process::exit(1);
    }

    let mut config = if let Some(config) = config {
      config
    } else if let Some(url) = url {
      Config::new(url, None)
    } else {
      let url = crate::auth::get_tty_url().await;
      Config::new(url, None)
    };

    let token = crate::auth::get_tty_token(&mut config, config_path).await;
    ApiClient::new(token, config.app_url)
  }

  pub async fn request_token(url: Url, code: &str) -> Result<String> {
    let url = url.join(&format!("/api/cli?code={}", code))?;
    let res = reqwest::get(url).await?.error_for_status()?;

    check_server_version(&res);

    Ok(res.text().await?)
  }

  pub async fn test(&self) -> Result<()> {
    self
      .req("/api/cache/test", Method::GET)?
      .send()
      .await?
      .error_for_status()?;
    Ok(())
  }

  pub async fn test_token(&self) -> Result<bool> {
    let res = self
      .req("/api/auth/test_token", Method::GET)?
      .send()
      .await?
      .error_for_status()?;

    check_server_version(&res);

    let valid = res
      .text()
      .await?
      .parse()
      .context("Failed to parse bool res")?;

    Ok(valid)
  }

  fn req(&self, path: &str, method: Method) -> Result<RequestBuilder> {
    let url = self.url.join(path)?;
    Ok(self.client.request(method, url).bearer_auth(&self.token))
  }
}

fn check_server_version(res: &Response) {
  if let Some(version) = res
    .headers()
    .get(HIBERNATION_VERSION_HEADER)
    .and_then(|v| v.to_str().ok())
  {
    if version != env!("CARGO_PKG_VERSION") {
      warn!(
        "Warning: Server version ({version}) does not match client version ({}). Consider updating your CLI.",
        env!("CARGO_PKG_VERSION")
      );
    }
  } else {
    warn!("Warning: Server did not provide version information. Consider updating your CLI.");
  }
}
