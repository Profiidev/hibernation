use std::time::Duration;

use centaurus::db::init::Connection;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

use crate::{cache::storage::FileStorage, db::DBTrait};

pub fn start(db: Connection, storage: FileStorage) {
  tokio::spawn(async move {
    info!("Starting cache cleanup job");
    let mut interval = interval(Duration::from_mins(5));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
      interval.tick().await;
      info!("Running cache cleanup job");
      run_cleanup(&db, &storage).await;
    }
  });
}

async fn run_cleanup(db: &Connection, storage: &FileStorage) {
  let orphan = match db.nar().orphan_nars().await {
    Ok(orphan) => orphan,
    Err(e) => {
      warn!("Failed to get orphan nars: {e}");
      return;
    }
  };

  info!("Found {} orphan nars", orphan.len());
  for orphan in orphan {
    if let Err(e) = storage.delete_file(orphan.id).await {
      warn!("Failed to delete orphan nar {}: {e}", orphan.id);
    } else {
      if let Err(e) = db.nar().delete_nar(orphan.id).await {
        warn!("Failed to delete orphan nar {} from db: {e}", orphan.id);
      }
    }
  }
}
