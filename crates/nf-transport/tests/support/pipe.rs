use futures::{AsyncRead, AsyncWrite};
use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
#[derive(Default)]
struct Queue {
    bytes: VecDeque<u8>,
    reader: Option<Waker>,
    writer: Option<Waker>,
    closed: bool,
}
pub struct Pipe {
    incoming: Arc<Mutex<Queue>>,
    outgoing: Arc<Mutex<Queue>>,
}
pub fn pair() -> (Pipe, Pipe) {
    let a = Arc::new(Mutex::new(Queue::default()));
    let b = Arc::new(Mutex::new(Queue::default()));
    (
        Pipe {
            incoming: a.clone(),
            outgoing: b.clone(),
        },
        Pipe {
            incoming: b,
            outgoing: a,
        },
    )
}
impl AsyncRead for Pipe {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut q = self.incoming.lock().unwrap();
        let n = q.bytes.len().min(buf.len());
        for b in &mut buf[..n] {
            *b = q.bytes.pop_front().unwrap();
        }
        if n > 0 {
            if let Some(w) = q.writer.take() {
                w.wake();
            }
            return Poll::Ready(Ok(n));
        }
        if q.closed {
            return Poll::Ready(Ok(0));
        }
        q.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}
impl AsyncWrite for Pipe {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut q = self.outgoing.lock().unwrap();
        let n = (65536 - q.bytes.len()).min(buf.len());
        if q.closed {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }
        if n == 0 {
            q.writer = Some(cx.waker().clone());
            return Poll::Pending;
        }
        q.bytes.extend(&buf[..n]);
        if let Some(w) = q.reader.take() {
            w.wake();
        }
        Poll::Ready(Ok(n))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut q = self.outgoing.lock().unwrap();
        q.closed = true;
        if let Some(w) = q.reader.take() {
            w.wake();
        }
        Poll::Ready(Ok(()))
    }
}
