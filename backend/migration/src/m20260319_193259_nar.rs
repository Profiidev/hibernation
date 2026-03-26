use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

const NAR_HASH_INDEX_NAME: &str = "nar.hash";
const NAR_SIZE_INDEX_NAME: &str = "nar.size";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Nar::Table)
          .if_not_exists()
          .col(pk_uuid(Nar::Id))
          .col(string(Nar::Hash))
          .col(big_integer(Nar::Size))
          .col(string(Nar::NarHash))
          .col(big_integer(Nar::NarSize))
          .col(date_time(Nar::CreatedAt))
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(NAR_HASH_INDEX_NAME)
          .table(Nar::Table)
          .col(Nar::Hash)
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(NAR_SIZE_INDEX_NAME)
          .table(Nar::Table)
          .col(Nar::Size)
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_index(Index::drop().name(NAR_HASH_INDEX_NAME).to_owned())
      .await?;

    manager
      .drop_index(Index::drop().name(NAR_SIZE_INDEX_NAME).to_owned())
      .await?;

    manager
      .drop_table(Table::drop().table(Nar::Table).to_owned())
      .await
  }
}

#[allow(clippy::enum_variant_names)]
#[derive(DeriveIden)]
pub enum Nar {
  Table,
  Id,
  Hash,
  Size,
  NarHash,
  NarSize,
  CreatedAt,
}
