use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    keys::{generate_identity, random_id},
    model::{Invitation, MembershipState, Roles, Scope},
    private_storage::LocalIdentity,
    signing::{admission_proof, sign_invitation},
};
use nf_store::chat::{
    Author, Channel, ChatMessage, ChatPolicy, SignedMessage, outbox::OutboxProfile,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub struct Fixture {
    pub profile: OutboxProfile,
    pub signed: SignedMessage,
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
        let directory = parent.join(format!("chat-outbox-pending-{suffix}"));
        fs::create_dir(&directory).unwrap();
        let alice = identity(b"outbox-fixture-alice".to_vec());
        let bob = identity(b"outbox-fixture-bob".to_vec());
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
        // Independent literal canonical message layout; no production message encoder signs setup.
        let mut canonical = b"NF-CHAT-MESSAGE-1\0".to_vec();
        canonical.extend_from_slice(scope.universe.as_bytes());
        canonical.extend_from_slice(scope.history.as_bytes());
        canonical.push(1);
        canonical.extend_from_slice(message.author.account.as_bytes());
        canonical.extend_from_slice(message.author.device.as_bytes());
        canonical.extend_from_slice(&[81; 16]);
        canonical.extend_from_slice(&1u64.to_be_bytes());
        canonical.extend_from_slice(&27u16.to_be_bytes());
        canonical.extend_from_slice(b"public outbox fixture hello");
        let signature = alice.device_key.sign(&Sha256::digest(canonical).into());
        Self {
            profile,
            signed: SignedMessage { message, signature },
            directory,
            parent,
        }
    }
    pub fn database(&self) -> PathBuf {
        self.directory.join("outbox.sqlite")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // The exclusive owner is constructed after Fixture and dropped first.
        if let Ok(resolved) = fs::canonicalize(&self.directory)
            && resolved.parent() == Some(self.parent.as_path())
            && resolved
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("chat-outbox-pending-"))
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
