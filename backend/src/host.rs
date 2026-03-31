use axum::{Router, extract::Request, response::Response};
use axum_extra::headers::{HeaderMapExt, Host};
use tower_service::Service;
use tracing::info;

use crate::config::Config;

#[derive(Clone)]
pub struct HostRouter<S> {
  prefix: String,
  inner: S,
}

impl<S, ResBody> Service<Request> for HostRouter<S>
where
  S: Service<Request, Response = Response<ResBody>>,
{
  type Error = S::Error;
  type Future = S::Future;
  type Response = S::Response;

  fn poll_ready(
    &mut self,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Result<(), Self::Error>> {
    self.inner.poll_ready(cx)
  }

  fn call(&mut self, mut req: Request) -> Self::Future {
    self.modify_req(&mut req);
    self.inner.call(req)
  }
}

impl HostRouter<Router> {
  pub fn new(router: &Router, config: &Config) -> Option<Self> {
    if !config.virtual_host_routing {
      return None;
    }

    let Some(host) = config.site_url.host() else {
      panic!("Virtual host routing is enabled, but the site URL does not contain a host");
    };
    let url::Host::Domain(host) = host else {
      panic!("Virtual host routing is enabled, but the site URL does not contain a valid host");
    };
    let subdomain = subdomain_from_host(host).unwrap_or_default();

    info!("Virtual host routing enabled with subdomain prefix: {subdomain}");

    Some(Self {
      prefix: subdomain,
      inner: router.clone(),
    })
  }
}

impl<S> HostRouter<S> {
  fn modify_req(&self, req: &mut Request) -> Option<()> {
    let host = req.headers().typed_get::<Host>()?;
    let subdomain = subdomain_from_host(host.hostname())?;

    let suffix = if self.prefix.is_empty() {
      ""
    } else {
      &format!(".{}", self.prefix)
    };
    let cache = subdomain.strip_suffix(suffix)?;

    let path = req.uri().path();
    let new_path = format!("/api/nix/{cache}{path}");

    let mut parts = req.uri().clone().into_parts();
    parts.path_and_query = Some(new_path.parse().ok()?);
    let new_uri = http::Uri::from_parts(parts).ok()?;
    *req.uri_mut() = new_uri;

    Some(())
  }
}

fn subdomain_from_host(host: &str) -> Option<String> {
  let domain = addr::parse_domain_name(host).ok()?;
  domain.prefix().map(|s| s.to_string())
}
