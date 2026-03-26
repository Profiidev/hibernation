use centaurus::db::init::Connection;
use centaurus::error::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

use crate::db::cache::CacheTable;
use crate::db::group::GroupTable;
use crate::db::invalid_jwt::InvalidJwtTable;
use crate::db::key::KeyTable;
use crate::db::nar::NarTable;
use crate::db::settings::SettingsTable;
use crate::db::setup::SetupTable;
use crate::db::token::TokenTable;
use crate::db::user::UserTable;

pub mod cache;
pub mod group;
pub mod invalid_jwt;
pub mod key;
pub mod nar;
pub mod settings;
pub mod setup;
pub mod token;
pub mod user;

pub trait DBTrait {
  fn key(&self) -> KeyTable<'_>;
  fn invalid_jwt(&self) -> InvalidJwtTable<'_>;
  fn setup(&self) -> SetupTable<'_>;
  fn group(&self) -> GroupTable<'_>;
  fn user(&self) -> UserTable<'_>;
  fn settings(&self) -> SettingsTable<'_>;
  fn token(&self) -> TokenTable<'_>;
  fn cache(&self) -> CacheTable<'_>;
  fn nar(&self) -> NarTable<'_>;
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

  fn token(&self) -> TokenTable<'_> {
    TokenTable::new(self)
  }

  fn cache(&self) -> CacheTable<'_> {
    CacheTable::new(self)
  }

  fn nar(&self) -> NarTable<'_> {
    NarTable::new(self)
  }
}

pub async fn init(db: &Connection) -> Result<()> {
  // Enable WAL if using SQLite
  if db.0.get_database_backend() == DatabaseBackend::Sqlite {
    db.execute(Statement::from_string(
      DatabaseBackend::Sqlite,
      "PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 60000;".to_string(),
    ))
    .await?;
  }

  Ok(())
}
