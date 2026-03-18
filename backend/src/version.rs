use axum::{
  Router,
  extract::Request,
  middleware::{Next, from_fn},
  response::Response,
};
use http::HeaderValue;
use shared::HIBERNATION_VERSION_HEADER;

pub const HIBERNATION_SERVER_VERSION: HeaderValue =
  HeaderValue::from_static(env!("CARGO_PKG_VERSION"));

pub fn middleware(router: Router) -> Router {
  router.layer(from_fn(version_middleware))
}

async fn version_middleware(request: Request, next: Next) -> Response {
  let mut response = next.run(request).await;

  response
    .headers_mut()
    .insert(HIBERNATION_VERSION_HEADER, HIBERNATION_SERVER_VERSION);

  response
}
