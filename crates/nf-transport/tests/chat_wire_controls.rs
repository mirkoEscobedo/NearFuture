#[path = "chat_wire_frames_support/mod.rs"]
mod chat_wire_frames_support;
use chat_wire_frames_support::{Fixture, literal_frame};
use futures::{AsyncRead, io::Cursor};
use libp2p::request_response::Codec as _;
use nf_contract::identity::RequestId;
use nf_identity::model::IdentityError;
use nf_store::chat::{ChallengeRequest, ChatStoreError, KnownChatFrontiers, ProofAttempt};
use nf_transport::{
    PeerError,
    chat::{ChatFrame, PROTOCOL, Refusal, WireContext, codec, framing::ChatCodec},
};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

fn refusal() -> ChatFrame {
    ChatFrame::Refused {
        context: WireContext {
            policy_digest: [7; 32],
            request: RequestId::from_bytes([8; 16]),
        },
        reason: Refusal::Offline,
    }
}
fn framed(body: &[u8]) -> Vec<u8> {
    let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
    bytes.extend_from_slice(body);
    bytes
}
fn replaced(body: &[u8], at: usize, replacement: &[u8]) -> Vec<u8> {
    let mut bytes = body.to_vec();
    bytes[at..at + replacement.len()].copy_from_slice(replacement);
    bytes
}

#[test]
fn closed_header_refusal_and_exact_body_eof_reject_malformed_values() {
    let body = literal_frame(&refusal());
    for (reason, code) in [
        (Refusal::Unsupported, 1u8),
        (Refusal::Unauthorized, 2),
        (Refusal::Limit, 3),
        (Refusal::Replay, 4),
        (Refusal::Conflict, 5),
        (Refusal::Offline, 6),
    ] {
        let expected = ChatFrame::Refused {
            context: match refusal() {
                ChatFrame::Refused { context, .. } => context,
                _ => unreachable!(),
            },
            reason,
        };
        let literal = replaced(&body, 64, &[code]);
        assert_eq!(codec::encode(&expected), Ok(literal.clone()));
        assert_eq!(codec::decode(&literal), Ok(expected));
    }
    let mut trailing = body.clone();
    trailing.push(0);
    let cases = [
        ("domain", replaced(&body, 0, b"X"), PeerError::Malformed),
        ("version", replaced(&body, 13, b"2"), PeerError::Malformed),
        ("tag_zero", replaced(&body, 15, &[0]), PeerError::Malformed),
        (
            "tag_unknown",
            replaced(&body, 15, &[6]),
            PeerError::Malformed,
        ),
        (
            "correlation_zero",
            replaced(&body, 48, &[0; 16]),
            PeerError::Malformed,
        ),
        (
            "refusal_zero",
            replaced(&body, 64, &[0]),
            PeerError::Malformed,
        ),
        (
            "refusal_unknown",
            replaced(&body, 64, &[7]),
            PeerError::Malformed,
        ),
        ("body_under_min", body[..64].to_vec(), PeerError::Limit),
        ("body_over_max", vec![0; 4097], PeerError::Limit),
        ("trailing_body", trailing, PeerError::Malformed),
    ];
    for (name, bytes, expected) in cases {
        assert_eq!(codec::decode(&bytes), Err(expected), "{name}");
    }
}

