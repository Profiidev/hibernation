use std::collections::HashMap;

use entity::token_cache_rule;
use regex::{Regex, RegexBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
  Pull,
  Push,
  Delete,
}

/// What a restricted token may do in a single cache
#[derive(Debug)]
pub struct CacheRule {
  pull: bool,
  push: bool,
  delete: bool,
  include: Vec<Regex>,
  exclude: Vec<Regex>,
}

impl CacheRule {
  pub fn new(
    pull: bool,
    push: bool,
    delete: bool,
    include: &[String],
    exclude: &[String],
  ) -> Result<Self, PatternError> {
    Ok(Self {
      pull,
      push,
      delete,
      include: compile(include)?,
      exclude: compile(exclude)?,
    })
  }

  pub fn allows(&self, action: Action) -> bool {
    match action {
      Action::Pull => self.pull,
      Action::Push => self.push,
      Action::Delete => self.delete,
    }
  }

  /// Matches against `<name>-<version>`, the part of the store path after the hash
  pub fn path_allowed(&self, store_path: &str) -> bool {
    let name = path_name(store_path);
    (self.include.is_empty() || self.include.iter().any(|r| r.is_match(name)))
      && !self.exclude.iter().any(|r| r.is_match(name))
  }
}

/// Rules of a restricted token, a cache without a rule is not accessible
#[derive(Debug, Default)]
pub struct TokenScope {
  rules: HashMap<Uuid, CacheRule>,
}

impl TokenScope {
  /// Rules that fail to compile are dropped, which denies access to their cache
  pub fn from_models(models: Vec<token_cache_rule::Model>) -> Self {
    let rules = models
      .into_iter()
      .filter_map(|m| {
        let rule = CacheRule::new(
          m.pull,
          m.push,
          m.delete,
          &split_patterns(&m.include),
          &split_patterns(&m.exclude),
        );
        match rule {
          Ok(rule) => Some((m.cache_id, rule)),
          Err(e) => {
            tracing::warn!("Ignoring token rule {} with invalid pattern: {e}", m.id);
            None
          }
        }
      })
      .collect();
    Self { rules }
  }

  pub fn rule(&self, cache: Uuid, action: Action) -> Option<&CacheRule> {
    self.rules.get(&cache).filter(|r| r.allows(action))
  }
}

pub const MAX_PATTERNS: usize = 20;
const MAX_PATTERN_LEN: usize = 200;
// Bounds compile time and memory, the defaults allow ~10 MB per regex and seconds of CPU for a full rule
const REGEX_SIZE_LIMIT: usize = 64 * 1024;
const REGEX_DFA_SIZE_LIMIT: usize = 256 * 1024;

#[derive(Debug)]
pub struct PatternError {
  pub pattern: String,
  pub message: String,
}

impl std::fmt::Display for PatternError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Invalid pattern '{}': {}", self.pattern, self.message)
  }
}

fn compile(patterns: &[String]) -> Result<Vec<Regex>, PatternError> {
  if patterns.len() > MAX_PATTERNS {
    return Err(PatternError {
      pattern: String::new(),
      message: format!("at most {MAX_PATTERNS} patterns are allowed"),
    });
  }
  patterns
    .iter()
    .map(|p| {
      let err = |message: String| PatternError {
        pattern: p.clone(),
        message,
      };
      if p.is_empty() || p.len() > MAX_PATTERN_LEN || p.chars().any(char::is_control) {
        return Err(err(format!(
          "must be 1 to {MAX_PATTERN_LEN} characters without control characters"
        )));
      }
      RegexBuilder::new(p)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_DFA_SIZE_LIMIT)
        .build()
        .map_err(|e| err(e.to_string()))
    })
    .collect()
}

/// Validates patterns without keeping the compiled regexes
pub fn validate(patterns: &[String]) -> Result<(), PatternError> {
  compile(patterns).map(|_| ())
}

pub fn join_patterns(patterns: &[String]) -> String {
  patterns.join("\n")
}

