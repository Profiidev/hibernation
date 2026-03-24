use std::{path::PathBuf, sync::Arc};

use axum::{Extension, extract::FromRequestParts};
use centaurus::{bail, error::Result, eyre::Context};
use s3::{Bucket, Region, creds::Credentials};
use tokio::{
  fs,
  io::{self, AsyncRead},
};
use tracing::info;
use uuid::Uuid;

use crate::config::StorageConfig;

#[derive(Clone, FromRequestParts)]
#[from_request(via(Extension))]
pub enum FileStorage {
  Local(PathBuf),
  S3(Arc<Bucket>),
}

impl FileStorage {
  pub async fn init(config: &StorageConfig) -> Result<Self> {
    if !config.use_s3() {
      let path = PathBuf::from(&config.storage_path);

      // Setup and check read and write permissions for the local storage path
      fs::create_dir_all(&path).await?;
      let test_file = path.join("test_permission.tmp");
      let test_content = b"test";
      fs::write(&test_file, test_content).await?;
      let read_content = fs::read(&test_file).await?;
      fs::remove_file(&test_file).await?;
      if read_content != test_content {
        bail!("Failed to verify access permission on storage path")
      }

      info!("Using local file storage at {}", path.display());
      return Ok(Self::Local(path));
    }

    let region = Region::Custom {
      region: config.s3_region.clone().unwrap(),
      endpoint: config.s3_host.clone().unwrap(),
    };

    let credentials = Credentials::new(
      Some(config.s3_access_key.as_ref().unwrap()),
      Some(config.s3_secret_key.as_ref().unwrap()),
      None,
      None,
      None,
    )
    .context("Failed to create s3 credentials")?;

    let mut bucket = Bucket::new(config.s3_bucket.as_ref().unwrap(), region, credentials)
      .context("Failed to create s3 bucket")?;

    if config.s3_force_path_style {
      bucket.set_path_style();
    }

    if !bucket
      .exists()
      .await
      .context("Failed to check if S3 Bucket exists")?
    {
      bail!("S3 bucket does not exist");
    }

    info!(
      "Using S3 file storage with bucket {}",
      config.s3_bucket.as_ref().unwrap()
    );
    Ok(Self::S3(Arc::new(*bucket)))
  }

  pub async fn save_file<R: AsyncRead + Unpin>(&self, reader: &mut R, nar_id: Uuid) -> Result<()> {
    let name = format!("{}.nar", nar_id);
    match self {
      Self::Local(path) => {
        let file_path = path.join(&name);
        let mut file = fs::File::create(&file_path).await?;
        io::copy(reader, &mut file).await?;
      }
      Self::S3(bucket) => {
        bucket
          .put_object_stream(reader, &name)
          .await
          .context("Failed to upload nar to S3 Bucket")?;
      }
    }

    Ok(())
  }

  pub async fn exists(&self, nar_id: Uuid) -> Result<bool> {
    let name = format!("{}.nar", nar_id);
    match self {
      Self::Local(path) => {
        let file_path = path.join(&name);
        Ok(file_path.exists())
      }
      Self::S3(bucket) => {
        let res = bucket.head_object(&name).await;

        match res {
          Ok(_) => Ok(true),
          Err(s3::error::S3Error::HttpFailWithBody(404, _)) => Ok(false),
          Err(e) => Err(e).context("Failed to check nar existence in S3 Bucket")?,
        }
      }
    }
  }

  pub async fn delete_file(&self, nar_id: Uuid) -> Result<()> {
    if !self.exists(nar_id).await? {
      return Ok(());
    }

    let name = format!("{}.nar", nar_id);
    match self {
      Self::Local(path) => {
        let file_path = path.join(&name);
        fs::remove_file(file_path).await?;
      }
      Self::S3(bucket) => {
        bucket
          .delete_object(&name)
          .await
          .context("Failed to delete nar from S3 Bucket")?;
      }
    }

    Ok(())
  }
}
