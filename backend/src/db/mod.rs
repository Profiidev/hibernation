use centaurus::db::init::Connection;

use crate::db::cache::CacheTable;
use crate::db::group::GroupTableExt;
use crate::db::nar::NarTable;
use crate::db::setup::SetupTable;
use crate::db::token::TokenTable;
use crate::db::user::UserTableExt;

pub mod cache;
pub mod group;
pub mod nar;
pub mod setup;
pub mod token;
pub mod user;

pub trait DBTrait {
  fn setup(&self) -> SetupTable<'_>;
  fn group_ext(&self) -> GroupTableExt<'_>;
  fn user_ext(&self) -> UserTableExt<'_>;
  fn token(&self) -> TokenTable<'_>;
  fn cache(&self) -> CacheTable<'_>;
  fn nar(&self) -> NarTable<'_>;
}

impl DBTrait for Connection {
  fn setup(&self) -> SetupTable<'_> {
    SetupTable::new(self)
  }

  fn group_ext(&self) -> GroupTableExt<'_> {
    GroupTableExt::new(self)
  }

  fn user_ext(&self) -> UserTableExt<'_> {
    UserTableExt::new(self)
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
