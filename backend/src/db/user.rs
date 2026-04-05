use std::io::Cursor;

use base64::prelude::*;
use centaurus::db::tables::{
  group::GroupTable,
  user::{SimpleGroupInfo, UserTable},
};
use entity::{cache, cache_access, group, group_user, user};
use image::{ImageFormat, imageops::FilterType};
use schemars::JsonSchema;
use sea_orm::{IntoActiveModel, JoinType, QuerySelect, Set, prelude::*};
use serde::{Deserialize, Serialize};

use crate::db::group::CacheMapping;

pub struct UserTableExt<'db> {
  db: &'db DatabaseConnection,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct UserInfo {
  pub uuid: Uuid,
  pub name: String,
  pub email: String,
  pub avatar: Option<String>,
  pub groups: Vec<SimpleGroupInfo>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DetailUserInfo {
  pub uuid: Uuid,
  pub name: String,
  pub email: String,
  pub avatar: Option<String>,
  pub groups: Vec<SimpleGroupInfo>,
  pub permissions: Vec<String>,
  pub caches: Vec<CacheMapping>,
}

impl<'db> UserTableExt<'db> {
  pub fn new(db: &'db DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn create_user(
    &self,
    username: String,
    email: String,
    password: String,
    salt: String,
  ) -> centaurus::error::Result<Uuid> {
    let url = centaurus::gravatar::get_gravatar_url(&email);
    let data = match reqwest::get(&url).await {
      Ok(response) => {
        if response.status().is_success() {
          match response.bytes().await {
            Ok(bytes) => {
              let img = image::load_from_memory(&bytes)?;
              let img = img.resize_exact(128, 128, FilterType::Lanczos3);

              let mut buf = Cursor::new(Vec::new());
              img.write_to(&mut buf, ImageFormat::WebP)?;
              let avatar = BASE64_STANDARD.encode(buf.into_inner());
              Some(avatar)
            }
            Err(_) => None,
          }
        } else {
          None
        }
      }
      Err(_) => None,
    };

    let model = user::Model {
      id: Uuid::new_v4(),
      name: username,
      email,
      password,
      salt,
      avatar: data,
    }
    .into_active_model();

    let ret = model.insert(self.db).await?;

    Ok(ret.id)
  }

  pub async fn get_user_by_id(&self, id: Uuid) -> Result<user::Model, DbErr> {
    user::Entity::find_by_id(id)
      .one(self.db)
      .await?
      .ok_or(DbErr::RecordNotFound(format!(
        "User with id {} not found",
        id
      )))
  }

  pub async fn update_user_avatar(&self, id: Uuid, new_avatar: String) -> Result<(), DbErr> {
    let mut user: user::ActiveModel = self.get_user_by_id(id).await?.into();

    user.avatar = Set(Some(new_avatar));

    user.update(self.db).await?;

    Ok(())
  }

  pub async fn list_users(&self) -> Result<Vec<UserInfo>, DbErr> {
    let users = user::Entity::find().all(self.db).await?;
    let group_user = users
      .load_many_to_many(group::Entity, group_user::Entity, self.db)
      .await?;

    let result = users
      .into_iter()
      .zip(group_user.into_iter())
      .map(|(user, groups)| UserInfo {
        uuid: user.id,
        name: user.name,
        email: user.email,
        avatar: user.avatar,
        groups: groups
          .into_iter()
          .map(|group| SimpleGroupInfo {
            uuid: group.id,
            name: group.name,
          })
          .collect(),
      })
      .collect();

    Ok(result)
  }

  async fn cache_access_for_group(&self, user: Uuid) -> Result<Vec<CacheMapping>, DbErr> {
    cache_access::Entity::find()
      .join(JoinType::InnerJoin, cache_access::Relation::Cache.def())
      .filter(cache_access::Column::UserId.eq(user))
      .select_only()
      .column_as(cache_access::Column::CacheId, "uuid")
      .column_as(cache::Column::Name, "name")
      .column_as(cache_access::Column::AccessType, "access_type")
      .into_model::<CacheMapping>()
      .all(self.db)
      .await
  }

  pub async fn user_info(&self, user_id: Uuid) -> Result<Option<DetailUserInfo>, DbErr> {
    let user = user::Entity::find_by_id(user_id).one(self.db).await?;
    let Some(user) = user else {
      return Ok(None);
    };

    let user_table = UserTable::new(self.db);
    let groups = user_table.get_user_groups(user_id).await?;
    let permissions = GroupTable::new(self.db)
      .get_user_permissions(user_id)
      .await?;

    let caches = self.cache_access_for_group(user_id).await?;

    Ok(Some(DetailUserInfo {
      uuid: user.id,
      name: user.name,
      email: user.email,
      avatar: user.avatar,
      groups,
      permissions,
      caches,
    }))
  }

  pub async fn reset_avatar(&self, user_id: Uuid) -> Result<(), DbErr> {
    let mut user: user::ActiveModel = self.get_user_by_id(user_id).await?.into();

    user.avatar = Set(None);

    user.update(self.db).await?;

    Ok(())
  }

  pub async fn update_cache_mappings(
    &self,
    user: Uuid,
    old_mappings: Vec<CacheMapping>,
    new_mappings: Vec<CacheMapping>,
  ) -> Result<(), DbErr> {
    // Delete old mappings
    let cache_ids_to_delete: Vec<Uuid> = old_mappings.iter().map(|m| m.uuid).collect();
    cache_access::Entity::delete_many()
      .filter(cache_access::Column::UserId.eq(user))
      .filter(cache_access::Column::CacheId.is_in(cache_ids_to_delete))
      .exec(self.db)
      .await?;

    // Insert new mappings
    let mut models = Vec::new();
    for mapping in new_mappings {
      let model = cache_access::ActiveModel {
        user_id: Set(Some(user)),
        cache_id: Set(mapping.uuid),
        access_type: Set(mapping.access_type),
        ..Default::default()
      }
      .into_active_model();
      models.push(model);
    }

    if models.is_empty() {
      return Ok(());
    }

    cache_access::Entity::insert_many(models)
      .exec(self.db)
      .await?;

    Ok(())
  }
}
