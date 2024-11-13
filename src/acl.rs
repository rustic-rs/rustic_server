use std::{collections::BTreeMap, fs, path::PathBuf, sync::OnceLock};

use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    config::AclSettings,
    error::{ApiErrorKind, ApiResult, AppResult},
    typed_path::TpeKind,
};

// Static storage of our credentials
pub static ACL: OnceLock<Acl> = OnceLock::new();

pub fn init_acl(acl: Acl) -> AppResult<()> {
    let _ = ACL.get_or_init(|| acl);
    Ok(())
}

/// Access Types
///
// IMPORTANT: The order of the variants is important, as it is used
// to determine the access level! Don't change it!
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize, Copy)]
pub enum AccessType {
    /// No access
    NoAccess,

    /// Force unlock
    ///
    /// # Note
    ///
    /// This is a special access type that allows a user to unlock a lock
    /// without having to have the Modify access type.
    ForceUnlock,

    /// Read-only access
    Read,

    /// Append access
    ///
    /// Can be used to add new data to a repository
    Append,

    /// Modify access
    ///
    /// Can be used to modify data in a repository, also delete data
    Modify,
}

pub trait AclChecker: Send + Sync + 'static {
    fn is_allowed(&self, user: &str, path: &str, tpe: Option<TpeKind>, access: AccessType) -> bool;
}

/// ACL for a repo
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct RepoAcl(BTreeMap<String, AccessType>);

impl RepoAcl {
    pub fn new() -> Self {
        Self::default()
    }
}

impl std::ops::DerefMut for RepoAcl {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for RepoAcl {
    type Target = BTreeMap<String, AccessType>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Acl holds ACLs for all repos
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Acl {
    repos: BTreeMap<String, RepoAcl>,
    append_only: bool,
    private_repo: bool,
}

impl Default for Acl {
    fn default() -> Self {
        Self {
            repos: BTreeMap::new(),
            append_only: true,
            private_repo: true,
        }
    }
}

// read_toml is a helper func that reads the given file in toml
// into a Hashmap mapping each user to the whole passwd line
fn read_toml(file_path: &PathBuf) -> ApiResult<BTreeMap<String, RepoAcl>> {
    let s = fs::read_to_string(file_path).map_err(|err| {
        ApiErrorKind::InternalError(format!(
            "Could not read toml file: {} at {:?}",
            err, file_path
        ))
    })?;
    // make the contents static in memory
    let s = Box::leak(s.into_boxed_str());

    let repos: BTreeMap<String, RepoAcl> = toml::from_str(s)
        .map_err(|err| ApiErrorKind::InternalError(format!("Could not parse TOML: {}", err)))?;

    // TODO: What is this for?
    //
    // copy key "default" into ""
    // if let Some(default) = repos.get("default") {
    //     let default = default.clone();
    //     let _ = repos.insert("".to_owned(), default);
    // }

    Ok(repos)
}

impl Acl {
    pub fn from_file(
        append_only: bool,
        private_repo: bool,
        file_path: Option<PathBuf>,
    ) -> ApiResult<Self> {
        let repos = match file_path {
            Some(file_path) => read_toml(&file_path)?,
            None => BTreeMap::new(),
        };
        Ok(Self {
            append_only,
            private_repo,
            repos,
        })
    }

    pub fn from_config(settings: &AclSettings) -> ApiResult<Self> {
        let path = settings.acl_path.clone();
        Self::from_file(settings.append_only, settings.private_repo, path)
    }

    // The default repo has not been removed from the self.repos list, so we do not need to add here
    // But we still need to remove the ""-tag that was added during the from_file()
    pub fn to_file(&self, pth: &PathBuf) -> ApiResult<()> {
        let repos = self.repos.clone();

        // TODO: What is this for? Why do we need an empty string key?
        // clone.remove("");

        let toml_string = toml::to_string(&repos).map_err(|err| {
            ApiErrorKind::InternalError(format!(
                "Could not serialize ACL config to TOML value: {}",
                err
            ))
        })?;
        fs::write(pth, toml_string).map_err(|err| {
            ApiErrorKind::WritingToFileFailed(format!("Could not write ACL file: {}", err))
        })?;
        Ok(())
    }

