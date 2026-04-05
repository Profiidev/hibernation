use centaurus::backend::websocket;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type Updater = websocket::state::Updater<UpdateMessage>;

impl websocket::state::UpdateMessage for UpdateMessage {}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(tag = "type")]
pub enum UpdateMessage {
  Settings,
  User { uuid: Uuid },
  UserPermissions,
  Group { uuid: Uuid },
  Token { uuid: Uuid },
  Cache { uuid: Uuid },
}
