use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{Display, Formatter},
    fs::{self, read_to_string},
    io::Write,
    path::PathBuf,
};

use htpasswd_verify::md5::{format_hash, md5_apr1_encode};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::error::{ApiErrorKind, ApiResult, AppResult, ErrorKind};
use abscissa_core::SecretString;

pub mod constants {
    pub(super) const SALT_LEN: usize = 8;
}

#[derive(Clone, Debug)]
pub struct Htpasswd {
    pub path: PathBuf,
    pub credentials: HashMap<String, Credential>,
}

impl Htpasswd {
    pub fn from_file(pth: &PathBuf) -> ApiResult<Htpasswd> {
        let mut c: HashMap<String, Credential> = HashMap::new();
        if pth.exists() {
            read_to_string(pth)
                .map_err(|err| {
                    ApiErrorKind::InternalError(format!(
                        "Could not read htpasswd file: {} at {:?}",
                        err, pth
                    ))
                })?
                .lines() // split the string into an iterator of string slices
                .map(String::from) // make each slice into a string
                .for_each(|line| match Credential::from_line(line) {
                    None => {}
                    Some(cred) => {
                        let _ = c.insert(cred.name.clone(), cred);
                    }
                })
        }
        Ok(Htpasswd {
            path: pth.clone(),
            credentials: c,
        })
    }

    pub fn users(&self) -> Vec<String> {
        self.credentials.keys().cloned().collect()
    }

    pub fn create(&mut self, name: &str, pass: &str) -> AppResult<()> {
        let cred = Credential::new(name, pass);

        let _ = self.insert(cred)?;

        Ok(())
    }

    pub fn read(&self, name: &str) -> Option<&Credential> {
        self.credentials.get(name)
    }

    /// Update can be used for both new, and existing credentials
    pub fn update(&mut self, name: &str, pass: &str) -> AppResult<()> {
        let cred = Credential::new(name, pass);

        let _ = self.insert(cred)?;

        Ok(())
    }

    /// Removes one credential by username
    pub fn delete(&mut self, name: &str) -> Option<Credential> {
        self.credentials.remove(name)
    }

    pub fn insert(&mut self, cred: Credential) -> AppResult<Credential> {
        let result = self.credentials.entry(cred.name.clone());

        match result {
            Entry::Occupied(mut entry) => Ok(entry.insert(cred)),
            Entry::Vacant(entry) => {
                return Err(ErrorKind::Io
                    .context(format!(
                        "Entry already exists, could not insert credential: `{}`. Please use update instead.",
                        entry.key()
                    ))
                    .into());
            }
        }
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
            let _e = file.write(c.to_line().as_bytes()).map_err(|err| {
                ApiErrorKind::WritingToFileFailed(format!(
                    "Could not write to htpasswd file: {} at {:?}",
                    err, self.path
                ))
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Credential {
    name: String,
    hash_val: Option<String>,
    pw: Option<SecretString>,
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

        Credential {
            name: name.into(),
            hash_val: Some(hash),
            pw: Some(pass.into()),
        }
    }

    /// Returns a credential struct from a htpasswd file line
    pub fn from_line(line: String) -> Option<Credential> {
        let spl: Vec<&str> = line.split(':').collect();
        if !spl.is_empty() {
            return Some(Credential {
                name: spl.first().unwrap().to_string(),
                hash_val: Some(spl.get(1).unwrap().to_string()),
                pw: None,
            });
        }
        None
    }

    pub fn to_line(&self) -> String {
        if self.hash_val.is_some() {
            format!(
                "{}:{}\n",
                self.name.as_str(),
                self.hash_val.as_ref().unwrap()
            )
        } else {
            "".into()
        }
    }
}

impl Display for Credential {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Struct: Credential")?;
        writeln!(f, "\tUser: {}", self.name.as_str())?;
        writeln!(f, "\tHash: {}", self.hash_val.as_ref().unwrap())?;
        if self.pw.is_none() {
            writeln!(f, "\tPassword: None")?;
        } else {
            writeln!(f, "\tPassword: {:?}", &self.pw.as_ref().unwrap())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::auth::{Auth, AuthChecker};
    use crate::htpasswd::Htpasswd;
    use anyhow::Result;
    use insta::assert_debug_snapshot;
    
    use std::path::Path;

    #[test]
    fn test_htpasswd() -> Result<()> {
        let htpasswd_path = Path::new("tests/fixtures/test_data");
        let htpasswd_file = htpasswd_path.join(".htpasswd");

        let mut ht = Htpasswd::from_file(&htpasswd_file)?;

        assert_debug_snapshot!(ht);

        let _ = ht.update("Administrator", "stuff");
        let _ = ht.update("backup-user", "its_me");

        assert_debug_snapshot!(ht);

        let auth = Auth::from_file(false, &htpasswd_file)?;
        assert!(auth.verify("Administrator", "stuff"));
        assert!(auth.verify("backup-user", "its_me"));

        Ok(())
    }
}
