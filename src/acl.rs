use std::{collections::BTreeMap, fs, path::PathBuf, sync::OnceLock};

use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    config::AclSettings,
    error::{AppResult, ErrorKind},
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Serialize, Deserialize, Copy)]
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

type HtPasswdUsername = String;

/// ACL for a repo
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct RepoAcl(BTreeMap<HtPasswdUsername, AccessType>);

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

type Repository = String;

/// `Acl` holds ACLs for all repos
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Acl {
    private_repo: bool,
    append_only: bool,
    repos: BTreeMap<Repository, RepoAcl>,
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
fn read_toml(file_path: &PathBuf) -> AppResult<BTreeMap<String, RepoAcl>> {
    let s = fs::read_to_string(file_path).map_err(|err| {
        ErrorKind::Io.context(format!(
            "Could not read toml file: {} at {:?}",
            err, file_path
        ))
    })?;
    // make the contents static in memory
    let s = Box::leak(s.into_boxed_str());

    let mut repos: BTreeMap<String, RepoAcl> = toml::from_str(s)
        .map_err(|err| ErrorKind::Config.context(format!("Could not parse TOML: {}", err)))?;

    // copy key "default" into ""
    if let Some(default) = repos.get("default") {
        let default = default.clone();
        let _ = repos.insert(String::new(), default);
    }

    Ok(repos)
}

impl Acl {
    pub fn from_file(
        append_only: bool,
        private_repos: bool,
        file_path: Option<PathBuf>,
    ) -> AppResult<Self> {
        let repos = match file_path {
            Some(file_path) => read_toml(&file_path).map_err(|err| {
                ErrorKind::Config.context(format!("Could not read ACL file: {err}"))
            })?,
            None => BTreeMap::new(),
        };

        Ok(Self {
            append_only,
            private_repo: private_repos,
            repos,
        })
    }

    pub fn from_config(settings: &AclSettings, path: Option<PathBuf>) -> AppResult<Self> {
        Self::from_file(
            settings.append_only,
            !settings.disable_acl || settings.private_repos,
            path,
        )
    }

    // The default repo has not been removed from the self.repos list, so we do not need to add here
    // But we still need to remove the ""-tag that was added during the from_file()
    pub fn to_file(&self, pth: &PathBuf) -> AppResult<()> {
        let mut repos = self.repos.clone();

        _ = repos.remove("");

        let toml_string = toml::to_string(&repos).map_err(|err| {
            ErrorKind::Config.context(format!(
                "Could not serialize ACL config to TOML value: {err}"
            ))
        })?;

        fs::write(pth, toml_string)
            .map_err(|err| ErrorKind::Io.context(format!("Could not write ACL file: {err}")))?;

        Ok(())
    }

    pub fn set_append_only(self, append_only: bool) -> Self {
        Self {
            append_only,
            ..self
        }
    }

    pub fn default_repo_access(&mut self, user: &str, access: AccessType) {
        // If we do not have a key with ""-value then "default" is also not a key
        // Since we guarantee this during the reading of a acl-file
        if !self.repos.contains_key("default") {
            let mut acl = RepoAcl::new();
            _ = acl.insert(user.into(), access);
            _ = self.repos.insert("default".to_owned(), acl.clone());
            _ = self.repos.insert(String::new(), acl);
        } else {
            _ = self
                .repos
                .get_mut("default")
                .unwrap()
                .insert(user.into(), access);
            _ = self.repos.get_mut("").unwrap().insert(user.into(), access);
        }
    }
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
        let access_type = if tpe.is_some_and(|v| v == TpeKind::Locks) {
            AccessType::Read
        } else {
            access_type
        };

        self.repos.get(path).map_or_else(
            || {
                debug!("No ACL for repository found, applying default ACL.");

                let is_user_path = user == path;
                let is_not_private_repo = !self.private_repo;
                let is_not_modify_access = access_type != AccessType::Modify;
                let is_not_append_only = !self.append_only;

                debug!(%is_user_path, %is_not_private_repo, %is_not_modify_access, %is_not_append_only);

                // If the user is the path, and the repo is not private, or the user has modify access
                // or the repo is not append only, then allow the access
                let access = (is_user_path || is_not_private_repo)
                    && (is_not_modify_access || is_not_append_only);

                debug!(%access, "Access check");

                access
            },
            |repo_acl| {
                let access =
                    matches!(repo_acl.get(user), Some(user_access) if user_access >= &access_type);

                debug!(?repo_acl, %access, "Access check");

                access
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AccessType::{Append, Modify, Read};
    use super::*;
    use crate::testing::server_config;
    use rstest::rstest;

    use std::env;

    #[rstest]
    fn test_static_acl_access_passes() {
        let acl = server_config().acl;
        let auth = Acl::from_config(&acl.clone(), acl.acl_path).unwrap();

        init_acl(auth).unwrap();

        let acl = ACL.get().unwrap();
        assert!(&acl.private_repo);
        assert!(!&acl.append_only);
        let access = acl.repos.get("test_repo").unwrap();
        let access_type = access.get("rustic").unwrap();
        assert_eq!(access_type, &Append);
    }

    #[test]
    fn test_allowed_flags_passes() {
        let mut acl = Acl::default();

        insta::assert_debug_snapshot!("acl_default_impl", acl);

        // Private repo
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

        // Not-append only
        acl.append_only = false;
        assert!(!acl.is_allowed("bob", "sam", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Data), Modify));

        // Public repo
        acl.private_repo = false;
        assert!(acl.is_allowed("bob", "sam", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Data), Modify));
        assert!(acl.is_allowed("bob", "", Some(TpeKind::Data), Modify));
    }

    #[test]
    fn test_repo_acl_passes() {
        let mut acl = Acl::default();

        let mut acl_all = RepoAcl::new();
        _ = acl_all.insert("bob".to_string(), Modify);
        _ = acl_all.insert("sam".to_string(), Append);
        _ = acl_all.insert("paul".to_string(), Read);
        _ = acl.repos.insert("all".to_string(), acl_all);

        let mut acl_bob = RepoAcl::new();
        _ = acl_bob.insert("bob".to_string(), Modify);
        _ = acl.repos.insert("bob".to_string(), acl_bob);

        let mut acl_sam = RepoAcl::new();
        _ = acl_sam.insert("sam".to_string(), Append);
        _ = acl_sam.insert("bob".to_string(), Read);
        _ = acl.repos.insert("sam".to_string(), acl_sam);

        insta::assert_debug_snapshot!(acl);

        // test ACLs for repo all
        assert!(acl.is_allowed("paul", "all", Some(TpeKind::Data), Read));
        assert!(acl.is_allowed("sam", "all", Some(TpeKind::Keys), Append));
        assert!(!acl.is_allowed("paul", "all", Some(TpeKind::Data), Append));
        assert!(acl.is_allowed("bob", "all", Some(TpeKind::Keys), Modify));
        assert!(!acl.is_allowed("sam", "all", Some(TpeKind::Keys), Modify));
        assert!(acl.is_allowed("sam", "all", Some(TpeKind::Locks), Modify));
        assert!(acl.is_allowed("paul", "all", Some(TpeKind::Locks), Modify));
        assert!(!acl.is_allowed("attack", "all", Some(TpeKind::Data), Modify));

        // test ACLs for repo bob
        assert!(!acl.is_allowed("sam", "bob", Some(TpeKind::Data), Read));
        assert!(acl.is_allowed("bob", "bob", Some(TpeKind::Data), Modify));
        assert!(!acl.is_allowed("attack", "bob", Some(TpeKind::Locks), Modify));

        // test ACLs for repo sam
        assert!(acl.is_allowed("bob", "sam", Some(TpeKind::Keys), Read));
        assert!(!acl.is_allowed("attack", "sam", Some(TpeKind::Locks), Read));
        assert!(acl.is_allowed("sam", "sam", Some(TpeKind::Data), Append));
        assert!(!acl.is_allowed("bob", "sam", Some(TpeKind::Keys), Append));
        assert!(!acl.is_allowed("sam", "sam", Some(TpeKind::Data), Modify));

        // test ACLs for repo paul => fall back to flags
        assert!(!acl.is_allowed("sam", "paul", Some(TpeKind::Data), Read));
        assert!(acl.is_allowed("paul", "paul", Some(TpeKind::Data), Append));
        assert!(!acl.is_allowed("paul", "paul", Some(TpeKind::Data), Modify));
    }
}
