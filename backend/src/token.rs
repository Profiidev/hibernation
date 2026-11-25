use aide::axum::ApiRouter;
use aide::axum::routing::{delete_with, get_with, post_with, put_with};
use axum::{Json, extract::Path};
use centaurus::backend::auth::jwt_auth::JwtAuth;
use centaurus::backend::auth::pw_state::PasswordState;
use centaurus::{bail, db::init::Connection, error::Result};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::collections::HashSet;

use entity::sea_orm_active_enums::AccessType;
use harmonia_store_core::store_path::StorePath;

use crate::auth::cli_auth::CliTokenCache;
use crate::auth::scope::{CacheRule, validate};
use crate::db::token::TokenRule;
use crate::utils::{UpdateMessage, Updater};
use crate::{cli, db::DBTrait};

pub fn router() -> ApiRouter {
  ApiRouter::new()
    .api_route("/", get_with(list_tokens, |op| op.id("listTokens")))
    .api_route("/", post_with(create_token, |op| op.id("createToken")))
    .api_route("/", delete_with(delete_token, |op| op.id("deleteToken")))
    .api_route("/", put_with(edit_token, |op| op.id("editToken")))
    .api_route(
      "/expired",
      delete_with(delete_expired_tokens, |op| op.id("deleteExpiredTokens")),
    )
    .api_route(
      "/test-rule",
      post_with(test_rule, |op| op.id("testTokenRule")),
    )
    .api_route("/{uuid}", get_with(token_info, |op| op.id("tokenInfo")))
    .api_route(
      "/{uuid}",
      post_with(token_regenerate, |op| op.id("tokenRegenerate")),
    )
}

#[derive(Serialize, JsonSchema)]
struct TokenInfo {
  uuid: Uuid,
  name: String,
  exp: DateTime<Utc>,
  last_used: Option<DateTime<Utc>>,
  /// Only the rules apply, without any rule the token can't access a cache
  restricted: bool,
}

#[derive(Serialize, JsonSchema)]
struct TokenDetails {
  #[serde(flatten)]
  info: TokenInfo,
  rules: Vec<TokenRule>,
}

impl From<entity::token::Model> for TokenInfo {
  fn from(value: entity::token::Model) -> Self {
    Self {
      uuid: value.id,
      name: value.name,
      exp: DateTime::from_naive_utc_and_offset(value.exp, Utc),
      last_used: value
        .last_used
        .map(|d| DateTime::from_naive_utc_and_offset(d, Utc)),
      restricted: value.restricted,
    }
  }
}

async fn list_tokens(auth: JwtAuth, db: Connection) -> Result<Json<Vec<TokenInfo>>> {
  let tokens = db.token().get_by_user(auth.user_id).await?;
  let token_info = tokens.into_iter().map(|t| t.into()).collect();
  Ok(Json(token_info))
}

#[derive(Deserialize, JsonSchema)]
struct TokenViewPath {
  uuid: Uuid,
}

async fn token_info(
  auth: JwtAuth,
  db: Connection,
  Path(path): Path<TokenViewPath>,
) -> Result<Json<TokenDetails>> {
  let info = db.token().by_id_user(path.uuid, auth.user_id).await?;
  let Some(info) = info else {
    bail!(NOT_FOUND, "Token not found");
  };
  let rules = db.token().rules(info.id).await?;
  Ok(Json(TokenDetails {
    info: info.into(),
    rules: rules.into_iter().map(Into::into).collect(),
  }))
}

/// Rejects rules the user could not use themselves, a token never exceeds its user's access
async fn validate_rules(db: &Connection, user: Uuid, rules: &[TokenRule]) -> Result<()> {
  let mut seen = HashSet::new();
  for rule in rules {
    if !seen.insert(rule.cache) {
      bail!(BAD_REQUEST, "Only one rule per cache is allowed");
    }
    if let Err(e) = validate(&rule.include).and_then(|_| validate(&rule.exclude)) {
      bail!(BAD_REQUEST, "{e}");
    }
    // Global cache permissions grant access to any id, so check the cache exists
    if db.cache().by_id(rule.cache).await?.is_none() {
      bail!(BAD_REQUEST, "Cache {} not found", rule.cache);
    }
    let access = db.cache().cache_user_access(user, rule.cache).await?;
    let ok = match access {
      Some(AccessType::Edit) => true,
      Some(AccessType::View) => !rule.push && !rule.delete,
      None => false,
    };
    if !ok {
      bail!(
        BAD_REQUEST,
        "Rule grants more access to cache {} than you have",
        rule.cache
      );
    }
  }
  Ok(())
}

