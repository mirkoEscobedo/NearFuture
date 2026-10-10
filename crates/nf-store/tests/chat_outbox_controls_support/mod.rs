use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::{generate_identity, random_id},
    model::{Invitation, MembershipState, Roles, Scope},
    private_storage::LocalIdentity,
    signing::{admission_proof, sign_invitation},
};
use nf_store::chat::{
    Author, Channel, ChatMessage, ChatPolicy, Result, SignedMessage,
    outbox::{ClientOutbox, OutboxProfile, OutgoingEntry, OutgoingState},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub struct Fixture {
    pub profile: OutboxProfile,
    pub signed: SignedMessage,
    alice: LocalIdentity,
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
        let directory = parent.join(format!("chat-outbox-controls-{suffix}"));
        fs::create_dir(&directory).unwrap();
        let alice = identity(b"outbox-controls-alice".to_vec());
        let bob = identity(b"outbox-controls-bob".to_vec());
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
        let current = founder.redeem(&signed, &admission, 1).unwrap();
        let profile = OutboxProfile::from_current(
            &ChatPolicy { scope },
            &current,
            author(&alice),
            author(&bob),
        )
        .unwrap();
        let message = ChatMessage {
            scope,
            channel: Channel::General,
            author: author(&alice),
            message: [81; 16],
            sequence: 1,
            text: "public outbox controls hello".to_owned(),
        };
        let signature = alice
            .device_key
            .sign(&Sha256::digest(canonical(&message)).into());
        Self {
            profile,
            signed: SignedMessage { message, signature },
            alice,
            directory,
            parent,
        }
    }
    pub fn database(&self) -> PathBuf {
        self.directory.join("outbox.sqlite")
    }
    pub fn sign(&self, message: ChatMessage) -> SignedMessage {
        let signature = self
            .alice
            .device_key
            .sign(&Sha256::digest(canonical(&message)).into());
        SignedMessage { message, signature }
    }
    pub fn expected_original(&self) -> OutgoingEntry {
        OutgoingEntry {
            original_request: RequestId::from_bytes([101; 16]),
            signed: self.signed.clone(),
            state: OutgoingState::Pending,
        }
    }
    pub fn seed(&self) -> ClientOutbox {
        let mut outbox = ClientOutbox::create(self.database(), &self.profile).unwrap();
        assert_eq!(
            outbox.enqueue(RequestId::from_bytes([101; 16]), &self.signed),
            Ok(self.expected_original())
        );
        assert_eq!(outbox.entry([81; 16]), Ok(Some(self.expected_original())));
        assert_eq!(outbox.known_revision(), Ok(1));
        outbox
    }
    pub fn reopen(&self, known: u64) -> Result<ClientOutbox> {
        ClientOutbox::open_existing(self.database(), &self.profile, known)
    }
    pub fn snapshot(&self) -> Vec<u8> {
        fs::read(self.database()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(resolved) = fs::canonicalize(&self.directory)
            && resolved.parent() == Some(self.parent.as_path())
            && resolved
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("chat-outbox-controls-"))
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
// Independent literal canonical signature/setup encoder, never a production owner result oracle.
pub fn canonical(message: &ChatMessage) -> Vec<u8> {
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
