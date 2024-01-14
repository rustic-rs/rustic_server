use anyhow::Result;
use htpasswd_verify::md5::{format_hash, md5_apr1_encode};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::read_to_string;
use std::io::Write;
use std::path::PathBuf;

const SALT_LEN: usize = 8;

#[derive(Clone)]
pub struct HtAccess {
    pub path: PathBuf,
    pub credentials: HashMap<String, Credential>,
}

impl HtAccess {
    pub fn from_file(pth: &PathBuf) -> Result<HtAccess> {
        let mut c: HashMap<String, Credential> = HashMap::new();
        if pth.exists() {
            read_to_string(pth)?
                .lines() // split the string into an iterator of string slices
                .map(String::from) // make each slice into a string
                .for_each(|line| match Credential::from_line(line) {
                    None => {}
                    Some(cred) => {
                        c.insert(cred.name.clone(), cred);
                    }
                })
        }
        return Ok(HtAccess {
            path: pth.clone(),
            credentials: c,
        });
    }

    pub fn get(&self, name: &str) -> Option<&Credential> {
        self.credentials.get(name)
    }

    pub fn users(&self) -> Vec<String> {
        let ret: Vec<String> = self.credentials.keys().cloned().collect();
        return ret;
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

    /// FIXME: Nicer error logging for when we can not write file ...
    pub fn to_file(&mut self) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.path)?;

        for (_n, c) in self.credentials.iter() {
            file.write(c.to_line().as_bytes()).unwrap();
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
            .take(SALT_LEN)
            .map(char::from)
            .collect();
        let hash = md5_apr1_encode(pass, salt.as_str());
        let hash = format_hash(&hash.as_str(), &salt.as_str());

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
                name: spl.get(0).unwrap().to_string(),
                hash_val: Some(spl.get(1).unwrap().to_string()),
                pw: None,
            });
        }
        None
    }

    pub fn to_line(&self) -> String {
        if self.hash_val.is_some() {
            return format!(
                "{}:{}\n",
                self.name.as_str(),
                self.hash_val.as_ref().unwrap()
            );
        } else {
            "".into()
        }
    }
}

impl Display for Credential {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Struct: Credential\n")?;
        write!(f, "\tUser: {}\n", self.name.as_str())?;
        write!(f, "\tHash: {}\n", self.hash_val.as_ref().unwrap())?;
        if self.pw.is_none() {
            write!(f, "\tPassword: None\n")?;
        } else {
            write!(f, "\tPassword: {}\n", &self.pw.as_ref().unwrap())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::auth::{Auth, AuthChecker};
    use anyhow::Result;
    use std::fs;
    use std::path::Path;
    use crate::config::auth_file::HtAccess;

    #[test]
    fn test_htaccess() -> Result<()> {
        let htaccess_pth = Path::new("tmp_test_data").join("rustic");
        fs::create_dir_all(&htaccess_pth).unwrap();

        let ht_file = htaccess_pth.join(".htaccess");

        let mut ht = HtAccess::from_file(&ht_file)?;
        ht.update("Administrator", "stuff");
        ht.update("backup-user", "itsme");
        ht.to_file()?;

        let ht = HtAccess::from_file(&ht_file)?;
        assert!(ht.get(&"Administrator").is_some());
        assert!(ht.get(&"backup-user").is_some());

        let auth = Auth::from_file(false, &ht_file).unwrap();
        assert!(auth.verify("Administrator", "stuff"));
        assert!(auth.verify(&"backup-user", "itsme"));

        Ok(())
    }
}
