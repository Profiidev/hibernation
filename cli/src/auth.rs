use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use centaurus::error::Result;
use centaurus::eyre::ContextCompat;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio::{select, spawn};
use tracing::{info, warn};
use url::Url;

use crate::api::ApiClient;
use crate::config::Config;

pub async fn get_tty_url() -> Url {
  loop {
    info!("Please enter the URL of the hibernation server.");
    print!("URL: ");
    std::io::stdout().flush().unwrap();
    let mut lines = BufReader::new(io::stdin()).lines();
    let Some(url_input) = lines.next_line().await.ok().flatten() else {
      warn!("Failed to read URL from input");
      continue;
    };
    let url_input = url_input.trim();
    let Ok(url) = Url::parse(url_input) else {
      warn!("Invalid URL format");
      continue;
    };

    break url;
  }
}

pub async fn get_tty_token(config: &mut Config, config_path: Option<PathBuf>) -> String {
  info!(
    "Opening browser for authentication: {}auth/cli",
    config.app_url
  );
  if opener::open(format!("{}auth/cli", config.app_url)).is_err() {
    warn!("Failed to open browser for authentication.");
  }

  info!("Waiting for authentication...");

  let mut server = match CodeServer::new(config.app_url.clone()).await {
    Ok(server) => {
      info!("If the authentication fails, please enter the code manually.");
      server
    }
    Err(e) => {
      warn!(
        "Failed to start authentication server. Please enter the code manually. Error: {}",
        e
      );
      CodeServer::dummy()
    }
  };

  let token = loop {
    print!("Code: ");
    std::io::stdout().flush().unwrap();
    let mut lines = BufReader::new(io::stdin()).lines();

    select! {
      Some(token) = server.wait_for_code() => {
        info!("\nGot token from authentication server.");
        break token;
      }
      Ok(Some(code)) = lines.next_line() => {
        let code = code.trim().to_string();
        if let Ok(token) = ApiClient::request_token(config.app_url.clone(), &code).await {
          break token;
        } else {
          warn!("Invalid code. Please try again.");
        }
      }
      else => {
        warn!("Failed to read code from input");
      }
    }
  };

  server.cleanup();
  config.token = Some(token.clone());
  if config.save(config_path).await.is_err() {
    warn!("Failed to save config");
  }
  info!("Authenticated successfully.");

  token
}

pub struct CodeServer {
  receiver: oneshot::Receiver<Option<String>>,
  task: JoinHandle<()>,
}

impl CodeServer {
  pub async fn new(app_url: Url) -> Result<Self> {
    let listener = TcpListener::bind(("0.0.0.0", 16401)).await?;
    let (sender, receiver) = oneshot::channel();

    let task = spawn(async {
      sender
        .send(
          code_server(listener, app_url)
            .await
            .map_err(|err| {
              warn!("Error in code server: {:?}", err);
              err
            })
            .ok(),
        )
        .unwrap_or_else(|_| warn!("Failed to send code result"));
    });

    Ok(Self { receiver, task })
  }

  pub fn dummy() -> Self {
    let (sender, receiver) = oneshot::channel();
    Self {
      receiver,
      task: spawn(async {
        sleep(Duration::from_hours(100000)).await;
        let _s = sender;
      }),
    }
  }

  pub fn cleanup(&self) {
    self.task.abort();
  }

  pub async fn wait_for_code(&mut self) -> Option<String> {
    (&mut self.receiver).await.ok().flatten()
  }
}

async fn code_server(listener: TcpListener, app_url: Url) -> Result<String> {
  let (mut stream, _) = listener.accept().await?;
  let (reader, mut writer) = stream.split();
  let mut reader = BufReader::new(reader);

  let mut request_line = String::new();
  reader.read_line(&mut request_line).await?;

  let token_result = get_token(request_line, app_url).await;

  let body = if token_result.is_ok() { "OK" } else { "Error" };
  let res_code = if token_result.is_ok() {
    "200 OK"
  } else {
    "400 Bad Request"
  };

  let response = format!(
    "HTTP/1.1 {}\r\n\
Content-Type: text/html; charset=utf-8\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\
Access-Control-Allow-Origin: *\r\n\
\r\n\
{}",
    res_code,
    body.len(),
    body
  );

  writer.write_all(response.as_bytes()).await?;
  writer.flush().await?;

  let _ = stream.shutdown().await;

  sleep(Duration::from_millis(250)).await;

  token_result
}

async fn get_token(request_line: String, app_url: Url) -> Result<String> {
  // Extract the path (e.g., "/callback?code=xyz&user=abc")
  let parts: Vec<&str> = request_line.split_whitespace().collect();
  let path = parts.get(1).context("Invalid request")?;

  let url = Url::parse(&format!("http://localhost{}", path))?;
  let code = url
    .query_pairs()
    .find(|(k, _)| k == "code")
    .map(|(_, v)| v.into_owned())
    .context("Code not found in query parameters")?;

  let token = ApiClient::request_token(app_url, &code).await?;

  Ok(token)
}
