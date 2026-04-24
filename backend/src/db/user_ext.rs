use centaurus::{
  db::tables::{
    group::GroupTable,
    user::{SimpleGroupInfo, UserTable},
  },
  error::Result,
};
use entity::{cache, cache_access, user};
use schemars::JsonSchema;
use sea_orm::{IntoActiveModel, JoinType, QuerySelect, Set, prelude::*};
use serde::{Deserialize, Serialize};

use crate::db::group_ext::CacheMapping;

pub struct UserTableExt<'db> {
  db: &'db DatabaseConnection,
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

  async fn cache_access_for_group(&self, user: Uuid) -> Result<Vec<CacheMapping>> {
    Ok(
      cache_access::Entity::find()
        .join(JoinType::InnerJoin, cache_access::Relation::Cache.def())
        .filter(cache_access::Column::UserId.eq(user))
        .select_only()
        .column_as(cache_access::Column::CacheId, "uuid")
        .column_as(cache::Column::Name, "name")
        .column_as(cache_access::Column::AccessType, "access_type")
        .into_model::<CacheMapping>()
        .all(self.db)
        .await?,
    )
  }

  pub async fn user_info(&self, user_id: Uuid) -> Result<Option<DetailUserInfo>> {
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

  pub async fn update_cache_mappings(
    &self,
    user: Uuid,
    old_mappings: Vec<CacheMapping>,
    new_mappings: Vec<CacheMapping>,
  ) -> Result<()> {
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
