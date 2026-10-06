use crate::model::*;

pub fn redeem_persisted(
    repository: &mut impl MembershipRepository,
    scope: Scope,
    invitation: &SignedInvitation,
    proof: &AdmissionProof,
    now: u64,
) -> Result<MembershipState, IdentityError> {
    let state = repository
        .load_membership(scope)?
        .ok_or(IdentityError::MissingLocalState)?;
    if state.scope != scope {
        return Err(IdentityError::Scope);
    }
    let next = state.redeem(invitation, proof, now)?;
    repository.commit_membership(Some(state.revision), &next)?;
    Ok(next)
}
pub fn revoke_persisted(
    repository: &mut impl MembershipRepository,
    scope: Scope,
    change: &DeviceRevocation,
    signature: &[u8; 64],
) -> Result<MembershipState, IdentityError> {
    let state = repository
        .load_membership(scope)?
        .ok_or(IdentityError::MissingLocalState)?;
    if state.scope != scope {
        return Err(IdentityError::Scope);
    }
    let next = state.revoke_device(change, signature)?;
    repository.commit_membership(Some(state.revision), &next)?;
    Ok(next)
}
pub fn rotate_device_persisted(
    repository: &mut impl MembershipRepository,
    scope: Scope,
    change: &DeviceRotation,
    account_signature: &[u8; 64],
    device_signature: &[u8; 64],
) -> Result<MembershipState, IdentityError> {
    let state = repository
        .load_membership(scope)?
        .ok_or(IdentityError::MissingLocalState)?;
    if state.scope != scope {
        return Err(IdentityError::Scope);
    }
    let next = state.rotate_device(change, account_signature, device_signature)?;
    repository.commit_membership(Some(state.revision), &next)?;
    Ok(next)
}