#[derive(Deserialize, JsonSchema)]
struct CreateTokenRequest {
  name: String,
  exp: DateTime<Utc>,
  #[serde(default)]
  restricted: bool,
  #[serde(default)]
  rules: Vec<TokenRule>,
}

#[derive(Serialize, JsonSchema)]
struct CreateTokenResponse {
  token: String,
  uuid: Uuid,
}

async fn create_token(
  auth: JwtAuth,
  db: Connection,
  pw: PasswordState,
  updater: Updater,
  Json(req): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>> {
  if req.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Token name cannot be empty");
  }

  if db
    .token()
    .get_by_name(auth.user_id, &req.name)
    .await
    .is_ok()
  {
    bail!(CONFLICT, "A token with this name already exists");
  }
  validate_rules(&db, auth.user_id, &req.rules).await?;

  let token = cli::gen_token();
  let hash = pw.pw_hash_raw("", &token)?;

  let uuid = db
    .token()
    .insert(auth.user_id, req.name, hash, req.exp.naive_utc())
    .await?
    .id;
  if req.restricted || !req.rules.is_empty() {
    db.token()
      .set_scope(uuid, req.restricted, req.rules)
      .await?;
  }
  updater
    .send_to(auth.user_id, UpdateMessage::Token { uuid })
    .await;

  Ok(Json(CreateTokenResponse { token, uuid }))
}

#[derive(Deserialize, JsonSchema)]
struct DeleteTokenRequest {
  uuid: Uuid,
}

async fn delete_token(
  auth: JwtAuth,
  db: Connection,
  updater: Updater,
  token_cache: CliTokenCache,
  Json(req): Json<DeleteTokenRequest>,
) -> Result<()> {
  db.token().invalidate(auth.user_id, req.uuid).await?;
  token_cache.invalidate(req.uuid);
  updater
    .send_to(auth.user_id, UpdateMessage::Token { uuid: req.uuid })
    .await;
  Ok(())
}

#[derive(Serialize, JsonSchema)]
struct DeleteExpiredTokensResponse {
  deleted: u64,
}

async fn delete_expired_tokens(
  auth: JwtAuth,
  db: Connection,
  updater: Updater,
) -> Result<Json<DeleteExpiredTokensResponse>> {
  let deleted = db.token().invalidate_expired(auth.user_id).await?;
  if deleted > 0 {
    updater
      .send_to(auth.user_id, UpdateMessage::Token { uuid: Uuid::nil() })
      .await;
  }
  Ok(Json(DeleteExpiredTokensResponse { deleted }))
}

#[derive(Deserialize, JsonSchema)]
struct EditTokenRequest {
  uuid: Uuid,
  name: String,
  exp: DateTime<Utc>,
  /// Omitted keeps the current restriction and rules
  restricted: Option<bool>,
  /// Omitted keeps the current rules
  rules: Option<Vec<TokenRule>>,
}

async fn edit_token(
  auth: JwtAuth,
  db: Connection,
  updater: Updater,
  token_cache: CliTokenCache,
  Json(req): Json<EditTokenRequest>,
) -> Result<()> {
  if req.name.trim().is_empty() {
    bail!(BAD_REQUEST, "Token name cannot be empty");
  }

  if let Ok(token) = db.token().get_by_name(auth.user_id, &req.name).await
    && token.id != req.uuid
  {
    bail!(CONFLICT, "A token with this name already exists");
  }
  if let Some(rules) = &req.rules {
    validate_rules(&db, auth.user_id, rules).await?;
  }

  if !db
    .token()
    .update(auth.user_id, req.uuid, req.name, req.exp.naive_utc())
    .await?
  {
    bail!(NOT_FOUND, "Token not found");
  }
  if req.restricted.is_some() || req.rules.is_some() {
    let current = db.token().by_id(req.uuid).await?;
    let restricted = req.restricted.unwrap_or(current.restricted);
    let rules = match req.rules {
      Some(rules) => rules,
      None => db
        .token()
        .rules(req.uuid)
        .await?
        .into_iter()
        .map(Into::into)
        .collect(),
    };
    db.token().set_scope(req.uuid, restricted, rules).await?;
  }
  token_cache.invalidate(req.uuid);
  updater
    .send_to(auth.user_id, UpdateMessage::Token { uuid: req.uuid })
    .await;
  Ok(())
}

