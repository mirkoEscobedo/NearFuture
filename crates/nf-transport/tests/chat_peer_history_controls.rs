mod chat_peer_history_controls_support;
#[path = "chat_peer_delivery_support/mod.rs"]
mod original_fixture;
use chat_peer_history_controls_support::{
    BadPage, Prepared, RawPeer, context, exchange, issued, literal, malicious, proof, proof_frame,
    query, ready, unknown,
};
use futures::{AsyncRead, io::Cursor};
use libp2p::request_response::Codec as _;
use nf_identity::signing::device_digest;
use nf_store::chat::{HistoryEntry, HistoryPage, IssuedChallenge, codec};
use nf_transport::{
    PeerError,
    chat::{
        Refusal,
        history::{self, HistoryCodec, HistoryFrame},
    },
};
use original_fixture::Fixture;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};
const CASE_AGE: Duration = Duration::from_secs(30);
fn replace(bytes: &[u8], at: usize, with: &[u8]) -> Vec<u8> {
    let mut b = bytes.to_vec();
    b[at..at + with.len()].copy_from_slice(with);
    b
}
fn framed(body: &[u8]) -> Vec<u8> {
    let mut b = (body.len() as u32).to_be_bytes().to_vec();
    b.extend_from_slice(body);
    b
}
fn closed(f: &Fixture) -> HistoryFrame {
    let q = query(f);
    HistoryFrame::Refused {
        context: context(f, q),
        query: q,
        reason: Refusal::Offline,
    }
}
#[test]
fn history_closed_profile_has_independent_all_tag_lengths_refusals_and_exact_eof() {
    let f = Fixture::new();
    let q = query(&f);
    let c = context(&f, q);
    let issued = IssuedChallenge {
        ticket: [1; 16],
        challenge: [2; 32],
        membership_revision: 1,
    };
    let mut proof = proof(&f.alice, f.policy.scope, issued);
    proof.peer = vec![9; 128];
    proof.signature = f.alice.device_key.sign(&device_digest(&proof).unwrap());
    let frames = [
        HistoryFrame::Challenge {
            context: c,
            query: q,
        },
        HistoryFrame::Proof {
            context: c,
            query: q,
            ticket: issued.ticket,
            proof: Box::new(proof),
        },
        HistoryFrame::Issued {
            context: c,
            query: q,
            challenge: issued,
        },
        HistoryFrame::Page {
            context: c,
            query: q,
            page: Box::new(HistoryPage {
                entries: vec![],
                next_cursor: 0,
            }),
        },
        closed(&f),
    ];
    for (frame, length) in frames.into_iter().zip([115, 429, 171, 124, 116]) {
        let b = literal(&frame);
        assert_eq!(b.len(), length);
        assert_eq!(history::encode(&frame), Ok(b.clone()));
        assert_eq!(history::decode(&b), Ok(frame));
        for n in 0..b.len() {
            assert!(history::decode(&b[..n]).is_err(), "truncation {n}/{length}");
        }
        let mut extra = b;
        extra.push(0);
        assert_eq!(history::decode(&extra), Err(PeerError::Malformed));
    }
    let b = literal(&closed(&f));
    for (reason, code) in [
        (Refusal::Unsupported, 1),
        (Refusal::Unauthorized, 2),
        (Refusal::Limit, 3),
        (Refusal::Replay, 4),
        (Refusal::Conflict, 5),
        (Refusal::Offline, 6),
    ] {
        let expected = HistoryFrame::Refused {
            context: c,
            query: q,
            reason,
        };
        let bytes = replace(&b, 115, &[code]);
        assert_eq!(history::encode(&expected), Ok(bytes.clone()));
        assert_eq!(history::decode(&bytes), Ok(expected));
    }
    for (bytes, error) in [
        (replace(&b, 0, b"X"), PeerError::Malformed),
        (replace(&b, 21, b"2"), PeerError::Malformed),
        (replace(&b, 23, &[0]), PeerError::Malformed),
        (replace(&b, 23, &[6]), PeerError::Malformed),
        (replace(&b, 56, &[0; 16]), PeerError::Malformed),
        (replace(&b, 104, &[2]), PeerError::Malformed),
        (replace(&b, 115, &[0]), PeerError::Malformed),
        (replace(&b, 115, &[7]), PeerError::Malformed),
        (vec![0; 114], PeerError::Limit),
        (vec![0; 4097], PeerError::Limit),
    ] {
        assert_eq!(history::decode(&bytes), Err(error));
    }
    for n in [0u16, 2, 64] {
        let mut bad = q;
        bad.limit = n;
        assert_eq!(
            history::encode(&HistoryFrame::Challenge {
                context: c,
                query: bad
            }),
            Err(PeerError::Limit)
        );
        assert_eq!(
            history::decode(&replace(&b, 113, &n.to_be_bytes())),
            Err(PeerError::Limit)
        );
    }
}
#[test]
fn maximum_history_page_is_literal_2355_bytes_without_skipping_or_unauthorized_signature_claim() {
    let f = Fixture::new();
    let q = query(&f);
    let mut signed = f.signed.clone();
    signed.message.text = "x".repeat(2048);
    signed.signature = f
        .alice
        .device_key
        .sign(&codec::message_digest(&signed.message).unwrap());
    let page = HistoryPage {
        entries: vec![HistoryEntry {
            receiver_cursor: 1,
            signed: signed.clone(),
        }],
        next_cursor: 1,
    };
    let frame = HistoryFrame::Page {
        context: context(&f, q),
        query: q,
        page: Box::new(page.clone()),
    };
    let b = literal(&frame);
    assert_eq!(b.len(), 2355);
    assert_eq!(history::encode(&frame), Ok(b.clone()));
    assert_eq!(history::decode(&b), Ok(frame.clone()));
    assert_eq!(
        history::decode(&replace(&b, 123, &[2])),
        Err(PeerError::Limit)
    );
    assert_eq!(
        history::decode(&replace(&b, 132, &173u16.to_be_bytes())),
        Err(PeerError::Limit)
    );
    assert_eq!(
        history::decode(&replace(&b, 132, &2222u16.to_be_bytes())),
        Err(PeerError::Limit)
    );
    assert_eq!(
        history::decode(&replace(&b, 243, &[255])),
        Err(PeerError::Malformed)
    );
    assert_eq!(
        history::decode(&replace(&b, 124, &0u64.to_be_bytes())),
        Err(PeerError::Malformed)
    );
    assert_eq!(
        history::decode(&replace(&b, 115, &2u64.to_be_bytes())),
        Err(PeerError::Malformed)
    );
    let mut too_many = page.clone();
    too_many.entries.push(too_many.entries[0].clone());
    assert_eq!(history::validate_page(q, &too_many), Err(PeerError::Limit));
    let mut oversized = signed;
    oversized.message.text.push('x');
    assert_eq!(
        history::encode(&HistoryFrame::Page {
            context: context(&f, q),
            query: q,
            page: Box::new(HistoryPage {
                entries: vec![HistoryEntry {
                    receiver_cursor: 1,
                    signed: oversized
                }],
                next_cursor: 1
            })
        }),
        Err(PeerError::Limit)
    );
    let mut foreign = page;
    foreign.entries[0].signed.message.scope.history =
        nf_contract::identity::HistoryId::from_bytes([99; 16]);
    assert_eq!(
        history::encode(&HistoryFrame::Page {
            context: context(&f, q),
            query: q,
            page: Box::new(foreign)
        }),
        Err(PeerError::Policy)
    );
    let mut forged = b.clone();
    *forged.last_mut().unwrap() ^= 1;
    assert!(
        history::decode(&forged).is_ok(),
        "shape decoding alone grants no author signature authority"
    );
    let mut padded = b;
    padded.resize(4096, 0);
    assert_eq!(history::decode(&padded), Err(PeerError::Malformed));
}
struct PrefixOnly {
    prefix: [u8; 4],
    at: usize,
    body_reads: usize,
}
impl AsyncRead for PrefixOnly {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        out: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if self.at == 4 {
            self.body_reads += 1;
            return Poll::Ready(Err(io::Error::other("body must not be read")));
        }
        let n = (4 - self.at).min(out.len());
        out[..n].copy_from_slice(&self.prefix[self.at..self.at + n]);
        self.at += n;
        Poll::Ready(Ok(n))
    }
}
#[tokio::test(flavor = "current_thread")]
async fn history_framing_bounds_trap_body_reads_and_directions_before_any_write() {
    let f = Fixture::new();
    let response = closed(&f);
    let q = query(&f);
    let request = HistoryFrame::Challenge {
        context: context(&f, q),
        query: q,
    };
    let mut codec = HistoryCodec;
    let protocol = libp2p::StreamProtocol::new(history::PROTOCOL);
    for n in [114u32, 4097] {
        let mut io = PrefixOnly {
            prefix: n.to_be_bytes(),
            at: 0,
            body_reads: 0,
        };
        assert_eq!(
            codec.read(&mut io).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(io.body_reads, 0);
    }
    let body = literal(&response);
    assert_eq!(
        codec.read(&mut Cursor::new(framed(&body))).await.unwrap(),
        response
    );
    let truncated = framed(&body);
    assert_eq!(
        codec
            .read(&mut Cursor::new(truncated[..truncated.len() - 1].to_vec()))
            .await
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
    let mut trailing = framed(&body);
    trailing.push(0);
    assert_eq!(
        codec
            .read(&mut Cursor::new(trailing))
            .await
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    let mut output = Cursor::new(Vec::<u8>::new());
    assert_eq!(
        codec
            .write_request(&protocol, &mut output, response.clone())
            .await
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert!(output.into_inner().is_empty());
    let mut output = Cursor::new(Vec::<u8>::new());
    assert_eq!(
        codec
            .write_response(&protocol, &mut output, request.clone())
            .await
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert!(output.into_inner().is_empty());
    assert_eq!(
        codec
            .read_request(&protocol, &mut Cursor::new(framed(&body)))
            .await
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        codec
            .read_response(&protocol, &mut Cursor::new(framed(&literal(&request))))
            .await
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    let invalid_limit = replace(&body, 113, &0u16.to_be_bytes());
    assert_eq!(
        codec
            .read(&mut Cursor::new(framed(&invalid_limit)))
            .await
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}
#[tokio::test(flavor = "current_thread")]
async fn unknown_and_currently_revoked_actual_noise_readers_never_get_a_history_ticket() {
    for revoked in [false, true] {
        let f = Fixture::new();
        let mut state = Prepared::new(&f, revoked);
        tokio::time::timeout(CASE_AGE, async {
            let mut server = state.server(&f);
            let address = ready(&mut server, &f).await;
            let (vault, reader) = if revoked {
                (None, None)
            } else {
                let (v, r) = unknown(&f);
                (Some(v), Some(r))
            };
            let mut q = query(&f);
            if let Some(r) = reader {
                q.reader = r;
            }
            let mut raw = RawPeer::new(vault.as_ref().unwrap_or(&f.client_vault));
            let (connection, _) = raw.connect(&mut server, address, f.server_peer).await;
            assert_eq!(
                exchange(
                    &mut raw,
                    &mut server,
                    f.server_peer,
                    connection,
                    HistoryFrame::Challenge {
                        context: context(&f, q),
                        query: q
                    }
                )
                .await,
                HistoryFrame::Refused {
                    context: context(&f, q),
                    query: q,
                    reason: Refusal::Unauthorized
                }
            );
            drop(raw);
            state.finish(server.into_store(), &f);
        })
        .await
        .expect("bounded genuine unknown/revoked reader refusal");
    }
}
#[tokio::test(flavor = "current_thread")]
async fn changed_request_cursor_foreign_peer_or_forged_proof_consumes_only_its_original_admission()
{
    for bad in 0..4 {
        let f = Fixture::new();
        let mut state = Prepared::new(&f, false);
        tokio::time::timeout(CASE_AGE, async {
            let mut server = state.server(&f);
            let address = ready(&mut server, &f).await;
            let mut raw = RawPeer::new(&f.client_vault);
            let (connection, _) = raw.connect(&mut server, address, f.server_peer).await;
            let q = query(&f);
            let challenge = issued(&mut raw, &mut server, &f, connection, q).await;
            let mut altered = q;
            if bad == 0 {
                altered.request = nf_contract::identity::RequestId::from_bytes([112; 16]);
            }
            if bad == 1 {
                altered.after_cursor = 1;
            }
            let mut frame = proof_frame(&f, altered, challenge);
            if let HistoryFrame::Proof { proof, .. } = &mut frame {
                if bad == 2 {
                    proof.peer = f.bob.public.peer.clone();
                    proof.signature = f.alice.device_key.sign(&device_digest(proof).unwrap());
                    assert_eq!(
                        nf_contract::signatures::verify_digest(
                            &f.alice.public.device_key,
                            &device_digest(proof).unwrap(),
                            &proof.signature
                        ),
                        Ok(())
                    );
                }
                if bad == 3 {
                    proof.signature[0] ^= 1;
                }
            }
            assert!(history::decode(&literal(&frame)).is_ok());
            assert_eq!(
                exchange(&mut raw, &mut server, f.server_peer, connection, frame).await,
                HistoryFrame::Refused {
                    context: context(&f, altered),
                    query: altered,
                    reason: Refusal::Unauthorized
                }
            );
            assert_eq!(
                exchange(
                    &mut raw,
                    &mut server,
                    f.server_peer,
                    connection,
                    proof_frame(&f, q, challenge)
                )
                .await,
                HistoryFrame::Refused {
                    context: context(&f, q),
                    query: q,
                    reason: Refusal::Replay
                }
            );
            drop(raw);
            state.finish(server.into_store(), &f);
        })
        .await
        .expect("bounded genuine current proof/query refusal");
    }
}
#[tokio::test(flavor = "current_thread")]
async fn a_consumed_actual_history_ticket_replays_without_sql_frontier_or_pending_changes() {
    let f = Fixture::new();
    let mut state = Prepared::new(&f, false);
    tokio::time::timeout(CASE_AGE, async {
        let mut server = state.server(&f);
        let address = ready(&mut server, &f).await;
        let mut raw = RawPeer::new(&f.client_vault);
        let (connection, _) = raw.connect(&mut server, address, f.server_peer).await;
        let q = query(&f);
        let started = Instant::now();
        let challenge = issued(&mut raw, &mut server, &f, connection, q).await;
        let frame = proof_frame(&f, q, challenge);
        assert_eq!(
            exchange(
                &mut raw,
                &mut server,
                f.server_peer,
                connection,
                frame.clone()
            )
            .await,
            HistoryFrame::Page {
                context: context(&f, q),
                query: q,
                page: Box::new(HistoryPage {
                    entries: vec![],
                    next_cursor: 0
                })
            }
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "replay setup retains actual original deadline"
        );
        assert_eq!(
            exchange(&mut raw, &mut server, f.server_peer, connection, frame).await,
            HistoryFrame::Refused {
                context: context(&f, q),
                query: q,
                reason: Refusal::Replay
            }
        );
        drop(raw);
        state.finish(server.into_store(), &f);
    })
    .await
    .expect("bounded genuine consumed history ticket");
}
#[tokio::test(flavor = "current_thread")]
async fn history_tickets_cannot_cross_actual_reconnect_or_outlive_the_original_five_seconds() {
    for reconnect in [true, false] {
        let f = Fixture::new();
        let mut state = Prepared::new(&f, false);
        tokio::time::timeout(CASE_AGE, async {
            let mut server = state.server(&f);
            let address = ready(&mut server, &f).await;
            let mut raw = RawPeer::new(&f.client_vault);
            let before = raw
                .connect(&mut server, address.clone(), f.server_peer)
                .await;
            let q = query(&f);
            let started = Instant::now();
            let challenge = issued(&mut raw, &mut server, &f, before.0, q).await;
            let connection = if reconnect {
                raw.disconnect(&mut server, f.server_peer).await;
                let after = raw.connect(&mut server, address, f.server_peer).await;
                assert_ne!(before.0, after.0);
                assert_ne!(before.1, after.1);
                assert!(
                    started.elapsed() < Duration::from_secs(5),
                    "connection-refusal setup retains nonexpired ticket"
                );
                after.0
            } else {
                tokio::time::sleep(Duration::from_millis(5050)).await;
                before.0
            };
            assert_eq!(
                exchange(
                    &mut raw,
                    &mut server,
                    f.server_peer,
                    connection,
                    proof_frame(&f, q, challenge)
                )
                .await,
                HistoryFrame::Refused {
                    context: context(&f, q),
                    query: q,
                    reason: Refusal::Replay
                }
            );
            drop(raw);
            state.finish(server.into_store(), &f);
        })
        .await
        .expect("bounded genuine reconnect/expiry history admission");
    }
}
#[tokio::test(flavor = "current_thread")]
async fn malicious_history_context_query_and_signed_page_are_refused_without_state_changes() {
    for bad in [BadPage::Request, BadPage::Cursor, BadPage::Signature] {
        let f = Fixture::new();
        let mut state = Prepared::new(&f, false);
        tokio::time::timeout(CASE_AGE, async {
            assert_eq!(
                malicious(&mut state, &f, bad).await,
                Err(if matches!(bad, BadPage::Signature) {
                    PeerError::Unauthorized
                } else {
                    PeerError::Session
                })
            );
            let receiver = state.receiver.take().unwrap();
            state.finish(receiver, &f);
        })
        .await
        .expect("bounded explicitly adversarial physical history page");
    }
}
