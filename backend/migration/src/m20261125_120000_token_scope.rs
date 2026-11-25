use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20260317_135750_token::Token, m20260319_192505_cache::Cache};

#[derive(DeriveMigrationName)]
pub struct Migration;

const RULE_TOKEN_CACHE_INDEX_NAME: &str = "token_cache_rule.token_id_cache_id";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .alter_table(
        Table::alter()
          .table(Token::Table)
          .add_column(boolean(TokenScope::Restricted).default(false))
          .to_owned(),
      )
      .await?;

    manager
      .create_table(
        Table::create()
          .table(TokenCacheRule::Table)
          .if_not_exists()
          .col(pk_uuid(TokenCacheRule::Id))
          .col(uuid(TokenCacheRule::TokenId))
          .col(uuid(TokenCacheRule::CacheId))
          .col(boolean(TokenCacheRule::Pull))
          .col(boolean(TokenCacheRule::Push))
          .col(boolean(TokenCacheRule::Delete))
          // Regexes separated by newlines, a store path name can't contain one
          .col(text(TokenCacheRule::Include))
          .col(text(TokenCacheRule::Exclude))
          .foreign_key(
            ForeignKey::create()
              .from(TokenCacheRule::Table, TokenCacheRule::TokenId)
              .to(Token::Table, Token::Id)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .from(TokenCacheRule::Table, TokenCacheRule::CacheId)
              .to(Cache::Table, Cache::Id)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .if_not_exists()
          .name(RULE_TOKEN_CACHE_INDEX_NAME)
          .table(TokenCacheRule::Table)
          .col(TokenCacheRule::TokenId)
          .col(TokenCacheRule::CacheId)
          .unique()
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_index(
        Index::drop()
          .name(RULE_TOKEN_CACHE_INDEX_NAME)
          .table(TokenCacheRule::Table)
          .to_owned(),
      )
      .await?;

    manager
      .drop_table(Table::drop().table(TokenCacheRule::Table).to_owned())
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Token::Table)
          .drop_column(TokenScope::Restricted)
          .to_owned(),
      )
      .await
  }
}

#[derive(DeriveIden)]
enum TokenScope {
  Restricted,
}

#[derive(DeriveIden)]
enum TokenCacheRule {
  Table,
  Id,
  TokenId,
  CacheId,
  Pull,
  Push,
  Delete,
  Include,
  Exclude,
}