#[test]
fn genuine_maximum_proof_has_independent_length_utf8_policy_and_binding_fences() {
    let fixture = Fixture::new();
    let (receiver, issued, proof, _) = fixture.prepare();
    let frame = ChatFrame::PostProof {
        context: fixture.context(),
        signed: Box::new(fixture.signed.clone()),
        ticket: issued.ticket,
        proof: Box::new(proof),
    };
    let body = literal_frame(&frame);
    let before = fixture.snapshot();
    let known = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    // Literal maximum-frame offsets. The signed message header is18+32+1+32+16+8+2=109 bytes.
    let mut trailing = body.clone();
    trailing.push(0);
    let cases = [
        (
            "signed_under_min",
            replaced(&body, 64, &173u16.to_be_bytes()),
            PeerError::Limit,
        ),
        (
            "signed_over_max",
            replaced(&body, 64, &2222u16.to_be_bytes()),
            PeerError::Limit,
        ),
        (
            "signed_dishonest_length",
            replaced(&body, 64, &2220u16.to_be_bytes()),
            PeerError::Malformed,
        ),
        (
            "message_domain",
            replaced(&body, 66, b"X"),
            PeerError::Malformed,
        ),
        ("channel", replaced(&body, 116, &[2]), PeerError::Malformed),
        (
            "message_id_zero",
            replaced(&body, 149, &[0; 16]),
            PeerError::Malformed,
        ),
        (
            "source_zero",
            replaced(&body, 165, &[0; 8]),
            PeerError::Malformed,
        ),
        (
            "text_empty",
            replaced(&body, 173, &0u16.to_be_bytes()),
            PeerError::Limit,
        ),
        (
            "text_over_max",
            replaced(&body, 173, &2049u16.to_be_bytes()),
            PeerError::Limit,
        ),
        (
            "text_invalid_utf8",
            replaced(&body, 175, &[255]),
            PeerError::Malformed,
        ),
        (
            "policy",
            replaced(&body, 16, &[body[16] ^ 1]),
            PeerError::Policy,
        ),
        (
            "ticket_zero",
            replaced(&body, 2287, &[0; 16]),
            PeerError::Malformed,
        ),
        (
            "proof_scope",
            replaced(&body, 2303, &[body[2303] ^ 1]),
            PeerError::Malformed,
        ),
        (
            "proof_account",
            replaced(&body, 2335, &[body[2335] ^ 1]),
            PeerError::Malformed,
        ),
        (
            "proof_device",
            replaced(&body, 2351, &[body[2351] ^ 1]),
            PeerError::Malformed,
        ),
        (
            "peer_empty",
            replaced(&body, 2375, &0u16.to_be_bytes()),
            PeerError::Limit,
        ),
        (
            "peer_over_max",
            replaced(&body, 2375, &129u16.to_be_bytes()),
            PeerError::Limit,
        ),
        (
            "proof_truncated",
            body[..2600].to_vec(),
            PeerError::Malformed,
        ),
        ("proof_trailing", trailing, PeerError::Malformed),
    ];
    for (name, bytes, expected) in cases {
        assert_eq!(codec::decode(&bytes), Err(expected), "{name}");
        assert_eq!(receiver.known_frontiers(), Ok(known), "{name}");
        assert_eq!(fixture.snapshot(), before, "{name}");
    }
}

struct PrefixOnly {
    prefix: [u8; 4],
    at: usize,
    body_read_attempts: usize,
}
impl AsyncRead for PrefixOnly {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        out: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if self.at == 4 {
            self.body_read_attempts += 1;
            return Poll::Ready(Err(io::Error::other("body trap touched")));
        }
        let length = out.len().min(4 - self.at);
        out[..length].copy_from_slice(&self.prefix[self.at..self.at + length]);
        self.at += length;
        Poll::Ready(Ok(length))
    }
}
#[tokio::test(flavor = "current_thread")]
async fn hard_prefix_limits_reject_before_any_body_read() {
    for length in [64u32, 4097] {
        let mut input = PrefixOnly {
            prefix: length.to_be_bytes(),
            at: 0,
            body_read_attempts: 0,
        };
        assert_eq!(
            ChatCodec
                .read(&mut input)
                .await
                .map_err(|error| error.kind()),
            Err(io::ErrorKind::InvalidData)
        );
        assert_eq!(input.body_read_attempts, 0);
    }
    // Proves no body read. Allocation ordering is separately inspected in framing.rs.
}

