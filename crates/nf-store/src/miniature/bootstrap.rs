use super::{
    auth::{AuthRuntime, Context, ProofAttempt, Purpose, identity},
    error::{MiniatureStoreError, Result},
    model::*,
};
use crate::{StoreError, schema::hash};
use nf_contract::identity::*;
use nf_identity::{
    codec::{decode_state, encode_state},
    model::{MembershipState, Roles},
};
use nf_kernel::miniature::*;
pub(crate) fn genesis(
    spec: MiniatureGenesisSpec,
    body: &[u8],
    policy: &BootstrapPolicy,
    signer: &mut impl BootstrapSigner,
    runtime: &mut AuthRuntime,
) -> Result<(MiniatureWorld, MembershipState, super::auth::Evidence)> {
    if body.len() > 262_144 {
        return Err(StoreError::Limit.into());
    }
    let membership = decode_state(body)?;
    let g = spec.genesis;
    if encode_state(&membership)? != body
        || membership.scope != policy.scope
        || g.universe != policy.scope.universe
        || g.history != policy.scope.history
        || g.accounts != policy.controllers
        || hash(body) != policy.membership_digest
    {
        return Err(MiniatureStoreError::Unauthorized);
    }
    if identity(&membership, policy.owner.account, policy.owner.device)? != policy.owner
        || !policy.controllers.contains(&policy.owner.account)
    {
        return Err(MiniatureStoreError::Unauthorized);
    }
    for account in policy.controllers {
        if !membership
            .accounts
            .get(&account)
            .is_some_and(|a| a.roles.contains(Roles::PLAYER))
        {
            return Err(MiniatureStoreError::Unauthorized);
        }
    }
    if !membership.accounts[&policy.owner.account]
        .roles
        .contains(Roles::AUTHORITY_CANDIDATE)
    {
        return Err(MiniatureStoreError::Unauthorized);
    }
    let world = MiniatureWorld::new(
        MiniatureMetadata {
            tick: WorldTick(0),
            event_sequence: EventSeq(0),
            aggregate: spec.aggregate,
            provider: spec.provider,
            provider_aggregate: spec.provider_aggregate,
            provider_revision: AggregateRevision(0),
        },
        nf_world::generate(g).map_err(MiniatureRejection::from)?,
    )?;
    let binding = bootstrap_binding_preimage(&world, policy.membership_digest, &policy.owner)?;
    let context = Context {
        purpose: Purpose::Bootstrap,
        scope: policy.scope,
        ruleset: nf_world::ruleset_hash(),
        term: AuthorityTerm(0),
        proposed_term: AuthorityTerm(0),
        session: RuntimeSession(0),
        tick: WorldTick(0),
        membership: membership.revision,
        actor: policy.owner.account,
        device: policy.owner.device,
        binding: hash(&binding),
    };
    let issued = runtime.issue(context, &membership)?;
    let proof = signer.sign(&issued.template)?;
    let evidence = runtime.consume(
        ProofAttempt {
            ticket: issued.ticket,
            proof,
        },
        context,
        &membership,
        true,
    )?;
    Ok((world, membership, evidence))
}

/// Public data only; this preimage establishes no membership or authority admission.
pub fn bootstrap_binding_preimage(
    world: &MiniatureWorld,
    membership_digest: [u8; 32],
    owner: &nf_identity::model::PublicIdentity,
) -> Result<Vec<u8>> {
    if owner.peer.is_empty() || owner.peer.len() > 128 {
        return Err(StoreError::Limit.into());
    }
    let snapshot = encode_miniature_snapshot(world)?;
    let maximum = MAX_MINIATURE_BYTES + 281;
    let length = 153usize
        .checked_add(snapshot.len())
        .and_then(|n| n.checked_add(owner.peer.len()))
        .ok_or(StoreError::Limit)?;
    if length > maximum {
        return Err(StoreError::Limit.into());
    }
    let mut binding = Vec::with_capacity(length);
    binding.extend_from_slice(b"NF-MINI-CREATE-1\0");
    binding.extend_from_slice(&(snapshot.len() as u32).to_le_bytes());
    binding.extend_from_slice(&snapshot);
    binding.extend_from_slice(&membership_digest);
    binding.extend_from_slice(owner.account.as_bytes());
    binding.extend_from_slice(&owner.account_key);
    binding.extend_from_slice(owner.device.as_bytes());
    binding.extend_from_slice(&owner.device_key);
    binding.extend_from_slice(&(owner.peer.len() as u32).to_le_bytes());
    binding.extend_from_slice(&owner.peer);
    Ok(binding)
}
