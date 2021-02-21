use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};

// Access Types
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Deserialize)]
pub enum AccessType {
    Nothing,
    Read,
    Append,
    Modify,
}

pub trait AclChecker: Send + Sync + 'static {
    fn allowed(&self, user: &str, path: &str, tpe: &str, access: AccessType) -> bool;
}

// ACL for a repo
type RepoAcl = HashMap<&'static str, AccessType>;

// Acl holds ACLs for all repos
#[derive(Clone)]
pub struct Acl {
    repos: HashMap<&'static str, RepoAcl>,
    append_only: bool,
    private_repo: bool,
}

// read_toml  is a helper func that reads the given file in toml
// into a Hashmap mapping each user to the whole passwd line
fn read_toml(file_path: &PathBuf) -> io::Result<HashMap<&'static str, RepoAcl>> {
    let s = fs::read_to_string(file_path)?;
    // make the contents static in memory
    let s = Box::leak(s.into_boxed_str());

    let mut repos: HashMap<&'static str, RepoAcl> = toml::from_str(s)?;
    // copy key "default" into ""
    if let Some(default) = repos.get("default") {
        let default = default.clone();
        repos.insert("", default);
    }
    Ok(repos)
}

impl Acl {
    pub fn from_file(
        append_only: bool,
        private_repo: bool,
        file_path: Option<PathBuf>,
    ) -> io::Result<Self> {
        let repos = match file_path {
            Some(file_path) => read_toml(&file_path)?,
            None => HashMap::new(),
        };
        Ok(Self {
            append_only,
            private_repo,
            repos,
        })
    }
}

impl AclChecker for Acl {
    // allowed yields whether thes access to {path,tpe, access} is allowed by user
    fn allowed(&self, user: &str, path: &str, tpe: &str, access: AccessType) -> bool {
        // Access to locks is always treated as Read
        let access = if tpe == "locks" {
            AccessType::Read
        } else {
            access
        };

        match self.repos.get(path) {
            // We have ACLs for this repo, use them!
            Some(repo_acl) => match repo_acl.get(user) {
                Some(user_access) => user_access >= &access,
                None => false,
            },
            // Use standards defined by flags --private-repo and --append-only
            None => {
                (user == path || !self.private_repo)
                    && (access != AccessType::Modify || !self.append_only)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AccessType::*;
    use super::*;

    #[test]
    fn allowed_flags() {
        let mut acl = Acl {
            repos: HashMap::new(),
            append_only: true,
            private_repo: true,
        };
        assert_eq!(acl.allowed("bob", "sam", "keys", Read), false);
        assert_eq!(acl.allowed("bob", "sam", "data", Read), false);
        assert_eq!(acl.allowed("bob", "sam", "data", Append), false);
        assert_eq!(acl.allowed("bob", "sam", "data", Modify), false);
        assert_eq!(acl.allowed("bob", "bob", "data", Modify), false);
        assert_eq!(acl.allowed("bob", "bob", "locks", Modify), true);
        assert_eq!(acl.allowed("bob", "bob", "keys", Append), true);
        assert_eq!(acl.allowed("bob", "bob", "data", Append), true);
        assert_eq!(acl.allowed("", "", "data", Append), true);
        assert_eq!(acl.allowed("bob", "", "data", Read), false);

        acl.append_only = false;
        assert_eq!(acl.allowed("bob", "sam", "data", Modify), false);
        assert_eq!(acl.allowed("bob", "bob", "data", Modify), true);

        acl.private_repo = false;
        assert_eq!(acl.allowed("bob", "sam", "data", Modify), true);
        assert_eq!(acl.allowed("bob", "bob", "data", Modify), true);
        assert_eq!(acl.allowed("bob", "", "data", Modify), true);
    }

    #[test]
    fn repo_acl() {
        let mut acl = Acl {
            repos: HashMap::new(),
            append_only: true,
            private_repo: true,
        };

        let mut acl_all = HashMap::new();
        acl_all.insert("bob", Modify);
        acl_all.insert("sam", Append);
        acl_all.insert("paul", Read);
        acl.repos.insert("all", acl_all);

        let mut acl_bob = HashMap::new();
        acl_bob.insert("bob", Modify);
        acl.repos.insert("bob", acl_bob);

        let mut acl_sam = HashMap::new();
        acl_sam.insert("sam", Append);
        acl_sam.insert("bob", Read);
        acl.repos.insert("sam", acl_sam);

        // test ACLs for repo all
        assert_eq!(acl.allowed("bob", "all", "keys", Modify), true);
        assert_eq!(acl.allowed("sam", "all", "keys", Modify), false);
        assert_eq!(acl.allowed("sam", "all", "keys", Append), true);
        assert_eq!(acl.allowed("sam", "all", "locks", Modify), true);
        assert_eq!(acl.allowed("paul", "all", "data", Append), false);
        assert_eq!(acl.allowed("paul", "all", "data", Read), true);
        assert_eq!(acl.allowed("paul", "all", "locks", Modify), true);
        assert_eq!(acl.allowed("attack", "all", "data", Modify), false);

        // test ACLs for repo bob
        assert_eq!(acl.allowed("bob", "bob", "data", Modify), true);
        assert_eq!(acl.allowed("sam", "bob", "data", Read), false);
        assert_eq!(acl.allowed("attack", "bob", "locks", Modify), false);

        // test ACLs for repo sam
        assert_eq!(acl.allowed("sam", "sam", "data", Modify), false);
        assert_eq!(acl.allowed("sam", "sam", "data", Append), true);
        assert_eq!(acl.allowed("bob", "sam", "keys", Append), false);
        assert_eq!(acl.allowed("bob", "sam", "keys", Read), true);
        assert_eq!(acl.allowed("attack", "sam", "locks", Read), false);

        // test ACLs for repo paul => fall back to flags
        assert_eq!(acl.allowed("paul", "paul", "data", Modify), false);
        assert_eq!(acl.allowed("paul", "paul", "data", Append), true);
        assert_eq!(acl.allowed("sam", "paul", "data", Read), false);
    }
}
