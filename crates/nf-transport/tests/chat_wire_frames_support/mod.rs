use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::{generate_identity, random_id},
    model::{DeviceProof, Invitation, MembershipState, Roles, Scope},
    private_storage::LocalIdentity,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::chat::{
    Author, ChallengeRequest, Channel, ChatMessage, ChatPolicy, ChatReceipt, ChatStore,
    IssuedChallenge, KnownChatFrontiers, LocalReceiptIssuer, ProofAttempt, SignedMessage,
    outbox::{ChatDeliveryReceipt, SignedChatReceipt},
};
use nf_transport::chat::{ChatFrame, Refusal, WireContext};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub struct Fixture {
    pub signed: SignedMessage,
    pub policy: ChatPolicy,
    alice: LocalIdentity,
    bob: LocalIdentity,
    membership: MembershipState,
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
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let directory = parent.join(format!("chat-wire-frame-{suffix}"));
        fs::create_dir(&directory).unwrap();
        let alice = identity(vec![41; 128]);
        let bob = identity(vec![42; 128]);
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
        let invitation = sign_invitation(invitation, &alice.account_key).unwrap();
        let membership = founder.redeem(&invitation, &admission, 1).unwrap();
        let message = ChatMessage {
            scope,
            channel: Channel::General,
            author: author(&alice),
            message: [81; 16],
            sequence: 1,
            text: "é".repeat(1024),
        };
        let signature = alice.device_key.sign(&hash(&message_bytes(&message)));
        Self {
            signed: SignedMessage { message, signature },
            policy: ChatPolicy { scope },
            alice,
            bob,
            membership,
            directory,
            parent,
        }
    }
    pub fn prepare(&self) -> (ChatStore, IssuedChallenge, DeviceProof, SignedChatReceipt) {
        let path = self.directory.join("receiver.sqlite");
        let mut receiver = ChatStore::create(&path, &self.policy, &self.membership).unwrap();
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
        let attempt = self.proof(issued);
        assert_eq!(
            receiver.post(
                request,
                &self.signed,
                ProofAttempt {
                    ticket: issued.ticket,
                    proof: &attempt,
                    peer: &self.alice.public.peer
                }
            ),
            Ok(ChatReceipt {
                message: [81; 16],
                author: author(&self.alice),
                source_sequence: 1,
                receiver_cursor: 1,
                original_request: request
            })
        );
        let known = KnownChatFrontiers {
            scope: self.policy.scope,
            revision: 1,
            membership_revision: 1,
        };
        assert_eq!(receiver.known_frontiers(), Ok(known));
        drop(receiver);
        let mut receiver = ChatStore::open_existing(&path, &self.policy, known).unwrap();
        let receipt = receiver
            .issue_delivery_receipt(
                request,
                LocalReceiptIssuer {
                    policy: self.policy,
                    receiver: author(&self.bob),
                    peer: &self.bob.public.peer,
                    device_key: &self.bob.device_key,
                },
            )
            .unwrap();
        assert_eq!(receipt.receipt.original.original_request, request);
        assert_eq!(
            nf_contract::signatures::verify_digest(
                &self.bob.public.device_key,
                &hash(&receipt_bytes(&receipt.receipt)),
                &receipt.signature
            ),
            Ok(())
        );
        let issued = receiver
            .issue_challenge(
                ChallengeRequest::Post {
                    request,
                    message: &self.signed,
                },
                &self.alice.public.peer,
            )
            .unwrap();
        let proof = self.proof(issued);
        assert_eq!(
            self.membership.authorize(
                &proof,
                &self.alice.public.peer,
                &issued.challenge,
                issued.membership_revision,
                nf_identity::model::ProtectedOperation::Chat
            ),
            Ok(1)
        );
        (receiver, issued, proof, receipt)
    }
    pub fn snapshot(&self) -> Vec<u8> {
        fs::read(self.directory.join("receiver.sqlite")).unwrap()
    }
    pub fn context(&self) -> WireContext {
        let mut bytes = b"NF-CHAT-POLICY-1\0".to_vec();
        bytes.extend_from_slice(self.policy.scope.universe.as_bytes());
        bytes.extend_from_slice(self.policy.scope.history.as_bytes());
        bytes.push(1);
        for value in [2048u16, 4096, 16384, 64] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        WireContext {
            policy_digest: hash(&bytes),
            request: RequestId::from_bytes([101; 16]),
        }
    }
    fn proof(&self, issued: IssuedChallenge) -> DeviceProof {
        let mut proof = DeviceProof {
            scope: self.policy.scope,
            account: self.alice.public.account,
            device: self.alice.public.device,
            frontier: issued.membership_revision,
            peer: self.alice.public.peer.clone(),
            challenge: issued.challenge,
            signature: [0; 64],
        };
        proof.signature = self.alice.device_key.sign(&device_digest(&proof).unwrap());
        proof
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(resolved) = fs::canonicalize(&self.directory)
            && resolved.parent() == Some(self.parent.as_path())
            && resolved
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("chat-wire-frame-"))
        {
            let _ = fs::remove_dir_all(resolved);
        }
    }
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
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn message_bytes(message: &ChatMessage) -> Vec<u8> {
    let mut out = b"NF-CHAT-MESSAGE-1\0".to_vec();
    out.extend_from_slice(message.scope.universe.as_bytes());
    out.extend_from_slice(message.scope.history.as_bytes());
    out.push(1);
    out.extend_from_slice(message.author.account.as_bytes());
    out.extend_from_slice(message.author.device.as_bytes());
    out.extend_from_slice(&message.message);
    out.extend_from_slice(&message.sequence.to_be_bytes());
    out.extend_from_slice(&(message.text.len() as u16).to_be_bytes());
    out.extend_from_slice(message.text.as_bytes());
    out
}
fn receipt_bytes(receipt: &ChatDeliveryReceipt) -> Vec<u8> {
    let mut out = b"NF-CHAT-RECEIPT-1\0".to_vec();
    out.extend_from_slice(&receipt.policy_digest);
    out.extend_from_slice(receipt.scope.universe.as_bytes());
    out.extend_from_slice(receipt.scope.history.as_bytes());
    out.push(1);
    out.extend_from_slice(receipt.receiver.account.as_bytes());
    out.extend_from_slice(receipt.receiver.device.as_bytes());
    out.extend_from_slice(&receipt.receiver_peer_digest);
    out.extend_from_slice(receipt.original.original_request.as_bytes());
    out.extend_from_slice(&receipt.original.message);
    out.extend_from_slice(receipt.original.author.account.as_bytes());
    out.extend_from_slice(receipt.original.author.device.as_bytes());
    out.extend_from_slice(&receipt.original.source_sequence.to_be_bytes());
    out.extend_from_slice(&receipt.original.receiver_cursor.to_be_bytes());
    out.extend_from_slice(&receipt.signed_message_digest);
    out
}
/// Independent literal wire frames: never use production Chat codec exports for expected bytes.
pub fn literal_frame(frame: &ChatFrame) -> Vec<u8> {
    let (tag, context) = match frame {
        ChatFrame::PostChallenge { context, .. } => (1, context),
        ChatFrame::PostProof { context, .. } => (2, context),
        ChatFrame::Issued { context, .. } => (3, context),
        ChatFrame::Delivered { context, .. } => (4, context),
        ChatFrame::Refused { context, .. } => (5, context),
    };
    let mut out = b"NF-CHAT-WIRE-1\0".to_vec();
    out.push(tag);
    out.extend_from_slice(&context.policy_digest);
    out.extend_from_slice(context.request.as_bytes());
    match frame {
        ChatFrame::PostChallenge { signed, .. } => literal_signed(&mut out, signed),
        ChatFrame::PostProof {
            signed,
            ticket,
            proof,
            ..
        } => {
            literal_signed(&mut out, signed);
            out.extend_from_slice(ticket);
            out.extend_from_slice(proof.scope.universe.as_bytes());
            out.extend_from_slice(proof.scope.history.as_bytes());
            out.extend_from_slice(proof.account.as_bytes());
            out.extend_from_slice(proof.device.as_bytes());
            out.extend_from_slice(&proof.frontier.to_be_bytes());
            out.extend_from_slice(&(proof.peer.len() as u16).to_be_bytes());
            out.extend_from_slice(&proof.peer);
            out.extend_from_slice(&proof.challenge);
            out.extend_from_slice(&proof.signature);
        }
        ChatFrame::Issued { challenge, .. } => {
            out.extend_from_slice(&challenge.ticket);
            out.extend_from_slice(&challenge.challenge);
            out.extend_from_slice(&challenge.membership_revision.to_be_bytes());
        }
        ChatFrame::Delivered { signed, .. } => {
            out.extend_from_slice(&receipt_bytes(&signed.receipt));
            out.extend_from_slice(&signed.signature);
        }
        ChatFrame::Refused { reason, .. } => out.push(match reason {
            Refusal::Unsupported => 1,
            Refusal::Unauthorized => 2,
            Refusal::Limit => 3,
            Refusal::Replay => 4,
            Refusal::Conflict => 5,
            Refusal::Offline => 6,
        }),
    }
    out
}
fn literal_signed(out: &mut Vec<u8>, signed: &SignedMessage) {
    let mut bytes = message_bytes(&signed.message);
    bytes.extend_from_slice(&signed.signature);
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(&bytes);
}
