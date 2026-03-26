use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260123_144752_user::User;

#[derive(DeriveMigrationName)]
pub struct Migration;

const TOKEN_USER_ID_INDEX_NAME: &str = "token.user_id";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Token::Table)
          .if_not_exists()
          .col(pk_uuid(Token::Id))
          .col(uuid(Token::UserId))
          .col(string(Token::Name))
          .col(date_time(Token::Exp))
          .col(string(Token::Token))
          .col(date_time_null(Token::LastUsed))
          .foreign_key(
            ForeignKey::create()
              .from(Token::Table, Token::UserId)
              .to(User::Table, User::Id)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(TOKEN_USER_ID_INDEX_NAME)
          .table(Token::Table)
          .col(Token::UserId)
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_index(Index::drop().name(TOKEN_USER_ID_INDEX_NAME).to_owned())
      .await?;

    manager
      .drop_table(Table::drop().table(Token::Table).to_owned())
      .await
  }
}

#[allow(clippy::enum_variant_names)]
#[derive(DeriveIden)]
enum Token {
  Table,
  Id,
  UserId,
  Name,
  Exp,
  Token,
  LastUsed,
}
