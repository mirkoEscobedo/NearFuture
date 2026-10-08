use super::{
    chat_revocation_support::proof,
    chat_support::{Fixture, author},
};
use nf_contract::identity::RequestId;
use nf_identity::{keys::generate_identity, private_storage::LocalIdentity};
use nf_store::chat::{
    ChallengeRequest, Channel, ChatReceipt, ChatStore, HistoryPage, HistoryQuery, ProofAttempt,
    Result, SignedMessage,
};

pub(super) fn player() -> LocalIdentity {
    let (public, account_key, device_key) =
        generate_identity(b"chat-admission-charlie".to_vec()).unwrap();
    LocalIdentity {
        public,
        account_key,
        device_key,
    }
}
pub(super) fn history(
    fixture: &Fixture,
    actor: &LocalIdentity,
    store: &mut ChatStore,
    request: u8,
) -> Result<HistoryPage> {
    let query = HistoryQuery {
        request: RequestId::from_bytes([request; 16]),
        reader: author(actor),
        channel: Channel::General,
        after_cursor: 0,
        limit: 64,
    };
    let issued = store.issue_challenge(ChallengeRequest::History(&query), &actor.public.peer)?;
    let signed = proof(actor, fixture.policy.scope, issued);
    store.history(
        &query,
        ProofAttempt {
            ticket: issued.ticket,
            proof: &signed,
            peer: &actor.public.peer,
        },
    )
}
pub(super) fn post(
    fixture: &Fixture,
    actor: &LocalIdentity,
    store: &mut ChatStore,
    request: u8,
    message: &SignedMessage,
) -> Result<ChatReceipt> {
    let request = RequestId::from_bytes([request; 16]);
    let issued = store.issue_challenge(
        ChallengeRequest::Post { request, message },
        &actor.public.peer,
    )?;
    let signed = proof(actor, fixture.policy.scope, issued);
    store.post(
        request,
        message,
        ProofAttempt {
            ticket: issued.ticket,
            proof: &signed,
            peer: &actor.public.peer,
        },
    )
}
