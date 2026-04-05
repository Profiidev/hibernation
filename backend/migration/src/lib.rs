pub use sea_orm_migration::prelude::*;

mod m20260126_160754_setup;
mod m20260129_154755_user_avatar;
mod m20260317_135750_token;
mod m20260319_192505_cache;
mod m20260319_193259_nar;
mod m20260319_194332_nar_info;
mod m20260320_164249_cache_access;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
      Box::new(centaurus::db::migrations::key::Migration),
      Box::new(centaurus::db::migrations::invalid_jwt::Migration),
      Box::new(centaurus::db::migrations::settings::Migration),
      Box::new(centaurus::db::migrations::user::Migration),
      Box::new(centaurus::db::migrations::groups::Migration),
      Box::new(m20260126_160754_setup::Migration),
      Box::new(m20260129_154755_user_avatar::Migration),
      Box::new(m20260317_135750_token::Migration),
      Box::new(m20260319_192505_cache::Migration),
      Box::new(m20260319_193259_nar::Migration),
      Box::new(m20260319_194332_nar_info::Migration),
      Box::new(m20260320_164249_cache_access::Migration),
    ]
  }
}