pub fn split_patterns(stored: &str) -> Vec<String> {
  stored
    .lines()
    .filter(|l| !l.is_empty())
    .map(String::from)
    .collect()
}

fn path_name(store_path: &str) -> &str {
  let base = store_path.rsplit('/').next().unwrap_or(store_path);
  base.split_once('-').map_or(base, |(_, name)| name)
}

#[cfg(test)]
mod test {
  use super::*;

  fn rule(include: &[&str], exclude: &[&str]) -> CacheRule {
    let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    CacheRule::new(true, true, false, &s(include), &s(exclude)).unwrap()
  }

  const HASH: &str = "7zrlz2bfkbgnnhklhm7w9dy1sk1zkjmx";

  #[test]
  fn empty_include_allows_everything() {
    assert!(rule(&[], &[]).path_allowed(&format!("{HASH}-glibc-2.40")));
  }

  #[test]
  fn include_matches_name_not_hash() {
    let r = rule(&["^myapp-"], &[]);
    assert!(r.path_allowed(&format!("{HASH}-myapp-1.0")));
    assert!(r.path_allowed(&format!("/nix/store/{HASH}-myapp-1.0")));
    assert!(!r.path_allowed(&format!("{HASH}-glibc-2.40")));
    // the hash is not part of the matched name
    assert!(!rule(&["^7zrlz"], &[]).path_allowed(&format!("{HASH}-glibc-2.40")));
  }

  #[test]
  fn exclude_wins_over_include() {
    let r = rule(&["^myapp"], &["-dev$"]);
    assert!(r.path_allowed(&format!("{HASH}-myapp-1.0")));
    assert!(!r.path_allowed(&format!("{HASH}-myapp-1.0-dev")));
    let all_but = rule(&[], &["^glibc-"]);
    assert!(all_but.path_allowed(&format!("{HASH}-hello-2.12")));
    assert!(!all_but.path_allowed(&format!("{HASH}-glibc-2.40")));
  }

  #[test]
  fn actions() {
    let r = rule(&[], &[]);
    assert!(r.allows(Action::Pull) && r.allows(Action::Push) && !r.allows(Action::Delete));
  }

  #[test]
  fn invalid_patterns_rejected() {
    assert!(validate(&["(".into()]).is_err());
    assert!(validate(&["".into()]).is_err());
    assert!(validate(&["a\nb".into()]).is_err());
    // `lines()` strips a trailing \r when loading, so a stored pattern would differ from the validated one
    assert!(validate(&["\r".into()]).is_err());
    assert!(validate(&["^x$\r".into()]).is_err());
    assert!(validate(&vec!["a".to_string(); MAX_PATTERNS + 1]).is_err());
    assert!(validate(&["a".repeat(MAX_PATTERN_LEN + 1)]).is_err());
    // lookaround is not supported by the regex crate
    assert!(validate(&["^(?!glibc)".into()]).is_err());
    assert!(validate(&["^myapp-.*$".into()]).is_ok());
  }

  #[test]
  fn expensive_patterns_are_cheap_to_reject() {
    let start = std::time::Instant::now();
    let heavy = vec![r"\w{150}".to_string(); MAX_PATTERNS];
    assert!(validate(&heavy).is_err());
    assert!(validate(&[r"^myapp-\w+$".into()]).is_ok());
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
  }

  #[test]
  fn invalid_stored_rule_denies_cache() {
    let cache = Uuid::new_v4();
    let scope = TokenScope::from_models(vec![token_cache_rule::Model {
      id: Uuid::new_v4(),
      token_id: Uuid::new_v4(),
      cache_id: cache,
      pull: true,
      push: true,
      delete: true,
      include: "(".into(),
      exclude: String::new(),
    }]);
    assert!(scope.rule(cache, Action::Pull).is_none());
  }

  #[test]
  fn patterns_round_trip() {
    let p = vec!["^a".to_string(), "b$".to_string()];
    assert_eq!(split_patterns(&join_patterns(&p)), p);
    assert!(split_patterns("").is_empty());
  }
}
