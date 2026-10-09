use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    keys::random_id,
    model::{Invitation, MembershipState, Roles, Scope},
    private_storage::{LocalIdentity, PrivateVault},
    signing::{admission_proof, sign_invitation},
};
use nf_ipc::SessionConfig;
use nf_store::chat::{
    Author, ChatPolicy, ChatStore, KnownChatFrontiers,
    outbox::{ClientOutbox, OutboxProfile},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub struct Fixture {
    pub vault: PrivateVault,
    pub vault_path: PathBuf,
    pub saves_root: PathBuf,
    pub alice: LocalIdentity,
    pub bob: LocalIdentity,
    pub state: MembershipState,
    pub policy: ChatPolicy,
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
        let directory = parent.join(format!("chat-ipc-enqueue-java-{suffix}"));
        fs::create_dir(&directory).unwrap();
        let saves = directory.join("saves");
        fs::create_dir(&saves).unwrap();
        let vault_path = directory.join("alice-private");
        let vault = PrivateVault::create(&vault_path, &saves).unwrap();
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
        let profile =
            OutboxProfile::from_current(&policy, &state, author(&alice), author(&bob)).unwrap();
        let store_path = directory.join("chat.sqlite");
        let outbox_path = directory.join("outbox.sqlite");
        let store = ChatStore::create(&store_path, &policy, &state).unwrap();
        let outbox = ClientOutbox::create(&outbox_path, &profile).unwrap();
        assert_eq!(store.known_frontiers().unwrap().revision, 0);
        assert_eq!(outbox.known_revision(), Ok(0));
        drop(outbox);
        drop(store);
        Self {
            vault,
            vault_path,
            saves_root: saves,
            alice,
            bob,
            state,
            policy,
            profile,
            store_path,
            outbox_path,
            directory,
            parent,
        }
    }
    pub fn initial_known(&self) -> KnownChatFrontiers {
        KnownChatFrontiers {
            scope: self.policy.scope,
            revision: 0,
            membership_revision: 1,
        }
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
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(resolved) = fs::canonicalize(&self.directory)
            && resolved.parent() == Some(self.parent.as_path())
            && resolved
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("chat-ipc-enqueue-java-"))
        {
            let _ = fs::remove_dir_all(resolved);
        }
    }
}
fn author(local: &LocalIdentity) -> Author {
    Author {
        account: local.public.account,
        device: local.public.device,
    }
}
pub fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn message_bytes(message: &nf_store::chat::ChatMessage) -> Vec<u8> {
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
