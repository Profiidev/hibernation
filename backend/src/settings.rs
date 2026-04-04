use aide::axum::ApiRouter;
use aide::axum::routing::{get_with, post_with};
use axum::Json;
use centaurus::{
  db::init::Connection,
  error::{ErrorReportStatusExt, Result},
};
use http::StatusCode;
use schemars::JsonSchema;
use serde::Serialize;
use url::Url;

use crate::{
  auth::{jwt_auth::JwtAuth, oidc::OidcState},
  config::Config,
  db::{
    DBTrait,
    settings::{MailSettings, Settings, UserSettings},
  },
  mail::state::Mailer,
  permissions::{SettingsEdit, SettingsView},
  ws::state::{UpdateMessage, Updater},
};

pub fn router() -> ApiRouter {
  ApiRouter::new()
    .api_route(
      "/general",
      get_with(general_settings, |op| op.id("generalSettings")),
    )
    .api_route(
      "/user",
      get_with(get_settings::<UserSettings>, |op| op.id("getSettings")),
    )
    .api_route(
      "/user",
      post_with(save_user_settings, |op| op.id("saveUserSettings")),
    )
    .api_route(
      "/mail",
      get_with(get_settings::<MailSettings>, |op| op.id("getSettings")),
    )
    .api_route(
      "/mail",
      post_with(save_mail_settings, |op| op.id("saveMailSettings")),
    )
}

#[derive(Serialize, JsonSchema)]
struct GeneralSettings {
  site_url: Url,
  virtual_host_routing: bool,
}

async fn general_settings(_auth: JwtAuth, config: Config) -> Result<Json<GeneralSettings>> {
  Ok(Json(GeneralSettings {
    site_url: config.site_url,
    virtual_host_routing: config.virtual_host_routing,
  }))
}

async fn get_settings<S: Settings>(
  _auth: JwtAuth<SettingsView>,
  db: Connection,
) -> Result<Json<S>> {
  Ok(Json(db.settings().get_settings::<S>().await?))
}

#[allow(unused)]
async fn save_settings<S: Settings>(
  _auth: JwtAuth<SettingsEdit>,
  db: Connection,
  updater: Updater,
  Json(settings): Json<S>,
) -> Result<()> {
  db.settings().save_settings(&settings).await?;
  updater.broadcast(UpdateMessage::Settings).await;
  Ok(())
}

async fn save_user_settings(
  _auth: JwtAuth<SettingsEdit>,
  db: Connection,
  state: OidcState,
  updater: Updater,
  Json(settings): Json<UserSettings>,
) -> Result<()> {
  if let Some(oidc_settings) = &settings.oidc {
    state.try_init(oidc_settings).await.status_context(
      StatusCode::NOT_ACCEPTABLE,
      "Failed to initialize OIDC state",
    )?;
  } else {
    state.deactivate().await;
  }

  db.settings().save_settings(&settings).await?;
  updater.broadcast(UpdateMessage::Settings).await;

  Ok(())
}

async fn save_mail_settings(
  _auth: JwtAuth<SettingsEdit>,
  db: Connection,
  state: Mailer,
  updater: Updater,
  Json(settings): Json<MailSettings>,
) -> Result<()> {
  if let Some(smtp_settings) = &settings.smtp {
    state.try_init(smtp_settings).await?;
  } else {
    state.deactivate().await;
  }

  db.settings().save_settings(&settings).await?;
  updater.broadcast(UpdateMessage::Settings).await;

  Ok(())
}
