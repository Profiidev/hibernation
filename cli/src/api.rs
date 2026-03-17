use centaurus::error::Result;
use reqwest::{Client, Method, RequestBuilder};
use url::Url;

pub struct ApiClient {
  client: Client,
  token: String,
  url: Url,
}

impl ApiClient {
  pub fn new(token: String, url: Url) -> Self {
    Self {
      token,
      url,
      client: Client::new(),
    }
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

  fn req(&self, path: &str, method: Method) -> Result<RequestBuilder> {
    let url = self.url.join(path)?;
    Ok(self.client.request(method, url).bearer_auth(&self.token))
  }
}
