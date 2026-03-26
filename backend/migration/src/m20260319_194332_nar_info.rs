use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20260319_192505_cache::Cache, m20260319_193259_nar::Nar};

#[derive(DeriveMigrationName)]
pub struct Migration;

const NAR_INFO_CACHE_ID_INDEX_NAME: &str = "nar_info.cache_id";
const NAR_INFO_NAR_ID_INDEX_NAME: &str = "nar_info.nar_id";
const NAR_INFO_STORE_PATH_INDEX_NAME: &str = "nar_info.store_path";
const NAR_INFO_STORE_PATH_HASH_INDEX_NAME: &str = "nar_info.store_path_hash";
const NAR_INFO_REFERENCE_NAR_INFO_ID_INDEX_NAME: &str = "nar_info_reference.nar_info_id";

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
          .col(string(NarInfo::StorePathHash))
          .col(string(NarInfo::Compression))
          .col(string_null(NarInfo::Deriver))
          .col(string(NarInfo::Signature))
          .col(date_time_null(NarInfo::LastAccessedAt))
          .col(date_time(NarInfo::CreatedAt))
          .col(big_integer(NarInfo::Accessed))
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
      .await?;

    manager
      .create_index(
        Index::create()
          .name(NAR_INFO_CACHE_ID_INDEX_NAME)
          .table(NarInfo::Table)
          .col(NarInfo::CacheId)
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(NAR_INFO_NAR_ID_INDEX_NAME)
          .table(NarInfo::Table)
          .col(NarInfo::NarId)
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(NAR_INFO_STORE_PATH_INDEX_NAME)
          .table(NarInfo::Table)
          .col(NarInfo::StorePath)
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(NAR_INFO_STORE_PATH_HASH_INDEX_NAME)
          .table(NarInfo::Table)
          .col(NarInfo::StorePathHash)
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(NAR_INFO_REFERENCE_NAR_INFO_ID_INDEX_NAME)
          .table(NarInfoReference::Table)
          .col(NarInfoReference::NarInfoId)
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_index(
        Index::drop()
          .name(NAR_INFO_REFERENCE_NAR_INFO_ID_INDEX_NAME)
          .to_owned(),
      )
      .await?;

    manager
      .drop_index(
        Index::drop()
          .name(NAR_INFO_STORE_PATH_HASH_INDEX_NAME)
          .to_owned(),
      )
      .await?;

    manager
      .drop_index(
        Index::drop()
          .name(NAR_INFO_STORE_PATH_INDEX_NAME)
          .to_owned(),
      )
      .await?;

    manager
      .drop_index(Index::drop().name(NAR_INFO_NAR_ID_INDEX_NAME).to_owned())
      .await?;

    manager
      .drop_index(Index::drop().name(NAR_INFO_CACHE_ID_INDEX_NAME).to_owned())
      .await?;

    manager
      .drop_table(Table::drop().table(NarInfoReference::Table).to_owned())
      .await?;
    manager
      .drop_table(Table::drop().table(NarInfo::Table).to_owned())
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
  StorePathHash,
  Compression,
  Deriver,
  Signature,
  LastAccessedAt,
  CreatedAt,
  Accessed,
}

#[derive(DeriveIden)]
enum NarInfoReference {
  Table,
  Id,
  NarInfoId,
  StorePath,
}
