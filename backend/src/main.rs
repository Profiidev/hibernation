use aide::axum::ApiRouter;
use axum::Extension;
use centaurus::{
  backend::{
    endpoints::{mail, websocket},
    init::{listener_setup, run_app_connect_info},
    middleware::rate_limiter::RateLimiter,
    rewrite::virtual_host::HostRouter,
    router::build_router,
  },
  db::init::init_db,
  logging::init_logging,
  version_header,
};
#[cfg(debug_assertions)]
use dotenvy::dotenv;
use tracing::info;

use crate::{config::Config, utils::UpdateMessage};

mod auth;
mod cache;
mod cli;
mod config;
mod db;
mod group;
mod nix;
mod settings;
mod setup;
mod token;
mod user;
mod utils;

#[tokio::main]
async fn main() {
  #[cfg(debug_assertions)]
  dotenv().ok();

  let config = Config::parse();
  init_logging(config.base.log_level);

  let listener = listener_setup(config.base.port).await;
  let mut app = build_router(api_router, state, config.clone()).await;
  version_header!(app);

  if config.virtual_host_routing {
    let host_router = HostRouter::new(
      app,
      config.site.site_url,
      "/api/nix/{subdomain}{path}".into(),
    );
    run_app_connect_info(listener, host_router).await;
  } else {
    info!("Starting application");
    run_app_connect_info(listener, app).await;
  }
}

fn api_router(rate_limiter: &mut RateLimiter) -> ApiRouter {
  ApiRouter::new()
    .nest("/ws", websocket::router::<UpdateMessage>())
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
  centaurus::backend::endpoints::setup::create_admin_group(&db, utils::permissions())
    .await
    .expect("Failed to create admin group");

  let mut router = websocket::state::<UpdateMessage>(router).await;
  router = auth::state(router, &config.auth, &db).await;
  router = mail::state(router, &db).await;
  router = cli::state(router);
  router = cache::state(router, db.clone(), &config).await;

  router.layer(Extension(db))
}
