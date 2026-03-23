use centaurus::{bail, error::ErrorReportStatusExt};
use entity::{
  cache, cache_access, downstream_cache, group_user, nar, nar_info, nar_info_reference,
  sea_orm_active_enums::AccessType,
};
use harmonia_store_core::store_path::StorePath;
use http::StatusCode;
use sea_orm::{
  ActiveValue::Set, Condition, FromQueryResult, IntoActiveModel, Iterable, JoinType, QuerySelect,
  TransactionTrait, prelude::*,
};
use serde::Serialize;

use crate::{db::group::GroupTable, permissions::Permission};

#[derive(Serialize, FromQueryResult)]
pub struct CacheInfo {
  #[serde(rename = "uuid")]
  id: Uuid,
  name: String,
  #[serde(serialize_with = "serialize_i64_or_zero")]
  size: Option<i64>,
  quota: i64,
  public: bool,
}

#[derive(Serialize, FromQueryResult)]
pub struct CacheDetails {
  #[serde(rename = "uuid")]
  id: Uuid,
  name: String,
  #[serde(serialize_with = "serialize_i64_or_zero")]
  size: Option<i64>,
  quota: i64,
  public: bool,
  #[serde(rename = "sig_key")]
  public_signing_key: String,
  priority: i32,
}

#[derive(Serialize, FromQueryResult)]
pub struct SimpleCacheInfo {
  #[serde(rename = "uuid")]
  id: Uuid,
  name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryAs {
  GroupId,
}

pub struct CacheTable<'db> {
  db: &'db DatabaseConnection,
}

impl<'db> CacheTable<'db> {
  pub fn new(db: &'db DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn list_caches(&self, user: Uuid) -> Result<Vec<CacheInfo>, DbErr> {
    let mut query = cache::Entity::find()
      .join(JoinType::LeftJoin, cache::Relation::NarInfo.def())
      .join(JoinType::LeftJoin, cache::Relation::CacheAccess.def());

    query = apply_user_filter(query, self.db, user, AccessType::View).await?;

    query
      .group_by(cache::Column::Id)
      .select_only()
      .columns(cache::Column::iter())
      .column_as(nar_info::Column::FileSize.sum().cast_as("BIGINT"), "size")
      .into_model::<CacheInfo>()
      .all(self.db)
      .await
  }

  pub async fn list_caches_simple(&self, user: Uuid) -> Result<Vec<SimpleCacheInfo>, DbErr> {
    let mut query =
      cache::Entity::find().join(JoinType::LeftJoin, cache::Relation::CacheAccess.def());

    query = apply_user_filter(query, self.db, user, AccessType::View).await?;

    query
      .select_only()
      .column(cache::Column::Id)
      .column(cache::Column::Name)
      .into_model::<SimpleCacheInfo>()
      .all(self.db)
      .await
  }

  pub async fn cache_details(&self, uuid: Uuid, user: Uuid) -> Result<Option<CacheDetails>, DbErr> {
    let mut query = cache::Entity::find_by_id(uuid)
      .join(JoinType::LeftJoin, cache::Relation::NarInfo.def())
      .join(JoinType::LeftJoin, cache::Relation::CacheAccess.def());

    query = apply_user_filter(query, self.db, user, AccessType::View).await?;

    query
      .group_by(cache::Column::Id)
      .select_only()
      .columns(cache::Column::iter())
      .column_as(nar_info::Column::FileSize.sum().cast_as("BIGINT"), "size")
      .into_model::<CacheDetails>()
      .one(self.db)
      .await
  }

  pub async fn by_name(&self, name: String) -> Result<Option<cache::Model>, DbErr> {
    cache::Entity::find()
      .filter(cache::Column::Name.eq(name))
      .one(self.db)
      .await
  }

  pub async fn by_id_filtered(
    &self,
    uuid: Uuid,
    user: Uuid,
    access_type: AccessType,
  ) -> Result<Option<cache::Model>, DbErr> {
    let mut query =
      cache::Entity::find_by_id(uuid).join(JoinType::LeftJoin, cache::Relation::CacheAccess.def());

    query = apply_user_filter(query, self.db, user, access_type).await?;

    query.one(self.db).await
  }

