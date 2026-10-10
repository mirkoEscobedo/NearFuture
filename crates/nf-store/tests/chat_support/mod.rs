use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::{generate_identity, random_id},
    model::{DeviceProof, Invitation, MembershipState, Roles, Scope},
    private_storage::LocalIdentity,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::chat::{
    Author, ChallengeRequest, Channel, ChatMessage, ChatPolicy, ChatReceipt, ChatStore,
    HistoryPage, HistoryQuery, IssuedChallenge, ProofAttempt, Result, SignedMessage, codec,
};
use std::{fs, path::PathBuf};
pub struct Scratch(pub PathBuf);
impl Scratch {
    fn new() -> Self {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
        fs::create_dir_all(&parent).unwrap();
        let suffix: String = random_id()
            .unwrap()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let path = parent.join(format!("chat-delivery-{suffix}"));
        assert!(path.starts_with(&parent));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub struct Fixture {
    pub alice: LocalIdentity,
    pub bob: LocalIdentity,
    pub membership: MembershipState,
    pub policy: ChatPolicy,
    pub scratch: Scratch,
}
impl Fixture {
    pub fn new() -> Self {
        let scratch = Scratch::new();
        let alice = identity(b"fixture-chat-alice".to_vec());
        let bob = identity(b"fixture-chat-bob".to_vec());
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
        assert_eq!(membership.revision, 1);
        Self {
            alice,
            bob,
            membership,
            policy: ChatPolicy { scope },
            scratch,
        }
    }
    pub fn database(&self) -> PathBuf {
        self.scratch.0.join("chat.sqlite")
    }
    pub fn signed_message(&self) -> SignedMessage {
        let message = ChatMessage {
            scope: self.policy.scope,
            channel: Channel::General,
            author: author(&self.alice),
            message: [81; 16],
            sequence: 1,
            text: "public fixture hello".to_owned(),
        };
        let signature = self
            .alice
            .device_key
            .sign(&codec::message_digest(&message).unwrap());
        SignedMessage { message, signature }
    }
    pub fn post(
        &self,
        store: &mut ChatStore,
        request: u8,
        message: &SignedMessage,
    ) -> Result<ChatReceipt> {
        let request = RequestId::from_bytes([request; 16]);
        let issued = store.issue_challenge(
            ChallengeRequest::Post { request, message },
            &self.alice.public.peer,
        )?;
        let proof = proof(&self.alice, self.policy.scope, issued);
        store.post(
            request,
            message,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &proof,
                peer: &self.alice.public.peer,
            },
        )
    }
    pub fn permitted_history(&self, store: &mut ChatStore, request: u8) -> Result<HistoryPage> {
        let query = HistoryQuery {
            request: RequestId::from_bytes([request; 16]),
            reader: author(&self.bob),
            channel: Channel::General,
            after_cursor: 0,
            limit: 64,
        };
        let issued =
            store.issue_challenge(ChallengeRequest::History(&query), &self.bob.public.peer)?;
        let proof = proof(&self.bob, self.policy.scope, issued);
        store.history(
            &query,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &proof,
                peer: &self.bob.public.peer,
            },
        )
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
