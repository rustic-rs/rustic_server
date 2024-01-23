use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    fs::{self, read_to_string},
    io::Write,
    path::PathBuf,
};

use htpasswd_verify::md5::{format_hash, md5_apr1_encode};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::error::{ErrorKind, Result};

pub mod constants {
    pub(super) const SALT_LEN: usize = 8;
}

#[derive(Clone)]
pub struct HtAccess {
    pub path: PathBuf,
    pub credentials: HashMap<String, Credential>,
}

impl HtAccess {
    pub fn from_file(pth: &PathBuf) -> Result<HtAccess> {
        let mut c: HashMap<String, Credential> = HashMap::new();
        if pth.exists() {
            read_to_string(pth)
                .map_err(|err| {
                    ErrorKind::InternalError(format!(
                        "Could not read HtAccess file: {} at {:?}",
                        err, pth
                    ))
                })?
                .lines() // split the string into an iterator of string slices
                .map(String::from) // make each slice into a string
                .for_each(|line| match Credential::from_line(line) {
                    None => {}
                    Some(cred) => {
                        c.insert(cred.name.clone(), cred);
                    }
                })
        }
        Ok(HtAccess {
            path: pth.clone(),
            credentials: c,
        })
    }

    pub fn get(&self, name: &str) -> Option<&Credential> {
        self.credentials.get(name)
    }

    pub fn users(&self) -> Vec<String> {
        self.credentials.keys().cloned().collect()
    }

    /// Update can be used for both new, and existing credentials
    pub fn update(&mut self, name: &str, pass: &str) {
        let cred = Credential::new(name, pass);
        self.insert(cred);
    }

    /// Removes one credential by user name
    pub fn delete(&mut self, name: &str) {
        self.credentials.remove(name);
    }

    fn insert(&mut self, cred: Credential) {
        self.credentials.insert(cred.name.clone(), cred);
    }

    pub fn to_file(&self) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.path)
            .map_err(|err| {
                ErrorKind::OpeningFileFailed(format!(
                    "Could not open HtAccess file: {} at {:?}",
                    err, self.path
                ))
            })?;

        for (_n, c) in self.credentials.iter() {
            let _e = file.write(c.to_line().as_bytes()).map_err(|err| {
                ErrorKind::WritingToFileFailed(format!(
                    "Could not write to HtAccess file: {} at {:?}",
                    err, self.path
                ))
            });
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Credential {
    name: String,
    hash_val: Option<String>,
    pw: Option<String>,
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

    /// Returns a credential struct from a htaccess file line
    /// Of cause without password :-)
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
            writeln!(f, "\tPassword: {}", &self.pw.as_ref().unwrap())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::auth::{Auth, AuthChecker};
    use crate::config::auth_file::HtAccess;
    use anyhow::Result;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_htaccess() -> Result<()> {
        let htaccess_pth = Path::new("tmp_test_data").join("rustic");
        fs::create_dir_all(&htaccess_pth).unwrap();

        let ht_file = htaccess_pth.join("htaccess");

        let mut ht = HtAccess::from_file(&ht_file)?;
        ht.update("Administrator", "stuff");
        ht.update("backup-user", "its_me");
        ht.to_file()?;

        let ht = HtAccess::from_file(&ht_file)?;
        assert!(ht.get("Administrator").is_some());
        assert!(ht.get("backup-user").is_some());

        let auth = Auth::from_file(false, &ht_file).unwrap();
        assert!(auth.verify("Administrator", "stuff"));
        assert!(auth.verify("backup-user", "its_me"));

        Ok(())
    }
}
