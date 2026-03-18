use axum::{Extension, Router};
use centaurus::{
  db::init::init_db,
  init::{
    axum::{listener_setup, run_app_connect_info},
    logging::init_logging,
    router::base_router,
  },
};
#[cfg(debug_assertions)]
use dotenv::dotenv;
use tracing::info;

use crate::{config::Config, rate_limit::RateLimiter};

mod auth;
mod cache;
mod cli;
mod config;
mod db;
mod gravatar;
mod group;
mod mail;
mod permissions;
mod rate_limit;
mod settings;
mod setup;
mod token;
mod user;
mod version;
mod ws;

#[tokio::main]
async fn main() {
  #[cfg(debug_assertions)]
  dotenv().ok();

  let config = Config::parse();
  init_logging(config.base.log_level);

  let listener = listener_setup(config.base.port).await;
  let mut rate_limiter = RateLimiter::default();

  let mut router = api_router(&mut rate_limiter);
  router = base_router(router, &config.base, &config.metrics).await;
  let app = state(router, config).await;

  rate_limiter.init();

  info!("Starting application");
  run_app_connect_info(listener, app).await;
}

fn api_router(rate_limiter: &mut RateLimiter) -> Router {
  Router::new()
    .nest("/ws", ws::router())
    .nest("/setup", setup::router())
    .nest("/auth", auth::router(rate_limiter))
    .nest("/user", user::router(rate_limiter))
    .nest("/settings", settings::router())
    .nest("/mail", mail::router(rate_limiter))
    .nest("/group", group::router())
    .nest("/cli", cli::router(rate_limiter))
    .nest("/cache", cache::router())
    .nest("/token", token::router())
}

async fn state(router: Router, config: Config) -> Router {
  let db = init_db::<migration::Migrator>(&config.db, &config.db_url).await;
  db::init(&db).await.expect("Failed to initialize database");
  setup::create_admin_group(&db)
    .await
    .expect("Failed to create admin group");

  let (mut router, _) = ws::state(router).await;
  router = auth::state(router, &config, &db).await;
  router = mail::state(router, &db).await;
  router = cli::state(router);
  router = version::middleware(router);

  router.layer(Extension(db)).layer(Extension(config))
}
