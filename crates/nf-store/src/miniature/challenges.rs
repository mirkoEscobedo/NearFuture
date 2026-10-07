use super::{
    MiniatureStore,
    auth::{Context, IssuedChallenge, Purpose, random},
    error::{MiniatureStoreError, Result},
    model::*,
    state::State,
};
use crate::{StoreError, schema::hash};
use nf_contract::identity::*;
use nf_identity::model::MembershipState;
use nf_kernel::miniature::*;
impl MiniatureStore {
    pub fn issue_challenge(&mut self, request: ChallengeRequest<'_>) -> Result<IssuedChallenge> {
        self.ensure()?;
        let membership = crate::identity::load(&self.connection, self.state.scope())?
            .ok_or(StoreError::Corrupt)?;
        if !matches!(request, ChallengeRequest::Claim { .. }) && !self.runtime.claimed {
            return Err(MiniatureStoreError::Unclaimed);
        }
        let (purpose, actor, device, binding) = match request {
            ChallengeRequest::Claim { actor, device } => {
                if !self.world().component().genesis().accounts.contains(&actor) {
                    return Err(MiniatureStoreError::Unauthorized);
                }
                (Purpose::Claim, actor, device, [0; 32])
            }
            ChallengeRequest::Activity { actor, device } => {
                if !self.world().component().genesis().accounts.contains(&actor) {
                    return Err(MiniatureStoreError::Unauthorized);
                }
                (Purpose::Lease, actor, device, [0; 32])
            }
            ChallengeRequest::Prepare(i) => {
                if i.universe != self.state.scope().universe
                    || i.history != self.state.scope().history
                {
                    return Err(StoreError::Scope.into());
                }
                (
                    Purpose::Prepare,
                    i.actor,
                    i.device,
                    hash(&encode_miniature_intent(i)?),
                )
            }
            ChallengeRequest::Resume(request) => {
                let r = self
                    .state
                    .requests
                    .get(&request)
                    .ok_or(StoreError::RequestConflict)?;
                if !matches!(r.status, MiniatureRequestStatus::Pending { .. }) {
                    return Err(StoreError::RequestConflict.into());
                }
                (
                    Purpose::Resume,
                    r.intent.actor,
                    r.intent.device,
                    hash(&encode_miniature_intent(&r.intent)?),
                )
            }
            ChallengeRequest::Commit | ChallengeRequest::Cancel => {
                let a = self.authority().ok_or(MiniatureStoreError::Unclaimed)?;
                let f = self.pending().ok_or(StoreError::InvalidTransition)?;
                (
                    if matches!(request, ChallengeRequest::Commit) {
                        Purpose::Commit
                    } else {
                        Purpose::Cancel
                    },
                    a.account,
                    a.device,
                    hash(&encode_miniature_frontier(f)?),
                )
            }
        };
        let old_session = self.authority().map_or(RuntimeSession(0), |a| a.session);
        let session = if purpose == Purpose::Claim {
            fresh_session(old_session)?
        } else {
            old_session
        };
        let context = context(
            &self.state,
            &membership,
            purpose,
            actor,
            device,
            binding,
            session,
        )?;
        self.runtime.issue(context, &membership)
    }
}
pub(crate) fn context(
    state: &State,
    membership: &MembershipState,
    purpose: Purpose,
    actor: AccountId,
    device: DeviceId,
    binding: [u8; 32],
    session: RuntimeSession,
) -> Result<Context> {
    let term = state.authority.map_or(AuthorityTerm(0), |a| a.term);
    let proposed_term = if purpose == Purpose::Claim {
        term.checked_next().ok_or(StoreError::Limit)?
    } else {
        term
    };
    let tick = if purpose == Purpose::Cancel {
        state.world.metadata().tick
    } else {
        state
            .world
            .metadata()
            .tick
            .checked_next()
            .ok_or(StoreError::Limit)?
    };
    Ok(Context {
        purpose,
        scope: state.scope(),
        ruleset: nf_world::ruleset_hash(),
        term,
        proposed_term,
        session,
        tick,
        membership: membership.revision,
        actor,
        device,
        binding,
    })
}
fn fresh_session(previous: RuntimeSession) -> Result<RuntimeSession> {
    for _ in 0..8 {
        let n = u64::from_le_bytes(random()?);
        if n != 0 && n != previous.0 {
            return Ok(RuntimeSession(n));
        }
    }
    Err(MiniatureStoreError::Entropy)
}
