use base64::{Engine, prelude::BASE64_STANDARD};
use ed25519_compact::Signature;
use harmonia_store_core::store_path::{StoreDir, StorePath};

#[derive(Clone)]
pub struct KeyPair {
  name: String,
  keypair: ed25519_compact::KeyPair,
}

#[derive(Clone)]
pub struct PublicKey {
  name: String,
  public_key: ed25519_compact::PublicKey,
}

impl KeyPair {
  pub fn from_string(s: &str) -> Option<Self> {
    let parts = s.splitn(2, ':').collect::<Vec<_>>();
    if parts.len() != 2 {
      return None;
    }

    let name = parts[0].to_string();
    let keypair_bytes = BASE64_STANDARD.decode(parts[1]).ok()?;
    let keypair = ed25519_compact::KeyPair::from_slice(&keypair_bytes).ok()?;

    Some(Self { name, keypair })
  }

  pub fn export_public_key(&self) -> String {
    format!("{}:{}", self.name, BASE64_STANDARD.encode(*self.keypair.pk))
  }

  pub fn export_keypair(&self) -> String {
    format!("{}:{}", self.name, BASE64_STANDARD.encode(*self.keypair))
  }

  pub fn to_public_key(&self) -> PublicKey {
    PublicKey {
      name: self.name.clone(),
      public_key: self.keypair.pk,
    }
  }

  pub fn sign(
    &self,
    path: &StorePath,
    nar_hash: &str,
    nar_size: u64,
    references: &[StorePath],
  ) -> String {
    let message = fingerprint(path, nar_hash, nar_size, references);
    let sig = BASE64_STANDARD.encode(self.keypair.sk.sign(message, None));
    format!("{}:{}", self.name, sig)
  }

  pub fn verify(
    &self,
    signature: &str,
    path: &StorePath,
    nar_hash: &str,
    nar_size: u64,
    references: &[StorePath],
  ) -> bool {
    self
      .to_public_key()
      .verify(signature, path, nar_hash, nar_size, references)
  }
}

impl PublicKey {
  pub fn from_string(s: &str) -> Option<Self> {
    let parts = s.splitn(2, ':').collect::<Vec<_>>();
    if parts.len() != 2 {
      return None;
    }

    let name = parts[0].to_string();
    let public_key_bytes = BASE64_STANDARD.decode(parts[1]).ok()?;
    let public_key = ed25519_compact::PublicKey::from_slice(&public_key_bytes).ok()?;

    Some(Self { name, public_key })
  }

  pub fn verify(
    &self,
    signature: &str,
    path: &StorePath,
    nar_hash: &str,
    nar_size: u64,
    references: &[StorePath],
  ) -> bool {
    let message = fingerprint(path, nar_hash, nar_size, references);

    let parts = signature.splitn(2, ':').collect::<Vec<_>>();
    if parts.len() != 2 {
      return false;
    }

    let sig_name = parts[0];
    let sig_value = parts[1];

    if sig_name != self.name {
      return false;
    }

    let Some(signature) = BASE64_STANDARD
      .decode(sig_value)
      .ok()
      .and_then(|bytes| Signature::from_slice(&bytes).ok())
    else {
      return false;
    };

    self.public_key.verify(message, &signature).is_ok()
  }
}

fn fingerprint(
  path: &StorePath,
  nar_hash: &str,
  nar_size: u64,
  references: &[StorePath],
) -> Vec<u8> {
  let store_dir = StoreDir::default();

  // 1;{storePath};{narHash};{narSize};{commaDelimitedReferences}
  let mut fingerprint = b"1;".to_vec();

  let path_str = store_dir.display(path).to_string();
  fingerprint.extend_from_slice(path_str.as_bytes());
  fingerprint.push(b';');

  let nar_hash = format!("sha256:{}", nar_hash);
  fingerprint.extend_from_slice(nar_hash.as_bytes());
  fingerprint.push(b';');

  let mut buf = itoa::Buffer::new();
  let nar_size_str = buf.format(nar_size);
  fingerprint.extend_from_slice(nar_size_str.as_bytes());
  fingerprint.push(b';');

  let mut iter = references.iter().peekable();
  while let Some(ref_path) = iter.next() {
    let ref_str = store_dir.display(ref_path).to_string();
    fingerprint.extend_from_slice(ref_str.as_bytes());

    if iter.peek().is_some() {
      fingerprint.push(b',');
    }
  }

  fingerprint
}
