use super::{auth, book::{self, Book}, bytes::{hash, next}, catalog::{Catalog, PendingChange},
    catalog_action::Action, catalog_payload::Payload, catalog_rules, challenge::{Binding, Challenge, Owned},
    delta::Delta, effect, error::{ManagedError, Result}, file_io, input::FilePin, maps::ReceiptRow,
    mutation::{Derived, Mutation}, pending::Pending, receipt::Receipt, registry::Manifest,
    stamp::OwnerStamp, summary::Summary, upgrade::Descriptor, world_file::{Attempt, WorldFile}};
use nf_contract::identity::{AuthorityTerm, JobId, RuntimeSession};
use nf_identity::model::{DeviceProof, MembershipState, ProtectedOperation, PublicIdentity};
use std::collections::BTreeSet;

pub(super) struct PrepareReservation {
    pub sequence: u64, pub operation: [u8; 16], pub frontier: [u8; 32],
    pub stamp: OwnerStamp, pub state: [u8; 32], pub revision: u64, pub head: [u8; 32],
}
pub(super) struct Engine {
    pub catalog: Catalog,
    pub world: Option<WorldFile>,
    pub book: Book,
    pub stamp: Option<OwnerStamp>,
    pub prepare_reservation: Option<PrepareReservation>,
    pub provenance: String,
}
pub(super) enum ExecuteFailure { Paused(Vec<JobId>), Refused(ManagedError) }
impl From<ManagedError> for ExecuteFailure { fn from(value: ManagedError) -> Self { Self::Refused(value) } }
impl Engine {
    pub fn new(catalog: Catalog, world: Option<WorldFile>, stamp: Option<OwnerStamp>) -> Result<Self> {
        let provenance = Manifest::decode(&catalog.maps.registry.get(&1).ok_or(ManagedError::Corrupt)?.manifest)?.provenance;
        Ok(Self { catalog, world, book: Book::new()?, stamp, prepare_reservation: None, provenance })
    }
    pub fn world(&self) -> Result<&WorldFile> { self.world.as_ref().ok_or(ManagedError::ReconciliationRequired) }
    pub fn member(&self) -> Result<&MembershipState> {
        self.world()?.state.member.as_ref().ok_or(ManagedError::Unauthorized)
    }
    pub fn owned_stamp(&self) -> Result<&OwnerStamp> { self.stamp.as_ref().ok_or(ManagedError::OwnerMismatch) }
    /// Actual gate failure is absorbing; caller-input checks run outside this boundary.
    pub fn guard<T>(&mut self, check: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        match check(self) { Ok(value) => Ok(value), Err(error) => { self.stop(); Err(error) } }
    }
    pub fn gate(&mut self, sealed: bool) -> Result<()> { self.guard(|engine| engine.verify_owned(sealed)) }
    pub fn verify_owned(&self, sealed: bool) -> Result<()> {
        self.catalog.verify_cached()?;
        if self.catalog.state.pending.is_some() { return Err(ManagedError::ReconciliationRequired); }
        let phase = self.catalog.state.phase;
        if if sealed { phase != 3 } else { !matches!(phase, 2 | 7) } { return Err(ManagedError::OwnerMismatch); }
        let stamp = self.owned_stamp()?; catalog_rules::stamp_owner(&self.catalog.state, stamp)?;
        let foot = self.catalog.state.current.as_ref().ok_or(ManagedError::Corrupt)?;
        let world = self.world()?; world.verify(foot, self.catalog.state.retirement)?;
        let member = world.state.member.as_ref().ok_or(ManagedError::Unauthorized)?;
        let account = member.accounts.get(&stamp.account).ok_or(ManagedError::Unauthorized)?;
        let device = member.devices.get(&stamp.device).ok_or(ManagedError::Unauthorized)?;
        if member.owner != stamp.account || device.account != stamp.account || device.revoked
            || !account.roles.contains(nf_identity::model::Roles::AUTHORITY_CANDIDATE)
            || (sealed && !account.roles.contains(nf_identity::model::Roles::MODERATOR)) {
            return Err(ManagedError::Unauthorized);
        }
        if stamp.ruleset != world.state.core.world.to_spec().ruleset_hash { return Err(ManagedError::StaleSession); }
        let kind = if foot.profile == 1 { 1 } else { 3 };
        let manifest = Manifest::decode(&self.catalog.maps.registry.get(&kind).ok_or(ManagedError::Corrupt)?.manifest)?;
        let descriptor = Descriptor::read(&manifest.composition)?;
        if descriptor.schema != foot.schema || descriptor.storage != foot.profile || descriptor.digest() != stamp.ruleset {
            return Err(ManagedError::UnsupportedFormat);
        }
        descriptor.validate_registry(&world.state.core.world, &descriptor.providers)?;
        self.verify_binary(if foot.profile == 1 { 4 } else { 5 })
    }
    pub fn verify_binary(&self, kind: u32) -> Result<()> {
        let row = self.catalog.maps.registry.get(&kind).ok_or(ManagedError::Corrupt)?;
        let manifest = Manifest::decode(&row.manifest)?;
        file_io::verify(&FilePin::new(row.path.clone(), row.bytes, row.digest, &manifest.provenance)?)
    }
    pub fn issue_owned(&mut self, purpose: u32, payload: Vec<u8>, public: &PublicIdentity,
        cutoff: u64, identity: Option<(u64, [u8; 16])>) -> Result<Challenge> {
        if !matches!(purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8 | 11 | 12 | 15) { return Err(ManagedError::OperationConflict); }
        self.gate(matches!(purpose, 8 | 12))?;
        let stamp = self.owned_stamp()?.clone();
        if (public.account, public.device) != (stamp.account, stamp.device) { return Err(ManagedError::Unauthorized); }
        let admin = matches!(purpose, 5 | 6 | 7 | 8 | 11 | 12 | 15)
            || purpose == 2 && payload.first() == Some(&3);
        book::eligible(self.member()?, public, admin)?;
        let times = self.book.times(cutoff, self.catalog.state.utc)?;
        let (sequence, operation) = match identity { Some(value) => value, None => (next(self.catalog.state.consumed)?, book::random()?) };
        if sequence != next(self.catalog.state.consumed)? { return Err(ManagedError::Replay); }
        let foot = &self.world()?.foot;
        let owned = Owned { stamp, purpose, sequence, operation, revision: foot.known.store_revision,
            state: foot.state, head: foot.head, payload: hash(&payload), times: times.clone() };
        let transcript = owned.transcript()?;
        self.book.issue(Binding::Owned(owned), &times, transcript, Vec::new(), payload, Vec::new())
    }
    pub fn consume_owned(&mut self, proof: &DeviceProof, purpose: u32, payload: Option<&[u8]>,
        role: ProtectedOperation) -> Result<(Owned, Vec<u8>, Option<Vec<u8>>)> {
        self.gate(matches!(purpose, 8 | 12))?;
        let member = self.member()?.clone();
        let (binding, transcript, unsigned) = self.book.consume(proof, &member, role, true)?;
        let owned = match binding { Binding::Owned(value) if value.purpose == purpose => value,
            _ => return Err(ManagedError::OperationConflict) };
        catalog_rules::stamp_owner(&self.catalog.state, &owned.stamp)?;
        let foot = &self.world()?.foot;
        if owned.sequence != next(self.catalog.state.consumed)? || owned.revision != foot.known.store_revision
            || owned.state != foot.state || owned.head != foot.head || payload.is_some_and(|p| hash(p) != owned.payload)
        { return Err(ManagedError::StaleSession); }
        Ok((owned, transcript, unsigned))
    }
    pub fn execute(&mut self, mutation: &Mutation, owned: &Owned, proof: &DeviceProof)
        -> std::result::Result<Receipt, ExecuteFailure> {
        self.gate(false)?;
        let before = self.catalog.state.clone(); let frontier = self.catalog.frontier;
        let world = self.world()?;
        let result = mutation.derive(&world.state, &world.foot, owned)?;
        let (state, batch, cancellations, fence) = match result {
            Derived::Paused(ids) => return Err(ExecuteFailure::Paused(ids)),
            Derived::Applied { state, batch, cancellations, fence } => (state, batch, cancellations, fence),
        };
        let payload = mutation.encode()?; let action_bytes = auth::action(owned, &payload, proof)?;
        let plan = world.plan(state, mutation, &action_bytes, batch.as_ref())?;
        let profile = super::world_file::profile(world.foot.profile)?;
        let post = plan.foot(&world.connection, profile, world.foot.lineage, &BTreeSet::new())?;
        let pages: u32 = world.connection.pragma_query_value(None, "page_count", |r| r.get(0))
            .map_err(ManagedError::from)?;
        if post.quota < pages { return Err(ManagedError::Limit.into()); }
        if world.foot.profile == 4 && matches!(mutation, Mutation::Quota(_)) {
            super::page_room::active_quota(&world.connection, &plan, post.quota)?;
        }
        let effect = effect::derive(&world.connection, &world.foot, &post, owned.purpose, &plan.patches)?;
        let utc = book::now_ms()?;
        if utc < before.utc || utc < owned.times.issued || utc >= owned.times.expires {
            return Err(ManagedError::Expired.into());
        }
        if matches!(mutation, Mutation::Prepare(value) if utc >= value.cut.cutoff)
            || matches!(mutation, Mutation::Drain) && world.state.policy.is_none_or(|cut| utc >= cut.cutoff) {
            return Err(ManagedError::Expired.into());
        }
        let receipt = Receipt { sequence: owned.sequence, operation: owned.operation, purpose: owned.purpose,
            result: 1, foot: post.clone(), action: hash(&action_bytes), revision: frontier.revision.checked_add(2)
                .ok_or(ManagedError::Limit)?, cancellations };
        let delta = Delta::outcome(&self.catalog.maps, ReceiptRow::new(&receipt)?)?;
        let maps = delta.apply(&self.catalog.maps, super::delta::RegistryPurpose::None, Some(&ReceiptRow::new(&receipt)?))?;
        let mut intended = before.clone(); intended.current = Some(post.clone()); intended.consumed = owned.sequence;
        intended.pending = None; intended.utc = utc; intended.receipts = maps.digests()?.1;
        intended.source_policy = if post.profile == 1 { post.policy } else { None };
        if fence { intended.phase = if post.profile == 1 { 1 } else { 6 }; intended.owner = None; }
        let pending = Pending { sequence: owned.sequence, operation: owned.operation, purpose: owned.purpose,
            stamp: owned.stamp.clone(), before: frontier, artifact: before.selected.unwrap_or(before.source),
            pre: world.foot.clone(), post: post.clone(), pre_state: world.envelope_bytes.clone(),
            post_state: plan.state.encode()?, action: action_bytes, intended: intended.clone(), effect, stage: 1 };
        let summary = Summary::from_pending(&pending)?;
        let mut admitted = before.clone(); admitted.pending = Some(pending.digest()?);
        admitted.consumed = owned.sequence; admitted.utc = utc;
        if let Err(error) = self.catalog.append(Action { payload: Payload::Pending(summary.clone()), delta: Delta::default() },
            admitted, PendingChange::Set(pending.clone())) { self.stop(); return Err(error.into()); }
        let expired_drain = matches!(mutation, Mutation::Drain) && self.world()?.state.policy
            .is_none_or(|cut| book::now_ms().map_or(true, |now| now >= cut.cutoff));
        let attempted = if expired_drain { Attempt::KnownPre(ManagedError::Expired) }
            else if pending.pre.profile == 1 && matches!(mutation, Mutation::Quota(_)) {
                self.world.as_mut().ok_or(ManagedError::ReconciliationRequired)?.apply_source_quota(&plan, &post)
            } else { self.world.as_mut().ok_or(ManagedError::ReconciliationRequired)?.apply(&plan, &post) };
        match attempted {
            Attempt::KnownPost => {
                if let Err(error) = self.catalog.append(Action { payload: Payload::Outcome {
                    summary, receipt: receipt.clone(), disposition: if fence { 2 } else { 1 } }, delta },
                    intended, PendingChange::Clear) { self.stop(); return Err(error.into()); }
                self.reset_after_outcome()?; Ok(receipt)
            },
            Attempt::KnownPre(error) => {
                let abort = Receipt { result: 2, foot: pending.pre.clone(), cancellations: [0; 32], ..receipt };
                let delta = Delta::outcome(&self.catalog.maps, ReceiptRow::new(&abort)?)?;
                let maps = delta.apply(&self.catalog.maps, super::delta::RegistryPurpose::None, Some(&ReceiptRow::new(&abort)?))?;
                let mut restored = before; restored.consumed = owned.sequence; restored.utc = utc;
                restored.receipts = maps.digests()?.1;
                if self.world()?.verify(&pending.pre, None).is_err() || self.catalog.append(Action {
                    payload: Payload::Abort { summary, receipt: abort, disposition: 1 }, delta },
                    restored, PendingChange::Clear).is_err() {
                    self.stop(); return Err(ManagedError::ReconciliationRequired.into());
                }
                self.reset_after_outcome()?; Err(error.into())
            },
            Attempt::Unknown(error) => { self.stop(); Err(error.into()) },
        }
    }
    pub fn reset_after_outcome(&mut self) -> Result<()> {
        self.book.clear(); self.prepare_reservation = None;
        if self.catalog.state.owner.is_none() { self.stamp = None; return Ok(()); }
        let world = self.world()?; let old = self.owned_stamp()?;
        self.stamp = Some(OwnerStamp { membership_revision: world.foot.known.membership_revision,
            membership_digest: world.foot.member.ok_or(ManagedError::Corrupt)?, bytes: Vec::new(), ..old.clone() }.seal()?);
        Ok(())
    }
    pub fn stop(&mut self) {
        self.book.clear(); self.prepare_reservation = None; self.stamp = None; self.catalog.quarantined = true;
        if let Some(world) = &mut self.world { world.quarantined = true; }
    }
    pub fn authority(&self) -> Result<nf_kernel::AuthorityContext> {
        let stamp = self.owned_stamp()?;
        Ok(nf_kernel::AuthorityContext { term: AuthorityTerm(stamp.term), session: RuntimeSession(stamp.session) })
    }
}
