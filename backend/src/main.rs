use aide::axum::ApiRouter;
use axum::Extension;
use centaurus::{
  backend::{
    init::{listener_setup, run_app_connect_info},
    rate_limiter::RateLimiter,
    router::build_router,
  },
  db::init::init_db,
  logging::init_logging,
};
#[cfg(debug_assertions)]
use dotenvy::dotenv;
use tracing::info;

use crate::{config::Config, host::HostRouter};

mod auth;
mod cache;
mod cli;
mod config;
mod db;
mod gravatar;
mod group;
mod host;
mod mail;
mod nix;
mod permissions;
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
  let app = build_router(api_router, state, config.clone()).await;

  info!("Starting application");
  if let Some(host_router) = HostRouter::new(&app, &config) {
    run_app_connect_info(listener, host_router).await;
  } else {
    run_app_connect_info(listener, app).await;
  }
}

fn api_router(rate_limiter: &mut RateLimiter) -> ApiRouter {
  ApiRouter::new()
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
    .nest("/nix", nix::router())
}

async fn state(router: ApiRouter, config: Config) -> ApiRouter {
  let db = init_db::<migration::Migrator>(&config.db, &config.db_url).await;
  db::init(&db).await.expect("Failed to initialize database");
  setup::create_admin_group(&db)
    .await
    .expect("Failed to create admin group");

  let (mut router, _) = ws::state(router).await;
  router = auth::state(router, &config, &db).await;
  router = mail::state(router, &db).await;
  router = cli::state(router);
  router = cache::state(router, db.clone(), &config).await;
  router = version::middleware(router);

  router.layer(Extension(db))
}
