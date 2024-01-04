use crate::acl::{AccessType, Acl, RepoAcl};
use crate::config::auth_config::HtAccess;
use crate::config::server_config::{
    AccessControl, Authorization, Repos, Server, ServerConfig, TLS,
};
use anyhow::{Context, Result};
use inquire::validator::Validation;
use inquire::{Confirm, CustomType, Password, Select, Text};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const PROTOCOL_HTTP: &str = "http"; // tide: must be lower case !!
const PROTOCOL_HTTPS: &str = "https"; // tide: must be lower case !!
const HT_ACCESS_FILE: &str = ".htaccess";
const ACL_FILE: &str = "acl.toml";

pub struct ServerConfigurator<'a> {
    server_path: PathBuf,
    file_name: &'a str,
    server_config: Option<ServerConfig>,
}

impl<'a> ServerConfigurator<'a> {
    /// creates a new configurator structure
    pub fn new(server_path: PathBuf, file_name: &'a str) -> Self {
        ServerConfigurator {
            server_path,
            file_name,
            server_config: None,
        }
    }

    /// Saves the configuration to a file.
    /// Here it is assumed the path to the file location is existing, and writable.
    pub fn save_configuration_file(&self) -> Result<()> {
        if self.server_config.is_some() {
            self.server_config
                .as_ref()
                .unwrap()
                .to_file(&self.server_path.join(self.file_name))?;
        }
        Ok(())
    }

    /// This script probes the user for answers that allows it to configure the server
    /// The server can then be started with a command like
    ///     rustic_server -P /<path>/<to>/rustic_server.toml
    pub fn ask_user_for_configuration_input(&mut self) -> Result<()> {
        let server = server_configuration()?;
        let repos = repo_root(&self.server_path)?;
        let accesscontrol = access_control_config(&self.server_path)?;
        let authorization = authorization_config(&self.server_path)?;

        // No sense in asking when the protocol is not HTTPS ...
        let tls = if server.protocol == PROTOCOL_HTTPS {
            Some(tls_paths()?)
        } else {
            None
        };

        let server_config = ServerConfig {
            server,
            repos,
            tls,
            authorization,
            accesscontrol,
        };
        self.server_config = Some(server_config);
        Ok(())
    }
}

fn server_configuration() -> Result<Server> {
    let protocol = vec![PROTOCOL_HTTP, PROTOCOL_HTTPS];

    let server_name = Text::new("Server FQDN or IP address:")
        .with_help_message("On which IP address can external computers contact your server?")
        .with_default("localhost")
        .prompt()?;

    let port = CustomType::<usize>::new("Port number:")
        .with_help_message("Choose a port which is not yet used for your server")
        .with_default(2222)
        .prompt()?;

    let protocol = Select::new("Communication protocol:", protocol)
        .with_help_message("When choosing HTTP-S, then add encryption keys later")
        .prompt()?
        .to_owned();

    let server = Server {
        host_dns_name: server_name,
        port: port.into(),
        protocol,
    };
    Ok(server)
}

fn repo_root(server_path: &PathBuf) -> Result<Repos> {
    let repo_root = Text::new("Folder name containing all repositories:")
        .with_help_message("Data is stored under: <server_path>/<repo>")
        .with_validator(|txt: &str| abs_path_validator(txt, false))
        .with_default("repo")
        .prompt()?;

    let repo_root = server_path.join(&repo_root);
    if !repo_root.exists() {
        fs::create_dir_all(&repo_root).context(format!(
            "Error: Failed to create path: {}",
            &repo_root.to_string_lossy()
        ))?;
    }

    let repos = Repos {
        storage_path: repo_root.to_string_lossy().to_string(),
    };
    Ok(repos)
}

fn access_control_config(server_path: &PathBuf) -> Result<AccessControl> {
    let use_access_control = Confirm::new("Use Access control?")
        .with_default(true)
        .with_help_message("An open server might not be wise in these days...")
        .prompt()?;

    let limit_to_append = Confirm::new("Limit changes to `append` only?")
        .with_default(true)
        .with_help_message("i.e. Disallow modification and delete actions...")
        .prompt()?;

    let access_control = if use_access_control {
        let acl_path = Path::new(&server_path).join(ACL_FILE);
        //"touch acl_path"
        if !acl_path.exists() {
            fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&acl_path)?;
        }

        // Access control creates a list of required passwords
        // So we fill the .htaccess file with all required users, with dummy passwords
        let auth_path = Path::new(&server_path).join(HT_ACCESS_FILE);
        access_control_file(&acl_path, &auth_path)?;

        AccessControl {
            acl_path: Some(acl_path.to_string_lossy().into()),
            private_repo: true,
            append_only: limit_to_append,
        }
    } else {
        AccessControl {
            acl_path: None,
            private_repo: false,
            append_only: limit_to_append,
        }
    };

    Ok(access_control)
}

fn access_type(a: &str) -> AccessType {
    return match a {
        "Nothing" => AccessType::Nothing,
        "Read" => AccessType::Read,
        "Append" => AccessType::Append,
        "Modify" => AccessType::Modify,
        _ => AccessType::Nothing,
    };
}

