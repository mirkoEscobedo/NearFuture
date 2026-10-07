use libp2p::request_response::Codec;
use nf_transport::{framing::PeerCodec, records::Lane};
#[tokio::test(flavor = "current_thread")]
async fn declared_frame_quota_precedes_payload_allocation_and_read() {
    let mut c = PeerCodec::new(Lane::Control);
    let p = libp2p::StreamProtocol::new("/nearfuture/peer/control/1");
    let mut input = futures::io::Cursor::new(4097u32.to_be_bytes());
    assert_eq!(
        c.read_request(&p, &mut input).await.unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    let mut input = futures::io::Cursor::new(4096u32.to_be_bytes());
    assert_eq!(
        c.read_request(&p, &mut input).await.unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}