#[tokio::test(flavor = "current_thread")]
async fn framed_truncation_trailing_and_exact_eof_are_distinct() {
    let expected = refusal();
    let body = literal_frame(&expected);
    let complete = framed(&body);
    let mut trailing = complete.clone();
    trailing.push(0);
    let mut dishonest = 66u32.to_be_bytes().to_vec();
    dishonest.extend_from_slice(&body);
    let malformed = framed(&replaced(&body, 15, &[255]));
    let cases = [
        ("no_prefix", vec![], io::ErrorKind::UnexpectedEof),
        (
            "prefix_one",
            complete[..1].to_vec(),
            io::ErrorKind::UnexpectedEof,
        ),
        (
            "prefix_three",
            complete[..3].to_vec(),
            io::ErrorKind::UnexpectedEof,
        ),
        (
            "body_short",
            complete[..complete.len() - 1].to_vec(),
            io::ErrorKind::UnexpectedEof,
        ),
        ("dishonest_prefix", dishonest, io::ErrorKind::UnexpectedEof),
        ("stream_trailing", trailing, io::ErrorKind::InvalidData),
        ("malformed_body", malformed, io::ErrorKind::InvalidData),
    ];
    for (name, bytes, expected_error) in cases {
        assert_eq!(
            ChatCodec
                .read(&mut Cursor::new(bytes))
                .await
                .map_err(|error| error.kind()),
            Err(expected_error),
            "{name}"
        );
    }
    assert_eq!(
        ChatCodec.read(&mut Cursor::new(complete)).await.unwrap(),
        expected
    );
}

#[tokio::test(flavor = "current_thread")]
async fn request_response_direction_refuses_before_output_writes() {
    let fixture = Fixture::new();
    let request = ChatFrame::PostChallenge {
        context: fixture.context(),
        signed: Box::new(fixture.signed.clone()),
    };
    let response = refusal();
    let protocol = libp2p::StreamProtocol::new(PROTOCOL);
    let mut codec = ChatCodec;
    assert_eq!(
        codec
            .read_request(
                &protocol,
                &mut Cursor::new(framed(&literal_frame(&response)))
            )
            .await
            .map_err(|error| error.kind()),
        Err(io::ErrorKind::InvalidData)
    );
    assert_eq!(
        codec
            .read_response(
                &protocol,
                &mut Cursor::new(framed(&literal_frame(&request)))
            )
            .await
            .map_err(|error| error.kind()),
        Err(io::ErrorKind::InvalidData)
    );
    assert_eq!(
        codec
            .read_request(
                &protocol,
                &mut Cursor::new(framed(&literal_frame(&request)))
            )
            .await
            .unwrap(),
        request
    );
    assert_eq!(
        codec
            .read_response(
                &protocol,
                &mut Cursor::new(framed(&literal_frame(&response)))
            )
            .await
            .unwrap(),
        response
    );
    let mut matching_request = Cursor::new(Vec::new());
    codec
        .write_request(&protocol, &mut matching_request, request.clone())
        .await
        .unwrap();
    assert_eq!(
        matching_request.into_inner(),
        framed(&literal_frame(&request))
    );
    let mut matching_response = Cursor::new(Vec::new());
    codec
        .write_response(&protocol, &mut matching_response, response.clone())
        .await
        .unwrap();
    assert_eq!(
        matching_response.into_inner(),
        framed(&literal_frame(&response))
    );
    let mut request_output = Cursor::new(Vec::new());
    assert_eq!(
        codec
            .write_request(&protocol, &mut request_output, response)
            .await
            .map_err(|error| error.kind()),
        Err(io::ErrorKind::InvalidData)
    );
    assert_eq!(request_output.into_inner(), Vec::<u8>::new());
    let mut response_output = Cursor::new(Vec::new());
    assert_eq!(
        codec
            .write_response(&protocol, &mut response_output, request)
            .await
            .map_err(|error| error.kind()),
        Err(io::ErrorKind::InvalidData)
    );
    assert_eq!(response_output.into_inner(), Vec::<u8>::new());
}

