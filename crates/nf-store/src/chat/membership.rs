use super::{ChatPolicy, ChatStoreError, Result, schema};
use nf_identity::model::{
    AdmissionProof, DeviceRevocation, MembershipState, Roles, SignedInvitation,
};
use rusqlite::{Connection, params};

// Same Immediate snapshot verifies current scope/profile/history and authentic account authority.
// Only the account-signed transition produces next state; callers cannot provide raw membership.
pub(super) fn revoke(
    connection: &Connection,
    policy: &ChatPolicy,
    change: &DeviceRevocation,
    signature: &[u8; 64],
) -> Result<MembershipState> {
    schema::verify(connection, policy)?;
    let current = schema::current(connection, policy.scope)?;
    let next = current.revoke_device(change, signature)?;
    let body = nf_identity::codec::encode_state(&next)?;
    // Bounded canonical state and authenticated transition are complete before any SQL effect.
    if connection.execute(
        "UPDATE membership SET revision=?1,public_state=?2,digest=?3 WHERE universe=?4 AND history=?5 AND revision=?6",
        params![
            crate::schema::counter(next.revision),
            body,
            super::codec::hash(&body),
            policy.scope.universe.as_bytes(),
            policy.scope.history.as_bytes(),
            crate::schema::counter(current.revision)
        ],
    )? != 1
    {
        return Err(ChatStoreError::Conflict);
    }
    Ok(next)
}

// Only a fully verified issuer invitation and recipient account/device proofs may persist admission.
pub(super) fn admit(
    connection: &Connection,
    policy: &ChatPolicy,
    signed: &SignedInvitation,
    proof: &AdmissionProof,
    local_now: u64,
) -> Result<MembershipState> {
    schema::verify(connection, policy)?;
    let current = schema::current(connection, policy.scope)?;
    if signed.invitation.roles != Roles::PLAYER {
        return Err(nf_identity::model::IdentityError::RolePolicy.into());
    }
    let next = current.redeem(signed, proof, local_now)?;
    let body = nf_identity::codec::encode_state(&next)?;
    // Bounded canonical state and authenticated transition are complete before any SQL effect.
    if connection.execute(
        "UPDATE membership SET revision=?1,public_state=?2,digest=?3 WHERE universe=?4 AND history=?5 AND revision=?6",
        params![
            crate::schema::counter(next.revision),
            body,
            super::codec::hash(&body),
            policy.scope.universe.as_bytes(),
            policy.scope.history.as_bytes(),
            crate::schema::counter(current.revision)
        ],
    )? != 1
    {
        return Err(ChatStoreError::Conflict);
    }
    Ok(next)
}
