use crate::fixture::{Fixture, author};
use nf_contract::identity::RequestId;
use nf_identity::{model::DeviceProof, signing::device_digest};
use nf_store::chat::{ChallengeRequest, ChatStore, ProofAttempt, ReceiverClock, codec};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Clock(AtomicU64);
impl ReceiverClock for Clock {
    fn now(&self) -> Duration {
        Duration::from_secs(self.0.load(Ordering::SeqCst))
    }
}
pub fn populate(
    store: &mut ChatStore,
    fixture: &Fixture,
    clock: &Clock,
    from: u64,
    to: u64,
    whole: Instant,
) {
    for sequence in from + 1..=to {
        let mut signed = fixture.signed.clone();
        signed.message.message = [0; 16];
        signed.message.message[..8].copy_from_slice(&sequence.to_be_bytes());
        signed.message.message[15] = 81;
        signed.message.sequence = sequence;
        signed.message.text = format!("retained public qualification {sequence}");
        signed.signature = fixture
            .alice
            .device_key
            .sign(&codec::message_digest(&signed.message).unwrap());
        for alias in [false, true] {
            // Advance the trusted receiver clock, preserving its default ten-token
            // bucket and two-second interval; neither cap nor runtime policy changes.
            clock.0.fetch_add(2, Ordering::SeqCst);
            let mut request = [0; 16];
            request[..8].copy_from_slice(&sequence.to_be_bytes());
            request[15] = if alias { 102 } else { 101 };
            let request = RequestId::from_bytes(request);
            let issued = store
                .issue_challenge(
                    ChallengeRequest::Post {
                        request,
                        message: &signed,
                    },
                    &fixture.alice.public.peer,
                )
                .unwrap();
            let mut proof = DeviceProof {
                scope: fixture.policy.scope,
                account: fixture.alice.public.account,
                device: fixture.alice.public.device,
                frontier: issued.membership_revision,
                peer: fixture.alice.public.peer.clone(),
                challenge: issued.challenge,
                signature: [0; 64],
            };
            proof.signature = fixture
                .alice
                .device_key
                .sign(&device_digest(&proof).unwrap());
            let receipt = store
                .post(
                    request,
                    &signed,
                    ProofAttempt {
                        ticket: issued.ticket,
                        proof: &proof,
                        peer: &fixture.alice.public.peer,
                    },
                )
                .unwrap();
            assert_eq!(receipt.receiver_cursor, sequence);
            assert_eq!(receipt.author, author(&fixture.alice));
            assert_eq!(receipt.original_request.as_bytes()[15], 101);
            assert!(
                whole.elapsed() < Duration::from_secs(90),
                "bounded public fixture preparation"
            );
        }
    }
}
