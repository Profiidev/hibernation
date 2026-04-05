use aide::axum::ApiRouter;
use aide::axum::routing::get_with;
use axum::Json;
use axum_extra::extract::CookieJar;
use centaurus::backend::auth::jwt_state::{JWT_COOKIE_NAME, JwtState};
use schemars::JsonSchema;
use serde::Serialize;

use crate::auth::cli_auth::CliAuth;

pub fn router() -> ApiRouter {
  ApiRouter::new().api_route("/", get_with(test_token, |op| op.id("testToken")))
}

#[derive(Serialize, JsonSchema)]
struct TestTokenResponse {
  valid: bool,
}

async fn test_token(
  auth: Option<CliAuth>,
  mut cookies: CookieJar,
  jwt: JwtState,
) -> (CookieJar, Json<TestTokenResponse>) {
  if auth.is_none() {
    cookies = cookies.remove(jwt.create_cookie(JWT_COOKIE_NAME, String::new()));

    (cookies, Json(TestTokenResponse { valid: false }))
  } else {
    (cookies, Json(TestTokenResponse { valid: true }))
  }
}
