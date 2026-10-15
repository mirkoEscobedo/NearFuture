use super::{bytes::{Encoder, Reader, hash, next, same}, cancellation::AdministrativeCancellation,
    challenge::Owned, error::{ManagedError, Result}, foot::StorageFoot, prepare::Prepare,
    settlement, state::{FullState, PendingPolicy}};
use nf_contract::identity::{AuthorityTerm, JobId, OperationId, RuntimeSession};
use nf_identity::model::{DeviceProof, MembershipState, ProtectedOperation, Roles};
use nf_kernel::{AuthorityContext, CommittedBatch, JobResult, MissingPolicy, Rejection, Settlement};

#[derive(Clone)]
pub(super) enum Mutation {
    Prepare(Prepare),
    Settlement { results: Vec<JobResult>, missing: MissingPolicy },
    Drain,
    Cancel { requested: u64 },
    Delivered(OperationId),
    Membership { expected: Option<u64>, member: MembershipState },
    Quota(u32),
    Compact,
}
pub(super) enum Derived {
    Paused(Vec<JobId>),
    Applied { state: FullState, batch: Option<CommittedBatch>, cancellations: [u8; 32], fence: bool },
}
impl Mutation {
    pub fn purpose(&self) -> u32 {
        match self { Self::Prepare(_) => 1, Self::Settlement { .. } | Self::Drain | Self::Cancel { .. } => 2,
            Self::Delivered(_) => 3, Self::Membership { .. } => 5, Self::Quota(_) => 6, Self::Compact => 7 }
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Encoder::plain(1_114_112);
        match self {
            Self::Prepare(value) => return value.encode(),
            Self::Settlement { results, missing } => { w.byte(1)?; w.blob(&settlement::encode(results, *missing)?, 1_048_576)?; },
            Self::Drain => w.byte(2)?,
            Self::Cancel { requested } => { w.byte(3)?; w.byte(1)?; w.u64(*requested)?; },
            Self::Delivered(operation) => w.raw(operation.as_bytes())?,
            Self::Membership { expected, member } => {
                w.option(*expected, |w, revision| w.u64(revision))?;
                w.blob(&nf_identity::codec::encode_state(member)?, 262_144)?;
            },
            Self::Quota(quota) => w.u32(*quota)?, Self::Compact => {},
        }
        Ok(w.finish())
    }
    pub fn decode(purpose: u32, bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::plain(bytes, 1_114_112)?;
        let value = match purpose {
            1 => return Ok(Self::Prepare(Prepare::decode(bytes)?)),
            2 => match r.byte()? {
                1 => { let (results, missing) = settlement::decode(r.blob(1_048_576)?)?;
                    Self::Settlement { results, missing } },
                2 => Self::Drain,
                3 => { if r.byte()? != 1 { return Err(ManagedError::Corrupt); }
                    Self::Cancel { requested: r.u64()? } },
                _ => return Err(ManagedError::Corrupt),
            },
            3 => Self::Delivered(OperationId::from_bytes(r.fixed()?)),
            5 => Self::Membership { expected: r.option(Reader::u64)?,
                member: nf_identity::codec::decode_state(r.blob(262_144)?)? },
            6 => Self::Quota(r.u32()?), 7 => Self::Compact,
            _ => return Err(ManagedError::OperationConflict),
        };
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
    pub fn derive(&self, before: &FullState, foot: &StorageFoot, owner: &Owned) -> Result<Derived> {
        self.derive_at(before, foot.profile, foot.known.store_revision, foot.head, owner)
    }
    pub fn derive_at(&self, before: &FullState, profile: u32, old_revision: u64, old_head: [u8; 32], owner: &Owned) -> Result<Derived> {
        before.validate(old_revision)?;
        let member_revision = before.member.as_ref().map(|m| m.revision);
        let member_digest = before.member.as_ref().map(nf_identity::codec::encode_state).transpose()?.as_deref().map(hash);
        if owner.purpose != self.purpose() || owner.revision != old_revision
            || owner.state != hash(&before.encode()?) || owner.head != old_head || owner.stamp.profile != profile
            || owner.stamp.schema != if profile == 1 { crate::schema::schema_digest() } else { super::schema::active_schema_digest() } || owner.stamp.ruleset != before.core.world.to_spec().ruleset_hash
            || owner.stamp.membership_revision != member_revision
            || Some(owner.stamp.membership_digest) != member_digest
        { return Err(ManagedError::StaleSession); }
        let source = profile == 1;
        if !source && profile != 4 { return Err(ManagedError::UnsupportedFormat); }
        let mut after = before.clone(); let mut batch = None; let mut cancellations = [0; 32];
        let mut fence = false;
        match self {
            Self::Prepare(value) => {
                if value.frontier.authority() != authority(owner) { return Err(ManagedError::StaleSession); }
                after = value.derive(before, owner, source)?;
            },
            Self::Settlement { .. } | Self::Drain | Self::Cancel { .. } => {
                let frontier = before.core.pending.as_ref().ok_or(ManagedError::NotDrained)?;
                let cut = before.policy.ok_or(ManagedError::Corrupt)?;
                let (results, missing, context) = match self {
                    Self::Settlement { results, missing } => {
                        if frontier.authority() != authority(owner) { return Err(ManagedError::StaleSession); }
                        (results.clone(), *missing, authority(owner))
                    },
                    Self::Drain => {
                        if cut.policy != PendingPolicy::Drain || owner.times.issued >= cut.cutoff {
                            return Err(ManagedError::Expired);
                        }
                        (Vec::new(), MissingPolicy::Recompute, frontier.authority())
                    },
                    Self::Cancel { requested } => {
                        if source || *requested < cut.admitted || *requested > owner.times.issued {
                            return Err(ManagedError::OperationConflict);
                        }
                        let results = frontier.jobs().iter().map(|job| JobResult {
                            job: job.id(), result: Err(Rejection::ProviderFailed) }).collect();
                        (results, MissingPolicy::Pause, frontier.authority())
                    },
                    _ => return Err(ManagedError::Corrupt),
                };
                let result = nf_kernel::settle(&before.core.world, frontier, results, context, missing)?;
                let (world, committed) = match result {
                    Settlement::Paused(ids) => return Ok(Derived::Paused(ids)),
                    Settlement::Committed { world, batch } => (world, batch),
                };
                after.core = crate::transitions::commit(&before.core, &committed)?;
                if after.core.world != *world { return Err(ManagedError::DerivationMismatch); }
                after.policy = None;
                if let Self::Cancel { requested } = self {
                    let mut added = Vec::new();
                    for job in frontier.jobs() {
                        let intent = job.intent();
                        let record = AdministrativeCancellation { request: intent.request, operation: intent.operation,
                            job: intent.job, account: owner.stamp.account, device: owner.stamp.device,
                            term: owner.stamp.term, session: owner.stamp.session, sequence: owner.sequence,
                            revision: next(old_revision)?, policy: cut.policy.tag(),
                            admitted: cut.admitted, cutoff: cut.cutoff, requested: *requested };
                        if after.cancellations.insert(record.request, record.clone()).is_some() {
                            return Err(ManagedError::OperationConflict);
                        }
                        added.push(record);
                    }
                    added.sort_unstable_by_key(|record| record.request);
                    if !added.is_empty() {
                        let bytes = added.into_iter().flat_map(|record| record.canonical_bytes()).collect::<Vec<_>>();
                        cancellations = hash(&bytes);
                    }
                }
                batch = Some(committed);
            },
            Self::Delivered(operation) => {
                if after.core.outbox.remove(operation).is_none() { return Err(ManagedError::OperationConflict); }
            },
            Self::Membership { expected, member } => {
                let old = before.member.as_ref().ok_or(ManagedError::Unauthorized)?;
                if *expected != Some(old.revision) || member.revision != next(old.revision)?
                    || member.scope != old.scope || member.owner != old.owner
                    || !member.accounts.contains_key(&old.owner)
                { return Err(ManagedError::Unauthorized); }
                nf_identity::codec::encode_state(member)?;
                after.member = Some(member.clone());
                fence = member.devices.get(&owner.stamp.device).is_none_or(|device| {
                    device.revoked || device.account != owner.stamp.account
                }) || member.accounts.get(&owner.stamp.account).is_none_or(|account| {
                    !account.roles.contains(Roles::AUTHORITY_CANDIDATE)
                });
            },
            Self::Quota(quota) => {
                if *quota == before.quota { return Err(ManagedError::Storage(crate::StoreError::InvalidTransition)); }
                if *quota == 0 || *quota > before.quota { return Err(ManagedError::Limit); }
                after.quota = *quota;
            },
            Self::Compact => {},
        }
        let revision = if source && matches!(self, Self::Membership { .. } | Self::Quota(_) | Self::Compact) {
            old_revision
        } else { next(old_revision)? };
        after.validate(revision)?;
        Ok(Derived::Applied { state: after, batch, cancellations, fence })
    }
    pub fn role(&self) -> ProtectedOperation {
        if matches!(self, Self::Membership { .. } | Self::Quota(_) | Self::Compact | Self::Cancel { .. }) {
            ProtectedOperation::Administration
        } else { ProtectedOperation::Economic }
    }
}
pub(super) fn authority(owner: &Owned) -> AuthorityContext {
    AuthorityContext { term: AuthorityTerm(owner.stamp.term), session: RuntimeSession(owner.stamp.session) }
}
pub(super) fn verify_owner(before: &FullState, owner: &Owned, proof: &DeviceProof, role: ProtectedOperation) -> Result<()> {
    let member = before.member.as_ref().ok_or(ManagedError::Unauthorized)?;
    let account = member.accounts.get(&owner.stamp.account).ok_or(ManagedError::Unauthorized)?;
    let device = member.devices.get(&owner.stamp.device).ok_or(ManagedError::Unauthorized)?;
    if member.owner != owner.stamp.account || !account.roles.contains(Roles::AUTHORITY_CANDIDATE)
        || device.account != owner.stamp.account || device.revoked
    { return Err(ManagedError::Unauthorized); }
    member.authorize(proof, &device.peer, &hash(&owner.transcript()?), member.revision, role)?;
    Ok(())
}
