use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

// Access Types
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Deserialize)]
pub enum AccessType {
    Nothing,
    Read,
    Append,
    Modify,
}

// ACL for a repo
type RepoAcl = HashMap<String, AccessType>;

// Acl holds ACLs for all repos
#[derive(Clone)]
pub struct Acl {
    repos: HashMap<String, RepoAcl>,
    append_only: bool,
    private_repo: bool,
}

// read_toml  is a helper func that reads the given file in toml
// into a Hashmap mapping each user to the whole passwd line
fn read_toml(file_path: &PathBuf) -> io::Result<HashMap<String, RepoAcl>> {
    let mut file = File::open(file_path)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    let mut repos: HashMap<String, RepoAcl> = toml::from_str(&s)?;
    // copy key "default" into ""
    if let Some(default) = repos.get("default") {
        let default = default.clone();
        repos.insert("".to_string(), default);
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

    // allowed yields whether thes access to {path,tpe, access} is allowed by user
    pub fn allowed(&self, user: &str, path: &str, tpe: &str, access: AccessType) -> bool {
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
