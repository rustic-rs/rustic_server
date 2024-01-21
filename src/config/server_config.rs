use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerConfig {
    pub server: Server,
    pub repos: Repos,
    pub tls: Option<TLS>,
    pub authorization: Authorization,
    pub accesscontrol: AccessControl,
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
    //HTTP, or HTTPS
    pub protocol: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TLS {
    pub key_path: String,
    pub cert_path: String,
}

impl ServerConfig {
    pub fn from_file(pth: &Path) -> Result<Self> {
        let s = fs::read_to_string(pth).context("Can not read server configuration file")?;
        let config: ServerConfig =
            toml::from_str(&s).context("Can not convert file to server configuration")?;
        Ok(config)
    }

    pub fn to_file(&self, pth: &Path) -> Result<()> {
        let toml_string =
            toml::to_string(&self).context("Could not serialize SeverConfig to TOML value")?;
        fs::write(&pth, toml_string).context("Could not write ServerConfig to file!")?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::Server;
    use crate::config::server_config::{AccessControl, Authorization, Repos, ServerConfig, TLS};
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_file_read() {
        let config_path = Path::new("test_data").join("rustic_server.toml");
        //let config_path = Path::new("/data/rustic/rustic_server.toml");
        let config = ServerConfig::from_file(&config_path);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.server.host_dns_name, "127.0.0.1");
        assert_eq!(config.repos.storage_path, "./test_data/test_repos/");
    }

    #[test]
    fn test_file_write() {
        let server_path = Path::new("tmp_test_data").join("rustic");
        fs::create_dir_all(&server_path).unwrap();

        let server = Server {
            host_dns_name: "127.0.0.1".to_string(),
            port: 2222,
            protocol: "HTTP".to_string(),
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
        let config = ServerConfig {
            log_level: Some(log),
            server,
            repos,
            tls,
            authorization: auth,
            accesscontrol: access,
        };
        let config_file = server_path.join("rustic_server.test.toml");
        config.to_file(&config_file).unwrap();

        // Try to read
        let _tmp_config = ServerConfig::from_file(&config_file).unwrap();
    }
}
