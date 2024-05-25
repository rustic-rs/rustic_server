use std::{fs, path::Path};

use serde_derive::{Deserialize, Serialize};

use crate::error::{ErrorKind, Result};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerConfiguration {
    pub server: Server,
    pub repos: Repos,
    pub tls: Option<TLS>,
    pub authorization: Authorization,
    pub access_control: AccessControl,
    pub log_level: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Repos {
    pub storage_path: String,
}

// This assumes that it makes no sense to have one but not the other
// So we if acl_path is given, we require the auth_path too.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AccessControl {
    pub acl_path: Option<String>,
    //if not private all repo are accessible for any user
    pub private_repo: bool,
    //force access to append only for all
    pub append_only: bool,
}

// This assumes that it makes no sense to have one but not the other
// So we if acl_path is given, we require the auth_path too.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Authorization {
    pub auth_path: Option<String>,
    //use authorization file
    pub use_auth: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Server {
    pub host_dns_name: String,
    pub port: usize,
    pub common_root_path: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TLS {
    pub key_path: String,
    pub cert_path: String,
}

impl ServerConfiguration {
    pub fn from_file(pth: &Path) -> Result<Self> {
        let s = fs::read_to_string(pth).map_err(|err| {
            ErrorKind::InternalError(format!(
                "Could not read server config file: {} at {:?}",
                err, pth
            ))
        })?;
        let config: ServerConfiguration = toml::from_str(&s).map_err(|err| {
            ErrorKind::InternalError(format!("Could not parse TOML file: {}", err))
        })?;
        Ok(config)
    }

    pub fn to_file(&self, pth: &Path) -> Result<()> {
        let toml_string = toml::to_string(&self).map_err(|err| {
            ErrorKind::InternalError(format!(
                "Could not serialize SeverConfig to TOML value: {}",
                err
            ))
        })?;
        fs::write(pth, toml_string).map_err(|err| {
            ErrorKind::InternalError(format!("Could not write ServerConfig to file: {}", err))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use anyhow::Result;
    use rstest::*;

    use super::{AccessControl, Authorization, Repos, Server, ServerConfiguration, TLS};

    #[fixture]
    fn rustic_server_config() -> PathBuf {
        Path::new("tests")
            .join("fixtures")
            .join("test_data")
            .join("rustic_server.toml")
    }

    #[test]
    fn test_file_read() -> Result<()> {
        let config_path = rustic_server_config();
        let config = ServerConfiguration::from_file(&config_path)?;

        assert_eq!(config.server.host_dns_name, "127.0.0.1");
        assert_eq!(
            config.repos.storage_path,
            "rustic_server/tests/fixtures/test_data/test_repos/"
        );
        Ok(())
    }

    #[test]
    fn test_file_write() -> Result<()> {
        let server_path = Path::new("tmp_test_data").join("rustic");
        fs::create_dir_all(&server_path)?;

        let server = Server {
            host_dns_name: "127.0.0.1".to_string(),
            port: 2222,
            common_root_path: "".into(),
        };

        let tls: Option<TLS> = Some(TLS {
            key_path: "somewhere".to_string(),
            cert_path: "somewhere/else".to_string(),
        });

        let repos: Repos = Repos {
            storage_path: server_path.join("repos").to_string_lossy().into(),
        };

        let auth = Authorization {
            auth_path: Some("auth_path".to_string()),
            use_auth: true,
        };

        let access = AccessControl {
            acl_path: Some("acl_path".to_string()),
            private_repo: true,
            append_only: true,
        };

        let log = "debug".to_string();

        // Try to write
        let config = ServerConfiguration {
            log_level: Some(log),
            server,
            repos,
            tls,
            authorization: auth,
            access_control: access,
        };
        let config_file = server_path.join("rustic_server.toml");
        config.to_file(&config_file)?;

        // Try to read
        let _tmp_config = ServerConfiguration::from_file(&config_file)?;

        Ok(())
    }
}
