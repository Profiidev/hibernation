//! Restricted tokens: per cache pull / push / delete rules with store path name patterns.
mod common;
mod push_common;

use common::{CacheOptions, TestNar, TestServer};
use entity::sea_orm_active_enums::AccessType;
use push_common::*;
use reqwest::{Method, Response, StatusCode};
use serde_json::{Value, json};
use shared::api::push::CHUNK_SIZE;
use uuid::Uuid;

async fn setup() -> (TestServer, Uuid, String) {
  let (server, _) = TestServer::start_with_admin().await;
  let cache = server
    .create_cache(
      "main",
      &CacheOptions {
        public: false,
        ..Default::default()
      },
    )
    .await;
  let (admin, _) = server.create_cli_token("admin").await;
  (server, cache, admin)
}

fn rule(cache: Uuid, actions: &[&str], include: &[&str], exclude: &[&str]) -> Value {
  json!({
    "cache": cache,
    "pull": actions.contains(&"pull"),
    "push": actions.contains(&"push"),
    "delete": actions.contains(&"delete"),
    "include": include,
    "exclude": exclude,
  })
}

/// Create a restricted token as the admin. Returns `(secret, id)`.
async fn scoped_token(server: &TestServer, rules: Value) -> (String, Uuid) {
  let exp = chrono::Utc::now() + chrono::Duration::days(1);
  let resp = server
    .post(
      "/token",
      json!({ "name": common::unique("scoped"), "exp": exp, "restricted": true, "rules": rules }),
    )
    .await;
  assert_eq!(resp.status(), StatusCode::OK, "create scoped token");
  let body: Value = resp.json().await.unwrap();
  (
    body["token"].as_str().unwrap().to_string(),
    Uuid::parse_str(body["uuid"].as_str().unwrap()).unwrap(),
  )
}

async fn get(server: &TestServer, token: &str, path: &str) -> StatusCode {
  server
    .raw(Method::GET, path)
    .bearer_auth(token)
    .send()
    .await
    .unwrap()
    .status()
}

async fn delete_paths(server: &TestServer, token: &str, cache: &str, paths: &[&str]) -> Response {
  server
    .raw(Method::DELETE, "/cache/push/path")
    .bearer_auth(token)
    .json(&json!({ "cache": cache, "paths": paths }))
    .send()
    .await
    .unwrap()
}

#[tokio::test]
async fn restricted_without_rules_has_no_access() {
  let (server, _, admin) = setup().await;
  let app = TestNar::new("myapp-1.0", b"app");
  server.push_path(&admin, "main", &app).await;
  let (token, _) = scoped_token(&server, json!([])).await;

  let resp = info(&server, &token, "main", &[&app.store_path], false).await;
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
  assert_eq!(
    get(&server, &token, &format!("/nix/main/{}.narinfo", app.hash)).await,
    StatusCode::FORBIDDEN
  );
  assert_eq!(
    get(&server, &token, "/nix/main/nix-cache-info").await,
    StatusCode::FORBIDDEN
  );
}

#[tokio::test]
async fn unrestricted_token_keeps_full_access() {
  let (server, cache, _) = setup().await;
  // rules are ignored while the token is not restricted
  let exp = chrono::Utc::now() + chrono::Duration::days(1);
  let resp = server
    .post(
      "/token",
      json!({ "name": "free", "exp": exp, "rules": [rule(cache, &["pull"], &["^nothing$"], &[])] }),
    )
    .await;
  let token = resp.json::<Value>().await.unwrap()["token"]
    .as_str()
    .unwrap()
    .to_string();
  let glibc = TestNar::new("glibc-2.40", b"libc");
  server.push_path(&token, "main", &glibc).await;
}

#[tokio::test]
async fn push_info_splits_denied_paths() {
  let (server, cache, _) = setup().await;
  let (token, _) = scoped_token(&server, json!([rule(cache, &["push"], &["^myapp-"], &[])])).await;
  let app = TestNar::new("myapp-1.0", b"app");
  let glibc = TestNar::new("glibc-2.40", b"libc");

  let resp = info(
    &server,
    &token,
    "main",
    &[&app.store_path, &glibc.store_path],
    false,
  )
  .await;
  assert_eq!(resp.status(), StatusCode::OK);
  let body: Value = resp.json().await.unwrap();
  assert_eq!(body["paths"], json!([app.store_path]));
  assert_eq!(body["denied"], json!([glibc.store_path]));

  server.push_path(&token, "main", &app).await;

  // Only denied paths left: still OK so the client can report them
  let resp = info(
    &server,
    &token,
    "main",
    &[&app.store_path, &glibc.store_path],
    false,
  )
  .await;
  assert_eq!(resp.status(), StatusCode::OK);
  let body: Value = resp.json().await.unwrap();
  assert_eq!(body["paths"], json!([]));
  assert_eq!(body["denied"], json!([glibc.store_path]));
}

