use serde::{Serialize, Serializer};
use std::cell::RefCell;
use std::fs;
use std::io::Result as IoResult;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWrite;

// helper struct which is like a async_std|tokio::fs::File but removes the file
// if finalize() was not called.
pub struct WriteOrDeleteFile {
    file: File,
    path: PathBuf,
    finalized: bool,
}
#[async_trait::async_trait]
pub trait Finalizer {
    type Error;
    async fn finalize(&mut self) -> Result<(), Self::Error>;
}

impl WriteOrDeleteFile {
    pub async fn new(file_path: PathBuf) -> IoResult<Self> {
        Ok(Self {
            file: OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&file_path)
                .await?,
            path: file_path,
            finalized: false,
        })
    }
}

#[async_trait::async_trait]
impl Finalizer for WriteOrDeleteFile {
    type Error = std::io::Error;

    async fn finalize(&mut self) -> IoResult<()> {
        self.file.sync_all().await?;
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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.borrow_mut().by_ref())
    }
}
