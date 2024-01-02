use std::fs;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::process::exit;
use inquire::Password;
use rustic_server::config::auth_config::HtAccess;
use rustic_server::config::server_config::ServerConfig;

/// Tool to change the `.htaccess` file for a given rustic_server.
///
/// Design decision: Use the server configuration file as input.
/// This way the user only has 1 file to "know".
fn main() {
    let cmd = HtAccessCmd::parse();
    cmd.exec().unwrap();
}

/// rustic_server_htaccess:
/// Tool to edit the '.htaccess' file given a `rustic_server` configuration.
/// We take the server configuration to allow a cross check with the repository ACL list.
/// For a clean start, start with creating the server configuration using 'rustic_server_config'
#[derive(Parser)]
#[command(version, bin_name = "rustic_server_htaccess", disable_help_subcommand = false)]
struct HtAccessCmd {

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

        let ht_access_path = PathBuf::new()
            .join(server_config.authorization.auth_path.unwrap());
        HtAccessCmd::check(&ht_access_path);

        let mut ht_access = HtAccess::from_file(&ht_access_path)?;
        match &self.command {
            Commands::Add(arg) => { add(&mut ht_access, arg)?; }
            Commands::Delete(arg) => { delete(&mut ht_access, arg)?; }
            Commands::List => { print(&ht_access, &ht_access_path); }
        } ;
        Ok(())
    }

    fn check(path:&PathBuf) {
        //Check
        if path.exists() {
            if ! path.is_file() {
                println!("Error: Given path leads to a folder, not a file: \n\t{}",
                         path.to_string_lossy());
                exit(0);
            }
            match fs::OpenOptions::new()
                //Test: "open for writing"
                .create(false)
                .truncate(false)
                .append(true)
                .open(&path) {
                Ok(_) => {}
                Err(e) => {
                    println!("No write access to the htaccess file.{}",
                             path.to_string_lossy());
                    println!( "Got error: {}.", e);
                    exit(0);
                }
            }
        } else {
            //"touch server_config file"
            match fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&path) {
                Ok(_) => {}
                Err(e) => {
                    println!("Failed to create empty server configuration file.{}",
                             &path.to_string_lossy());
                    println!( "Got error: {}.", e);
                    exit(0);
                }
            }
        }

    }
}


#[derive(Subcommand)]
enum Commands {
    /// Add a new credential to the .htaccess file
    /// If the user name already exists it will update the password only.
    Add(AddArg),
    /// Delete an existing credential from the .htaccess file
    Delete(DelArg),
    /// List all users known in the .htaccess file
    List,
}

#[derive(Args)]
struct AddArg{
    /// Name of the user to be added
    #[arg(short = 'u')]
    user:String
}

#[derive(Args)]
struct DelArg{
    /// Name of the user to be removed
    #[arg(short = 'u')]
    user:String
}

fn add(hta:&mut HtAccess, arg:&AddArg) -> Result<()>{
    if hta.users().contains(&arg.user.to_string()) {
        println!("Update the password for a user with name {}?", arg.user.as_str())
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

fn delete(hta:&mut HtAccess, arg:&DelArg) -> Result<()> {
    if hta.users().contains(&arg.user.to_string()) {
        println!("Deleting user with name {}.", arg.user.as_str());
        hta.delete(arg.user.as_str());
        hta.to_file()?;
    } else {
        println!("Could not find a user with name {}. No changes made.", arg.user.as_str())
    };
    Ok(())
}

fn print(hta:&HtAccess, pth:&PathBuf) {
    println!( "Listing users in the .htaccess file for a rustic_server.");
    println!( "\tConfiguration file used: {} ", pth.to_string_lossy());
    println!( "List:");
    for u in hta.users() {
        println!("\t{}", u);
    }
    println!( "Done.");
}
