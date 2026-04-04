use aide::axum::ApiRouter;
use aide::axum::routing::get_with;
use axum::Json;
use axum_extra::extract::CookieJar;

use crate::auth::{
  cli_auth::CliAuth,
  jwt_state::{JWT_COOKIE_NAME, JwtState},
};

pub fn router() -> ApiRouter {
  ApiRouter::new().api_route("/", get_with(test_token, |op| op.id("testToken")))
}

async fn test_token(
  auth: Option<CliAuth>,
  mut cookies: CookieJar,
  jwt: JwtState,
) -> (CookieJar, Json<bool>) {
  if auth.is_none() {
    cookies = cookies.remove(jwt.create_cookie(JWT_COOKIE_NAME, String::new()));

    (cookies, Json(false))
  } else {
    (cookies, Json(true))
  }
}