  pub async fn by_name_filtered(
    &self,
    name: String,
    user: Uuid,
    access_type: AccessType,
  ) -> Result<Option<cache::Model>, DbErr> {
    let mut query = cache::Entity::find()
      .filter(cache::Column::Name.eq(name))
      .join(JoinType::LeftJoin, cache::Relation::CacheAccess.def());

    query = apply_user_filter(query, self.db, user, access_type).await?;

    query.one(self.db).await
  }

  pub async fn create_cache(
    &self,
    name: String,
    public: bool,
    quota: i64,
    sig_key: String,
    user: Uuid,
  ) -> centaurus::error::Result<Uuid> {
    self
      .db
      .transaction::<_, Uuid, DbErr>(|db| {
        Box::pin(async move {
          let cache = cache::ActiveModel {
            id: Set(Uuid::new_v4()),
            name: Set(name),
            public: Set(public),
            quota: Set(quota),
            public_signing_key: Set(sig_key),
            priority: Set(50),
            allow_force_push: Set(false),
          };
          let res = cache.insert(db).await?;

          let cache_access = cache_access::ActiveModel {
            cache_id: Set(res.id),
            user_id: Set(Some(user)),
            group_id: Set(None),
            access_type: Set(AccessType::Edit),
            ..Default::default()
          };
          cache_access.insert(db).await?;

          let downstream_cache = downstream_cache::ActiveModel {
            id: Set(Uuid::new_v4()),
            cache_id: Set(res.id),
            url: Set("https://cache.nixos.org".to_string()),
          };
          downstream_cache.insert(db).await?;

          Ok(res.id)
        })
      })
      .await
      .status_context(StatusCode::INTERNAL_SERVER_ERROR, "DB Error")
  }

  pub async fn delete_cache(&self, uuid: Uuid, user: Uuid) -> centaurus::error::Result<()> {
    if self
      .by_id_filtered(uuid, user, AccessType::Edit)
      .await?
      .is_none()
    {
      bail!(NOT_FOUND, "Cache not found or insufficient permissions");
    }

    cache::Entity::delete_by_id(uuid).exec(self.db).await?;

    Ok(())
  }

  pub async fn downstream_caches(&self, uuid: Uuid) -> Result<Vec<downstream_cache::Model>, DbErr> {
    downstream_cache::Entity::find()
      .filter(downstream_cache::Column::CacheId.eq(uuid))
      .all(self.db)
      .await
  }

  pub async fn missing_paths(
    &self,
    cache: Uuid,
    paths: Vec<StorePath>,
  ) -> Result<Vec<StorePath>, DbErr> {
    if paths.is_empty() {
      return Ok(vec![]);
    }

    let paths = paths.into_iter().map(|p| p.to_string()).collect::<Vec<_>>();

    let mut query = sea_orm::sea_query::Query::select();
    query
      .column(("inputs_paths", "column1"))
      .from_values(paths, "inputs_paths")
      .left_join(
        nar_info::Entity,
        Expr::col(("inputs_paths", "column1"))
          .equals((nar_info::Entity, nar_info::Column::StorePath))
          .and(Expr::col((nar_info::Entity, nar_info::Column::CacheId)).eq(cache)),
      )
      .and_where(Expr::col((nar_info::Entity, nar_info::Column::StorePath)).is_null());

    let builder = self.db.get_database_backend();
    let missing = self
      .db
      .query_all(builder.build(&query))
      .await?
      .into_iter()
      .map(|row| StorePath::from_base_path(&row.try_get_by_index::<String>(0).unwrap()).unwrap())
      .collect();

    Ok(missing)
  }

