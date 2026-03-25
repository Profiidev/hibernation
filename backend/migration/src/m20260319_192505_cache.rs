use sea_orm_migration::{
  prelude::{extension::postgres::Type, *},
  schema::*,
  sea_orm::DatabaseBackend,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let backend = match manager.get_connection() {
      sea_orm_migration::SchemaManagerConnection::Connection(conn) => conn.get_database_backend(),
      sea_orm_migration::SchemaManagerConnection::Transaction(trans) => {
        trans.get_database_backend()
      }
    };

    if backend == DatabaseBackend::Postgres {
      manager
        .create_type(
          Type::create()
            .as_enum(EvictionPolicy::Enum)
            .values([
              EvictionPolicy::OldestFirst,
              EvictionPolicy::LeastRecentlyUsed,
              EvictionPolicy::LeastFrequentlyUsed,
            ])
            .to_owned(),
        )
        .await?;
    }

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
          .col(custom(Cache::EvictionPolicy, EvictionPolicy::Enum))
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
      .await?;

    let backend = match manager.get_connection() {
      sea_orm_migration::SchemaManagerConnection::Connection(conn) => conn.get_database_backend(),
      sea_orm_migration::SchemaManagerConnection::Transaction(trans) => {
        trans.get_database_backend()
      }
    };

    if backend == DatabaseBackend::Postgres {
      manager
        .drop_type(Type::drop().name(EvictionPolicy::Enum).to_owned())
        .await?;
    }

    Ok(())
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
  EvictionPolicy,
}

#[derive(DeriveIden)]
enum DownstreamCache {
  Table,
  Id,
  CacheId,
  Url,
}

#[derive(DeriveIden)]
enum EvictionPolicy {
  #[sea_orm(iden = "eviction_policy")]
  Enum,
  OldestFirst,
  LeastRecentlyUsed,
  LeastFrequentlyUsed,
}