#[test]
fn valid_shaped_forged_message_and_proof_decode_but_current_store_refuses_authority() {
    let fixture = Fixture::new();
    let (mut receiver, issued, proof, _) = fixture.prepare();
    let mut bad_message = fixture.signed.clone();
    bad_message.signature[0] ^= 1;
    let frame = ChatFrame::PostChallenge {
        context: fixture.context(),
        signed: Box::new(bad_message.clone()),
    };
    assert_eq!(codec::decode(&literal_frame(&frame)), Ok(frame));
    let before = fixture.snapshot();
    let known = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    assert_eq!(
        receiver.issue_challenge(
            ChallengeRequest::Post {
                request: fixture.context().request,
                message: &bad_message
            },
            &proof.peer
        ),
        Err(ChatStoreError::Signature)
    );
    assert_eq!(receiver.known_frontiers(), Ok(known));
    assert_eq!(fixture.snapshot(), before);
    let mut bad_proof = proof;
    bad_proof.signature[0] ^= 1;
    let frame = ChatFrame::PostProof {
        context: fixture.context(),
        signed: Box::new(fixture.signed.clone()),
        ticket: issued.ticket,
        proof: Box::new(bad_proof.clone()),
    };
    assert_eq!(codec::decode(&literal_frame(&frame)), Ok(frame));
    assert_eq!(
        receiver.post(
            fixture.context().request,
            &fixture.signed,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &bad_proof,
                peer: &bad_proof.peer
            }
        ),
        Err(ChatStoreError::Identity(IdentityError::Signature))
    );
    // The genuine one-use ticket is intentionally consumed on a bad attempt; SQL/history is unchanged.
    assert_eq!(
        receiver.post(
            fixture.context().request,
            &fixture.signed,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &bad_proof,
                peer: &bad_proof.peer
            }
        ),
        Err(ChatStoreError::Replay)
    );
    assert_eq!(receiver.known_frontiers(), Ok(known));
    assert_eq!(fixture.snapshot(), before);
}

#[test]
fn every_frame_tag_has_exact_eof_and_variant_specific_shape_fences() {
    let fixture = Fixture::new();
    let (receiver, issued, proof, receipt) = fixture.prepare();
    let context = fixture.context();
    let before = fixture.snapshot();
    let known = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    let frames = [
        ChatFrame::PostChallenge {
            context,
            signed: Box::new(fixture.signed.clone()),
        },
        ChatFrame::PostProof {
            context,
            signed: Box::new(fixture.signed.clone()),
            ticket: issued.ticket,
            proof: Box::new(proof),
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
    for frame in &frames {
        let body = literal_frame(frame);
        let truncated_error = if body.len() == 65 {
            PeerError::Limit
        } else {
            PeerError::Malformed
        };
        assert_eq!(codec::decode(&body[..body.len() - 1]), Err(truncated_error));
        let mut trailing = body;
        trailing.push(0);
        assert_eq!(codec::decode(&trailing), Err(PeerError::Malformed));
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(fixture.snapshot(), before);
    }
    let issued = literal_frame(&frames[2]);
    assert_eq!(
        codec::decode(&replaced(&issued, 64, &[0; 16])),
        Err(PeerError::Malformed)
    );
    let receipt = literal_frame(&frames[3]);
    assert_eq!(
        codec::decode(&replaced(&receipt, 64, b"X")),
        Err(PeerError::Malformed)
    );
    assert_eq!(
        codec::decode(&replaced(&receipt, 16, &[receipt[16] ^ 1])),
        Err(PeerError::Policy)
    );
    let challenge = literal_frame(&frames[0]);
    assert_eq!(
        codec::decode(&replaced(&challenge, 16, &[challenge[16] ^ 1])),
        Err(PeerError::Policy)
    );
}
