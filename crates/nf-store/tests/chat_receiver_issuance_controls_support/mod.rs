use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::{generate_identity, random_id},
    model::{DeviceProof, DeviceRevocation, Invitation, MembershipState, Roles, Scope},
    private_storage::LocalIdentity,
    rotation::revocation_digest,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::chat::{
    Author, ChallengeRequest, Channel, ChatMessage, ChatPolicy, ChatReceipt, ChatStore,
    KnownChatFrontiers, LocalReceiptIssuer, ProofAttempt, SignedMessage, outbox::SignedChatReceipt,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub struct Fixture {
    pub alice: LocalIdentity,
    pub bob: LocalIdentity,
    pub policy: ChatPolicy,
    pub signed: SignedMessage,
    membership: MembershipState,
    directory: PathBuf,
    parent: PathBuf,
}
impl Fixture {
    pub fn new() -> Self {
        Self::with_receiver_roles(Roles::PLAYER)
    }
    pub fn with_receiver_roles(roles: Roles) -> Self {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
        fs::create_dir_all(&parent).unwrap();
        let parent = fs::canonicalize(parent).unwrap();
        let suffix: String = random_id()
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let directory = parent.join(format!("chat-receiver-issuance-controls-{suffix}"));
        fs::create_dir(&directory).unwrap();
        let alice = identity(b"issuer-control-alice".to_vec());
        let bob = identity(b"issuer-control-bob".to_vec());
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let founder = MembershipState::bootstrap(scope, &alice.public).unwrap();
        // Founder authority signs the actual invitation; Bob supplies both recipient proofs.
        // RELAY genesis is authenticated by the owner, not an arbitrary raw role mutation.
        let invitation = Invitation {
            scope,
            id: [3; 16],
            issuer: alice.public.account,
            recipient: bob.public.clone(),
            roles,
            expires_at: 100,
            issued_revision: 0,
            reusable: false,
        };
        let admission = admission_proof(&invitation, &bob.account_key, &bob.device_key).unwrap();
        let signed_invitation = sign_invitation(invitation, &alice.account_key).unwrap();
        let membership = founder.redeem(&signed_invitation, &admission, 1).unwrap();
        let message = ChatMessage {
            scope,
            channel: Channel::General,
            author: author(&alice),
            message: [81; 16],
            sequence: 1,
            text: "receiver issuance independent controls".to_owned(),
        };
        let signature = alice.device_key.sign(&hash(&message_frame(&message)));
        Self {
            alice,
            bob,
            policy: ChatPolicy { scope },
            signed: SignedMessage { message, signature },
            membership,
            directory,
            parent,
        }
    }
    pub fn database(&self) -> PathBuf {
        self.directory.join("receiver.sqlite")
    }
    pub fn snapshot(&self) -> Vec<u8> {
        fs::read(self.database()).unwrap()
    }
    pub fn issuer(&self) -> LocalReceiptIssuer<'_> {
        LocalReceiptIssuer {
            policy: self.policy,
            receiver: author(&self.bob),
            peer: &self.bob.public.peer,
            device_key: &self.bob.device_key,
        }
    }
    pub fn create_receiver(&self) -> ChatStore {
        ChatStore::create(self.database(), &self.policy, &self.membership).unwrap()
    }
    pub fn committed_receiver(&self) -> ChatStore {
        let mut receiver = self.create_receiver();
        assert_eq!(
            receiver.known_frontiers(),
            Ok(KnownChatFrontiers {
                scope: self.policy.scope,
                revision: 0,
                membership_revision: 1,
            })
        );
        assert_eq!(
            self.post(&mut receiver, RequestId::from_bytes([101; 16])),
            Ok(ChatReceipt {
                message: [81; 16],
                author: author(&self.alice),
                source_sequence: 1,
                receiver_cursor: 1,
                original_request: RequestId::from_bytes([101; 16]),
            })
        );
        let known = KnownChatFrontiers {
            scope: self.policy.scope,
            revision: 1,
            membership_revision: 1,
        };
        assert_eq!(receiver.known_frontiers(), Ok(known));
        drop(receiver);
        let receiver = ChatStore::open_existing(self.database(), &self.policy, known).unwrap();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        receiver
    }
    pub fn post(
        &self,
        receiver: &mut ChatStore,
        request: RequestId,
    ) -> nf_store::chat::Result<ChatReceipt> {
        let issued = receiver.issue_challenge(
            ChallengeRequest::Post {
                request,
                message: &self.signed,
            },
            &self.alice.public.peer,
        )?;
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
        receiver.post(
            request,
            &self.signed,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &proof,
                peer: &self.alice.public.peer,
            },
        )
    }
    pub fn revoke(
        &self,
        receiver: &mut ChatStore,
        target: &LocalIdentity,
    ) -> nf_store::chat::Result<MembershipState> {
        let change = DeviceRevocation {
            scope: self.policy.scope,
            issuer: target.public.account,
            device: target.public.device,
            frontier: receiver.known_frontiers()?.membership_revision,
        };
        let signature = target.account_key.sign(&revocation_digest(&change));
        receiver.revoke_device(&change, &signature)
    }
    pub fn verify_signature(
        &self,
        receipt: &SignedChatReceipt,
    ) -> Result<(), nf_contract::signatures::InvalidSignature> {
        // Independent receipt frame, verified against Bob's original public key.
        nf_contract::signatures::verify_digest(
            &self.bob.public.device_key,
            &hash(&receipt_frame(receipt)),
            &receipt.signature,
        )
    }
    pub fn durable_rows_after_owner_close(&self) -> (i64, i64, Vec<u8>, Vec<u8>) {
        // This method is called only after dropping the exclusive ChatStore owner.
        let connection = rusqlite::Connection::open_with_flags(
            self.database(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let messages = connection
            .query_row("SELECT count(*) FROM chat_messages", [], |row| row.get(0))
            .unwrap();
        let requests = connection
            .query_row("SELECT count(*) FROM chat_requests", [], |row| row.get(0))
            .unwrap();
        let revision = connection
            .query_row("SELECT revision FROM chat_meta", [], |row| row.get(0))
            .unwrap();
        let membership = connection
            .query_row("SELECT revision FROM membership", [], |row| row.get(0))
            .unwrap();
        (messages, requests, revision, membership)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(resolved) = fs::canonicalize(&self.directory)
            && resolved.parent() == Some(self.parent.as_path())
            && resolved.file_name().is_some_and(|name| {
                name.to_string_lossy()
                    .starts_with("chat-receiver-issuance-controls-")
            })
        {
            let _ = fs::remove_dir_all(resolved);
        }
    }
}
pub fn author(identity: &LocalIdentity) -> Author {
    Author {
        account: identity.public.account,
        device: identity.public.device,
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
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
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
fn receipt_frame(signed: &SignedChatReceipt) -> Vec<u8> {
    let receipt = &signed.receipt;
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
