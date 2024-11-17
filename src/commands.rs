//! `RusticServer` Subcommands
//!
//! This is where you specify the subcommands of your application.
//!
//! The default application comes with two subcommands:
//!
//! - `start`: launches the application
//! - `--version`: print application version
//!
//! See the `impl Configurable` below for how to specify the path to the
//! application's configuration file.

mod auth;
mod serve;

use crate::{
    commands::{auth::AuthCmd, serve::ServeCmd},
    config::RusticServerConfig,
};
use abscissa_core::{
    config::Override, tracing::info, Command, Configurable, FrameworkError, Runnable,
};
use clap::builder::{
    styling::{AnsiColor, Effects},
    Styles,
};
use std::path::PathBuf;

/// `RusticServer` Configuration Filename
pub const CONFIG_FILE: &str = "rustic_server.toml";

/// `RusticServer` Subcommands
/// Subcommands need to be listed in an enum.
#[derive(clap::Parser, Command, Debug, Runnable)]
pub enum RusticServerCmd {
    /// Authentication for users. Add, update, delete, or list users.
    Auth(AuthCmd),

    /// Start a server with the specified configuration
    Serve(ServeCmd),
}

fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Red.on_default() | Effects::BOLD)
        .usage(AnsiColor::Red.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Green.on_default())
}

/// Entry point for the application. It needs to be a struct to allow using subcommands!
#[derive(clap::Parser, Command, Debug)]
#[command(author, about, version)]
#[command(author, about, name="rustic-server", styles=styles(), version = env!("CARGO_PKG_VERSION"))]
pub struct EntryPoint {
    #[command(subcommand)]
    cmd: RusticServerCmd,

    /// Enable verbose logging
    #[arg(short, long, global = true, env = "RUSTIC_SERVER_VERBOSE")]
    pub verbose: bool,

    /// Use the specified config file
    #[arg(short, long, global = true, env = "RUSTIC_SERVER_CONFIG_PATH")]
    pub config: Option<String>,
}

impl Runnable for EntryPoint {
    fn run(&self) {
        self.cmd.run();
    }
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<RusticServerConfig> for EntryPoint {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        // Early return if no config file was provided
        if self.config.is_none() {
            info!("No configuration file provided.");
            return None;
        }

        let filename = self
            .config
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| CONFIG_FILE.into());

        if filename.exists() {
            // Check if the config file exists, and if it does not,
            info!("Using configuration file: `{filename:?}`");
            Some(filename)
        } else {
            info!("Provided configuration file not found. Trying default.");
            // for a missing configuration file to be a hard error
            // instead, always return `Some(CONFIG_FILE)` here.
            Some(PathBuf::from(CONFIG_FILE))
        }
    }

    /// Apply changes to the config after it's been loaded, e.g. overriding
    /// values in a config file using command-line options.
    ///
    /// This can be safely deleted if you don't want to override config
    /// settings from command-line options.
    fn process_config(
        &self,
        config: RusticServerConfig,
    ) -> Result<RusticServerConfig, FrameworkError> {
        match &self.cmd {
            RusticServerCmd::Serve(cmd) => cmd.override_config(config),
            _ => Ok(config),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::EntryPoint;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        EntryPoint::command().debug_assert();
    }
}
