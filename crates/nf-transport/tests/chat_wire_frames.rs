mod chat_wire_frames_support;
use chat_wire_frames_support::{Fixture, literal_frame};
use futures::io::Cursor;
use nf_store::chat::KnownChatFrontiers;
use nf_transport::chat::{ChatFrame, MAX_FRAME_BYTES, Refusal, codec, framing::ChatCodec};

#[tokio::test(flavor = "current_thread")]
async fn maximum_authentic_post_proof_and_receiver_receipt_have_exact_bounded_canonical_frames() {
    let fixture = Fixture::new();
    let (receiver, issued, proof, receipt) = fixture.prepare();
    let known = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    let before = fixture.snapshot();
    let context = fixture.context();
    assert_eq!(fixture.signed.message.text.len(), 2048);
    assert_eq!(proof.peer.len(), 128);
    let frames = [
        ChatFrame::PostProof {
            context,
            signed: Box::new(fixture.signed.clone()),
            ticket: issued.ticket,
            proof: Box::new(proof),
        },
        ChatFrame::PostChallenge {
            context,
            signed: Box::new(fixture.signed.clone()),
        },
        ChatFrame::Issued {
            context,
            challenge: issued,
        },
        ChatFrame::Delivered {
            context,
            signed: Box::new(receipt),
        },
        ChatFrame::Refused {
            context,
            reason: Refusal::Offline,
        },
    ];
    let sizes = [2601usize, 2287, 120, 387, 65];
    for (frame, size) in frames.iter().zip(sizes) {
        let body = literal_frame(frame);
        assert_eq!(body.len(), size);
        assert!(body.len() <= MAX_FRAME_BYTES);
        assert_eq!(codec::encode(frame), Ok(body.clone()));
        // Intended first RED: validated authentic2601-byte PostProof returns Unsupported.
        assert_eq!(codec::decode(&body), Ok(frame.clone()));
        let mut input = (body.len() as u32).to_be_bytes().to_vec();
        input.extend_from_slice(&body);
        assert_eq!(
            ChatCodec
                .read(&mut Cursor::new(input.clone()))
                .await
                .unwrap(),
            frame.clone()
        );
        let mut output = Cursor::new(Vec::new());
        ChatCodec.write(&mut output, frame).await.unwrap();
        assert_eq!(output.into_inner(), input);
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(fixture.snapshot(), before);
    }
}

#[test]
fn transient_rate_limited_wire2_is_bounded_and_old_domain_is_unsupported() {
    use nf_contract::identity::RequestId;
    use nf_transport::chat::WireContext;
    let context = WireContext {
        policy_digest: [4; 32],
        request: RequestId::from_bytes([9; 16]),
    };
    let frame = ChatFrame::RateLimited {
        context,
        retry_after_ms: 2_000,
    };
    let mut expected = b"NF-CHAT-WIRE-2\0".to_vec();
    expected.push(6);
    expected.extend_from_slice(&[4; 32]);
    expected.extend_from_slice(&[9; 16]);
    expected.extend_from_slice(&2_000u32.to_be_bytes());
    assert_eq!(expected.len(), 68);
    assert_eq!(codec::encode(&frame), Ok(expected.clone()));
    assert_eq!(codec::decode(&expected), Ok(frame.clone()));
    let mut old = expected.clone();
    old[13] = b'1';
    assert_eq!(
        codec::decode(&old),
        Err(nf_transport::PeerError::Unsupported)
    );
    assert!(codec::decode(&expected[..67]).is_err());
    let mut extra = expected.clone();
    extra.push(0);
    assert!(codec::decode(&extra).is_err());
    for delay in [0u32, 86_400_001] {
        let mut invalid = expected.clone();
        invalid[64..].copy_from_slice(&delay.to_be_bytes());
        assert!(codec::decode(&invalid).is_err());
        assert!(
            codec::encode(&ChatFrame::RateLimited {
                context,
                retry_after_ms: delay
            })
            .is_err()
        );
    }
}
