use crate::config::auth_config::HtAccess;
use crate::config::server_config::ServerConfig;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use inquire::Password;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser)]
#[command()]
pub struct HtAccessCmd {
    ///Give the path where the `rustic_server` configuration can be found
    #[arg(short = 'c')]
    pub config_path: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

/// The server configuration file should point us to the `.htaccess` file.
/// If not we complain to the user.
///
/// To be nice, if the `.htaccess` file pointed to does not exist, then we create it.
/// We do so, even if it is not called `.htaccess`.
impl HtAccessCmd {
    pub fn exec(&self) -> Result<()> {
        let server_config = ServerConfig::from_file(&self.config_path)?;
        if server_config.authorization.auth_path.is_none() {
            println!("The server configuration does not point to an authorization file.");
            exit(0);
        }

        let ht_access_path = PathBuf::new().join(server_config.authorization.auth_path.unwrap());
        HtAccessCmd::check(&ht_access_path);

        let mut ht_access = HtAccess::from_file(&ht_access_path)?;
        match &self.command {
            Commands::Add(arg) => {
                add(&mut ht_access, arg)?;
            }
            Commands::Update(arg) => {
                update(&mut ht_access, arg)?;
            }
            Commands::Delete(arg) => {
                delete(&mut ht_access, arg)?;
            }
            Commands::List => {
                print(&ht_access, &ht_access_path);
            }
        };
        Ok(())
    }

    fn check(path: &PathBuf) {
        //Check
        if path.exists() {
            if !path.is_file() {
                println!(
                    "Error: Given path leads to a folder, not a file: \n\t{}",
                    path.to_string_lossy()
                );
                exit(0);
            }
            match fs::OpenOptions::new()
                //Test: "open for writing" (fail fast)
                .create(false)
                .truncate(false)
                .append(true)
                .open(&path)
            {
                Ok(_) => {}
                Err(e) => {
                    println!(
                        "No write access to the htaccess file.{}",
                        path.to_string_lossy()
                    );
                    println!("Got error: {}.", e);
                    exit(0);
                }
            }
        } else {
            //"touch server_config file" (fail fast)
            match fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&path)
            {
                Ok(_) => {}
                Err(e) => {
                    println!(
                        "Failed to create empty server configuration file.{}",
                        &path.to_string_lossy()
                    );
                    println!("Got error: {}.", e);
                    exit(0);
                }
            }
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new credential to the .htaccess file.
    /// If the user name already exists it will update the password only.
    Add(AddArg),
    /// Change the password for an existing user.
    Update(AddArg),
    /// Delete an existing credential from the .htaccess file.
    Delete(DelArg),
    /// List all users known in the .htaccess file.
    List,
}

#[derive(Args)]
struct AddArg {
    /// Name of the user to be added.
    #[arg(short = 'u')]
    user: String,
}

#[derive(Args)]
struct DelArg {
    /// Name of the user to be removed.
    #[arg(short = 'u')]
    user: String,
}

fn add(hta: &mut HtAccess, arg: &AddArg) -> Result<()> {
    if hta.users().contains(&arg.user.to_string()) {
        println!(
            "Give the password for a user with name {}?",
            arg.user.as_str()
        )
    } else {
        println!("Creating a new user with name {}?", arg.user.as_str())
    };

    let msg = format!("Give a password:");
    let password = Password::new(&msg)
        .prompt()
        .expect("Inquiry.rs: Could not get a password");

    hta.update(arg.user.as_str(), password.as_str());

    hta.to_file()?;
    Ok(())
}

fn update(hta: &mut HtAccess, arg: &AddArg) -> Result<()> {
    if !hta.credentials.contains_key(arg.user.as_str()) {
        println!(
            "I can not find a user with name {}. Use add command?",
            arg.user.as_str()
        );
        exit(0);
    }
    add(hta, arg)
}

fn delete(hta: &mut HtAccess, arg: &DelArg) -> Result<()> {
    if hta.users().contains(&arg.user.to_string()) {
        println!("Deleting user with name {}.", arg.user.as_str());
        hta.delete(arg.user.as_str());
        hta.to_file()?;
    } else {
        println!(
            "Could not find a user with name {}. No changes made.",
            arg.user.as_str()
        )
    };
    Ok(())
}

fn print(hta: &HtAccess, pth: &PathBuf) {
    println!("Listing users in the .htaccess file for a rustic_server.");
    println!("\tConfiguration file used: {} ", pth.to_string_lossy());
    println!("List:");
    for u in hta.users() {
        println!("\t{}", u);
    }
    println!("Done.");
}
