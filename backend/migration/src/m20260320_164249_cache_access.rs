use sea_orm_migration::{
  prelude::{extension::postgres::Type, *},
  schema::*,
  sea_orm::DatabaseBackend,
};

use crate::{
  m20260123_144752_user::User, m20260126_155842_group::Group, m20260319_192505_cache::Cache,
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
            .as_enum(AccessType::Enum)
            .values([AccessType::Edit, AccessType::View])
            .to_owned(),
        )
        .await?;
    }

    manager
      .create_table(
        Table::create()
          .table(CacheAccess::Table)
          .if_not_exists()
          .col(pk_auto(CacheAccess::Id))
          .col(uuid(CacheAccess::CacheId))
          .col(uuid_null(CacheAccess::UserId))
          .col(uuid_null(CacheAccess::GroupId))
          .col(custom(CacheAccess::AccessType, AccessType::Enum))
          .foreign_key(
            ForeignKey::create()
              .from(CacheAccess::Table, CacheAccess::CacheId)
              .to(Cache::Table, Cache::Id)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .from(CacheAccess::Table, CacheAccess::UserId)
              .to(User::Table, User::Id)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .from(CacheAccess::Table, CacheAccess::GroupId)
              .to(Group::Table, Group::Id)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .check(
            Expr::col(CacheAccess::UserId)
              .is_not_null()
              .and(Expr::col(CacheAccess::GroupId).is_null())
              .or(
                Expr::col(CacheAccess::UserId)
                  .is_null()
                  .and(Expr::col(CacheAccess::GroupId).is_not_null()),
              ),
          )
          .to_owned(),
      )
      .await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(CacheAccess::Table).to_owned())
      .await?;

    let backend = match manager.get_connection() {
      sea_orm_migration::SchemaManagerConnection::Connection(conn) => conn.get_database_backend(),
      sea_orm_migration::SchemaManagerConnection::Transaction(trans) => {
        trans.get_database_backend()
      }
    };

    if backend == DatabaseBackend::Postgres {
      manager
        .drop_type(Type::drop().name(AccessType::Enum).to_owned())
        .await?;
    }

    Ok(())
  }
}

#[derive(DeriveIden)]
enum AccessType {
  #[sea_orm(iden = "access_type")]
  Enum,
  View,
  Edit,
}

#[derive(DeriveIden)]
enum CacheAccess {
  Table,
  Id,
  CacheId,
  UserId,
  GroupId,
  AccessType,
}
