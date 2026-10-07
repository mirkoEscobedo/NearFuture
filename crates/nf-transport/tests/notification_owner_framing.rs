//! Completed reads are data parsing, never authentication or a runtime grant.
use futures::{AsyncRead, io::Cursor};
use libp2p::{StreamProtocol, request_response::Codec};
use nf_transport::{
    notification::PROTOCOL,
    notification_effects::{NotifyOwnerCodec, NotifyRequest},
};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
fn framed_hello() -> Vec<u8> {
    let row = include_str!("../../../docs/transport/vectors/notify-v1.tsv")
        .lines()
        .find(|s| s.starts_with("record\thello\tshape\t"))
        .unwrap();
    let hex = row.split('\t').nth(4).unwrap();
    assert!(hex.len() <= 2048 && hex.len().is_multiple_of(2));
    let body: Vec<u8> = hex
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let n = |x| match x {
                b'0'..=b'9' => x - b'0',
                b'a'..=b'f' => x - b'a' + 10,
                _ => panic!("fixture hex"),
            };
            n(pair[0]) * 16 + n(pair[1])
        })
        .collect();
    let mut framed = (body.len() as u32).to_be_bytes().to_vec();
    framed.extend(body);
    framed
}
struct BlockingRead {
    inner: Cursor<Vec<u8>>,
    first: bool,
}
impl AsyncRead for BlockingRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if self.first {
            self.first = false;
            std::thread::sleep(Duration::from_millis(4100));
        }
        Pin::new(&mut self.inner).poll_read(cx, bytes)
    }
}
#[tokio::test]
async fn valid_independent_hello_is_owner_visible_data() {
    let result = NotifyOwnerCodec::default()
        .read_request(
            &StreamProtocol::new(PROTOCOL),
            &mut Cursor::new(framed_hello()),
        )
        .await
        .unwrap();
    assert!(matches!(result, NotifyRequest::Record(_)));
}
#[tokio::test]
async fn completed_read_past_four_seconds_is_owner_visible_rejection() {
    let mut input = BlockingRead {
        inner: Cursor::new(framed_hello()),
        first: true,
    };
    let result = NotifyOwnerCodec::default()
        .read_request(&StreamProtocol::new(PROTOCOL), &mut input)
        .await
        .unwrap();
    assert!(matches!(result, NotifyRequest::Rejected));
}
