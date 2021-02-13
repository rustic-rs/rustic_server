use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

// read_htpasswd is a helper func that reads the given file in .httpasswd format
// into a Hashmap mapping each user to the whole passwd line
fn read_htpasswd(file_path: &PathBuf) -> io::Result<HashMap<String, String>> {
    let mut file = File::open(file_path)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;

    let mut user_map = HashMap::new();
    for line in s.lines() {
        let user = line.split(':').collect::<Vec<&str>>()[0];
        user_map.insert(user.to_string(), line.to_string());
    }
    Ok(user_map)
}

#[derive(Clone)]
pub struct Auth {
    users: Option<HashMap<String, String>>,
}

impl Auth {
    pub fn from_file(no_auth: bool, path: &PathBuf) -> io::Result<Self> {
        Ok(Self {
            users: match no_auth {
                true => None,
                false => Some(read_htpasswd(path)?),
            },
        })
    }

    // verify verifies user/passwd against the credentials saved in users.
    // returns true if Auth::users is None.
    pub fn verify(&self, user: &str, passwd: &str) -> bool {
        match &self.users {
            Some(users) => match users.get(user) {
                Some(passwd_data) if htpasswd_verify::load(passwd_data).check(user, passwd) => true,
                _ => false,
            },
            None => true,
        }
    }
}
