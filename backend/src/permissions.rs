use centaurus::{
  backend::auth::permission::{self, Permission},
  permission,
};

pub fn permissions() -> Vec<&'static str> {
  let mut perms = permission::permissions();
  perms.extend_from_slice(&[CacheCreate::name(), CacheView::name(), CacheEdit::name()]);
  perms
}

// Caches
permission!(CacheCreate, "cache:create");
permission!(CacheView, "cache:view");
permission!(CacheEdit, "cache:edit");
