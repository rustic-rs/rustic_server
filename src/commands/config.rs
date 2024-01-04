use anyhow::Context;
use anyhow::Result;
use inquire::validator::Validation;
use inquire::Text;
use rustic_server::config::configurator::ServerConfigurator;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

/// This configuration helper assumes a fixed folder layout:
///  - server path is the folder that contains all config
///  - rustic_server config file name is defined by the user and stored under /<server_path>/.htaccess
///  - the '.htaccess' file has predefined name and location: /<server_path>/.htaccess
///  - the acl.toml file has predefined name and location /<server_path>/acl.toml
/// Hence the user is asked for a folder, and rustic_server config file name. We can deduce the rest.
///
/// The ServerConfiguration TOML file definition allows the user to change the file locations.
/// This can be done by manually later. For example if the configuration files should go to
/// a mandatory location like "/etc/..." for some reason.
pub fn main() -> Result<()> {
    let server_path = ask_server_path()?;
    let file_name = ask_configuration_file_name(&server_path)?;

    let mut configurator = ServerConfigurator::new(server_path, &file_name);
    configurator.ask_user_for_configuration_input()?;
    configurator.save_configuration_file()?;
    Ok(())
}

///Ask user to provide a file name for the rustic_server configuration
fn ask_configuration_file_name(server_path: &PathBuf) -> Result<String> {
    let file_name = Text::new("Give the file name?")
        .with_help_message("`rustic_server` can later be started using this configuration file.")
        .with_validator(|txt: &str| abs_path_validator(txt, false))
        .with_default("rustic_server.toml")
        .prompt()?;

    //Fail fast, so we try to create the file,or check for writeability existing file
    //Pre condition: server_path already exists!!
    let config_file = server_path.join(&file_name);
    if config_file.exists() {
        if !config_file.is_file() {
            println!(
                "Error: The server file configuration leads to a folder, not a file: \n\t{}",
                &server_path.to_string_lossy()
            );
            exit(0);
        }
        match fs::OpenOptions::new()
            //Test: "open for writing"
            .create(false)
            .truncate(false)
            .append(true)
            .open(&config_file)
        {
            Ok(_) => {}
            Err(e) => {
                println!(
                    "No write access to the server configuration file.{}",
                    config_file.to_string_lossy()
                );
                println!("Got error: {}.", e);
                exit(0);
            }
        }
    } else {
        //"touch server_config file"
        match fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&config_file)
        {
            Ok(_) => {}
            Err(e) => {
                println!(
                    "Failed to create empty server configuration file.{}",
                    config_file.to_string_lossy()
                );
                println!("Got error: {}.", e);
                exit(0);
            }
        }
    }

    Ok(file_name)
}

/// Ask the user for a server path, and do some checks
fn ask_server_path() -> Result<PathBuf> {
    let server_path = Text::new("Give the root path for the server")
        .with_help_message("Under this path both the repositories, and configuration are stored")
        .with_validator(|txt: &str| abs_path_validator(txt, true))
        .with_default("/data/rustic")
        .prompt()?;
    let server_path = Path::new(&server_path).to_path_buf();

    if !server_path.exists() {
        fs::create_dir_all(&server_path).context(format!(
            "Error: Failed to create path: {}",
            &server_path.to_string_lossy()
        ))?;
    }
    if server_path.is_file() {
        println!(
            "Error: The given path leads to a file, not a folder: \n\t{}",
            &server_path.to_string_lossy()
        );
        exit(0);
    }
    if server_path.metadata().unwrap().permissions().readonly() {
        println!(
            "Error: We can not write the parent folder: \n\t{}",
            &server_path.to_string_lossy()
        );
        exit(0);
    }

    Ok(server_path)
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
