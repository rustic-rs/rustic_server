use enum_dispatch::enum_dispatch;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};

#[enum_dispatch]
#[derive(Debug, Clone)]
pub(crate) enum AuthCheckerEnum {
    Auth(Auth),
}

#[enum_dispatch(AuthCheckerEnum)]
pub trait AuthChecker: Send + Sync + 'static {
    fn verify(&self, user: &str, passwd: &str) -> bool;
}

// read_htpasswd is a helper func that reads the given file in .httpasswd format
// into a Hashmap mapping each user to the whole passwd line
fn read_htpasswd(file_path: &PathBuf) -> io::Result<HashMap<&'static str, &'static str>> {
    let s = fs::read_to_string(file_path)?;
    // make the contents static in memory
    let s = Box::leak(s.into_boxed_str());

    let mut user_map = HashMap::new();
    for line in s.lines() {
        let user = line.split(':').collect::<Vec<&str>>()[0];
        user_map.insert(user, line);
    }
    Ok(user_map)
}

#[derive(Debug, Clone)]
pub struct Auth {
    users: Option<HashMap<&'static str, &'static str>>,
}

impl Default for Auth {
    fn default() -> Self {
        Self { users: None }
    }
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
}

impl AuthChecker for Auth {
    // verify verifies user/passwd against the credentials saved in users.
    // returns true if Auth::users is None.
    fn verify(&self, user: &str, passwd: &str) -> bool {
        match &self.users {
            Some(users) => {
                matches!(users.get(user), Some(passwd_data) if htpasswd_verify::Htpasswd::from(*passwd_data).check(user, passwd))
            }
            None => true,
        }
    }
}
