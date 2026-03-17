use std::path::PathBuf;

use clap::{Parser, Subcommand};
use url::Url;

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
}
