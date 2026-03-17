use std::{io::IsTerminal, path::PathBuf};

use centaurus::{error::Result, eyre::Context};
use reqwest::{Client, Method, RequestBuilder};
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
  ) -> Option<Self> {
    if let Some(config) = &config
      && let Some(token) = &config.token
    {
      let client = ApiClient::new(token.clone(), url.clone().unwrap_or(config.app_url.clone()));

      if client.test_token().await.ok()? {
        return Some(client);
      } else {
        eprintln!("Token is invalid.");
      }
    }

    let tty = std::io::stdin().is_terminal();
    if !tty {
      if url.is_some() {
        eprintln!("Token not set. Please use \"hibernation auth\" first");
      } else {
        eprintln!("Cli not configured. Please login with \"hibernation auth --url <url>\" first");
      }
      return None;
    }

    let mut config = if let Some(config) = config {
      config
    } else if let Some(url) = url {
      Config::new(url, None)
    } else {
      let url = crate::auth::get_tty_url().await;
      Config::new(url, None)
    };

    let token = crate::auth::get_tty_token(&mut config, config_path).await?;
    Some(ApiClient::new(token, config.app_url))
  }

  pub async fn request_token(url: Url, code: &str) -> Result<String> {
    let url = url.join(&format!("/api/cli?code={}", code))?;
    Ok(reqwest::get(url).await?.error_for_status()?.text().await?)
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
    let res: bool = self
      .req("/api/auth/test_token", Method::GET)?
      .send()
      .await?
      .error_for_status()?
      .text()
      .await?
      .parse()
      .context("Failed to parse bool res")?;

    Ok(res)
  }

  fn req(&self, path: &str, method: Method) -> Result<RequestBuilder> {
    let url = self.url.join(path)?;
    Ok(self.client.request(method, url).bearer_auth(&self.token))
  }
}
