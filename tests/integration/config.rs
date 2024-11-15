//! Support for writing config files and using them in tests
//
// Taken from https://github.com/iqlusioninc/abscissa/blob/091c84d388a8ec6a1e2d69b9f61cf4439c839de1/core/src/testing/config.rs
// Licensed under Apache License Version 2.0, Copyright © 2018-2024 iqlusion
//
// The abscissa crate is distributed under the terms of the Apache License (Version 2.0).
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License.
// You may obtain a copy of the License at https://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions
// and limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0
//
// Remove this file once the `abscissa` crate is updated to a version that exposes this functionality:
// https://github.com/iqlusioninc/abscissa/pull/944

use serde::Serialize;
use std::{
    env,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Number of times to attempt to create a file before giving up
const FILE_CREATE_ATTEMPTS: usize = 1024;

/// Configuration file RAII guard which deletes it on completion
#[derive(Debug)]
pub struct ConfigFile {
    /// Path to the config file
    path: PathBuf,
}

impl ConfigFile {
    /// Create a config file by serializing it to the given location
    pub fn create<C>(app_name: &OsStr, config: &C) -> Self
    where
        C: Serialize,
    {
        let (path, mut file) = Self::open(app_name);

        let config_toml = toml::to_string_pretty(config)
            .unwrap_or_else(|e| panic!("error serializing config as TOML: {}", e))
            .into_bytes();

        file.write_all(&config_toml)
            .unwrap_or_else(|e| panic!("error writing config to {}: {}", path.display(), e));

        Self { path }
    }

    /// Get path to the configuration file
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    /// Create a temporary filename for the config
    fn open(app_name: &OsStr) -> (PathBuf, File) {
        // TODO: fully `OsString`-based path building
        let filename_prefix = app_name.to_string_lossy().to_string();

        for n in 0..FILE_CREATE_ATTEMPTS {
            let filename = format!("{}-{}.toml", &filename_prefix, n);
            let path = env::temp_dir().join(filename);

            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return (path, file),
                Err(e) => {
                    if e.kind() == io::ErrorKind::AlreadyExists {
                        continue;
                    } else {
                        panic!("couldn't create {}: {}", path.display(), e);
                    }
                }
            }
        }

        panic!(
            "couldn't create {}.toml after {} attempts!",
            filename_prefix, FILE_CREATE_ATTEMPTS
        )
    }
}

impl Drop for ConfigFile {
    fn drop(&mut self) {
        fs::remove_file(&self.path).unwrap_or_else(|e| {
            eprintln!("error removing {}: {}", self.path.display(), e);
        })
    }
}