  #[allow(clippy::too_many_arguments)]
  pub async fn create_path(
    &self,
    nar_id: Uuid,
    cache: Uuid,
    store_path: String,
    nar_hash: String,
    nar_size: u64,
    file_hash: String,
    file_size: u64,
    deriver: Option<String>,
    signature: String,
    references: Vec<String>,
  ) -> centaurus::error::Result<()> {
    self
      .db
      .transaction::<_, (), DbErr>(|db| {
        Box::pin(async move {
          // Check if a nar with the same hash and size exists
          let existing_nar = nar::Entity::find()
            .filter(nar::Column::Hash.eq(file_hash.clone()))
            .filter(nar::Column::Size.eq(file_size as i64))
            .one(db)
            .await?;

          let nar = if let Some(existing_nar) = existing_nar {
            existing_nar
          } else {
            let mut nar = nar::Entity::find_by_id(nar_id)
              .one(db)
              .await?
              .ok_or(DbErr::RecordNotFound("Nar not found".to_string()))?
              .into_active_model();

            nar.hash = Set(file_hash.clone());
            nar.size = Set(file_size as i64);
            nar.update(db).await?
          };

          let nar_info = nar_info::ActiveModel {
            id: Set(Uuid::new_v4()),
            nar_id: Set(nar.id),
            cache_id: Set(cache),
            compression: Set("zst".to_string()),
            store_path: Set(store_path),
            nar_hash: Set(nar_hash),
            nar_size: Set(nar_size as i64),
            file_hash: Set(file_hash),
            file_size: Set(file_size as i64),
            deriver: Set(deriver),
            signature: Set(signature),
          };
          let nar_info = nar_info.insert(db).await?;

          let mut nar_refernces = Vec::new();
          for reference in references {
            let reference = nar_info_reference::ActiveModel {
              id: Set(Uuid::new_v4()),
              nar_info_id: Set(nar_info.id),
              store_path: Set(reference),
            };
            nar_refernces.push(reference);
          }

          if !nar_refernces.is_empty() {
            nar_info_reference::Entity::insert_many(nar_refernces)
              .exec(db)
              .await?;
          }

          Ok(())
        })
      })
      .await
      .status_context(StatusCode::INTERNAL_SERVER_ERROR, "DB Error")
  }

  pub async fn is_store_path_in_cache(&self, cache: Uuid, store_path: &str) -> Result<bool, DbErr> {
    let count = nar_info::Entity::find()
      .filter(nar_info::Column::CacheId.eq(cache))
      .filter(nar_info::Column::StorePath.eq(store_path.to_string()))
      .count(self.db)
      .await?;

    Ok(count > 0)
  }

  pub async fn create_nar(&self, nar_id: Uuid) -> Result<(), DbErr> {
    let nar = nar::ActiveModel {
      id: Set(nar_id),
      hash: Set("".to_string()),
      size: Set(0),
    };
    nar.insert(self.db).await?;

    Ok(())
  }
}

fn serialize_i64_or_zero<S>(num: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  let num = num.unwrap_or(0);
  serializer.serialize_i64(num)
}

async fn apply_user_filter<Q: QueryFilter>(
  mut query: Q,
  db: &DatabaseConnection,
  user: Uuid,
  access_type: AccessType,
) -> Result<Q, DbErr> {
  let permission = match access_type {
    AccessType::View => crate::permissions::CacheView::name(),
    AccessType::Edit => crate::permissions::CacheEdit::name(),
  };

  let hash_general_permission = GroupTable::new(db)
    .user_hash_permissions(user, permission)
    .await?;

  if !hash_general_permission {
    // If the user doesn't have the general permission, they must have specific access to the cache
    let group_ids = group_user::Entity::find()
      .filter(group_user::Column::UserId.eq(user))
      .select_only()
      .column_as(group_user::Column::GroupId, QueryAs::GroupId)
      .into_values::<Uuid, QueryAs>()
      .all(db)
      .await?;

    let mut user_cond = cache_access::Column::UserId.eq(user);
    let mut group_cond = cache_access::Column::GroupId.is_in(group_ids);
    let mut cond = Condition::any();

    // if only read access is requested, also include entries with edit access
    if access_type == AccessType::Edit {
      user_cond = user_cond.and(cache_access::Column::AccessType.eq(AccessType::Edit));
      group_cond = group_cond.and(cache_access::Column::AccessType.eq(AccessType::Edit));
    } else {
      // Only allow public caches if read access is sufficient. write requires explicit permissions
      cond = cond.add(cache::Column::Public.eq(true))
    }

    query = query.filter(cond.add(user_cond).add(group_cond));
  };

  Ok(query)
}
