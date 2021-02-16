use std::fs;
use std::path::PathBuf;

use async_std::fs::{File, OpenOptions};
use async_std::io::{Result, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::web::Finalizer;

pub struct WriteOrDeleteFile {
    file: File,
    path: PathBuf,
    finalized: bool,
}

impl WriteOrDeleteFile {
    pub async fn new(file_path: PathBuf) -> Result<Self> {
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
    async fn finalize(&mut self) -> Result<()> {
        self.file.sync_all().await?;
        self.finalized = true;
        Ok(())
    }
}

impl Write for WriteOrDeleteFile {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Pin::new(&mut self.get_mut().file).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().file).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
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
