use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20260319_192505_cache::Cache, m20260319_193259_nar::Nar};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(NarInfo::Table)
          .if_not_exists()
          .col(pk_uuid(NarInfo::Id))
          .col(uuid(NarInfo::CacheId))
          .col(uuid(NarInfo::NarId))
          .col(string(NarInfo::StorePath))
          .col(string(NarInfo::Compression))
          .col(string(NarInfo::FileHash))
          .col(big_integer(NarInfo::FileSize))
          .col(string(NarInfo::NarHash))
          .col(big_integer(NarInfo::NarSize))
          .col(string_null(NarInfo::Deriver))
          .col(string(NarInfo::Signature))
          .foreign_key(
            ForeignKey::create()
              .from(NarInfo::Table, NarInfo::CacheId)
              .to(Cache::Table, Cache::Id)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .from(NarInfo::Table, NarInfo::NarId)
              .to(Nar::Table, Nar::Id)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_table(
        Table::create()
          .table(NarInfoReference::Table)
          .if_not_exists()
          .col(pk_uuid(NarInfoReference::Id))
          .col(uuid(NarInfoReference::NarInfoId))
          .col(string(NarInfoReference::StorePath))
          .foreign_key(
            ForeignKey::create()
              .from(NarInfoReference::Table, NarInfoReference::NarInfoId)
              .to(NarInfo::Table, NarInfo::Id)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(NarInfo::Table).to_owned())
      .await?;
    manager
      .drop_table(Table::drop().table(NarInfoReference::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
enum NarInfo {
  Table,
  Id,
  CacheId,
  NarId,
  StorePath,
  Compression,
  FileHash,
  FileSize,
  NarHash,
  NarSize,
  Deriver,
  Signature,
}

#[derive(DeriveIden)]
enum NarInfoReference {
  Table,
  Id,
  NarInfoId,
  StorePath,
}