#[tokio::test]
async fn push_without_push_action_is_not_found() {
  let (server, cache, _) = setup().await;
  let (token, _) = scoped_token(&server, json!([rule(cache, &["pull", "delete"], &[], &[])])).await;
  let app = TestNar::new("myapp-1.0", b"app");
  let resp = info(&server, &token, "main", &[&app.store_path], false).await;
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn upload_path_rejects_denied_path() {
  let (server, cache, _) = setup().await;
  let (token, _) = scoped_token(&server, json!([rule(cache, &["push"], &["^myapp-"], &[])])).await;
  let glibc = TestNar::new("glibc-2.40", b"libc");
  let resp = upload(&server, &token, &glibc.upload_request(cache, false)).await;
  assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn exclude_beats_include() {
  let (server, cache, _) = setup().await;
  let (token, _) = scoped_token(
    &server,
    json!([rule(cache, &["push"], &["^myapp"], &["-dev$"])]),
  )
  .await;
  let app = TestNar::new("myapp-1.0", b"app");
  let dev = TestNar::new("myapp-1.0-dev", b"dev");
  let body: Value = info(
    &server,
    &token,
    "main",
    &[&app.store_path, &dev.store_path],
    false,
  )
  .await
  .json()
  .await
  .unwrap();
  assert_eq!(body["paths"], json!([app.store_path]));
  assert_eq!(body["denied"], json!([dev.store_path]));
}

#[tokio::test]
async fn upload_session_bound_to_token() {
  let (server, cache, owner) = setup().await;
  let (other, _) = server.create_cli_token("other").await;
  let app = TestNar::new("myapp-1.0", b"app");
  let (upload, nar_found) = start_upload(&server, &owner, cache, &app).await;
  assert!(!nar_found);

  let parts = app.chunks(CHUNK_SIZE);
  assert!(
    !chunk(&server, &other, upload, 0, parts[0].clone())
      .await
      .status()
      .is_success()
  );
  assert_eq!(
    chunk(&server, &owner, upload, 0, parts[0].clone())
      .await
      .status(),
    StatusCode::OK
  );
  assert!(
    !finish(&server, &other, upload, &app.finish_request())
      .await
      .status()
      .is_success()
  );
  // the failed attempt must not consume the session
  assert_eq!(
    finish(&server, &owner, upload, &app.finish_request())
      .await
      .status(),
    StatusCode::OK
  );
}

#[tokio::test]
async fn finish_rechecks_rules_changed_during_upload() {
  let (server, cache, _) = setup().await;
  let (token, id) = scoped_token(&server, json!([rule(cache, &["push"], &[], &[])])).await;
  let app = TestNar::new("myapp-1.0", b"app");
  let (upload, _) = start_upload(&server, &token, cache, &app).await;
  assert_eq!(
    chunk(&server, &token, upload, 0, app.file.clone())
      .await
      .status(),
    StatusCode::OK
  );

  let exp = chrono::Utc::now() + chrono::Duration::days(1);
  let resp = server
    .put(
      "/token",
      json!({ "uuid": id, "name": "narrowed", "exp": exp, "restricted": true,
              "rules": [rule(cache, &["push"], &["^other-"], &[])] }),
    )
    .await;
  assert_eq!(resp.status(), StatusCode::OK);

  let resp = finish(&server, &token, upload, &app.finish_request()).await;
  assert_eq!(resp.status(), StatusCode::FORBIDDEN);
  assert!(nar_infos(&*server.db().await, cache).await.is_empty());
}

#[tokio::test]
async fn pull_filters_by_pattern() {
  let (server, cache, admin) = setup().await;
  let app = TestNar::new("myapp-1.0", b"app");
  let glibc = TestNar::new("glibc-2.40", b"libc");
  server.push_path(&admin, "main", &app).await;
  server.push_path(&admin, "main", &glibc).await;
  let (token, _) = scoped_token(&server, json!([rule(cache, &["pull"], &["^myapp-"], &[])])).await;

  assert_eq!(
    get(&server, &token, "/nix/main/nix-cache-info").await,
    StatusCode::OK
  );
  assert_eq!(
    get(&server, &token, &format!("/nix/main/{}.narinfo", app.hash)).await,
    StatusCode::OK
  );
  assert_eq!(
    get(&server, &token, &format!("/nix/main/{}", app.nar_url())).await,
    StatusCode::OK
  );
  assert_eq!(
    get(
      &server,
      &token,
      &format!("/nix/main/{}.narinfo", glibc.hash)
    )
    .await,
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    get(&server, &token, &format!("/nix/main/{}", glibc.nar_url())).await,
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn public_cache_pull_ignores_scope() {
  let (server, _) = TestServer::start_with_admin().await;
  server.create_cache("pub", &CacheOptions::default()).await;
  let (admin, _) = server.create_cli_token("admin").await;
  let app = TestNar::new("myapp-1.0", b"app");
  server.push_path(&admin, "pub", &app).await;
  let (token, _) = scoped_token(&server, json!([])).await;
  assert_eq!(
    get(&server, &token, &format!("/nix/pub/{}.narinfo", app.hash)).await,
    StatusCode::OK
  );
}

#[tokio::test]
async fn delete_respects_scope() {
  let (server, cache, admin) = setup().await;
  let db = server.db().await;
  let app = TestNar::new("myapp-1.0", b"app");
  let glibc = TestNar::new("glibc-2.40", b"libc");
  server.push_path(&admin, "main", &app).await;
  server.push_path(&admin, "main", &glibc).await;

  let (no_delete, _) = scoped_token(&server, json!([rule(cache, &["push"], &[], &[])])).await;
  let resp = delete_paths(&server, &no_delete, "main", &[&app.store_path]).await;
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);

  let (token, _) = scoped_token(
    &server,
    json!([rule(cache, &["delete"], &["^myapp-"], &[])]),
  )
  .await;
  let missing = TestNar::new("myapp-2.0", b"none");
  let resp = delete_paths(
    &server,
    &token,
    "main",
    &[&app.full_path(), &glibc.store_path, &missing.store_path],
  )
  .await;
  assert_eq!(resp.status(), StatusCode::OK);
  let body: Value = resp.json().await.unwrap();
  assert_eq!(body["deleted"], json!([app.store_path]));
  assert_eq!(body["denied"], json!([glibc.store_path]));
  assert_eq!(body["not_found"], json!([missing.store_path]));

  let left = nar_infos(&db, cache).await;
  assert_eq!(left.len(), 1);
  assert_eq!(left[0].store_path, glibc.store_path);

  let resp = delete_paths(&server, &token, "main", &["not-a-path"]).await;
  assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn editing_rules_applies_immediately() {
  let (server, cache, _) = setup().await;
  let (token, id) = scoped_token(&server, json!([])).await;
  let app = TestNar::new("myapp-1.0", b"app");
  let resp = info(&server, &token, "main", &[&app.store_path], false).await;
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);

  let exp = chrono::Utc::now() + chrono::Duration::days(1);
  let resp = server
    .put(
      "/token",
      json!({ "uuid": id, "name": "renamed", "exp": exp, "restricted": true,
              "rules": [rule(cache, &["push"], &[], &[])] }),
    )
    .await;
  assert_eq!(resp.status(), StatusCode::OK);
  let resp = info(&server, &token, "main", &[&app.store_path], false).await;
  assert_eq!(resp.status(), StatusCode::OK);

  let body: Value = server
    .get(&format!("/token/{id}"))
    .await
    .json()
    .await
    .unwrap();
  assert_eq!(body["restricted"], json!(true));
  assert_eq!(body["rules"], json!([rule(cache, &["push"], &[], &[])]));

  // omitting the scope keeps it, so older clients can't lift a restriction by accident
  let resp = server
    .put(
      "/token",
      json!({ "uuid": id, "name": "renamed2", "exp": exp }),
    )
    .await;
  assert_eq!(resp.status(), StatusCode::OK);
  let body: Value = server
    .get(&format!("/token/{id}"))
    .await
    .json()
    .await
    .unwrap();
  assert_eq!(body["restricted"], json!(true));
  assert_eq!(body["rules"], json!([rule(cache, &["push"], &[], &[])]));

  // lifting the restriction also applies right away
  let resp = server
    .put(
      "/token",
      json!({ "uuid": id, "name": "renamed", "exp": exp, "restricted": false, "rules": [] }),
    )
    .await;
  assert_eq!(resp.status(), StatusCode::OK);
  let body: Value = server
    .get(&format!("/token/{id}"))
    .await
    .json()
    .await
    .unwrap();
  assert_eq!(body["restricted"], json!(false));
  assert_eq!(body["rules"], json!([]));
}

#[tokio::test]
async fn invalid_rules_are_rejected() {
  let (server, cache, _) = setup().await;
  let exp = chrono::Utc::now() + chrono::Duration::days(1);
  let create = |rules: Value| {
    let server = &server;
    async move {
      server
        .post(
          "/token",
          json!({ "name": common::unique("bad"), "exp": exp, "restricted": true, "rules": rules }),
        )
        .await
        .status()
    }
  };

  assert_eq!(
    create(json!([rule(cache, &["push"], &["("], &[])])).await,
    StatusCode::BAD_REQUEST
  );
  assert_eq!(
    create(json!([rule(cache, &["push"], &[], &["^(?!glibc)"])])).await,
    StatusCode::BAD_REQUEST
  );
  assert_eq!(
    create(json!([
      rule(cache, &["push"], &[], &[]),
      rule(cache, &["pull"], &[], &[])
    ]))
    .await,
    StatusCode::BAD_REQUEST
  );
  assert_eq!(
    create(json!([rule(Uuid::new_v4(), &["pull"], &[], &[])])).await,
    StatusCode::BAD_REQUEST
  );
}

#[tokio::test]
async fn rules_cannot_exceed_user_access() {
  let (server, cache, _) = setup().await;
  let db = server.db().await;
  let viewer = create_user(&server).await;
  grant_user(&db, cache, viewer.id, AccessType::View).await;
  let exp = chrono::Utc::now() + chrono::Duration::days(1);
  let create = |rules: Value| {
    let server = &server;
    let viewer = &viewer;
    async move {
      viewer
        .auth(server.raw(Method::POST, "/token"))
        .json(
          &json!({ "name": common::unique("v"), "exp": exp, "restricted": true, "rules": rules }),
        )
        .send()
        .await
        .unwrap()
        .status()
    }
  };
  assert_eq!(
    create(json!([rule(cache, &["pull"], &[], &[])])).await,
    StatusCode::OK
  );
  assert_eq!(
    create(json!([rule(cache, &["push"], &[], &[])])).await,
    StatusCode::BAD_REQUEST
  );
}

#[tokio::test]
async fn test_rule_endpoint() {
  let (server, cache, admin) = setup().await;
  let app = TestNar::new("myapp-1.0", b"app");
  let glibc = TestNar::new("glibc-2.40", b"libc");
  server.push_path(&admin, "main", &app).await;
  server.push_path(&admin, "main", &glibc).await;

  let resp = server
    .post(
      "/token/test-rule",
      json!({ "cache": cache, "include": ["^myapp"], "exclude": [],
              "samples": ["myapp-2.0", "glibc-2.40", app.store_path] }),
    )
    .await;
  assert_eq!(resp.status(), StatusCode::OK);
  let body: Value = resp.json().await.unwrap();
  assert_eq!(body["error"], Value::Null);
  assert_eq!(
    body["samples"],
    json!([
      { "path": "myapp-2.0", "allowed": true },
      { "path": "glibc-2.40", "allowed": false },
      { "path": app.store_path, "allowed": true },
    ])
  );
  assert_eq!(body["allowed"], json!([app.store_path]));
  assert_eq!(body["denied"], json!([glibc.store_path]));

  let body: Value = server
    .post(
      "/token/test-rule",
      json!({ "include": ["("], "exclude": [], "samples": ["a"] }),
    )
    .await
    .json()
    .await
    .unwrap();
  assert!(body["error"].as_str().unwrap().contains("("));
  assert_eq!(body["samples"], json!([]));
  for samples in [vec!["a".repeat(301)], vec!["a".to_string(); 101]] {
    let resp = server
      .post(
        "/token/test-rule",
        json!({ "include": [], "exclude": [], "samples": samples }),
      )
      .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
  }
}
