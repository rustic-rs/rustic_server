use std::{io, mem};
use std::pin::Pin;
use std::task::{Context, Poll};
use axum::body::Bytes;
use axum::response::{IntoResponse, Response};
use axum_range::RangeBody;
use futures::Stream;
use http_body_util::combinators::Frame;
use tokio::io::ReadBuf;
use tokio_util::bytes::BytesMut;
use pin_project::pin_project;

const IO_BUFFER_SIZE: usize = 64 * 1024;

// Code copied from: https://github.com/haileys/axum-range/blob/main/src/stream.rs

/// Response body stream. Implements [`Stream`], [`Body`], and [`IntoResponse`].
#[pin_project]
pub struct RangedStream<B> {
    state: StreamState,
    length: u64,
    #[pin]
    body: B,
}

impl<B: RangeBody + Send + 'static> RangedStream<B> {
    pub(crate) fn new(body: B, start: u64, length: u64) -> Self {
        RangedStream {
            state: StreamState::Seek { start },
            length,
            body,
        }
    }
}

#[derive(Debug)]
enum StreamState {
    Seek { start: u64 },
    Seeking { remaining: u64 },
    Reading { buffer: BytesMut, remaining: u64 },
}

impl<B: RangeBody + Send + 'static> IntoResponse for RangedStream<B> {
    fn into_response(self) -> Response {
        Response::new(axum::body::Body::new(self))
    }
}

impl<B: RangeBody> Body for RangedStream<B> {
    type Data = Bytes;
    type Error = io::Error;

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.length)
    }

    fn poll_frame(self: Pin<&mut Self>, cx: &mut Context<'_>)
                  -> Poll<Option<io::Result<Frame<Bytes>>>>
    {
        self.poll_next(cx).map(|item| item.map(|result| result.map(Frame::data)))
    }
}

impl<B: RangeBody> Stream for RangedStream<B> {
    type Item = io::Result<Bytes>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<io::Result<Bytes>>> {
        let mut this = self.project();

        if let StreamState::Seek { start } = *this.state {
            match this.body.as_mut().start_seek(start) {
                Err(e) => { return Poll::Ready(Some(Err(e))); }
                Ok(()) => {
                    let remaining = *this.length;
                    *this.state = StreamState::Seeking { remaining };
                }
            }
        }

        if let StreamState::Seeking { remaining } = *this.state {
            match this.body.as_mut().poll_complete(cx) {
                Poll::Pending => { return Poll::Pending; }
                Poll::Ready(Err(e)) => { return Poll::Ready(Some(Err(e))); }
                Poll::Ready(Ok(())) => {
                    let buffer = allocate_buffer();
                    *this.state = StreamState::Reading { buffer, remaining };
                }
            }
        }

        if let StreamState::Reading { mut buffer, remaining } = this.state {
            let uninit = buffer.spare_capacity_mut();

            // calculate max number of bytes to read in this iteration, the
            // smaller of the buffer size and the number of bytes remaining
            let nbytes = std::cmp::min(
                uninit.len(),
                usize::try_from(*remaining).unwrap_or(usize::MAX),
            );

            let mut read_buf = ReadBuf::uninit(&mut uninit[0..nbytes]);

            match this.body.as_mut().poll_read(cx, &mut read_buf) {
                Poll::Pending => { return Poll::Pending; }
                Poll::Ready(Err(e)) => { return Poll::Ready(Some(Err(e))); }
                Poll::Ready(Ok(())) => {
                    match read_buf.filled().len() {
                        0 => { return Poll::Ready(None); }
                        n => {
                            // SAFETY: poll_read has filled the buffer with `n`
                            // additional bytes. `buffer.len` should always be
                            // 0 here, but include it for rigorous correctness
                            unsafe { buffer.set_len(buffer.len() + n); }

                            // replace state buffer and take this one to return
                            let chunk = mem::replace(&mut buffer, allocate_buffer());

                            // subtract the number of bytes we just read from
                            // state.remaining, this usize->u64 conversion is
                            // guaranteed to always succeed, because n cannot be
                            // larger than remaining due to the cmp::min above
                            *remaining -= u64::try_from(n).unwrap();

                            // return this chunk
                            return Poll::Ready(Some(Ok(chunk.freeze())));
                        }
                    }
                }
            }
        }

        unreachable!();
    }
}

fn allocate_buffer() -> BytesMut {
    BytesMut::with_capacity(IO_BUFFER_SIZE)
}