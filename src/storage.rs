use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use tokio::fs::{create_dir_all, remove_dir_all, remove_file, File};
use walkdir::WalkDir;

use crate::{
    error::{ApiErrorKind, ApiResult, AppResult},
    handlers::file_helpers::WriteOrDeleteFile,
};

//Static storage of our credentials
pub static STORAGE: OnceLock<Arc<dyn Storage>> = OnceLock::new();

pub(crate) fn init_storage(storage: impl Storage) -> AppResult<()> {
    let _ = STORAGE.get_or_init(|| Arc::new(storage));
    Ok(())
}

#[async_trait::async_trait]
//#[enum_dispatch(StorageEnum)]
pub trait Storage: Send + Sync + 'static {
    async fn create_dir(&self, path: &Path, tpe: Option<&str>) -> ApiResult<()>;
    fn read_dir(
        &self,
        path: &Path,
        tpe: Option<&str>,
    ) -> Box<dyn Iterator<Item = walkdir::DirEntry>>;
    fn filename(&self, path: &Path, tpe: &str, name: Option<&str>) -> PathBuf;
    async fn open_file(&self, path: &Path, tpe: &str, name: Option<&str>) -> ApiResult<File>;
    async fn create_file(
        &self,
        path: &Path,
        tpe: &str,
        name: Option<&str>,
    ) -> ApiResult<WriteOrDeleteFile>;
    async fn remove_file(&self, path: &Path, tpe: &str, name: Option<&str>) -> ApiResult<()>;
    async fn remove_repository(&self, path: &Path) -> ApiResult<()>;
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
    pub fn try_new(path: &Path) -> AppResult<Self> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

#[async_trait::async_trait]
impl Storage for LocalStorage {
    async fn create_dir(&self, path: &Path, tpe: Option<&str>) -> ApiResult<()> {
        match tpe {
            Some(tpe) if tpe == "data" => {
                for i in 0..256 {
                    create_dir_all(self.path.join(path).join(tpe).join(format!("{:02x}", i)))
                        .await
                        .map_err(|err| {
                            ApiErrorKind::CreatingDirectoryFailed(format!(
                                "Could not create directory: {err}"
                            ))
                        })?
                }
                Ok(())
            }
            Some(tpe) => create_dir_all(self.path.join(path).join(tpe))
                .await
                .map_err(|err| {
                    ApiErrorKind::CreatingDirectoryFailed(format!(
                        "Could not create directory: {err}"
                    ))
                }),
            None => create_dir_all(self.path.join(path)).await.map_err(|err| {
                ApiErrorKind::CreatingDirectoryFailed(format!("Could not create directory: {err}"))
            }),
        }
    }

    // FIXME: Make async?
    fn read_dir(
        &self,
        path: &Path,
        tpe: Option<&str>,
    ) -> Box<dyn Iterator<Item = walkdir::DirEntry>> {
        let path = if let Some(tpe) = tpe {
            self.path.join(path).join(tpe)
        } else {
            self.path.join(path)
        };

        let walker = WalkDir::new(path)
            .into_iter()
            .filter_map(walkdir::Result::ok)
            .filter(|e| e.file_type().is_file());
        Box::new(walker)
    }

    fn filename(&self, path: &Path, tpe: &str, name: Option<&str>) -> PathBuf {
        match (tpe, name) {
            ("config", _) => self.path.join(path).join("config"),
            ("data", Some(name)) => self.path.join(path).join(tpe).join(&name[0..2]).join(name),
            (tpe, Some(name)) => self.path.join(path).join(tpe).join(name),
            (path, None) => self.path.join(path),
        }
    }

    async fn open_file(&self, path: &Path, tpe: &str, name: Option<&str>) -> ApiResult<File> {
        let file_path = self.filename(path, tpe, name);
        Ok(File::open(file_path).await.map_err(|err| {
            ApiErrorKind::OpeningFileFailed(format!("Could not open file: {}", err))
        })?)
    }

    async fn create_file(
        &self,
        path: &Path,
        tpe: &str,
        name: Option<&str>,
    ) -> ApiResult<WriteOrDeleteFile> {
        let file_path = self.filename(path, tpe, name);
        WriteOrDeleteFile::new(file_path).await
    }

    async fn remove_file(&self, path: &Path, tpe: &str, name: Option<&str>) -> ApiResult<()> {
        let file_path = self.filename(path, tpe, name);
        remove_file(file_path).await.map_err(|err| {
            ApiErrorKind::RemovingFileFailed(format!("Could not remove file: {err}"))
        })
    }

    async fn remove_repository(&self, path: &Path) -> ApiResult<()> {
        tracing::debug!(
            "Deleting repository: {}",
            self.path.join(path).to_string_lossy()
        );
        remove_dir_all(self.path.join(path)).await.map_err(|err| {
            ApiErrorKind::RemovingRepositoryFailed(format!("Could not remove repository: {err}"))
        })
    }
}

#[cfg(test)]
mod test {
    use crate::storage::{init_storage, LocalStorage, STORAGE};
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_file_access_passes() {
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
        let c = storage.read_dir(&path, Some("keys"));
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
    async fn test_config_access_passes() {
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
        let c = storage.open_file(&path, "", Some("config")).await;
        assert!(c.is_ok())
    }
}
