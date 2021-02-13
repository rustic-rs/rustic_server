// TODO: error handling in saving files!

//use std::sync::Arc;
//use parking_lot::RwLock;
//use tempfile::TempDir;

use async_std::fs::{File, OpenOptions};
use std::fs;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn try_new(path: &PathBuf) -> Result<Self, IoError> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn create_dir(&self, path: &Path, tpe: &str) -> std::io::Result<()> {
        match tpe {
            "data" => {
                for i in 0..256 {
                    fs::create_dir_all(self.path.join(path).join(tpe).join(format!("{:02x}", i)))?
                }
                Ok(())
            }
            _ => fs::create_dir_all(self.path.join(path).join(tpe)),
        }
    }

    pub fn read_dir(&self, path: &Path, tpe: &str) -> impl Iterator<Item = fs::DirEntry> {
        // TODO: error handling
        self.path
            .join(path)
            .join(tpe)
            .read_dir()
            .unwrap()
            .filter(|e| e.as_ref().unwrap().file_type().unwrap().is_file())
            .map(|e| e.unwrap())
    }

    pub fn filename(&self, path: &Path, tpe: &str, name: &str) -> PathBuf {
        match tpe {
            "config" => self.path.join(path).join("config"),
            "data" => self.path.join(path).join(tpe).join(&name[0..2]).join(name),
            _ => self.path.join(path).join(tpe).join(name),
        }
    }

    pub async fn open_file(&self, path: &Path, tpe: &str, name: &str) -> Result<File, IoError> {
        let file_path = self.filename(path, tpe, name);
        Ok(File::open(file_path).await?)
    }

    pub async fn create_file(&self, path: &Path, tpe: &str, name: &str) -> Result<File, IoError> {
        let file_path = self.filename(path, tpe, name);

        Ok(OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_path)
            .await?)
    }

    pub fn remove_file(&self, path: &Path, tpe: &str, name: &str) -> Result<(), IoError> {
        let file_path = self.filename(path, tpe, name);
        Ok(fs::remove_file(&file_path)?)
    }
}
