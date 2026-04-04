use aide::axum::ApiRouter;
use aide::axum::routing::get_with;
use axum::Json;
use centaurus::{db::init::Connection, error::Result};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
  auth::oidc::OidcState,
  db::{DBTrait, settings::UserSettings},
  mail::state::Mailer,
};

pub fn router() -> ApiRouter {
  ApiRouter::new().api_route("/", get_with(config, |op| op.id("authConfig")))
}

#[derive(Serialize, Debug, JsonSchema)]
enum SSOType {
  Oidc,
  None,
}

#[derive(Serialize, JsonSchema)]
struct AuthConfig {
  sso_type: SSOType,
  instant_redirect: bool,
  mail_enabled: bool,
}

async fn config(oidc: OidcState, mailer: Mailer, db: Connection) -> Result<Json<AuthConfig>> {
  let sso_type = if oidc.is_enabled().await {
    SSOType::Oidc
  } else {
    SSOType::None
  };

  let user_settings = db.settings().get_settings::<UserSettings>().await?;
  let mail_enabled = mailer.is_active().await;

  Ok(Json(AuthConfig {
    sso_type,
    instant_redirect: user_settings.sso_instant_redirect,
    mail_enabled,
  }))
}
