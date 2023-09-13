use std::fs;
use std::path::{Path, PathBuf};

use crate::helpers::WriteOrDeleteFile;
use std::io::Result as IoResult;
use tokio::fs::File;
use walkdir::WalkDir;

#[async_trait::async_trait]
pub trait Storage: Send + Sync + 'static {
    fn create_dir(&self, path: &Path, tpe: &str) -> IoResult<()>;
    fn read_dir(&self, path: &Path, tpe: &str) -> Box<dyn Iterator<Item = walkdir::DirEntry>>;
    fn filename(&self, path: &Path, tpe: &str, name: &str) -> PathBuf;
    async fn open_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<File>;
    async fn create_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<WriteOrDeleteFile>;
    fn remove_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<()>;
}

#[derive(Clone)]
pub struct LocalStorage {
    path: PathBuf,
}

impl Default for LocalStorage {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        return Self {
            path: PathBuf::from(r"C:\tmp\rustic"),
        };
        #[cfg(not(target_os = "windows"))]
        Self {
            path: PathBuf::from("/tmp/rustic"),
        }
    }
}

impl LocalStorage {
    pub fn try_new(path: &Path) -> IoResult<Self> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

#[async_trait::async_trait]
impl Storage for LocalStorage {
    fn create_dir(&self, path: &Path, tpe: &str) -> IoResult<()> {
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

    fn read_dir(&self, path: &Path, tpe: &str) -> Box<dyn Iterator<Item = walkdir::DirEntry>> {
        let walker = WalkDir::new(self.path.join(path).join(tpe))
            .into_iter()
            .filter_map(walkdir::Result::ok)
            .filter(|e| e.file_type().is_file());
        Box::new(walker)
    }

    fn filename(&self, path: &Path, tpe: &str, name: &str) -> PathBuf {
        match tpe {
            "config" => self.path.join(path).join("config"),
            "data" => self.path.join(path).join(tpe).join(&name[0..2]).join(name),
            _ => self.path.join(path).join(tpe).join(name),
        }
    }

    async fn open_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<File> {
        let file_path = self.filename(path, tpe, name);
        Ok(File::open(file_path).await?)
    }

    async fn create_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<WriteOrDeleteFile> {
        let file_path = self.filename(path, tpe, name);
        WriteOrDeleteFile::new(file_path).await
    }

    fn remove_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<()> {
        let file_path = self.filename(path, tpe, name);
        fs::remove_file(file_path)
    }
}
