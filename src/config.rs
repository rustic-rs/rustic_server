//! `RusticServer` Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use std::{
    fs::{self},
    net::SocketAddr,
    path::{Path, PathBuf},
};

use clap::{ArgAction, Args, Parser};
use conflate::Merge;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{AppResult, ErrorKind};

/// `RusticServer` Configuration
#[derive(Clone, Debug, Deserialize, Serialize, Default, Merge, Parser)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct RusticServerConfig {
    /// Server settings
    #[command(flatten)]
    pub server: ConnectionSettings,

    /// Storage settings
    #[command(flatten)]
    pub storage: StorageSettings,

    /// Htpasswd settings
    #[command(flatten)]
    pub auth: HtpasswdSettings,

    /// Acl Settings
    #[command(flatten)]
    pub acl: AclSettings,

    /// Optional TLS Settings
    #[command(flatten)]
    pub tls: TlsSettings,

    /// Optional Logging settings
    #[command(flatten)]
    pub log: LogSettings,
}

/// Overwrite the left value with the right value if the right value is `Some`.
fn overwrite_with_some<T>(left: &mut Option<T>, right: Option<T>) {
    if right.is_some() {
        *left = right;
    }
}

/// Overwrite the left value with the right value unconditionally.
#[allow(dead_code)]
fn overwrite_left<T>(left: &mut T, right: T) {
    *left = right;
}

#[derive(Clone, Serialize, Deserialize, Debug, Merge, Parser, Copy)]
#[serde(deny_unknown_fields, default, rename_all = "kebab-case")]
pub struct ConnectionSettings {
    /// IP address and port to bind to
    #[arg(long, env = "RUSTIC_SERVER_LISTEN")]
    #[merge(strategy = overwrite_with_some)]
    pub listen: Option<SocketAddr>,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            listen: Some(default_socket_address()),
        }
    }
}

pub(crate) fn default_socket_address() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 8000))
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Merge, Parser)]
#[serde(deny_unknown_fields, default, rename_all = "kebab-case")]
pub struct LogSettings {
    /// Optional log level (trace, debug, info, warn, error)
    // We don't want to expose this to the CLI, as we use the global verbose flag there
    #[clap(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(strategy = overwrite_with_some)]
    pub log_level: Option<String>,

    /// Write HTTP requests in the combined log format to the specified filename
    ///
    /// If provided, the application will write logs to the specified file.
    /// If `None`, logging will be disabled or will use a default logging mechanism.
    #[arg(long = "log", env = "RUSTIC_SERVER_LOG_FILE")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(strategy = overwrite_with_some)]
    pub log_file: Option<PathBuf>,
}

impl LogSettings {
    pub const fn is_disabled(&self) -> bool {
        self.log_file.is_none()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Merge, Parser)]
#[serde(deny_unknown_fields, default, rename_all = "kebab-case")]
pub struct StorageSettings {
    /// Path to the data directory
    ///
    /// If `None`, the default directory will be used.
    ///
    /// # Caution
    ///
    /// By default the server persists backup data in the OS temporary directory
    /// (/tmp/restic on Linux/BSD and others, in %TEMP%\\restic in Windows, etc).
    #[arg(long = "path", env = "RUSTIC_SERVER_DATA_DIR")]
    #[merge(strategy = overwrite_with_some)]
    pub data_dir: Option<PathBuf>,

    /// Optional maximum size (quota) of a repository in bytes
    #[arg(long = "max-size", env = "RUSTIC_SERVER_QUOTA")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(strategy = overwrite_with_some)]
    pub quota: Option<usize>,
}

pub(crate) fn default_data_dir() -> PathBuf {
    std::env::temp_dir().join("restic")
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            data_dir: Some(default_data_dir()),
            quota: None,
        }
    }
}

