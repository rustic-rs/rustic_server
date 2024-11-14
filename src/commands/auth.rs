//! `auth` subcommand

use std::path::PathBuf;

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};

use crate::{htpasswd::Htpasswd, prelude::RUSTIC_SERVER_APP};

/// `auth` subcommand
///
/// The `Parser` proc macro generates an option parser based on the struct
/// definition, and is defined in the `clap` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/clap/>
#[derive(Command, Debug, Parser)]
pub struct AuthCmd {
    #[command(subcommand)]
    command: Commands,
}

impl Runnable for AuthCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.inner_run() {
            status_err!("{}", err);
            RUSTIC_SERVER_APP.shutdown(Shutdown::Crash);
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Add a new credential to the .htpasswd file.
    /// If the username already exists it will update the password only.
    Add(AddArg),
    /// Change the password for an existing user.
    Update(AddArg),
    /// Delete an existing credential from the .htpasswd file.
    Delete(DelArg),
    /// List all users known in the .htpasswd file.
    List(PrintArg),
}

#[derive(Args, Debug)]
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

#[derive(Args, Debug)]
struct DelArg {
    ///Path to authorization file
    #[arg(short = 'f')]
    pub config_path: PathBuf,
    /// Name of the user to be removed.
    #[arg(short = 'u')]
    user: String,
}

#[derive(Args, Debug)]
struct PrintArg {
    ///Path to authorization file
    #[arg(short = 'f')]
    pub config_path: PathBuf,
}

/// The server configuration file should point us to the `.htpasswd` file.
/// If not we complain to the user.
///
/// To be nice, if the `.htpasswd` file pointed to does not exist, then we create it.
/// We do so, even if it is not called `.htpasswd`.
impl AuthCmd {
    pub fn inner_run(&self) -> Result<()> {
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
}

fn check(path: &PathBuf) -> Result<()> {
    //Check
    if path.exists() {
        if !path.is_file() {
            bail!(
                "Error: Given path leads to a folder, not a file: {}",
                path.to_string_lossy()
            );
        }

        if let Err(err) = std::fs::OpenOptions::new()
            //Test: "open for writing" (fail fast)
            .create(false)
            .truncate(false)
            .append(true)
            .open(path)
        {
            bail!(
                "No write access to the htpasswd file: {} due to {}",
                path.to_string_lossy(),
                err
            );
        };
    } else {
        //"touch server_config file" (fail fast)
        if let Err(err) = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
        {
            bail!(
                "Failed to create empty server configuration file: {} due to {}",
                &path.to_string_lossy(),
                err
            );
        };
    };

    Ok(())
}

fn add(arg: &AddArg) -> Result<()> {
    let ht_access_path = PathBuf::from(&arg.config_path);
    check(&ht_access_path)?;
    let mut ht_access = Htpasswd::from_file(&ht_access_path)?;

    if ht_access.users().contains(&arg.user.to_string()) {
        bail!(
            "User '{}' exists; use update to change password. No changes were made.",
            arg.user.as_str()
        );
    }

    let _ = ht_access.update(arg.user.as_str(), arg.password.as_str());

    ht_access.to_file()?;
    Ok(())
}

fn update(arg: &AddArg) -> Result<()> {
    let ht_access_path = PathBuf::from(&arg.config_path);
    check(&ht_access_path)?;
    let mut ht_access = Htpasswd::from_file(&ht_access_path)?;

    if !ht_access.credentials.contains_key(arg.user.as_str()) {
        bail!(
            "I can not find a user with name {}. Use add command?",
            arg.user.as_str()
        );
    }
    let _ = ht_access.update(arg.user.as_str(), arg.password.as_str());
    ht_access.to_file()?;
    Ok(())
}

fn delete(arg: &DelArg) -> Result<()> {
    let ht_access_path = PathBuf::from(&arg.config_path);
    check(&ht_access_path)?;
    let mut ht_access = Htpasswd::from_file(&ht_access_path)?;

    if ht_access.users().contains(&arg.user.to_string()) {
        println!("Deleting user with name {}.", arg.user.as_str());
        let _ = ht_access.delete(arg.user.as_str());
        ht_access.to_file()?;
    } else {
        println!(
            "Could not find a user with name {}. No changes were made.",
            arg.user.as_str()
        );
    };
    Ok(())
}

fn print(arg: &PrintArg) -> Result<()> {
    let ht_access_path = PathBuf::from(&arg.config_path);
    check(&ht_access_path)?;
    let ht_access = Htpasswd::from_file(&ht_access_path)?;

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
