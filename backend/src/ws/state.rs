use std::sync::Arc;

use aide::OperationIo;
use axum::{Extension, extract::FromRequestParts};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::{
  spawn,
  sync::mpsc::{self, Receiver, Sender},
  task::JoinHandle,
};
use tracing::debug;
use uuid::Uuid;

#[derive(Clone, FromRequestParts, OperationIo)]
#[from_request(via(Extension))]
pub struct UpdateState {
  sessions: Arc<DashMap<Uuid, DashMap<Uuid, Sender<UpdateMessage>>>>,
  #[allow(dead_code)]
  update_proxy: Arc<JoinHandle<()>>,
}

#[derive(Clone, FromRequestParts, OperationIo)]
#[from_request(via(Extension))]
pub struct Updater(Sender<UpdateTrigger>);

pub struct UpdateTrigger {
  target: Option<Uuid>,
  message: UpdateMessage,
}

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

impl UpdateState {
  pub async fn init() -> (Self, Updater) {
    let sessions: Arc<DashMap<Uuid, DashMap<Uuid, Sender<UpdateMessage>>>> =
      Arc::new(DashMap::default());
    let (sender, mut receiver) = mpsc::channel(100);
    let updater = Updater(sender);

    let update_proxy = spawn({
      let sessions = sessions.clone();
      async move {
        while let Some(message) = receiver.recv().await {
          if let Some(target) = message.target {
            debug!(
              "Sending update message to {}: {:?}",
              target, message.message
            );
            if let Some(pair) = sessions.get(&target) {
              for pair in pair.value().iter() {
                pair.value().send(message.message).await.ok();
              }
            }
          } else {
            debug!("Broadcasting update message: {:?}", message.message);
            for pair in sessions.iter() {
              for pair in pair.value().iter() {
                pair.value().send(message.message).await.ok();
              }
            }
          }
        }
      }
    });

    let state = Self {
      sessions,
      update_proxy: Arc::new(update_proxy),
    };

    (state, updater)
  }

  pub async fn create_session(&self, user: Uuid) -> (Uuid, Receiver<UpdateMessage>) {
    let (send, recv) = mpsc::channel(100);
    let user_sessions = self.sessions.entry(user).or_default();
    let uuid = Uuid::new_v4();
    user_sessions.insert(uuid, send);

    (uuid, recv)
  }

  pub async fn remove_session(&self, user: &Uuid, uuid: &Uuid) {
    if let Some(pair) = self.sessions.get(user) {
      pair.value().remove(uuid);
    }
  }
}

impl Updater {
  pub async fn broadcast(&self, msg: UpdateMessage) {
    let _ = self
      .0
      .send(UpdateTrigger {
        target: None,
        message: msg,
      })
      .await;
  }

  pub async fn send_to(&self, target: Uuid, msg: UpdateMessage) {
    let _ = self
      .0
      .send(UpdateTrigger {
        target: Some(target),
        message: msg,
      })
      .await;
  }
}
