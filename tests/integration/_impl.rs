use std::time::Duration;

use crate::config::ConfigFile;
use assert_cmd::Command;
use serde::Serialize;

pub trait AssertCmdExt {
    /// Add the given configuration file
    fn config(&mut self, config: &impl Serialize) -> &mut Self;

    /// Enable test mode
    fn test_mode_args(&mut self) -> &mut Self;
}

impl AssertCmdExt for Command {
    fn config(&mut self, config: &impl Serialize) -> &mut Self {
        let target_bin = self.get_program().to_owned();
        let config_file = ConfigFile::create(&target_bin, config);

        // Leak the config file to keep it alive for the duration of the test
        let static_config: &'static mut ConfigFile = Box::leak(Box::new(config_file));

        self.args(["-c", &static_config.path().display().to_string()]);
        self
    }

    fn test_mode_args(&mut self) -> &mut Self {
        self.timeout(Duration::from_secs(10)) // Set a timeout of 10 seconds
            .args(["-v"]) // Enable verbose logging
            .env("CI", "1"); // Enable CI test mode

        self
    }
}
