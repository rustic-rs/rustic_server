//! `RusticServer` Abscissa Application

use crate::{commands::EntryPoint, config::RusticServerConfig};
use abscissa_core::Config;
use abscissa_core::FrameworkErrorKind::IoError;
use abscissa_core::{
    application::{self, AppCell},
    config::{self, CfgCell},
    path::AbsPathBuf,
    trace, Application, FrameworkError, StandardPaths,
};
use abscissa_tokio::TokioComponent;
use std::path::Path;

/// Application state
pub static RUSTIC_SERVER_APP: AppCell<RusticServerApp> = AppCell::new();

/// `RusticServer` Application
#[derive(Debug)]
pub struct RusticServerApp {
    /// Application configuration.
    config: CfgCell<RusticServerConfig>,

    /// Application state.
    state: application::State<Self>,
}

/// Initialize a new application instance.
///
/// By default no configuration is loaded, and the framework state is
/// initialized to a default, empty state (no components, threads, etc).
impl Default for RusticServerApp {
    fn default() -> Self {
        Self {
            config: CfgCell::default(),
            state: application::State::default(),
        }
    }
}

impl Application for RusticServerApp {
    /// Entrypoint command for this application.
    type Cmd = EntryPoint;

    /// Application configuration.
    type Cfg = RusticServerConfig;

    /// Paths to resources within the application.
    type Paths = StandardPaths;

    /// Accessor for application configuration.
    fn config(&self) -> config::Reader<RusticServerConfig> {
        self.config.read()
    }

    /// Borrow the application state immutably.
    fn state(&self) -> &application::State<Self> {
        &self.state
    }

    /// Register all components used by this application.
    ///
    /// If you would like to add additional components to your application
    /// beyond the default ones provided by the framework, this is the place
    /// to do so.
    fn register_components(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        let mut components = self.framework_components(command)?;

        // Create `TokioComponent` and add it to your app's components here:
        components.push(Box::new(TokioComponent::new()?));

        self.state.components_mut().register(components)
    }

    /// Post-configuration lifecycle callback.
    ///
    /// Called regardless of whether config is loaded to indicate this is the
    /// time in app lifecycle when configuration would be loaded if
    /// possible.
    fn after_config(&mut self, config: Self::Cfg) -> Result<(), FrameworkError> {
        // Configure components
        self.state.components_mut().after_config(&config)?;
        self.config.set_once(config);
        Ok(())
    }

    /// Load configuration from the given path.
    ///
    /// Returns an error if the configuration could not be loaded.
    fn load_config(&mut self, path: &Path) -> Result<Self::Cfg, FrameworkError> {
        let canonical_path = AbsPathBuf::canonicalize(path).map_err(|_err| {
            FrameworkError::from(IoError.context(
                "It seems like your configuration wasn't found! Please make sure it exists at the given location!"
            ))
        })?;

        Self::Cfg::load_toml_file(canonical_path)
    }

    /// Get tracing configuration from command-line options
    fn tracing_config(&self, command: &EntryPoint) -> trace::Config {
        if command.verbose {
            trace::Config::verbose()
        } else {
            trace::Config::default()
        }
    }
}
