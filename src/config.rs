//! RusticServer Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use std::path::PathBuf;

use clap::Parser;
use conflate::Merge;
use serde::{Deserialize, Serialize};

/// RusticServer Configuration
#[derive(Clone, Debug, Deserialize, Serialize, Default, Merge, Parser)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct RusticServerConfig {
    /// Server settings
    #[clap(flatten)]
    pub server: ConnectionSettings,

    /// Storage settings
    #[clap(flatten)]
    pub storage: StorageSettings,

    /// Htpasswd settings
    #[clap(flatten)]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub auth: Option<HtpasswdSettings>,

    /// Acl Settings
    #[clap(flatten)]
    pub acl: AclSettings,

    /// Optional TLS Settings
    #[clap(flatten)]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub tls: Option<TlsSettings>,

    /// Optional Logging settings
    #[clap(flatten)]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub logging: Option<LogSettings>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Merge, Parser)]
#[serde(default, rename_all = "kebab-case")]
pub struct ConnectionSettings {
    /// IP address and port to bind to
    #[arg(long, default_value = "localhost:8000")]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub listen: Option<String>,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            listen: Some("localhost:8000".to_string()),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Merge, Parser)]
#[serde(default, rename_all = "kebab-case")]
pub struct LogSettings {
    /// Write HTTP requests in the combined log format to the specified filename
    #[arg(long = "log")]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub log_file: Option<PathBuf>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Merge, Parser)]
#[serde(default, rename_all = "kebab-case")]
pub struct StorageSettings {
    /// Optional path to the data directory
    #[serde(rename = "data_dir")]
    #[arg(long = "path", default_value = "/tmp/restic")]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub data_dir: Option<PathBuf>,

    /// Optional maximum size of the repository in Bytes
    #[arg(short, long)]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub max_size: Option<usize>,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            data_dir: Some("/tmp/restic".into()),
            max_size: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Merge, Parser)]
#[serde(rename_all = "kebab-case")]
pub struct TlsSettings {
    /// Enable TLS support
    #[arg(long)]
    #[merge(strategy = conflate::bool::overwrite_false)]
    pub tls: bool,

    /// Optional path to the TLS key file
    #[arg(long, requires = "tls")]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub tls_key: Option<PathBuf>,

    /// Optional path to the TLS certificate file
    #[arg(long, requires = "tls")]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub tls_cert: Option<PathBuf>,
}

// TODO: This assumes that it makes no sense to have one but not the other
// So we if acl_path is given, we require the auth_path too.
#[derive(Clone, Serialize, Deserialize, Debug, Default, Merge, Parser)]
#[serde(rename_all = "kebab-case")]
pub struct HtpasswdSettings {
    /// Disable .htpasswd authentication
    #[arg(long = "no-auth")]
    #[merge(strategy = conflate::bool::overwrite_true)]
    pub disable_auth: bool,

    /// Optional location of .htpasswd file (default: "<data directory>/.htpasswd")
    #[arg(long)]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub htpasswd_file: Option<PathBuf>,
}

// This assumes that it makes no sense to have one but not the other
// So we if acl_path is given, we require the auth_path too.
#[derive(Clone, Serialize, Deserialize, Debug, Default, Merge, Parser)]
#[serde(rename_all = "kebab-case")]
pub struct AclSettings {
    /// Full path including file name to read from. Governs per-repo ACLs.
    #[arg(long)]
    #[merge(strategy = conflate::option::overwrite_none)]
    pub acl_path: Option<PathBuf>,

    /// Users can only access their private repo
    #[arg(long)]
    #[merge(strategy = conflate::bool::overwrite_false)]
    pub private_repo: bool,

    /// Enable append only mode
    #[arg(long)]
    #[merge(strategy = conflate::bool::overwrite_false)]
    pub append_only: bool,
}

// impl RusticServerConfig {
//     pub fn from_file(pth: &Path) -> Result<Self> {
//         let s = fs::read_to_string(pth).map_err(|err| {
//             WebErrorKind::InternalError(format!(
//                 "Could not read server config file: {} at {:?}",
//                 err, pth
//             ))
//         })?;
//         let config: ServerConfiguration = toml::from_str(&s).map_err(|err| {
//             WebErrorKind::InternalError(format!("Could not parse TOML file: {}", err))
//         })?;
//         Ok(config)
//     }

//     pub fn to_file(&self, pth: &Path) -> Result<()> {
//         let toml_string = toml::to_string(&self).map_err(|err| {
//             WebErrorKind::InternalError(format!(
//                 "Could not serialize SeverConfig to TOML value: {}",
//                 err
//             ))
//         })?;
//         fs::write(pth, toml_string).map_err(|err| {
//             WebErrorKind::InternalError(format!("Could not write ServerConfig to file: {}", err))
//         })?;
//         Ok(())
//     }
// }

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
