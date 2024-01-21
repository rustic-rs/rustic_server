use anyhow::Result;
use clap::{Parser, Subcommand};
use rustic_server::commands::serve::{serve, Opts};

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = RusticServer::parse();
    cmd.exec().await?;
    Ok(())
}

/// rustic_server
/// A REST server built in rust for use with rustic and restic.
#[derive(Parser)]
#[command(version, bin_name = "rustic_server", disable_help_subcommand = false)]
struct RusticServer {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the REST web-server.
    Serve(Opts),
    // Modify credentials in the .htaccess file.
    //Auth(HtAccessCmd),
    // Create a configuration from scratch.
    //Config,
}

/// The server configuration file should point us to the `.htaccess` file.
/// If not we complain to the user.
///
/// To be nice, if the `.htaccess` file pointed to does not exist, then we create it.
/// We do so, even if it is not called `.htaccess`.
impl RusticServer {
    pub async fn exec(self) -> Result<()> {
        match self.command {
            // Commands::Auth(cmd) => {
            //     cmd.exec()?;
            // }
            // Commands::Config => {
            //     rustic_server_configuration()?;
            // }
            Commands::Serve(opts) => {
                serve(opts).await.unwrap();
            }
        }
        Ok(())
    }
}