    // TODO: What is this for? It's unsued and also not using the Entry API
    //
    // pub fn default_repo_access(&mut self, user: &str, access: AccessType) {
    //     // If we do not have a key with ""-value then "default" is also not a key
    //     // Since we guarantee this during the reading of a acl-file
    //     if !self.repos.contains_key("default") {
    //         let mut acl = RepoAcl::new();
    //         acl.insert(user.into(), access);
    //         self.repos.insert("default".to_owned(), acl.clone());
    //         self.repos.insert("".to_owned(), acl);
    //     } else {
    //         self.repos
    //             .get_mut("default")
    //             .unwrap()
    //             .insert(user.into(), access.clone());
    //         self.repos
    //             .get_mut("")
    //             .unwrap()
    //             .insert(user.into(), access.clone());
    //     }
    // }
}

impl AclChecker for Acl {
    // allowed yields whether these access to {path, tpe, access} is allowed by user
    #[tracing::instrument(level = "debug", skip(self))]
    fn is_allowed(
        &self,
        user: &str,
        path: &str,
        tpe: Option<TpeKind>,
        access_type: AccessType,
        // _force_unlock: bool,
    ) -> bool {
        // Access to locks is always treated as Read
        // FIXME: This is a bit of a hack, we should probably have a separate access type for locks
        // FIXME: to be able to force remove them with `unlock`
        let access = if tpe.is_some_and(|v| v == TpeKind::Locks) {
            AccessType::Read
        } else {
            access_type
        };

        if let Some(repo_acl) = self.repos.get(path) {
            matches!(repo_acl.get(user), Some(user_access) if user_access >= &access)
        } else {
            let is_user_path = user == path;
            let is_not_private_repo = !self.private_repo;
            let is_not_modify_access = access != AccessType::Modify;
            let is_not_append_only = !self.append_only;

            debug!(
                "is_user_path: {is_user_path}, is_not_private_repo: {is_not_private_repo}, is_not_modify_access: {is_not_modify_access}, is_not_append_only: {is_not_append_only}",
            );

            // If the user is the path, and the repo is not private, or the user has modify access
            // or the repo is not append only, then allow the access
            (is_user_path || is_not_private_repo) && (is_not_modify_access || is_not_append_only)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AccessType::{Append, Modify, Read};
    use super::*;
    use crate::test_helpers::server_config;
    use rstest::rstest;

    use std::env;

    #[rstest]
    fn test_static_acl_access_passes() {
        let auth = Acl::from_config(&server_config().acl).unwrap();
        init_acl(auth).unwrap();

        let acl = ACL.get().unwrap();
        assert!(&acl.private_repo);
        assert!(!&acl.append_only);
        let access = acl.repos.get("test_repo").unwrap();
        let access_type = access.get("restic").unwrap();
        assert_eq!(access_type, &Append);
    }

    #[test]
    fn test_allowed_flags_passes() {
        let mut acl = Acl::default();

        assert!(!acl.is_allowed("bob", "sam", Some(TpeKind::Keys), Read));
        assert!(!acl.is_allowed("bob", "sam", Some(TpeKind::Data), Read));
        assert!(!acl.is_allowed("bob", "sam", Some(TpeKind::Data), Append));
        assert!(!acl.is_allowed("bob", "sam", Some(TpeKind::Data), Modify));
        assert!(!acl.is_allowed("bob", "bob", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Locks), Modify));
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Keys), Append));
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Data), Append));
        assert!(acl.is_allowed("", "", Some(TpeKind::Data), Append));
        assert!(!acl.is_allowed("bob", "", Some(TpeKind::Data), Read));

        acl.append_only = false;
        assert!(!acl.is_allowed("bob", "sam", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Data), Modify));

        acl.private_repo = false;
        assert!(acl.is_allowed("bob", "sam", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("bob", "", Some(TpeKind::Data), Modify));
    }

    #[test]
    fn test_repo_acl_passes() {
        let mut acl = Acl::default();

        let mut acl_all = RepoAcl::new();
        let _ = acl_all.insert("bob".to_string(), Modify);
        let _ = acl_all.insert("sam".to_string(), Append);
        let _ = acl_all.insert("paul".to_string(), Read);
        let _ = acl.repos.insert("all".to_string(), acl_all);

        let mut acl_bob = RepoAcl::new();
        let _ = acl_bob.insert("bob".to_string(), Modify);
        let _ = acl.repos.insert("bob".to_string(), acl_bob);

        let mut acl_sam = RepoAcl::new();
        let _ = acl_sam.insert("sam".to_string(), Append);
        let _ = acl_sam.insert("bob".to_string(), Read);
        let _ = acl.repos.insert("sam".to_string(), acl_sam);

        insta::assert_debug_snapshot!(acl);

        // test ACLs for repo all
        assert!(acl.is_allowed("bob", "all", Some(TpeKind::Keys), Modify));
        assert!(!acl.is_allowed("sam", "all", Some(TpeKind::Keys), Modify));
        assert!(acl.is_allowed("sam", "all", Some(TpeKind::Keys), Append));
        assert!(acl.is_allowed("sam", "all", Some(TpeKind::Locks), Modify));
        assert!(!acl.is_allowed("paul", "all", Some(TpeKind::Data), Append));
        assert!(acl.is_allowed("paul", "all", Some(TpeKind::Data), Read));
        assert!(acl.is_allowed("paul", "all", Some(TpeKind::Locks), Modify));
        assert!(!acl.is_allowed("attack", "all", Some(TpeKind::Data), Modify));

        // test ACLs for repo bob
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Data), Modify));
        assert!(!acl.is_allowed("sam", "bob", Some(TpeKind::Data), Read));
        assert!(!acl.is_allowed("attack", "bob", Some(TpeKind::Locks), Modify));

        // test ACLs for repo sam
        assert!(!acl.is_allowed("sam", "sam", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("sam", "sam", Some(TpeKind::Data), Append));
        assert!(!acl.is_allowed("bob", "sam", Some(TpeKind::Keys), Append));
        assert!(acl.is_allowed("bob", "sam", Some(TpeKind::Keys), Read));
        assert!(!acl.is_allowed("attack", "sam", Some(TpeKind::Locks), Read));

        // test ACLs for repo paul => fall back to flags
        assert!(!acl.is_allowed("paul", "paul", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("paul", "paul", Some(TpeKind::Data), Append));
        assert!(!acl.is_allowed("sam", "paul", Some(TpeKind::Data), Read));
    }
}
