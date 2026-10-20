use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::random_id,
    model::{DeviceProof, Invitation, MembershipState, Roles, Scope},
    private_storage::{LocalIdentity, PrivateVault},
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_ipc::{
    AuthenticatedSession, ChatQueryPort, EndpointRole, FramePump, NodeServer, PublishedRendezvous,
    SessionConfig,
};
use nf_store::chat::{
    Author, ChallengeRequest, Channel, ChatMessage, ChatPolicy, ChatStore, KnownChatFrontiers,
    LocalReceiptIssuer, ProofAttempt, SignedMessage,
    outbox::{ClientOutbox, OutboxProfile, OutgoingEntry, OutgoingState},
};
use nf_wire::generated as g;
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::TcpStream,
    path::PathBuf,
    time::{Duration, Instant},
};
pub struct Fixture {
    pub vault: PrivateVault,
    pub alice: LocalIdentity,
    pub bob: LocalIdentity,
    pub state: MembershipState,
    pub policy: ChatPolicy,
    pub signed: SignedMessage,
    pub profile: OutboxProfile,
    pub store_path: PathBuf,
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
            .map(|b| format!("{b:02x}"))
            .collect();
        let directory = parent.join(format!("chat-ipc-status-{suffix}"));
        fs::create_dir(&directory).unwrap();
        let saves = directory.join("saves");
        fs::create_dir(&saves).unwrap();
        let vault = PrivateVault::create(&directory.join("alice-private"), &saves).unwrap();
        let bob_vault = PrivateVault::create(&directory.join("bob-private"), &saves).unwrap();
        // Full fixture membership pins, not a claim of actual Noise-network peer establishment.
        let alice = vault.create_identity(vec![41; 32]).unwrap();
        let bob = bob_vault.create_identity(vec![42; 32]).unwrap();
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
        let proof = admission_proof(&invitation, &bob.account_key, &bob.device_key).unwrap();
        let signed_invitation = sign_invitation(invitation, &alice.account_key).unwrap();
        let state = founder.redeem(&signed_invitation, &proof, 1).unwrap();
        assert_eq!(state.revision, 1);
        let policy = ChatPolicy { scope };
        let message = ChatMessage {
            scope,
            channel: Channel::General,
            author: author(&alice),
            message: [81; 16],
            sequence: 1,
            text: "Chat IPC status".into(),
        };
        let signature = alice.device_key.sign(&hash(&message_bytes(&message)));
        assert_eq!(
            nf_contract::signatures::verify_digest(
                &alice.public.device_key,
                &hash(&message_bytes(&message)),
                &signature
            ),
            Ok(())
        );
        let signed = SignedMessage { message, signature };
        let profile =
            OutboxProfile::from_current(&policy, &state, author(&alice), author(&bob)).unwrap();
        Self {
            vault,
            alice,
            bob,
            state,
            policy,
            signed,
            profile,
            store_path: directory.join("chat.sqlite"),
            outbox_path: directory.join("outbox.sqlite"),
            directory,
            parent,
        }
    }
    pub fn request(&self) -> RequestId {
        RequestId::from_bytes([101; 16])
    }
    pub fn pending(&self) -> OutgoingEntry {
        OutgoingEntry {
            original_request: self.request(),
            signed: self.signed.clone(),
            state: OutgoingState::Pending,
        }
    }
    pub fn initial_known(&self) -> KnownChatFrontiers {
        KnownChatFrontiers {
            scope: self.policy.scope,
            revision: 0,
            membership_revision: 1,
        }
    }
    pub fn create_pending(&self) {
        let store = ChatStore::create(&self.store_path, &self.policy, &self.state).unwrap();
        let mut outbox = ClientOutbox::create(&self.outbox_path, &self.profile).unwrap();
        assert_eq!(
            outbox.enqueue(self.request(), &self.signed).unwrap(),
            self.pending()
        );
        assert_eq!(outbox.known_revision().unwrap(), 1);
        assert_eq!(store.known_frontiers().unwrap(), self.initial_known());
        drop(outbox);
        drop(store);
    }
    pub fn config(&self, runtime: u64) -> SessionConfig {
        SessionConfig {
            universe: [1; 16],
            history: [2; 16],
            runtime_session: runtime,
            ruleset: [3; 32],
            content_policy: hash(&policy_bytes(self.policy)),
            limits: nf_ipc::default_limits(),
        }
    }
    pub fn query(&self, request: u8) -> g::QueryChatOutgoing {
        g::QueryChatOutgoing {
            request_id: Some(g::RequestId {
                value: vec![request; 16],
            }),
            principal: Some(g::Principal {
                account_id: Some(g::AccountId {
                    value: self.alice.public.account.as_bytes().to_vec(),
                }),
                device_id: Some(g::DeviceId {
                    value: self.alice.public.device.as_bytes().to_vec(),
                }),
            }),
            universe_id: Some(g::UniverseId { value: vec![1; 16] }),
            history_id: Some(g::HistoryId { value: vec![2; 16] }),
            original_request_id: Some(g::RequestId {
                value: vec![101; 16],
            }),
            message_id: Some(g::ChatMessageId {
                value: vec![81; 16],
            }),
        }
    }
    pub fn expected(
        &self,
        query: &g::QueryChatOutgoing,
        phase: g::ChatOutgoingPhase,
        revision: u64,
        receipt: Vec<u8>,
    ) -> g::ChatOutgoingStatus {
        let mut signed = message_bytes(&self.signed.message);
        signed.extend_from_slice(&self.signed.signature);
        g::ChatOutgoingStatus {
            request_id: query.request_id.clone(),
            principal: query.principal.clone(),
            universe_id: query.universe_id.clone(),
            history_id: query.history_id.clone(),
            original_request_id: query.original_request_id.clone(),
            message_id: query.message_id.clone(),
            source_sequence: 1,
            outbox_revision: revision,
            phase: phase as i32,
            signed_message: signed,
            receiver_receipt: receipt,
        }
    }
    pub fn expected_receipt(&self) -> Vec<u8> {
        let mut peer = b"NF-CHAT-RECEIVER-PEER-1\0".to_vec();
        peer.extend_from_slice(&(self.bob.public.peer.len() as u16).to_be_bytes());
        peer.extend_from_slice(&self.bob.public.peer);
        let mut original = b"NF-CHAT-SIGNED-MESSAGE-1\0".to_vec();
        original.extend_from_slice(&message_bytes(&self.signed.message));
        original.extend_from_slice(&self.signed.signature);
        let mut bytes = b"NF-CHAT-RECEIPT-1\0".to_vec();
        bytes.extend_from_slice(&hash(&policy_bytes(self.policy)));
        bytes.extend_from_slice(self.policy.scope.universe.as_bytes());
        bytes.extend_from_slice(self.policy.scope.history.as_bytes());
        bytes.push(1);
        bytes.extend_from_slice(self.bob.public.account.as_bytes());
        bytes.extend_from_slice(self.bob.public.device.as_bytes());
        bytes.extend_from_slice(&hash(&peer));
        bytes.extend_from_slice(&[101; 16]);
        bytes.extend_from_slice(&[81; 16]);
        bytes.extend_from_slice(self.alice.public.account.as_bytes());
        bytes.extend_from_slice(self.alice.public.device.as_bytes());
        bytes.extend_from_slice(&1u64.to_be_bytes());
        bytes.extend_from_slice(&1u64.to_be_bytes());
        bytes.extend_from_slice(&hash(&original));
        assert_eq!(bytes.len(), 259);
        let signature = self.bob.device_key.sign(&hash(&bytes));
        assert_eq!(
            nf_contract::signatures::verify_digest(
                &self.bob.public.device_key,
                &hash(&bytes),
                &signature
            ),
            Ok(())
        );
        bytes.extend_from_slice(&signature);
        bytes
    }
    pub fn deliver_locally(
        &self,
        store: &mut ChatStore,
        outbox: &mut ClientOutbox,
    ) -> OutgoingEntry {
        let challenge = store
            .issue_challenge(
                ChallengeRequest::Post {
                    request: self.request(),
                    message: &self.signed,
                },
                &self.alice.public.peer,
            )
            .unwrap();
        let mut proof = DeviceProof {
            scope: self.policy.scope,
            account: self.alice.public.account,
            device: self.alice.public.device,
            frontier: challenge.membership_revision,
            peer: self.alice.public.peer.clone(),
            challenge: challenge.challenge,
            signature: [0; 64],
        };
        proof.signature = self.alice.device_key.sign(&device_digest(&proof).unwrap());
        let committed = store
            .post(
                self.request(),
                &self.signed,
                ProofAttempt {
                    ticket: challenge.ticket,
                    proof: &proof,
                    peer: &self.alice.public.peer,
                },
            )
            .unwrap();
        assert_eq!(committed.original_request, self.request());
        assert_eq!(committed.receiver_cursor, 1);
        let signed = store
            .issue_delivery_receipt(
                self.request(),
                LocalReceiptIssuer {
                    policy: self.policy,
                    receiver: author(&self.bob),
                    peer: &self.bob.public.peer,
                    device_key: &self.bob.device_key,
                },
            )
            .unwrap();
        assert_eq!(
            signed.to_canonical_bytes().unwrap(),
            self.expected_receipt()
        );
        outbox.acknowledge(&signed).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(resolved) = fs::canonicalize(&self.directory)
            && resolved.parent() == Some(self.parent.as_path())
            && resolved
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("chat-ipc-status-"))
        {
            let _ = fs::remove_dir_all(resolved);
        }
    }
}
pub fn connect(
    node: &mut NodeServer<ChatQueryPort>,
    publication: &PublishedRendezvous,
) -> (FramePump, AuthenticatedSession) {
    let socket = TcpStream::connect(node.address()).unwrap();
    assert!(socket.peer_addr().unwrap().ip().is_loopback());
    assert_eq!(socket.peer_addr().unwrap(), node.address());
    let mut client = FramePump::new(socket, 4096).unwrap();
    let hello = publication.record().client(EndpointRole::Control).unwrap();
    client.send(hello.hello()).unwrap();
    let challenge = receive(node, &mut client);
    let proof = hello.respond(&challenge).unwrap();
    client.send(proof.proof()).unwrap();
    let accepted = receive(node, &mut client);
    let session = proof.finish(&accepted).unwrap();
    client.activate(&session).unwrap();
    (client, session)
}
pub fn receive(node: &mut NodeServer<ChatQueryPort>, client: &mut FramePump) -> Vec<u8> {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        node.poll(Instant::now()).unwrap();
        if let Some(bytes) = client.poll(4096, 4096).unwrap() {
            return bytes;
        }
        assert!(
            Instant::now() < until,
            "bounded physical IPC response missing"
        );
        std::thread::yield_now();
    }
}
fn author(local: &LocalIdentity) -> Author {
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
fn policy_bytes(policy: ChatPolicy) -> Vec<u8> {
    let mut bytes = b"NF-CHAT-POLICY-1\0".to_vec();
    bytes.extend_from_slice(policy.scope.universe.as_bytes());
    bytes.extend_from_slice(policy.scope.history.as_bytes());
    bytes.push(1);
    for limit in [2048u16, 4096, 16384, 64] {
        bytes.extend_from_slice(&limit.to_be_bytes());
    }
    bytes
}
