use centaurus::db::init::Connection;
use centaurus::error::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

use crate::db::group::GroupTable;
use crate::db::invalid_jwt::InvalidJwtTable;
use crate::db::key::KeyTable;
use crate::db::settings::SettingsTable;
use crate::db::setup::SetupTable;
use crate::db::user::UserTable;

pub mod group;
pub mod invalid_jwt;
pub mod key;
pub mod settings;
pub mod setup;
pub mod user;

pub trait DBTrait {
  fn key(&self) -> KeyTable<'_>;
  fn invalid_jwt(&self) -> InvalidJwtTable<'_>;
  fn setup(&self) -> SetupTable<'_>;
  fn group(&self) -> GroupTable<'_>;
  fn user(&self) -> UserTable<'_>;
  fn settings(&self) -> SettingsTable<'_>;
}

impl DBTrait for Connection {
  fn key(&self) -> KeyTable<'_> {
    KeyTable::new(self)
  }

  fn invalid_jwt(&self) -> InvalidJwtTable<'_> {
    InvalidJwtTable::new(self)
  }

  fn setup(&self) -> SetupTable<'_> {
    SetupTable::new(self)
  }

  fn group(&self) -> GroupTable<'_> {
    GroupTable::new(self)
  }

  fn user(&self) -> UserTable<'_> {
    UserTable::new(self)
  }

  fn settings(&self) -> SettingsTable<'_> {
    SettingsTable::new(self)
  }
}

pub async fn init(db: &Connection) -> Result<()> {
  // Enable WAL if using SQLite
  if db.0.get_database_backend() == DatabaseBackend::Sqlite {
    db.execute(Statement::from_string(
      DatabaseBackend::Sqlite,
      "PRAGMA journal_mode = WAL;".to_string(),
    ))
    .await?;
  }

  Ok(())
}
