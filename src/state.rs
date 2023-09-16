use crate::{
    acl::{AccessType, Acl, AclChecker},
    auth::{Auth, AuthChecker},
    error::{ErrorKind, Result},
    helpers::{Finalizer, IteratorAdapter},
    storage::{LocalStorage, Storage},
};

use crate::acl::AclCheckerEnum;
use crate::auth::AuthCheckerEnum;
use crate::storage::StorageEnum;

#[derive(Debug, Clone)]
pub struct AppState {
    auth: AuthCheckerEnum,
    acl: AclCheckerEnum,
    storage: StorageEnum,
    tpe: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            auth: Auth::default().into(),
            acl: Acl::default().into(),
            storage: LocalStorage::default().into(),
            tpe: "".to_string(),
        }
    }
}

impl AppState {
    pub fn new(
        auth: AuthCheckerEnum,
        acl: AclCheckerEnum,
        storage: StorageEnum,
        tpe: String,
    ) -> Self {
        Self {
            storage,
            auth,
            acl,
            tpe,
        }
    }

    pub fn auth(&self) -> &AuthCheckerEnum {
        &self.auth
    }

    pub fn acl(&self) -> &AclCheckerEnum {
        &self.acl
    }

    pub fn storage(&self) -> &StorageEnum {
        &self.storage
    }

    pub fn tpe(&self) -> &str {
        &self.tpe
    }

    pub fn set_tpe(&mut self, tpe: String) {
        self.tpe = tpe;
    }
}
