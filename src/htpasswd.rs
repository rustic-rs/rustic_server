use std::{
    collections::{btree_map::Entry, BTreeMap},
    fmt::{Display, Formatter},
    fs::{self, read_to_string},
    io::Write,
    path::PathBuf,
};

use htpasswd_verify::md5::{format_hash, md5_apr1_encode};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Serialize;

use crate::error::{ApiErrorKind, ApiResult, AppResult, ErrorKind};

pub mod constants {
    pub(super) const SALT_LEN: usize = 8;
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CredentialMap(BTreeMap<String, Credential>);

impl CredentialMap {
    pub fn new() -> Self {
        Self::default()
    }
}

impl std::ops::DerefMut for CredentialMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for CredentialMap {
    type Target = BTreeMap<String, Credential>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Htpasswd {
    pub path: PathBuf,
    pub credentials: CredentialMap,
}

impl Htpasswd {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_file(pth: &PathBuf) -> AppResult<Self> {
        let mut c = CredentialMap::new();

        if pth.exists() {
            read_to_string(pth)
                .map_err(|err| {
                    ErrorKind::Io.context(format!(
                        "Could not read htpasswd file: {} at {:?}",
                        err, pth
                    ))
                })?
                .lines() // split the string into an iterator of string slices
                .map(str::trim)
                .map(String::from) // make each slice into a string
                .filter_map(|s| Credential::from_line(s).ok())
                .for_each(|cred| {
                    let _ = c.insert(cred.name.clone(), cred);
                });
        }

        Ok(Self {
            path: pth.clone(),
            credentials: c,
        })
    }

    pub fn users(&self) -> Vec<String> {
        self.credentials.keys().cloned().collect()
    }

    pub fn create(&mut self, name: &str, pass: &str) -> AppResult<()> {
        let cred = Credential::new(name, pass);

        self.insert(cred)?;

        Ok(())
    }

    pub fn read(&self, name: &str) -> Option<&Credential> {
        self.credentials.get(name)
    }

    pub fn update(&mut self, name: &str, pass: &str) -> AppResult<()> {
        let cred = Credential::new(name, pass);

        let _ = self
            .credentials
            .entry(name.to_owned())
            .and_modify(|entry| *entry = cred.clone())
            .or_insert(cred);

        Ok(())
    }

    /// Removes one credential by username
    pub fn delete(&mut self, name: &str) -> Option<Credential> {
        self.credentials.remove(name)
    }

    pub fn insert(&mut self, cred: Credential) -> AppResult<()> {
        let Entry::Vacant(entry) = self.credentials.entry(cred.name.clone()) else {
            return Err(ErrorKind::Io
                    .context(format!(
                        "Entry already exists, could not insert credential: `{}`. Please use update instead.",
                        cred.name.as_str()
                    ))
                    .into());
        };

        let _ = entry.insert(cred);

        Ok(())
    }

    pub fn to_file(&self) -> ApiResult<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.path)
            .map_err(|err| {
                ApiErrorKind::OpeningFileFailed(format!(
                    "Could not open htpasswd file: {} at {:?}",
                    err, self.path
                ))
            })?;

        for (_n, c) in self.credentials.iter() {
            let _e = file.write(c.to_string().as_bytes()).map_err(|err| {
                ApiErrorKind::WritingToFileFailed(format!(
                    "Could not write to htpasswd file: {} at {:?}",
                    err, self.path
                ))
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Credential {
    name: String,
    hash: String,
}

impl Credential {
    pub fn new(name: &str, pass: &str) -> Self {
        let salt: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(constants::SALT_LEN)
            .map(char::from)
            .collect();
        let hash = md5_apr1_encode(pass, salt.as_str());
        let hash = format_hash(hash.as_str(), salt.as_str());

        Self {
            name: name.into(),
            hash,
        }
    }

    /// Returns a credential struct from a htpasswd file line
    pub fn from_line(line: String) -> AppResult<Self> {
        let split: Vec<&str> = line.split(':').collect();

        if split.len() != 2 {
            return Err(ErrorKind::Io
                .context(format!(
                    "Could not parse htpasswd file line: `{}`. Expected format: `name:hash`",
                    line
                ))
                .into());
        }

        Ok(Self {
            name: split[0].to_string(),
            hash: split[1].to_string(),
        })
    }
}

impl Display for Credential {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}:{}", self.name, self.hash)
    }
}

#[cfg(test)]
mod test {
    use crate::auth::Auth;
    use crate::htpasswd::Htpasswd;
    use anyhow::Result;
    use insta::assert_ron_snapshot;

    #[test]
    fn test_htpasswd_passes() -> Result<()> {
        let mut htpasswd = Htpasswd::new();

        let _ = htpasswd.update("Administrator", "stuff");
        let _ = htpasswd.update("backup-user", "its_me");

        assert_ron_snapshot!(htpasswd, {
            ".credentials.*.hash" => "[hash]",
        });

        let auth = Auth::from(htpasswd);
        assert!(auth.verify("Administrator", "stuff"));
        assert!(auth.verify("backup-user", "its_me"));

        Ok(())
    }
}
