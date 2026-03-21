use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Cache::Table)
          .if_not_exists()
          .col(pk_uuid(Cache::Id))
          .col(string(Cache::Name))
          .col(integer(Cache::Priority))
          .col(boolean(Cache::Public))
          .col(big_integer(Cache::Quota))
          .col(string(Cache::PublicSigningKey))
          .col(boolean(Cache::AllowForcePush))
          .to_owned(),
      )
      .await?;

    manager
      .create_table(
        Table::create()
          .table(DownstreamCache::Table)
          .if_not_exists()
          .col(pk_uuid(DownstreamCache::Id))
          .col(uuid(DownstreamCache::CacheId))
          .col(string(DownstreamCache::Url))
          .foreign_key(
            ForeignKey::create()
              .from(DownstreamCache::Table, DownstreamCache::CacheId)
              .to(Cache::Table, Cache::Id)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(DownstreamCache::Table).to_owned())
      .await?;
    manager
      .drop_table(Table::drop().table(Cache::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
pub enum Cache {
  Table,
  Id,
  Name,
  Priority,
  Public,
  Quota,
  PublicSigningKey,
  AllowForcePush,
}

#[derive(DeriveIden)]
enum DownstreamCache {
  Table,
  Id,
  CacheId,
  Url,
}
