use crate::config::auth_file::HtAccess;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser)]
#[command()]
pub struct HtAccessCmd {
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
        match &self.command {
            Commands::Add(arg) => {
                add(arg)?;
            }
            Commands::Update(arg) => {
                update(arg)?;
            }
            Commands::Delete(arg) => {
                delete(arg)?;
            }
            Commands::List(arg) => {
                print(arg)?;
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
                .open(path)
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
                .open(path)
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
    /// If the username already exists it will update the password only.
    Add(AddArg),
    /// Change the password for an existing user.
    Update(AddArg),
    /// Delete an existing credential from the .htaccess file.
    Delete(DelArg),
    /// List all users known in the .htaccess file.
    List(PrintArg),
}

#[derive(Args)]
struct AddArg {
    ///Path to authorization file
    #[arg(short = 'f')]
    pub config_path: PathBuf,
    /// Name of the user to be added.
    #[arg(short = 'u')]
    user: String,
    /// Password.
    #[arg(short = 'p')]
    password: String,
}

#[derive(Args)]
struct DelArg {
    ///Path to authorization file
    #[arg(short = 'f')]
    pub config_path: PathBuf,
    /// Name of the user to be removed.
    #[arg(short = 'u')]
    user: String,
}

#[derive(Args)]
struct PrintArg {
    ///Path to authorization file
    #[arg(short = 'f')]
    pub config_path: PathBuf,
}

fn add(arg: &AddArg) -> Result<()> {
    let ht_access_path = PathBuf::from(&arg.config_path);
    HtAccessCmd::check(&ht_access_path);
    let mut ht_access = HtAccess::from_file(&ht_access_path)?;

    if ht_access.users().contains(&arg.user.to_string()) {
        println!(
            "User '{}' exists; use update to change password. No changes were made.",
            arg.user.as_str()
        );
        exit(0);
    }

    ht_access.update(arg.user.as_str(), arg.password.as_str());

    ht_access.to_file()?;
    Ok(())
}

fn update(arg: &AddArg) -> Result<()> {
    let ht_access_path = PathBuf::from(&arg.config_path);
    HtAccessCmd::check(&ht_access_path);
    let mut ht_access = HtAccess::from_file(&ht_access_path)?;

    if !ht_access.credentials.contains_key(arg.user.as_str()) {
        println!(
            "I can not find a user with name {}. Use add command?",
            arg.user.as_str()
        );
        exit(0);
    }
    ht_access.update(arg.user.as_str(), arg.password.as_str());
    ht_access.to_file()?;
    Ok(())
}

fn delete(arg: &DelArg) -> Result<()> {
    let ht_access_path = PathBuf::from(&arg.config_path);
    HtAccessCmd::check(&ht_access_path);
    let mut ht_access = HtAccess::from_file(&ht_access_path)?;

    if ht_access.users().contains(&arg.user.to_string()) {
        println!("Deleting user with name {}.", arg.user.as_str());
        ht_access.delete(arg.user.as_str());
        ht_access.to_file()?;
    } else {
        println!(
            "Could not find a user with name {}. No changes were made.",
            arg.user.as_str()
        )
    };
    Ok(())
}

fn print(arg: &PrintArg) -> Result<()> {
    let ht_access_path = PathBuf::from(&arg.config_path);
    HtAccessCmd::check(&ht_access_path);
    let ht_access = HtAccess::from_file(&ht_access_path)?;

    println!("Listing users in the access file for a rustic_server.");
    println!(
        "\tConfiguration file used: {} ",
        ht_access_path.to_string_lossy()
    );
    println!("List:");
    for u in ht_access.users() {
        println!("\t{}", u);
    }
    println!("Done.");
    Ok(())
}
