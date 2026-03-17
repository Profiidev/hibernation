use chrono::Utc;
use entity::{prelude::*, token};
use sea_orm::{ActiveValue::Set, IntoActiveModel, prelude::*};

pub struct TokenTable<'db> {
  db: &'db DatabaseConnection,
}

impl<'db> TokenTable<'db> {
  pub fn new(db: &'db DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn insert(
    &self,
    user: Uuid,
    name: String,
    token: String,
    exp: DateTime,
  ) -> Result<token::Model, DbErr> {
    let active_model = token::ActiveModel {
      id: Set(Uuid::new_v4()),
      user_id: Set(user),
      name: Set(name),
      exp: Set(exp),
      token: Set(token),
      ..Default::default()
    };
    active_model.insert(self.db).await
  }

  pub async fn get_by_user(&self, user: Uuid) -> Result<Vec<token::Model>, DbErr> {
    Token::find()
      .filter(token::Column::UserId.eq(user))
      .all(self.db)
      .await
  }

  pub async fn invalidate(&self, user: Uuid, id: Uuid) -> Result<(), DbErr> {
    Token::delete_by_id(id)
      .filter(token::Column::UserId.eq(user))
      .exec(self.db)
      .await?;

    Ok(())
  }

  pub async fn replace(&self, user: Uuid, id: Uuid, hash: String) -> Result<(), DbErr> {
    let mut token = self.by_id(id).await?.into_active_model();
    if token.user_id.as_ref() != &user {
      return Err(DbErr::RecordNotFound(format!(
        "Token with id {} not found for user {}",
        id, user
      )));
    }

    token.token = Set(hash);
    token.update(self.db).await?;
    Ok(())
  }

  pub async fn get_by_token(&self, token: &str) -> Result<token::Model, DbErr> {
    Token::find()
      .filter(token::Column::Token.eq(token))
      .one(self.db)
      .await?
      .ok_or(DbErr::RecordNotFound(format!(
        "Token with value {} not found",
        token
      )))
  }

  pub async fn by_id(&self, id: Uuid) -> Result<token::Model, DbErr> {
    Token::find_by_id(id)
      .one(self.db)
      .await?
      .ok_or(DbErr::RecordNotFound(format!(
        "Token with id {} not found",
        id
      )))
  }

  pub async fn by_id_user(&self, id: Uuid, user: Uuid) -> Result<Option<token::Model>, DbErr> {
    Token::find_by_id(id)
      .filter(token::Column::UserId.eq(user))
      .one(self.db)
      .await
  }

  pub async fn get_by_name(&self, user: Uuid, name: &str) -> Result<token::Model, DbErr> {
    Token::find()
      .filter(token::Column::UserId.eq(user))
      .filter(token::Column::Name.eq(name))
      .one(self.db)
      .await?
      .ok_or(DbErr::RecordNotFound(format!(
        "Token with name {} not found for user {}",
        name, user
      )))
  }

  pub async fn token_used(&self, id: Uuid) -> Result<(), DbErr> {
    let mut token = self.by_id(id).await?.into_active_model();
    token.last_used = Set(Some(Utc::now().naive_utc()));
    token.update(self.db).await?;
    Ok(())
  }

  pub async fn update(
    &self,
    user: Uuid,
    id: Uuid,
    name: String,
    exp: DateTime,
  ) -> Result<(), DbErr> {
    let mut token = self.by_id(id).await?.into_active_model();
    if token.user_id.as_ref() != &user {
      return Err(DbErr::RecordNotFound(format!(
        "Token with id {} not found for user {}",
        id, user
      )));
    }
    token.name = Set(name);
    token.exp = Set(exp);
    token.update(self.db).await?;
    Ok(())
  }
}
