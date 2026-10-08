use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::{generate_identity, random_id},
    model::{DeviceProof, Invitation, MembershipState, Roles, Scope},
    private_storage::LocalIdentity,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::chat::{
    Author, ChallengeRequest, Channel, ChatMessage, ChatPolicy, ChatReceipt, ChatStore,
    HistoryEntry, HistoryPage, HistoryQuery, IssuedChallenge, KnownChatFrontiers, ProofAttempt,
    SignedMessage,
    outbox::{ChatDeliveryReceipt, OutboxProfile, SignedChatReceipt},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub struct Fixture {
    pub profile: OutboxProfile,
    pub signed: SignedMessage,
    alice: LocalIdentity,
    bob: LocalIdentity,
    membership: MembershipState,
    policy: ChatPolicy,
    directory: PathBuf,
    parent: PathBuf,
}
impl Fixture {
    pub fn new() -> Self {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
        fs::create_dir_all(&parent).unwrap();
        let parent = fs::canonicalize(parent).unwrap();
        let suffix: String = random_id()
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let directory = parent.join(format!("chat-outbox-receipt-{suffix}"));
        fs::create_dir(&directory).unwrap();
        let alice = identity(b"outbox-receipt-alice".to_vec());
        let bob = identity(b"outbox-receipt-bob".to_vec());
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let founder = MembershipState::bootstrap(scope, &alice.public).unwrap();
        let invitation = Invitation {
            scope,
            id: [3; 16],
            issuer: alice.public.account,
            recipient: bob.public.clone(),
            roles: Roles::PLAYER,
            expires_at: 100,
            issued_revision: 0,
            reusable: false,
        };
        let admission = admission_proof(&invitation, &bob.account_key, &bob.device_key).unwrap();
        let signed = sign_invitation(invitation, &alice.account_key).unwrap();
        let membership = founder.redeem(&signed, &admission, 1).unwrap();
        let policy = ChatPolicy { scope };
        let profile =
            OutboxProfile::from_current(&policy, &membership, author(&alice), author(&bob))
                .unwrap();
        let message = ChatMessage {
            scope,
            channel: Channel::General,
            author: author(&alice),
            message: [81; 16],
            sequence: 1,
            text: "public outbox fixture hello".to_owned(),
        };
        let signature = alice.device_key.sign(&hash(&message_frame(&message)));
        Self {
            profile,
            signed: SignedMessage { message, signature },
            alice,
            bob,
            membership,
            policy,
            directory,
            parent,
        }
    }
    pub fn outbox_database(&self) -> PathBuf {
        self.directory.join("outbox.sqlite")
    }
    pub fn outbox_snapshot(&self) -> Vec<u8> {
        fs::read(self.outbox_database()).unwrap()
    }
    pub fn receiver_snapshot(&self) -> Vec<u8> {
        fs::read(self.directory.join("receiver.sqlite")).unwrap()
    }
    pub fn receiver_commit_reopen_and_issue_receipt(&self) -> SignedChatReceipt {
        let path = self.directory.join("receiver.sqlite");
        let mut receiver = ChatStore::create(&path, &self.policy, &self.membership).unwrap();
        assert_eq!(
            receiver.known_frontiers(),
            Ok(KnownChatFrontiers {
                scope: self.policy.scope,
                revision: 0,
                membership_revision: 1
            })
        );
        let request = RequestId::from_bytes([101; 16]);
        let issued = receiver
            .issue_challenge(
                ChallengeRequest::Post {
                    request,
                    message: &self.signed,
                },
                &self.alice.public.peer,
            )
            .unwrap();
        let sender_proof = proof(&self.alice, self.policy.scope, issued);
        let expected = ChatReceipt {
            message: [81; 16],
            author: author(&self.alice),
            source_sequence: 1,
            receiver_cursor: 1,
            original_request: request,
        };
        // Unsigned ChatReceipt establishes the actual existing receiver commit only.
        assert_eq!(
            receiver.post(
                request,
                &self.signed,
                ProofAttempt {
                    ticket: issued.ticket,
                    proof: &sender_proof,
                    peer: &self.alice.public.peer
                }
            ),
            Ok(expected)
        );
        let known = KnownChatFrontiers {
            scope: self.policy.scope,
            revision: 1,
            membership_revision: 1,
        };
        assert_eq!(receiver.known_frontiers(), Ok(known));
        drop(receiver);
        let mut receiver = ChatStore::open_existing(&path, &self.policy, known).unwrap();
        let query = HistoryQuery {
            request: RequestId::from_bytes([104; 16]),
            reader: author(&self.bob),
            channel: Channel::General,
            after_cursor: 0,
            limit: 1,
        };
        let issued = receiver
            .issue_challenge(ChallengeRequest::History(&query), &self.bob.public.peer)
            .unwrap();
        let receiver_proof = proof(&self.bob, self.policy.scope, issued);
        assert_eq!(
            receiver.history(
                &query,
                ProofAttempt {
                    ticket: issued.ticket,
                    proof: &receiver_proof,
                    peer: &self.bob.public.peer
                }
            ),
            Ok(HistoryPage {
                entries: vec![HistoryEntry {
                    receiver_cursor: 1,
                    signed: self.signed.clone()
                }],
                next_cursor: 1
            })
        );
        assert_eq!(receiver.known_frontiers(), Ok(known));
        drop(receiver);
        // Independent local receiver signing service, after real commit/reopen/current PLAYER proof.
        // No receiver key supplied by an incoming receipt; the same full peer/key is already retained.
        let mut policy = b"NF-CHAT-POLICY-1\0".to_vec();
        policy.extend_from_slice(self.policy.scope.universe.as_bytes());
        policy.extend_from_slice(self.policy.scope.history.as_bytes());
        policy.push(1);
        policy.extend_from_slice(&2048u16.to_be_bytes());
        policy.extend_from_slice(&4096u16.to_be_bytes());
        policy.extend_from_slice(&16384u16.to_be_bytes());
        policy.extend_from_slice(&64u16.to_be_bytes());
        let mut peer = b"NF-CHAT-RECEIVER-PEER-1\0".to_vec();
        peer.extend_from_slice(&(self.bob.public.peer.len() as u16).to_be_bytes());
        peer.extend_from_slice(&self.bob.public.peer);
        let mut original = b"NF-CHAT-SIGNED-MESSAGE-1\0".to_vec();
        original.extend_from_slice(&message_frame(&self.signed.message));
        original.extend_from_slice(&self.signed.signature);
        let receipt = ChatDeliveryReceipt {
            policy_digest: hash(&policy),
            scope: self.policy.scope,
            channel: Channel::General,
            receiver: author(&self.bob),
            receiver_peer_digest: hash(&peer),
            original: expected,
            signed_message_digest: hash(&original),
        };
        let body = receipt_frame(&receipt);
        assert_eq!(body.len(), 259);
        let signature = self.bob.device_key.sign(&hash(&body));
        let pinned = &self
            .membership
            .devices
            .get(&self.bob.public.device)
            .unwrap()
            .key;
        nf_contract::signatures::verify_digest(pinned, &hash(&body), &signature).unwrap();
        SignedChatReceipt { receipt, signature }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(resolved) = fs::canonicalize(&self.directory)
            && resolved.parent() == Some(self.parent.as_path())
            && resolved
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("chat-outbox-receipt-"))
        {
            let _ = fs::remove_dir_all(resolved);
        }
    }
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn identity(peer: Vec<u8>) -> LocalIdentity {
    let (public, account_key, device_key) = generate_identity(peer).unwrap();
    LocalIdentity {
        public,
        account_key,
        device_key,
    }
}
fn author(identity: &LocalIdentity) -> Author {
    Author {
        account: identity.public.account,
        device: identity.public.device,
    }
}
fn proof(identity: &LocalIdentity, scope: Scope, issued: IssuedChallenge) -> DeviceProof {
    let mut proof = DeviceProof {
        scope,
        account: identity.public.account,
        device: identity.public.device,
        frontier: issued.membership_revision,
        peer: identity.public.peer.clone(),
        challenge: issued.challenge,
        signature: [0; 64],
    };
    proof.signature = identity.device_key.sign(&device_digest(&proof).unwrap());
    proof
}
// Independent literal canonical encoders sign setup only; no production receipt helper.
fn message_frame(message: &ChatMessage) -> Vec<u8> {
    let mut body = b"NF-CHAT-MESSAGE-1\0".to_vec();
    body.extend_from_slice(message.scope.universe.as_bytes());
    body.extend_from_slice(message.scope.history.as_bytes());
    body.push(1);
    body.extend_from_slice(message.author.account.as_bytes());
    body.extend_from_slice(message.author.device.as_bytes());
    body.extend_from_slice(&message.message);
    body.extend_from_slice(&message.sequence.to_be_bytes());
    body.extend_from_slice(&(message.text.len() as u16).to_be_bytes());
    body.extend_from_slice(message.text.as_bytes());
    body
}
fn receipt_frame(receipt: &ChatDeliveryReceipt) -> Vec<u8> {
    let mut body = b"NF-CHAT-RECEIPT-1\0".to_vec();
    body.extend_from_slice(&receipt.policy_digest);
    body.extend_from_slice(receipt.scope.universe.as_bytes());
    body.extend_from_slice(receipt.scope.history.as_bytes());
    body.push(1);
    body.extend_from_slice(receipt.receiver.account.as_bytes());
    body.extend_from_slice(receipt.receiver.device.as_bytes());
    body.extend_from_slice(&receipt.receiver_peer_digest);
    body.extend_from_slice(receipt.original.original_request.as_bytes());
    body.extend_from_slice(&receipt.original.message);
    body.extend_from_slice(receipt.original.author.account.as_bytes());
    body.extend_from_slice(receipt.original.author.device.as_bytes());
    body.extend_from_slice(&receipt.original.source_sequence.to_be_bytes());
    body.extend_from_slice(&receipt.original.receiver_cursor.to_be_bytes());
    body.extend_from_slice(&receipt.signed_message_digest);
    body
}
