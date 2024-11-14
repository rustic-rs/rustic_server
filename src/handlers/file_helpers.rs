use std::{
    cell::RefCell,
    fs,
    io::Result as IoResult,
    path::PathBuf,
    pin::Pin,
    result::Result,
    task::{Context, Poll},
};

use serde::{Serialize, Serializer};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWrite,
};

use crate::error::{ApiErrorKind, ApiResult};

// helper struct which is like a async_std|tokio::fs::File but removes the file
// if finalize() was not called.
#[derive(Debug)]
pub struct WriteOrDeleteFile {
    file: File,
    path: PathBuf,
    finalized: bool,
}

#[async_trait::async_trait]
pub trait Finalizer {
    async fn finalize(&mut self) -> ApiResult<()>;
}

impl WriteOrDeleteFile {
    pub async fn new(path: PathBuf) -> ApiResult<Self> {
        tracing::debug!("[WriteOrDeleteFile] path: {path:?}");

        if !path.exists() {
            let parent = path.parent().ok_or_else(|| {
                ApiErrorKind::WritingToFileFailed("Could not get parent directory".to_string())
            })?;

            fs::create_dir_all(parent).map_err(|err| {
                ApiErrorKind::WritingToFileFailed(format!("Could not create directory: {}", err))
            })?;
        }

        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .map_err(|err| {
                ApiErrorKind::WritingToFileFailed(format!("Could not write to file: {}", err))
            })?;

        Ok(Self {
            file,
            path,
            finalized: false,
        })
    }
}

#[async_trait::async_trait]
impl Finalizer for WriteOrDeleteFile {
    async fn finalize(&mut self) -> ApiResult<()> {
        self.file.sync_all().await.map_err(|err| {
            ApiErrorKind::FinalizingFileFailed(format!("Could not sync file: {}", err))
        })?;
        self.finalized = true;
        Ok(())
    }
}

impl AsyncWrite for WriteOrDeleteFile {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.get_mut().file).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().file).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().file).poll_shutdown(cx)
    }
}

impl Drop for WriteOrDeleteFile {
    fn drop(&mut self) {
        if !self.finalized {
            // ignore errors
            fs::remove_file(&self.path).unwrap_or(());
        }
    }
}

// helper struct to make iterators serializable
pub struct IteratorAdapter<I>(RefCell<I>);

impl<I> IteratorAdapter<I> {
    pub const fn new(iterator: I) -> Self {
        Self(RefCell::new(iterator))
    }
}

impl<I> Serialize for IteratorAdapter<I>
where
    I: Iterator,
    I::Item: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.borrow_mut().by_ref())
    }
}
