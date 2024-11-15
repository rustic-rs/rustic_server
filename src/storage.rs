use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use tokio::fs::{create_dir_all, remove_dir_all, remove_file, File};
use walkdir::WalkDir;

use crate::{
    config::default_data_dir,
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
    /// Initialize the storage
    fn init(path: &Path) -> ApiResult<Self>
    where
        Self: Sized;

    /// Returns the path of the storage
    fn path(&self) -> &Path;

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
        Self {
            path: default_data_dir(),
        }
    }
}

impl LocalStorage {}

#[async_trait::async_trait]
impl Storage for LocalStorage {
    fn init(path: &Path) -> ApiResult<Self> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

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
                        })?;
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
        let path = tpe.map_or_else(
            || self.path.join(path),
            |tpe| self.path.join(path).join(tpe),
        );

        let walker = WalkDir::new(path)
            .into_iter()
            .filter_map(walkdir::Result::ok)
            // FIXME: Why do we filter out directories!?
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
    use crate::storage::{init_storage, LocalStorage, Storage, STORAGE};
    use std::path::PathBuf;

    #[test]
    fn test_file_access_passes() {
        let local_storage =
            LocalStorage::init(&PathBuf::from("tests/generated/test_storage")).unwrap();
        init_storage(local_storage).unwrap();

        let storage = STORAGE.get().unwrap();

        // path must not start with slash !! that will skip the self.path from Storage!
        let path = PathBuf::new().join("test_repo/");
        let c = storage.read_dir(&path, Some("keys"));
        let mut found = false;
        for a in c.into_iter() {
            let file_name = a.file_name().to_string_lossy();
            if file_name == "3f918b737a2b9f72f044d06d6009eb34e0e8d06668209be3ce86e5c18dac0295" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn test_config_access_passes() {
        let local_storage =
            LocalStorage::init(&PathBuf::from("tests/generated/test_storage")).unwrap();
        init_storage(local_storage).unwrap();

        let storage = STORAGE.get().unwrap();

        // path must not start with slash !! that will skip the self.path from Storage!
        let path = PathBuf::new().join("test_repo/");
        let c = storage.open_file(&path, "", Some("config")).await;
        assert!(c.is_ok());
    }
}
