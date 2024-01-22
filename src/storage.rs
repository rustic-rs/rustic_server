use crate::handlers::file_helpers::WriteOrDeleteFile;
use anyhow::Result;
use once_cell::sync::OnceCell;
use std::fs;
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use walkdir::WalkDir;

//Static storage of our credentials
pub static STORAGE: OnceCell<Arc<dyn Storage>> = OnceCell::new();

pub(crate) fn init_storage(storage: impl Storage) -> Result<()> {
    if STORAGE.get().is_none() {
        let storage = Arc::new(storage);
        let _ = STORAGE.set(storage);
    }
    Ok(())
}

#[async_trait::async_trait]
//#[enum_dispatch(StorageEnum)]
pub trait Storage: Send + Sync + 'static {
    fn create_dir(&self, path: &Path, tpe: &str) -> IoResult<()>;
    fn read_dir(&self, path: &Path, tpe: &str) -> Box<dyn Iterator<Item = walkdir::DirEntry>>;
    fn filename(&self, path: &Path, tpe: &str, name: &str) -> PathBuf;
    async fn open_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<File>;
    async fn create_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<WriteOrDeleteFile>;
    fn remove_file(&self, path: &Path, tpe: &str, name: &str) -> IoResult<()>;
    fn remove_repository(&self, path: &Path) -> IoResult<()>;
}

#[derive(Debug, Clone)]
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

    fn remove_repository(&self, path: &Path) -> IoResult<()> {
        tracing::debug!(
            "Deleting repository: {}",
            self.path.join(path).to_string_lossy()
        );
        fs::remove_dir_all(self.path.join(path))
    }
}

#[cfg(test)]
mod test {
    use crate::storage::{init_storage, LocalStorage, STORAGE};
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_file_access() {
        let cwd = env::current_dir().unwrap();
        let repo_path = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("test_repos");

        let local_storage = LocalStorage::try_new(&repo_path).unwrap();
        init_storage(local_storage).unwrap();

        let storage = STORAGE.get().unwrap();

        // path must not start with slash !! that will skip the self.path from Storage!
        let path = PathBuf::new().join("test_repo/");
        let c = storage.read_dir(&path, "keys");
        let mut found = false;
        for a in c.into_iter() {
            let file_name = a.file_name().to_string_lossy();
            if file_name == "2e734da3fccb98724ece44efca027652ba7a335c224448a68772b41c0d9229d5" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn test_config_access() {
        let cwd = env::current_dir().unwrap();
        let repo_path = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("test_repos");

        let local_storage = LocalStorage::try_new(&repo_path).unwrap();
        init_storage(local_storage).unwrap();

        let storage = STORAGE.get().unwrap();

        // path must not start with slash !! that will skip the self.path from Storage!
        let path = PathBuf::new().join("test_repo/");
        let c = storage.open_file(&path, "", "config").await;
        assert!(c.is_ok())
    }
}
