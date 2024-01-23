use std::{
    cell::RefCell,
    fs,
    io::Result as IoResult,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use serde::{Serialize, Serializer};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWrite,
};

use crate::error::{ErrorKind, Result};

// helper struct which is like a async_std|tokio::fs::File but removes the file
// if finalize() was not called.
pub struct WriteOrDeleteFile {
    file: File,
    path: PathBuf,
    finalized: bool,
}

#[async_trait::async_trait]
pub trait Finalizer {
    async fn finalize(&mut self) -> Result<()>;
}

impl WriteOrDeleteFile {
    pub async fn new(file_path: PathBuf) -> Result<Self> {
        Ok(Self {
            file: OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&file_path)
                .await
                .map_err(|err| {
                    ErrorKind::WritingToFileFailed(format!("Could not write to file: {}", err))
                })?,
            path: file_path,
            finalized: false,
        })
    }
}

#[async_trait::async_trait]
impl Finalizer for WriteOrDeleteFile {
    async fn finalize(&mut self) -> Result<()> {
        self.file.sync_all().await.map_err(|err| {
            ErrorKind::FinalizingFileFailed(format!("Could not sync file: {}", err))
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
    pub fn new(iterator: I) -> Self {
        Self(RefCell::new(iterator))
    }
}

impl<I> Serialize for IteratorAdapter<I>
where
    I: Iterator,
    I::Item: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.borrow_mut().by_ref())
    }
}
