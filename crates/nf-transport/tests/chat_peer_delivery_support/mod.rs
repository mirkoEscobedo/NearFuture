use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::random_id,
    model::{Invitation, MembershipState, Roles, Scope},
    private_storage::{LocalIdentity, PrivateVault},
    signing::{admission_proof, sign_invitation},
};
use nf_store::chat::{
    Author, Channel, ChatMessage, ChatPolicy, ChatReceipt, SignedMessage,
    outbox::{ChatDeliveryReceipt, OutboxProfile, OutgoingEntry, OutgoingState, SignedChatReceipt},
};
use nf_transport::identity::TransportIdentity;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub struct Fixture {
    pub client_vault: PrivateVault,
    pub server_vault: PrivateVault,
    pub client_peer: libp2p::PeerId,
    pub server_peer: libp2p::PeerId,
    pub alice: LocalIdentity,
    pub bob: LocalIdentity,
    pub state: MembershipState,
    pub policy: ChatPolicy,
    pub signed: SignedMessage,
    pub profile: OutboxProfile,
    pub receiver_path: PathBuf,
    pub outbox_path: PathBuf,
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
        let directory = parent.join(format!("chat-peer-delivery-{suffix}"));
        fs::create_dir(&directory).unwrap();
        let saves = directory.join("saves");
        fs::create_dir(&saves).unwrap();
        let client_vault = PrivateVault::create(&directory.join("alice-private"), &saves).unwrap();
        let server_vault = PrivateVault::create(&directory.join("bob-private"), &saves).unwrap();
        let client_noise = TransportIdentity::create(&client_vault).unwrap();
        let server_noise = TransportIdentity::create(&server_vault).unwrap();
        let client_peer = client_noise.peer_id();
        let server_peer = server_noise.peer_id();
        let alice = client_vault
            .create_identity(client_peer.to_bytes())
            .unwrap();
        let bob = server_vault
            .create_identity(server_peer.to_bytes())
            .unwrap();
        assert_eq!(
            TransportIdentity::load(&client_vault).unwrap().peer_id(),
            client_peer
        );
        assert_eq!(
            TransportIdentity::load(&server_vault).unwrap().peer_id(),
            server_peer
        );
        assert_ne!(client_peer, server_peer);
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
        let state = founder.redeem(&invitation, &admission, 1).unwrap();
        assert_eq!(state.revision, 1);
        let policy = ChatPolicy { scope };
        let message = ChatMessage {
            scope,
            channel: Channel::General,
            author: author(&alice),
            message: [81; 16],
            sequence: 1,
            text: "genuine Noise chat".into(),
        };
        let signature = alice.device_key.sign(&hash(&message_bytes(&message)));
        let signed = SignedMessage { message, signature };
        let profile =
            OutboxProfile::from_current(&policy, &state, author(&alice), author(&bob)).unwrap();
        Self {
            client_vault,
            server_vault,
            client_peer,
            server_peer,
            alice,
            bob,
            state,
            policy,
            signed,
            profile,
            receiver_path: directory.join("receiver.sqlite"),
            outbox_path: directory.join("outbox.sqlite"),
            directory,
            parent,
        }
    }
    pub fn request(&self) -> RequestId {
        RequestId::from_bytes([101; 16])
    }
    /// Independent expected receipt only; never injected into the transport or outbox.
    pub fn expected(&self) -> OutgoingEntry {
        let mut peer_bytes = b"NF-CHAT-RECEIVER-PEER-1\0".to_vec();
        peer_bytes.extend_from_slice(&(self.bob.public.peer.len() as u16).to_be_bytes());
        peer_bytes.extend_from_slice(&self.bob.public.peer);
        let mut signed_bytes = b"NF-CHAT-SIGNED-MESSAGE-1\0".to_vec();
        signed_bytes.extend_from_slice(&message_bytes(&self.signed.message));
        signed_bytes.extend_from_slice(&self.signed.signature);
        let receipt = ChatDeliveryReceipt {
            policy_digest: policy_digest(self.policy),
            scope: self.policy.scope,
            channel: Channel::General,
            receiver: author(&self.bob),
            receiver_peer_digest: hash(&peer_bytes),
            original: ChatReceipt {
                message: [81; 16],
                author: author(&self.alice),
                source_sequence: 1,
                receiver_cursor: 1,
                original_request: self.request(),
            },
            signed_message_digest: hash(&signed_bytes),
        };
        let body = receipt_bytes(&receipt);
        assert_eq!(body.len(), 259);
        let signature = self.bob.device_key.sign(&hash(&body));
        assert_eq!(
            nf_contract::signatures::verify_digest(
                &self.bob.public.device_key,
                &hash(&body),
                &signature
            ),
            Ok(())
        );
        OutgoingEntry {
            original_request: self.request(),
            signed: self.signed.clone(),
            state: OutgoingState::Delivered(Box::new(SignedChatReceipt { receipt, signature })),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(resolved) = fs::canonicalize(&self.directory)
            && resolved.parent() == Some(self.parent.as_path())
            && resolved
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("chat-peer-delivery-"))
        {
            let _ = fs::remove_dir_all(resolved);
        }
    }
}
pub fn author(local: &LocalIdentity) -> Author {
    Author {
        account: local.public.account,
        device: local.public.device,
    }
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn message_bytes(message: &ChatMessage) -> Vec<u8> {
    let mut bytes = b"NF-CHAT-MESSAGE-1\0".to_vec();
    bytes.extend_from_slice(message.scope.universe.as_bytes());
    bytes.extend_from_slice(message.scope.history.as_bytes());
    bytes.push(1);
    bytes.extend_from_slice(message.author.account.as_bytes());
    bytes.extend_from_slice(message.author.device.as_bytes());
    bytes.extend_from_slice(&message.message);
    bytes.extend_from_slice(&message.sequence.to_be_bytes());
    bytes.extend_from_slice(&(message.text.len() as u16).to_be_bytes());
    bytes.extend_from_slice(message.text.as_bytes());
    bytes
}
fn policy_digest(policy: ChatPolicy) -> [u8; 32] {
    let mut bytes = b"NF-CHAT-POLICY-1\0".to_vec();
    bytes.extend_from_slice(policy.scope.universe.as_bytes());
    bytes.extend_from_slice(policy.scope.history.as_bytes());
    bytes.push(1);
    for length in [2048u16, 4096, 16384, 64] {
        bytes.extend_from_slice(&length.to_be_bytes());
    }
    hash(&bytes)
}
fn receipt_bytes(receipt: &ChatDeliveryReceipt) -> Vec<u8> {
    let mut bytes = b"NF-CHAT-RECEIPT-1\0".to_vec();
    bytes.extend_from_slice(&receipt.policy_digest);
    bytes.extend_from_slice(receipt.scope.universe.as_bytes());
    bytes.extend_from_slice(receipt.scope.history.as_bytes());
    bytes.push(1);
    bytes.extend_from_slice(receipt.receiver.account.as_bytes());
    bytes.extend_from_slice(receipt.receiver.device.as_bytes());
    bytes.extend_from_slice(&receipt.receiver_peer_digest);
    bytes.extend_from_slice(receipt.original.original_request.as_bytes());
    bytes.extend_from_slice(&receipt.original.message);
    bytes.extend_from_slice(receipt.original.author.account.as_bytes());
    bytes.extend_from_slice(receipt.original.author.device.as_bytes());
    bytes.extend_from_slice(&receipt.original.source_sequence.to_be_bytes());
    bytes.extend_from_slice(&receipt.original.receiver_cursor.to_be_bytes());
    bytes.extend_from_slice(&receipt.signed_message_digest);
    bytes
}