/// This function will create the `ACL.toml` file.
///
/// For each user mentioned in the `ACL.toml` file, a dummy entry will be created in the
/// `.htaccess` file (if not already exists). This will facilitate the creation of
/// a `.htaccess` file with all the required users. Changing the `.htaccess` file can be
/// done later with a separate command `rustic_server_htaccess`
fn access_control_file(acl_path: &PathBuf, auth_path: &PathBuf) -> Result<()> {
    let access_list = vec!["Nothing", "Read", "Append", "Modify"];

    let pepper: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    let mut count = 0;
    let mut acl = Acl::default();
    let mut auth = HtAccess::from_file(&auth_path)?;
    loop {
        let repo = Text::new("Select repository name:")
            .with_validator(|txt: &str| single_word_validator(txt))
            .with_help_message("Next: Define user to be allowed access to this repo.")
            .with_default("default")
            .prompt()?;
        let user = Text::new("Give user name:")
            .with_validator(|txt: &str| single_word_validator(txt))
            .with_default("admin")
            .prompt()?;
        let access = Select::new("What access does this user have:", access_list.clone())
            .with_help_message("If unsure select 'Append', you can change later")
            .prompt()?;
        let add_to_list = Confirm::new("Ok to add this user to the configuration?")
            .with_default(true)
            .prompt()?;
        if add_to_list {
            count += 1;
            let u = user.to_string();
            let r: String = repo.to_string();
            if r == "default" {
                acl.default_repo_access(&u, access_type(access))
            } else {
                if acl.repos.contains_key(&r) {
                    acl.repos
                        .get_mut(&r)
                        .unwrap()
                        .insert(u.clone(), access_type(access));
                } else {
                    let mut aa = RepoAcl::new();
                    aa.insert(u.clone(), access_type(access));
                    acl.repos.insert(r.clone(), aa);
                }
            }
            //Make sure the auth file knows all of these too ...
            auth.update(&u, pepper.as_str());
        }
        let not_stop = Confirm::new("Do you want to add another user?")
            .with_default(true)
            .prompt()?;

        if !not_stop {
            if count == 0 {
                let stop =
                    Confirm::new("You have to set at least enter 1 user. Do you want to stop?")
                        .with_default(false)
                        .with_help_message("Try 'admin' for the 'default' repository'")
                        .prompt()?;
                if stop {
                    return Ok(());
                }
            } else {
                // We are done; save the files, and break out of the loop
                auth.to_file()?;
                acl.to_file(&acl_path)?;
                break;
            }
        }
    }

    Ok(())
}

fn authorization_config(server_path: &PathBuf) -> Result<Authorization> {
    let use_authorization = Confirm::new("Use password authorization?")
        .with_default(true)
        .with_help_message("An open server might not be wise in these days...")
        .prompt()?;

    let authorization = if use_authorization {
        let auth_path = Path::new(&server_path).join(HT_ACCESS_FILE);

        //"touch auth_path"
        if !auth_path.exists() {
            fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&auth_path)?;
        }
        authorization_file(&auth_path)?;

        Authorization {
            auth_path: Some(auth_path.to_string_lossy().into()),
            use_auth: true,
        }
    } else {
        Authorization {
            auth_path: None,
            use_auth: false,
        }
    };

    Ok(authorization)
}

fn authorization_file(auth_path: &PathBuf) -> Result<()> {
    let mut hta_file = HtAccess::from_file(&auth_path)?;
    let users = hta_file.users();
    for user in users.iter() {
        let msg = format!("Give password for user: {}", &user);
        let password = Password::new(&msg)
            .with_help_message("The repo 'default' defines generic access (eg. for admin)")
            .prompt()?;
        hta_file.update(&user, &password);
    }
    hta_file.to_file()?;
    Ok(())
}

/// Allows the user to enter the path to the key files.
///
/// I find it hard to remember long file paths, so it is probably copy paste for most users.
/// Maybe use https://lib.rs/crates/tere to facilitate user memory ...
fn tls_paths() -> Result<TLS> {
    let pub_file = Text::new("Path to public key file?")
        .with_validator(|txt: &str| abs_path_validator(txt, true))
        .prompt()?;
    let crt_file = Text::new("Path the the certificate file?")
        .with_validator(|txt: &str| abs_path_validator(txt, true))
        .prompt()?;

    Ok(TLS {
        key_path: pub_file.to_string(),
        cert_path: crt_file.to_string(),
    })
}

// Use XOR in check. We are OK when both are either true, or both are false.
fn abs_path_validator(
    txt: &str,
    require_absolute: bool,
) -> Result<Validation, Box<dyn Error + Send + Sync>> {
    if !(Path::new(txt).is_absolute() ^ require_absolute) {
        Ok(Validation::Valid)
    } else {
        Ok(Validation::Invalid(
            "Require absolute path; Should start with a slash".into(),
        ))
    }
}

fn single_word_validator(txt: &str) -> Result<Validation, Box<dyn Error + Send + Sync>> {
    if txt.split(' ').count() < 2 {
        Ok(Validation::Valid)
    } else {
        Ok(Validation::Invalid("Require single word, no spaces".into()))
    }
}
