use sea_orm_migration::prelude::*;

use crate::m20260319_194332_nar_info::NarInfo;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(
        "UPDATE nar_info SET last_accessed_at = created_at WHERE last_accessed_at IS NULL",
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(NarInfo::Table)
          .modify_column(
            ColumnDef::new(NarInfo::LastAccessedAt)
              .date_time()
              .not_null()
              .default(Expr::current_timestamp()),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .alter_table(
        Table::alter()
          .table(NarInfo::Table)
          .modify_column(ColumnDef::new(NarInfo::LastAccessedAt).date_time().null())
          .to_owned(),
      )
      .await
  }
}
