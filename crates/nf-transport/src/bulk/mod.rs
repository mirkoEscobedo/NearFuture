mod client;
mod server;
use crate::{
    PeerError,
    auth::BulkTranscript,
    records::{Lane, PeerBody, PeerRecord, reply_prefix_digest},
    session::{Binding, sign_challenge},
};
pub use client::{BulkClient, BulkResult};
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, MembershipState, ProtectedOperation},
};
pub use server::BulkServer;
fn current(b: &Binding<'_>, state: &MembershipState) -> Result<(), PeerError> {
    if b.context.lane != Lane::Bulk || b.context.selected_caps & 2 == 0 {
        return Err(PeerError::Unsupported);
    }
    b.current(state)
}
fn placeholder(b: &Binding<'_>, s: &MembershipState) -> DeviceProof {
    DeviceProof {
        scope: s.scope,
        account: b.local.account,
        device: b.local.device,
        frontier: s.revision,
        peer: b.local.peer.clone(),
        challenge: [0; 32],
        signature: [0; 64],
    }
}
fn signed(
    mut r: PeerRecord,
    t: BulkTranscript,
    b: &Binding<'_>,
    s: &MembershipState,
    key: &SecretSeed,
) -> Result<PeerRecord, PeerError> {
    let challenge = t.challenge(2, reply_prefix_digest(&r, Lane::Bulk, b.context.selected)?)?;
    let p = sign_challenge(r.context, challenge, s, b.local, key, t.minimum_membership)?;
    match &mut r.body {
        PeerBody::BulkReady { proof, .. }
        | PeerBody::BulkProgress { proof, .. }
        | PeerBody::BulkVerified { proof, .. } => *proof = p,
        _ => return Err(PeerError::Malformed),
    }
    Ok(r)
}
fn verify(
    r: &PeerRecord,
    t: BulkTranscript,
    b: &Binding<'_>,
    s: &MembershipState,
) -> Result<(), PeerError> {
    let proof = match &r.body {
        PeerBody::BulkReady { proof, .. }
        | PeerBody::BulkProgress { proof, .. }
        | PeerBody::BulkVerified { proof, .. } => proof,
        _ => return Err(PeerError::Malformed),
    };
    if proof.account != b.context.server_account
        || proof.device != b.context.server_device
        || s.revision != t.frontier
    {
        return Err(PeerError::Unauthorized);
    }
    let challenge = t.challenge(2, reply_prefix_digest(r, Lane::Bulk, b.context.selected)?)?;
    s.authorize(
        proof,
        &b.context.server_peer.to_bytes(),
        &challenge,
        t.minimum_membership,
        ProtectedOperation::Economic,
    )
    .map(|_| ())
    .map_err(|_| PeerError::Unauthorized)
}
