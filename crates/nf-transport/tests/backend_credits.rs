#[path = "support/pipe.rs"]
mod pipe;
use futures::{AsyncRead, AsyncWrite};
use libp2p::core::muxing::StreamMuxer;
use nf_transport::{mux::LaneMuxConfig, records::Lane};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
#[test]
fn maintained_bulk_backend_stalls_at_credit_and_refuses_second_stream_then_closes() {
    let (a, b) = pipe::pair();
    let mut left = LaneMuxConfig::new(Lane::Bulk).connection(a, yamux::Mode::Client);
    let mut right = LaneMuxConfig::new(Lane::Bulk).connection(b, yamux::Mode::Server);
    let w = futures::task::noop_waker();
    let mut cx = Context::from_waker(&w);
    let Poll::Ready(Ok(mut outbound)) = Pin::new(&mut left).poll_outbound(&mut cx) else {
        panic!("first stream")
    };
    let bytes = vec![42; 300000];
    let mut sent = 0;
    let mut inbound = None;
    for _ in 0..1000 {
        if sent < bytes.len()
            && let Poll::Ready(Ok(n)) = Pin::new(&mut outbound).poll_write(&mut cx, &bytes[sent..])
        {
            sent += n;
        }
        let _ = Pin::new(&mut left).poll(&mut cx);
        let _ = Pin::new(&mut right).poll(&mut cx);
        if inbound.is_none()
            && let Poll::Ready(Ok(s)) = Pin::new(&mut right).poll_inbound(&mut cx)
        {
            inbound = Some(s);
        }
    }
    assert_eq!(sent, 262144);
    assert!(
        Pin::new(&mut outbound)
            .poll_write(&mut cx, &bytes[sent..])
            .is_pending()
    );
    // A separate real maintained control connection still moves bytes while bulk has zero send credit.
    let (ca, cb) = pipe::pair();
    let mut control_left = LaneMuxConfig::new(Lane::Control).connection(ca, yamux::Mode::Client);
    let mut control_right = LaneMuxConfig::new(Lane::Control).connection(cb, yamux::Mode::Server);
    let Poll::Ready(Ok(mut control_out)) = Pin::new(&mut control_left).poll_outbound(&mut cx)
    else {
        panic!("control stream");
    };
    assert!(matches!(
        Pin::new(&mut control_out).poll_write(&mut cx, b"control-progress"),
        Poll::Ready(Ok(16))
    ));
    let mut control_in = None;
    let mut control_bytes = [0; 16];
    let mut control_received = 0;
    for _ in 0..1000 {
        let _ = Pin::new(&mut control_left).poll(&mut cx);
        let _ = Pin::new(&mut control_right).poll(&mut cx);
        if control_in.is_none()
            && let Poll::Ready(Ok(s)) = Pin::new(&mut control_right).poll_inbound(&mut cx)
        {
            control_in = Some(s);
        }
        if let Some(s) = &mut control_in
            && let Poll::Ready(Ok(n)) =
                Pin::new(s).poll_read(&mut cx, &mut control_bytes[control_received..])
        {
            control_received += n;
            if control_received == 16 {
                break;
            }
        }
    }
    assert_eq!(&control_bytes, b"control-progress");
    assert_eq!(control_received, 16);
    assert!(
        Pin::new(&mut outbound)
            .poll_write(&mut cx, &bytes[sent..])
            .is_pending()
    );
    let mut inbound = inbound.unwrap();
    let mut received = 0;
    let mut buf = [0; 8192];
    for _ in 0..1000 {
        if let Poll::Ready(Ok(n)) = Pin::new(&mut inbound).poll_read(&mut cx, &mut buf) {
            received += n;
        }
        let _ = Pin::new(&mut right).poll(&mut cx);
        let _ = Pin::new(&mut left).poll(&mut cx);
    }
    assert_eq!(received, 262144);
    assert!(
        matches!(Pin::new(&mut outbound).poll_write(&mut cx,&bytes[sent..]),Poll::Ready(Ok(n)) if n>0)
    );
    assert!(matches!(
        Pin::new(&mut left).poll_outbound(&mut cx),
        Poll::Ready(Err(yamux::ConnectionError::TooManyStreams))
    ));
    let mut closed = false;
    for _ in 0..1000 {
        if Pin::new(&mut right).poll_close(&mut cx).is_ready() {
            closed = true;
            break;
        }
        let _ = Pin::new(&mut left).poll(&mut cx);
    }
    assert!(closed);
}
#[test]
fn maintained_parser_rejects_oversized_body_declaration_before_missing_payload() {
    let mut bytes = vec![0, 0, 0, 1];
    bytes.extend(1u32.to_be_bytes());
    bytes.extend(1048577u32.to_be_bytes());
    let mut mux = LaneMuxConfig::new(Lane::Bulk).connection(
        HeaderOnly(futures::io::Cursor::new(bytes)),
        yamux::Mode::Server,
    );
    let w = futures::task::noop_waker();
    let mut cx = Context::from_waker(&w);
    let result = Pin::new(&mut mux).poll_inbound(&mut cx);
    assert!(
        matches!(
            result,
            Poll::Ready(Err(yamux::ConnectionError::Decode(
                yamux::FrameDecodeError::FrameTooLarge(1048577)
            )))
        ),
        "oversized parser declaration must fail: {result:?}"
    );
}
struct HeaderOnly(futures::io::Cursor<Vec<u8>>);
impl AsyncRead for HeaderOnly {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        b: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        assert!(
            self.0.position() < 12,
            "parser must reject oversized declaration before reading body"
        );
        Pin::new(&mut self.0).poll_read(cx, b)
    }
}
impl AsyncWrite for HeaderOnly {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        b: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(b.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
#[test]
fn maintained_control_backend_admits_four_streams_then_rejects_fifth() {
    let (a, _) = pipe::pair();
    let mut mux = LaneMuxConfig::new(Lane::Control).connection(a, yamux::Mode::Client);
    let w = futures::task::noop_waker();
    let mut cx = Context::from_waker(&w);
    let mut streams = Vec::new();
    for _ in 0..4 {
        let Poll::Ready(Ok(s)) = Pin::new(&mut mux).poll_outbound(&mut cx) else {
            panic!("four control streams admitted")
        };
        streams.push(s);
    }
    assert!(matches!(
        Pin::new(&mut mux).poll_outbound(&mut cx),
        Poll::Ready(Err(yamux::ConnectionError::TooManyStreams))
    ));
}
