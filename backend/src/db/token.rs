use chrono::Utc;
use entity::{prelude::*, token, token_cache_rule};
use schemars::JsonSchema;
use sea_orm::{ActiveValue::Set, IntoActiveModel, TransactionTrait, prelude::*};
use serde::{Deserialize, Serialize};

use crate::auth::scope::{join_patterns, split_patterns};

/// Per cache permissions of a restricted token
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct TokenRule {
  pub cache: Uuid,
  pub pull: bool,
  pub push: bool,
  pub delete: bool,
  /// Regexes matched against `<name>-<version>` of a store path, empty allows every path
  pub include: Vec<String>,
  /// Regexes matched against `<name>-<version>`, a match denies the path
  pub exclude: Vec<String>,
}

impl From<token_cache_rule::Model> for TokenRule {
  fn from(m: token_cache_rule::Model) -> Self {
    Self {
      cache: m.cache_id,
      pull: m.pull,
      push: m.push,
      delete: m.delete,
      include: split_patterns(&m.include),
      exclude: split_patterns(&m.exclude),
    }
  }
}

pub struct TokenTable<'db> {
  db: &'db DatabaseConnection,
}

impl<'db> TokenTable<'db> {
  pub fn new(db: &'db DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn insert(
    &self,
    user: Uuid,
    name: String,
    token: String,
    exp: DateTime,
  ) -> Result<token::Model, DbErr> {
    let active_model = token::ActiveModel {
      id: Set(Uuid::now_v7()),
      user_id: Set(user),
      name: Set(name),
      exp: Set(exp),
      token: Set(token),
      restricted: Set(false),
      ..Default::default()
    };
    active_model.insert(self.db).await
  }

  pub async fn get_by_user(&self, user: Uuid) -> Result<Vec<token::Model>, DbErr> {
    Token::find()
      .filter(token::Column::UserId.eq(user))
      .all(self.db)
      .await
  }

  pub async fn invalidate(&self, user: Uuid, id: Uuid) -> Result<(), DbErr> {
    Token::delete_by_id(id)
      .filter(token::Column::UserId.eq(user))
      .exec(self.db)
      .await?;

    Ok(())
  }

  pub async fn invalidate_expired(&self, user: Uuid) -> Result<u64, DbErr> {
    Ok(
      Token::delete_many()
        .filter(token::Column::UserId.eq(user))
        .filter(token::Column::Exp.lt(Utc::now().naive_utc()))
        .exec(self.db)
        .await?
        .rows_affected,
    )
  }

  /// Returns `false` if no token with this id belongs to `user`.
  pub async fn replace(&self, user: Uuid, id: Uuid, hash: String) -> Result<bool, DbErr> {
    let res = Token::update_many()
      .col_expr(token::Column::Token, Expr::value(hash))
      .filter(token::Column::Id.eq(id))
      .filter(token::Column::UserId.eq(user))
      .exec(self.db)
      .await?;
    Ok(res.rows_affected > 0)
  }

  pub async fn get_by_token(&self, token: &str) -> Result<token::Model, DbErr> {
    Token::find()
      .filter(token::Column::Token.eq(token))
      .one(self.db)
      .await?
      .ok_or(DbErr::RecordNotFound(format!(
        "Token with value {} not found",
        token
      )))
  }

  pub async fn by_id(&self, id: Uuid) -> Result<token::Model, DbErr> {
    Token::find_by_id(id)
      .one(self.db)
      .await?
      .ok_or(DbErr::RecordNotFound(format!(
        "Token with id {} not found",
        id
      )))
  }

  pub async fn by_id_user(&self, id: Uuid, user: Uuid) -> Result<Option<token::Model>, DbErr> {
    Token::find_by_id(id)
      .filter(token::Column::UserId.eq(user))
      .one(self.db)
      .await
  }

  pub async fn get_by_name(&self, user: Uuid, name: &str) -> Result<token::Model, DbErr> {
    Token::find()
      .filter(token::Column::UserId.eq(user))
      .filter(token::Column::Name.eq(name))
      .one(self.db)
      .await?
      .ok_or(DbErr::RecordNotFound(format!(
        "Token with name {} not found for user {}",
        name, user
      )))
  }

  pub async fn token_used(&self, id: Uuid) -> Result<(), DbErr> {
    let mut token = self.by_id(id).await?.into_active_model();
    token.last_used = Set(Some(Utc::now().naive_utc()));
    token.update(self.db).await?;
    Ok(())
  }

  /// Returns `false` if no token with this id belongs to `user`.
  pub async fn update(
    &self,
    user: Uuid,
    id: Uuid,
    name: String,
    exp: DateTime,
  ) -> Result<bool, DbErr> {
    let res = Token::update_many()
      .col_expr(token::Column::Name, Expr::value(name))
      .col_expr(token::Column::Exp, Expr::value(exp))
      .filter(token::Column::Id.eq(id))
      .filter(token::Column::UserId.eq(user))
      .exec(self.db)
      .await?;
    Ok(res.rows_affected > 0)
  }

  pub async fn rules(&self, token: Uuid) -> Result<Vec<token_cache_rule::Model>, DbErr> {
    TokenCacheRule::find()
      .filter(token_cache_rule::Column::TokenId.eq(token))
      .all(self.db)
      .await
  }

  /// Replaces the restriction flag and all rules of a token
  pub async fn set_scope(
    &self,
    token: Uuid,
    restricted: bool,
    rules: Vec<TokenRule>,
  ) -> Result<(), DbErr> {
    self
      .db
      .transaction::<_, (), DbErr>(|db| {
        Box::pin(async move {
          Token::update_many()
            .col_expr(token::Column::Restricted, Expr::value(restricted))
            .filter(token::Column::Id.eq(token))
            .exec(db)
            .await?;
          TokenCacheRule::delete_many()
            .filter(token_cache_rule::Column::TokenId.eq(token))
            .exec(db)
            .await?;
          if !rules.is_empty() {
            TokenCacheRule::insert_many(rules.into_iter().map(|r| token_cache_rule::ActiveModel {
              id: Set(Uuid::now_v7()),
              token_id: Set(token),
              cache_id: Set(r.cache),
              pull: Set(r.pull),
              push: Set(r.push),
              delete: Set(r.delete),
              include: Set(join_patterns(&r.include)),
              exclude: Set(join_patterns(&r.exclude)),
            }))
            .exec(db)
            .await?;
          }
          Ok(())
        })
      })
      .await
      .map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) | sea_orm::TransactionError::Transaction(e) => e,
      })
  }
}
