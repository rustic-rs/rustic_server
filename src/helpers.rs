// used by WriteOrDeleteFile
use std::fs;
use std::path::PathBuf;

use async_std::fs::{File, OpenOptions};
use async_std::io::{self, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::web::Finalizer;

// helper struct which is like a async_std::fs::File but removes the file
// if finalize() was not called.
pub struct WriteOrDeleteFile {
    file: File,
    path: PathBuf,
    finalized: bool,
}

impl WriteOrDeleteFile {
    pub async fn new(file_path: PathBuf) -> io::Result<Self> {
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
    async fn finalize(&mut self) -> io::Result<()> {
        self.file.sync_all().await?;
        self.finalized = true;
        Ok(())
    }
}

impl Write for WriteOrDeleteFile {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().file).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().file).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().file).poll_close(cx)
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

// used by IteratorAdapter
use serde::{Serialize, Serializer};
use std::cell::RefCell;

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