#[derive(Serialize, JsonSchema)]
struct TokenRegenerateResponse {
  token: String,
}

async fn token_regenerate(
  auth: JwtAuth,
  db: Connection,
  pw: PasswordState,
  token_cache: CliTokenCache,
  updater: Updater,
  Path(path): Path<TokenViewPath>,
) -> Result<Json<TokenRegenerateResponse>> {
  let token = cli::gen_token();
  let hash = pw.pw_hash_raw("", &token)?;

  if !db.token().replace(auth.user_id, path.uuid, hash).await? {
    bail!(NOT_FOUND, "Token not found");
  }
  token_cache.invalidate(path.uuid);
  updater
    .send_to(auth.user_id, UpdateMessage::Token { uuid: path.uuid })
    .await;

  Ok(Json(TokenRegenerateResponse { token }))
}

const TEST_RULE_SCAN_LIMIT: u64 = 1000;
const TEST_RULE_RESULT_LIMIT: usize = 50;
const TEST_RULE_MAX_SAMPLES: usize = 100;
// Longer than any store path (`/nix/store/` + 32 char hash + 211 char name)
const TEST_RULE_MAX_SAMPLE_LEN: usize = 300;

#[derive(Deserialize, JsonSchema)]
struct TestRuleRequest {
  /// Also classify paths already in this cache
  cache: Option<Uuid>,
  include: Vec<String>,
  exclude: Vec<String>,
  /// Store path names or full store paths to check
  samples: Vec<String>,
}

#[derive(Serialize, JsonSchema)]
struct TestRuleSample {
  path: String,
  allowed: bool,
}

#[derive(Serialize, JsonSchema)]
struct TestRuleResponse {
  /// Set when a pattern is invalid, everything else is empty then
  error: Option<String>,
  samples: Vec<TestRuleSample>,
  /// Existing paths of the cache that match
  allowed: Vec<String>,
  /// Existing paths of the cache that don't match
  denied: Vec<String>,
}

async fn test_rule(
  auth: JwtAuth,
  db: Connection,
  Json(req): Json<TestRuleRequest>,
) -> Result<Json<TestRuleResponse>> {
  if req.samples.len() > TEST_RULE_MAX_SAMPLES
    || req
      .samples
      .iter()
      .any(|s| s.len() > TEST_RULE_MAX_SAMPLE_LEN)
  {
    bail!(
      BAD_REQUEST,
      "At most {TEST_RULE_MAX_SAMPLES} samples of {TEST_RULE_MAX_SAMPLE_LEN} characters are allowed"
    );
  }
  let rule = match CacheRule::new(true, true, true, &req.include, &req.exclude) {
    Ok(rule) => rule,
    Err(e) => {
      return Ok(Json(TestRuleResponse {
        error: Some(e.to_string()),
        samples: Vec::new(),
        allowed: Vec::new(),
        denied: Vec::new(),
      }));
    }
  };

  let samples = req
    .samples
    .into_iter()
    .map(|path| {
      // A bare name has no hash, prefix a dummy one so it's matched like a store path
      let allowed = if path.contains('/') || has_hash(&path) {
        rule.path_allowed(&path)
      } else {
        rule.path_allowed(&format!("hash-{path}"))
      };
      TestRuleSample { path, allowed }
    })
    .collect();

  let (mut allowed, mut denied) = (Vec::new(), Vec::new());
  if let Some(cache) = req.cache
    && db
      .cache()
      .cache_user_access(auth.user_id, cache)
      .await?
      .is_some()
  {
    // shortcut: only scans the most recently accessed paths, page through the cache if users need full coverage
    for path in db.nar().store_paths(cache, TEST_RULE_SCAN_LIMIT).await? {
      let list = if rule.path_allowed(&path) {
        &mut allowed
      } else {
        &mut denied
      };
      if list.len() < TEST_RULE_RESULT_LIMIT {
        list.push(path);
      }
    }
  }

  Ok(Json(TestRuleResponse {
    error: None,
    samples,
    allowed,
    denied,
  }))
}

fn has_hash(path: &str) -> bool {
  StorePath::from_base_path(path).is_ok()
}