const fn default_true() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize, Debug, Merge, Args)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
#[group(id = "tls")]
pub struct TlsSettings {
    /// Disable TLS support
    // This is a bit of a hack to allow us to set the default value to false
    // and disable TLS support by default.
    #[arg(
        long = "tls",
        action=ArgAction::SetFalse,
        default_value = "true",
        help = "Enable TLS support",
        requires = "tls_key",
        requires = "tls_cert"
    )]
    #[serde(default = "default_true")]
    #[merge(strategy = conflate::bool::overwrite_true)]
    pub disable_tls: bool,

    /// Optional path to the TLS key file
    #[arg(long, requires = "disable_tls", env = "RUSTIC_SERVER_TLS_KEY")]
    #[merge(strategy = overwrite_with_some)]
    pub tls_key: Option<PathBuf>,

    /// Optional path to the TLS certificate file
    #[arg(long, requires = "disable_tls", env = "RUSTIC_SERVER_TLS_CERT")]
    #[merge(strategy = overwrite_with_some)]
    pub tls_cert: Option<PathBuf>,
}

impl TlsSettings {
    pub const fn is_disabled(&self) -> bool {
        self.disable_tls
    }
}

impl Default for TlsSettings {
    fn default() -> Self {
        Self {
            disable_tls: true,
            tls_cert: None,
            tls_key: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Merge, Default, Parser)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
#[group(id = "auth")]
pub struct HtpasswdSettings {
    /// Disable .htpasswd authentication
    #[arg(long = "no-auth")]
    #[serde(default)]
    #[merge(strategy = conflate::bool::overwrite_false)]
    pub disable_auth: bool,

    /// Optional location of .htpasswd file (default: "<data directory>/.htpasswd")
    #[arg(long, env = "RUSTIC_SERVER_HTPASSWD_FILE")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(strategy = overwrite_with_some)]
    pub htpasswd_file: Option<PathBuf>,
}

impl HtpasswdSettings {
    pub fn htpasswd_file_or_default(&self, data_dir: PathBuf) -> AppResult<PathBuf> {
        let default_file_name = ".htpasswd";
        let path = self.htpasswd_file.clone().unwrap_or_else(|| {
            let mut path = data_dir;
            path.push(default_file_name);
            info!(
                "No htpasswd path specified, using default: `{}`",
                path.display()
            );
            path
        });

        if path
            .canonicalize()
            .map_err(|err| {
                ErrorKind::Io.context(format!(
                    "Does the htpasswd file exist at `{}`? We encountered an error: `{}`",
                    path.display(),
                    err
                ))
            })?
            .exists()
        {
            Ok(path)
        } else {
            Err(ErrorKind::Io
                .context(format!(
                    "Could not find `htpasswd` file at: `{}`",
                    path.display()
                ))
                .into())
        }
    }

    pub const fn is_disabled(&self) -> bool {
        self.disable_auth
    }
}

// This assumes that it makes no sense to have one but not the other
// So we if acl_path is given, we require the auth_path too.
#[derive(Clone, Serialize, Deserialize, Debug, Merge, Parser)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
#[group(id = "acl")]
pub struct AclSettings {
    /// Disable per-repo ACLs
    #[arg(skip)]
    #[serde(default = "default_true")]
    #[merge(strategy = conflate::bool::overwrite_false)]
    pub disable_acl: bool,

    /// Users can only access their private repositories
    #[arg(long, default_value = "true")]
    #[serde(skip)]
    #[merge(strategy = conflate::bool::overwrite_false)]
    pub private_repos: bool,

    /// Enable append only mode
    #[arg(long)]
    #[merge(strategy = conflate::bool::overwrite_false)]
    pub append_only: bool,

    /// Full path including file name to read from. Governs per-repo ACLs.
    /// (default: "<data directory>/acl.toml")
    #[arg(long, requires = "private_repos", env = "RUSTIC_SERVER_ACL_PATH")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(strategy = overwrite_with_some)]
    pub acl_path: Option<PathBuf>,
}

impl AclSettings {
    pub fn acl_file_or_default(&self, data_dir: PathBuf) -> AppResult<PathBuf> {
        let default_file_name = "acl.toml";
        let path = self.acl_path.clone().unwrap_or_else(|| {
            let mut path = data_dir;
            path.push(default_file_name);
            info!("No ACL path specified, using default: `{}`", path.display());
            path
        });

        if path
            .canonicalize()
            .map_err(|err| {
                ErrorKind::Io.context(format!(
                    "Does the {default_file_name} file exist at `{}`? We encountered an error: `{err}`",
                    path.display(),
                ))
            })?
            .exists()
        {
            Ok(path)
        } else {
            Err(ErrorKind::Io
                .context(format!(
                    "Could not find `{default_file_name}` file at: `{}`",
                    path.display()
                ))
                .into())
        }
    }

    pub const fn is_disabled(&self) -> bool {
        self.disable_acl || !self.private_repos
    }
}

impl Default for AclSettings {
    fn default() -> Self {
        Self {
            private_repos: true,
            disable_acl: false,
            append_only: true,
            acl_path: None,
        }
    }
}

impl RusticServerConfig {
    pub fn from_file(pth: &Path) -> AppResult<Self> {
        let s = fs::read_to_string(pth)?;

        let config: Self = toml::from_str(&s).map_err(|err| {
            ErrorKind::Io.context(format!(
                "Could not parse file: {} due to {}",
                pth.to_string_lossy(),
                err
            ))
        })?;

        Ok(config)
    }

    pub fn to_file(&self, pth: &Path) -> AppResult<()> {
        let toml_string = toml::to_string(&self).map_err(|err| {
            ErrorKind::Io.context(format!(
                "Could not serialize configuration to toml due to {}",
                err
            ))
        })?;

        fs::write(pth, toml_string)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use insta::{assert_debug_snapshot, assert_toml_snapshot};
    use rstest::{fixture, rstest};

    use crate::config::RusticServerConfig;

    #[fixture]
    fn rustic_server_config() -> PathBuf {
        Path::new("tests")
            .join("fixtures")
            .join("test_data")
            .join("rustic_server.toml")
    }

    #[rstest]
    #[ignore = "FIXME: This test is not platform agnostic."]
    fn test_default_config_passes() -> Result<()> {
        let config = RusticServerConfig::default();
        assert_toml_snapshot!(config, {
            ".storage.data-dir" => "[directory]",
        });

        Ok(())
    }

    #[rstest]
    #[ignore = "FIXME: This test is not platform agnostic."]
    fn test_config_parsing_from_file_passes(rustic_server_config: PathBuf) -> Result<()> {
        let config = RusticServerConfig::from_file(&rustic_server_config)?;
        assert_toml_snapshot!(config, {
            ".storage.data_dir" => "[directory]",
        });
        Ok(())
    }

    #[test]
    fn test_optional_explicit_parse_config_passes() -> Result<()> {
        let toml_string = r#"
[server]
listen = "127.0.0.1:8000"

[storage]
data-dir = "./test_data/test_repos/"

[auth]
disable-auth = true

[acl]
disable-acl = true

[tls]
disable-tls = true

[log]
log-level = "info"
"#;

        let config: RusticServerConfig = toml::from_str(toml_string)?;
        assert_debug_snapshot!(config);
        Ok(())
    }

    #[test]
    fn test_optional_implicit_parse_config_passes() -> Result<()> {
        let toml_string = r#"
[server]
listen = "127.0.0.1:8000"

[storage]
data-dir = "./test_data/test_repos/"
"#;

        let config: RusticServerConfig = toml::from_str(toml_string)?;
        assert_debug_snapshot!(config);
        Ok(())
    }

    #[test]
    #[ignore = "FIXME: This test is not platform agnostic."]
    fn test_issue_60_parse_config_passes() -> Result<()> {
        let toml_string = r#"
[acl]
disable-acl = true
append-only = false
"#;

        let config: RusticServerConfig = toml::from_str(toml_string)?;
        assert_debug_snapshot!(config);
        Ok(())
    }
}
