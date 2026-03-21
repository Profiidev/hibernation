use std::collections::HashSet;

use harmonia_protocol::NarHash;
use harmonia_store_core::store_path::{StoreDir, StorePath};
use harmonia_store_remote::{DaemonClient, DaemonStore};
use tracing::{error, info, warn};

use crate::api::{ApiClient, PushInfoResult};

#[derive(Debug)]
struct PathInfo {
  store_path: StorePath,
  nar_hash: NarHash,
  nar_size: u64,
  deriver: Option<StorePath>,
  references: Vec<StorePath>,
  signatures: Vec<String>,
}

pub async fn push_paths(
  api: ApiClient,
  cache: String,
  paths: &[String],
  no_deps: bool,
  force: bool,
) {
  info!("Collecting paths for push");
  let store_dir = StoreDir::default();
  let mut conn = DaemonClient::builder().connect_daemon().await.unwrap();

  let mut queue = paths
    .iter()
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .flat_map(|s| store_dir.parse(s).ok())
    .collect::<Vec<StorePath>>();

  let mut visited = HashSet::new();
  let mut path_infos = Vec::new();

  while let Some(path) = queue.pop() {
    if visited.insert(path.clone()) {
      let info = conn.query_path_info(&path).await.unwrap().unwrap();

      path_infos.push(PathInfo {
        store_path: path,
        deriver: info.deriver,
        nar_hash: info.nar_hash,
        nar_size: info.nar_size,
        references: info.references.clone().into_iter().collect(),
        signatures: info.signatures.into_iter().map(|s| s.to_string()).collect(),
      });

      if !no_deps {
        for dep in info.references {
          queue.push(dep);
        }
      }
    }
  }

  let paths = visited.into_iter().map(|p| p.to_string()).collect();

  info!("Collected {} paths for push", path_infos.len());
  info!("Checking which paths are missing on the server");
  let res = api.push_info(cache, paths, force).await.unwrap();

  let paths = match res {
    PushInfoResult::Paths(paths) => paths,
    PushInfoResult::AllPathsExist => {
      warn!("All paths already exist on the server, nothing to push");
      std::process::exit(0);
    }
    PushInfoResult::CacheNotFound => {
      error!("Cache not found on the server");
      std::process::exit(1);
    }
    PushInfoResult::ForcePushNotAllowed => {
      error!("Force push not allowed by the cache");
      std::process::exit(1);
    }
  };

  let path_infos = path_infos
    .into_iter()
    .filter(|info| paths.contains(&info.store_path))
    .collect::<Vec<_>>();

  info!("Pushing {} paths to the server", path_infos.len());
}
