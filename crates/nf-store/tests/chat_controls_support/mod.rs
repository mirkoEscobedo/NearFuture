use super::chat_support::{Fixture, author};
use nf_contract::identity::RequestId;
use nf_identity::{
    keys::generate_identity,
    model::{DeviceProof, Invitation, MembershipState, Roles, Scope},
    private_storage::LocalIdentity,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::chat::{
    ChallengeRequest, ChatReceipt, ChatStore, HistoryPage, HistoryQuery, IssuedChallenge,
    ProofAttempt, Result, SignedMessage, codec,
};
use std::{collections::BTreeMap, fs};

pub(super) fn identity(peer: &[u8]) -> LocalIdentity {
    let (public, account_key, device_key) = generate_identity(peer.to_vec()).unwrap();
    LocalIdentity {
        public,
        account_key,
        device_key,
    }
}
pub(super) fn worker(fixture: &Fixture) -> (LocalIdentity, MembershipState) {
    let worker = identity(b"fixture-chat-controls-worker");
    let invitation = Invitation {
        scope: fixture.policy.scope,
        id: [63; 16],
        issuer: fixture.alice.public.account,
        recipient: worker.public.clone(),
        roles: Roles::WORKER,
        expires_at: 100,
        issued_revision: fixture.membership.revision,
        reusable: false,
    };
    let admission = admission_proof(&invitation, &worker.account_key, &worker.device_key).unwrap();
    let invitation = sign_invitation(invitation, &fixture.alice.account_key).unwrap();
    let membership = fixture
        .membership
        .redeem(&invitation, &admission, 1)
        .unwrap();
    assert_eq!(membership.revision, 2);
    (worker, membership)
}
pub(super) fn signed(
    fixture: &Fixture,
    actor: &LocalIdentity,
    message: u8,
    sequence: u64,
    text: &str,
) -> SignedMessage {
    let mut value = fixture.signed_message().message;
    value.author = author(actor);
    value.message = [message; 16];
    value.sequence = sequence;
    value.text = text.to_owned();
    let signature = actor
        .device_key
        .sign(&codec::message_digest(&value).unwrap());
    SignedMessage {
        message: value,
        signature,
    }
}
pub(super) fn proof(actor: &LocalIdentity, scope: Scope, issued: IssuedChallenge) -> DeviceProof {
    let mut proof = DeviceProof {
        scope,
        account: actor.public.account,
        device: actor.public.device,
        frontier: issued.membership_revision,
        peer: actor.public.peer.clone(),
        challenge: issued.challenge,
        signature: [0; 64],
    };
    proof.signature = actor.device_key.sign(&device_digest(&proof).unwrap());
    proof
}
pub(super) fn query(
    actor: &LocalIdentity,
    request: u8,
    after_cursor: u64,
    limit: u16,
) -> HistoryQuery {
    HistoryQuery {
        request: RequestId::from_bytes([request; 16]),
        reader: author(actor),
        channel: nf_store::chat::Channel::General,
        after_cursor,
        limit,
    }
}
pub(super) fn history(
    fixture: &Fixture,
    store: &mut ChatStore,
    actor: &LocalIdentity,
    query: &HistoryQuery,
) -> Result<HistoryPage> {
    let issued = store.issue_challenge(ChallengeRequest::History(query), &actor.public.peer)?;
    let proof = proof(actor, fixture.policy.scope, issued);
    store.history(
        query,
        ProofAttempt {
            ticket: issued.ticket,
            proof: &proof,
            peer: &actor.public.peer,
        },
    )
}
pub(super) fn post(
    fixture: &Fixture,
    store: &mut ChatStore,
    actor: &LocalIdentity,
    request: u8,
    signed: &SignedMessage,
) -> Result<ChatReceipt> {
    let request = RequestId::from_bytes([request; 16]);
    let issued = store.issue_challenge(
        ChallengeRequest::Post {
            request,
            message: signed,
        },
        &actor.public.peer,
    )?;
    let proof = proof(actor, fixture.policy.scope, issued);
    store.post(
        request,
        signed,
        ProofAttempt {
            ticket: issued.ticket,
            proof: &proof,
            peer: &actor.public.peer,
        },
    )
}
pub(super) fn issue_post(
    store: &mut ChatStore,
    actor: &LocalIdentity,
    request: u8,
    message: &SignedMessage,
) -> Result<IssuedChallenge> {
    store.issue_challenge(
        ChallengeRequest::Post {
            request: RequestId::from_bytes([request; 16]),
            message,
        },
        &actor.public.peer,
    )
}
#[derive(PartialEq, Eq)]
pub(super) struct Snapshot(BTreeMap<String, Vec<u8>>);
impl Snapshot {
    pub(super) fn take(fixture: &Fixture) -> Self {
        let entries = fs::read_dir(&fixture.scratch.0)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                assert!(entry.file_type().unwrap().is_file());
                (
                    entry.file_name().into_string().unwrap(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect();
        Self(entries)
    }
    pub(super) fn assert_unchanged(&self, fixture: &Fixture) {
        assert!(
            *self == Self::take(fixture),
            "CHAT_REFUSAL_CHANGED_DATABASE_OR_SIDECARS"
        );
    }
}
