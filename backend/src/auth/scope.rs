use std::collections::HashMap;

use entity::token_cache_rule;
use regex_automata::{
  Input,
  dfa::{Automaton, dense},
  nfa::thompson,
  util::syntax,
};
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
  include: Vec<Pattern>,
  exclude: Vec<Pattern>,
}

/// A fully built DFA, so matching is linear in the name length with no slow fallback. Patterns whose DFA would
/// be too large (e.g. `a[ab]{100}c`) are rejected when compiling instead of costing CPU on every request.
pub struct Pattern(dense::DFA<Vec<u32>>);

impl std::fmt::Debug for Pattern {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Pattern")
  }
}

impl Pattern {
  /// `None` if the search gave up, which can't happen without quit bytes but is handled by callers fail-closed
  fn is_match(&self, name: &str) -> Option<bool> {
    self
      .0
      .try_search_fwd(&Input::new(name).earliest(true))
      .ok()
      .map(|m| m.is_some())
  }
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
    (self.include.is_empty() || self.include.iter().any(|r| r.is_match(name) == Some(true)))
      && !self.exclude.iter().any(|r| r.is_match(name) != Some(false))
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
// Bound compile time and memory. Patterns are ASCII only (store path names are ASCII), which keeps classes like
// `\w` small enough for normal patterns to fit easily
const NFA_SIZE_LIMIT: usize = 64 * 1024;
const DFA_SIZE_LIMIT: usize = 256 * 1024;

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

fn build(pattern: &str) -> Result<Pattern, String> {
  dense::Builder::new()
    .configure(
      dense::Config::new()
        .dfa_size_limit(Some(DFA_SIZE_LIMIT))
        .determinize_size_limit(Some(DFA_SIZE_LIMIT)),
    )
    .syntax(syntax::Config::new().unicode(false).utf8(false))
    .thompson(thompson::Config::new().nfa_size_limit(Some(NFA_SIZE_LIMIT)))
    .build(pattern)
    .map(Pattern)
    .map_err(|e| e.to_string())
}

fn compile(patterns: &[String]) -> Result<Vec<Pattern>, PatternError> {
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
      build(p).map_err(err)
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
    // DFA blowup: would thrash a lazy DFA and fall back to a slow matcher on every request
    let start = std::time::Instant::now();
    let heavy = (0..MAX_PATTERNS)
      .map(|i| format!("a[ab]{{150}}c{i}"))
      .collect::<Vec<_>>();
    for p in &heavy {
      assert!(validate(std::slice::from_ref(p)).is_err(), "{p}");
    }
    let elapsed = start.elapsed();
    assert!(elapsed < std::time::Duration::from_secs(2), "{elapsed:?}");
  }

  #[test]
  fn common_patterns_are_accepted() {
    for p in [
      r"^myapp-\w+$",
      r"\w\w",
      r"\w+-\w+",
      r"^\w+-\w+-\w+-\w+$",
      r"\w{150}",
      r"(?i)MyApp",
      r"^[\w.-]+-dev$",
      r"^(foo|bar|baz)-[0-9.]+(-dev)?$",
    ] {
      assert!(validate(&[p.into()]).is_ok(), "{p}");
    }
    let r = rule(&[r"(?i)^myapp-\d"], &[]);
    assert!(r.path_allowed(&format!("{HASH}-MYAPP-1.0")));
    // unanchored unless the pattern says otherwise
    assert!(rule(&["app"], &[]).path_allowed(&format!("{HASH}-myapp-1.0")));
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
