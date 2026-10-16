use super::{
    admission::{Input, Operation},
    auth, book,
    bytes::{hash, next},
    catalog::{Catalog, PendingChange},
    catalog_action::Action,
    catalog_payload::Payload,
    challenge::{Binding, Challenge},
    delta::{Delta, RegistryPurpose},
    engine::Engine,
    error::{ManagedError, Result},
    foot::StorageFoot,
    maps::ReceiptRow,
    pending::Pending,
    pending_open, pending_verify,
    prospective::Prospective,
    receipt::Receipt,
    retire::{self, RetireAttempt},
    retirement::Retirement,
    stamp::{CatalogFrontier, OwnerStamp},
    summary::Summary,
};
use nf_identity::model::{DeviceProof, ProtectedOperation, PublicIdentity};
use std::path::PathBuf;

struct Resolution {
    engine: Engine,
    pending: Pending,
    post: bool,
    cancellations: [u8; 32],
}
/// Opaque recovery custody. It cannot expose or reconstruct a business owner.
pub struct ReconciliationHandle {
    path: PathBuf,
    installation: [u8; 16],
    protected: CatalogFrontier,
    pending: Option<[u8; 32]>,
    resolution: Option<Resolution>,
}
#[derive(Clone, Debug)]
pub struct ReconciledStatus {
    frontier: CatalogFrontier,
    phase: u32,
    foot: Option<StorageFoot>,
    pending: Option<[u8; 32]>,
}
impl ReconciledStatus {
    pub fn catalog_frontier(&self) -> CatalogFrontier {
        self.frontier
    }
    pub fn phase(&self) -> u32 {
        self.phase
    }
    pub fn storage_foot(&self) -> Option<&StorageFoot> {
        self.foot.as_ref()
    }
    pub fn pending_identity(&self) -> Option<[u8; 32]> {
        self.pending
    }
}
impl ReconciliationHandle {
    pub(super) fn from_catalog(catalog: &Catalog, pending: Option<[u8; 32]>) -> Self {
        Self {
            path: catalog.path.clone(),
            installation: catalog.state.installation,
            protected: catalog.frontier,
            pending: pending.or(catalog.state.pending),
            resolution: None,
        }
    }
    pub(super) fn from_engine(mut engine: Engine) -> Self {
        let handle = Self::from_catalog(&engine.catalog, None);
        engine.stop();
        drop(engine);
        handle
    }
    pub(super) fn for_adoption(path: PathBuf, installation: [u8; 16]) -> Self {
        Self {
            path,
            installation,
            protected: CatalogFrontier {
                revision: 0,
                state_digest: [0; 32],
                head: [0; 32],
            },
            pending: None,
            resolution: None,
        }
    }
    pub fn pending_identity(&self) -> Option<[u8; 32]> {
        self.pending
    }
    pub fn inspect_status(&self) -> Result<ReconciledStatus> {
        if self.resolution.is_some() {
            return Err(ManagedError::OperationConflict);
        }
        let catalog = Catalog::open(self.path.clone(), self.installation, self.protected)?;
        if catalog.pending.is_none() {
            let foot = catalog
                .state
                .current
                .as_ref()
                .ok_or(ManagedError::Corrupt)?;
            let kind = if foot.profile == 4 { 3 } else { 1 };
            let row = catalog
                .maps
                .registry
                .get(&kind)
                .ok_or(ManagedError::Corrupt)?;
            super::world_file::WorldFile::open_owned(
                row.path.clone(),
                foot,
                catalog.state.retirement,
            )?
            .close()?;
        }
        Ok(status(&catalog))
    }
    fn load(&mut self) -> Result<()> {
        if self.resolution.is_some() {
            return Ok(());
        }
        let catalog = Catalog::open(self.path.clone(), self.installation, self.protected)?;
        let pending = catalog
            .pending
            .clone()
            .ok_or(ManagedError::OperationConflict)?;
        if self
            .pending
            .is_some_and(|sha| catalog.state.pending != Some(sha))
        {
            return Err(ManagedError::OperationConflict);
        }
        let kind = if pending.pre.profile == 4 { 3 } else { 1 };
        let row = catalog
            .maps
            .registry
            .get(&kind)
            .ok_or(ManagedError::Corrupt)?;
        if row.artifact != pending.artifact {
            return Err(ManagedError::Corrupt);
        }
        let (world, post) = pending_open::open(row.path.clone(), &pending)?;
        if pending.purpose == 8 && post && pending.stage != 2 {
            return Err(ManagedError::ReconciliationRequired);
        }
        let engine = Engine::new(catalog, Some(world), None)?;
        let cancellations = pending_verify::verify(&engine, &pending, post)?;
        self.pending = Some(pending.digest()?);
        self.resolution = Some(Resolution {
            engine,
            pending,
            post,
            cancellations,
        });
        Ok(())
    }
    pub fn issue_reconciliation_challenge(
        &mut self,
        public: &PublicIdentity,
        cutoff: u64,
    ) -> Result<Challenge> {
        self.load()?;
        let resolution = self
            .resolution
            .as_mut()
            .ok_or(ManagedError::ReconciliationRequired)?;
        verify_resolution(resolution)?;
        let engine = &mut resolution.engine;
        let foot = engine.world()?.foot.clone();
        book::eligible(engine.member()?, public, true)?;
        let state = &engine.catalog.state;
        let candidate = OwnerStamp {
            installation: state.installation,
            generation: state.generation,
            account: public.account,
            device: public.device,
            incarnation: book::random()?,
            term: next(state.term)?,
            session: next(state.session)?,
            profile: foot.profile,
            schema: foot.schema,
            ruleset: engine.world()?.state.core.world.to_spec().ruleset_hash,
            membership_revision: foot.known.membership_revision,
            membership_digest: foot.member.ok_or(ManagedError::Unauthorized)?,
            bytes: Vec::new(),
        }
        .seal()?;
        let input = admission(resolution)?.encode()?;
        let digest = hash(&input);
        let times = resolution
            .engine
            .book
            .times(cutoff, resolution.engine.catalog.state.utc)?;
        let prospective = Prospective {
            purpose: 14,
            installation: candidate.installation,
            scope: foot.known.scope,
            catalog: Some(resolution.engine.catalog.frontier),
            candidate: candidate.clone(),
            foot,
            peer: public.peer.clone(),
            input: digest,
            times: times.clone(),
        };
        resolution.engine.book.issue(
            Binding::Prospective {
                purpose: 14,
                candidate,
                input: digest,
            },
            &times,
            prospective.encode()?,
            input,
            Vec::new(),
            Vec::new(),
        )
    }
    pub fn resolve_authenticated_once(mut self, proof: DeviceProof) -> Result<ReconciledStatus> {
        let mut resolution = self.resolution.take().ok_or(ManagedError::Replay)?;
        verify_resolution(&mut resolution)?;
        let member = resolution.engine.member()?.clone();
        let (binding, transcript, _) = resolution.engine.book.consume(
            &proof,
            &member,
            ProtectedOperation::Administration,
            true,
        )?;
        let (candidate, input) = match binding {
            Binding::Prospective {
                purpose: 14,
                candidate,
                input,
            } => (candidate, input),
            _ => return Err(ManagedError::OperationConflict),
        };
        let auth = Prospective::decode(&transcript)?;
        let utc = book::now_ms()?;
        if input != hash(&admission(&resolution)?.encode()?)
            || auth.catalog != Some(resolution.engine.catalog.frontier)
            || auth.candidate != candidate
            || auth.input != input
            || auth.foot != resolution.engine.world()?.foot
            || utc < auth.times.issued
            || utc >= auth.times.expires
        {
            return Err(ManagedError::StaleSession);
        }
        if resolution.pending.purpose == 8 {
            if !resolution.post {
                let (_, payload, _) = auth::decode_action(&resolution.pending.action)?;
                let record = Retirement::decode(&payload)?;
                let (plan, post) = retire::plan(&resolution.engine, &record)?;
                if post != resolution.pending.post {
                    return Err(ManagedError::DerivationMismatch);
                }
                match retire::attempt(&mut resolution.engine, &plan, &resolution.pending) {
                    RetireAttempt::KnownPost(pending) => resolution.pending = pending,
                    RetireAttempt::KnownPre(_) | RetireAttempt::Unknown(_) => {
                        return Err(ManagedError::ReconciliationRequired);
                    }
                }
            }
            let receipt = receipt(&resolution, 3, resolution.pending.post.clone())?;
            let delta =
                Delta::outcome(&resolution.engine.catalog.maps, ReceiptRow::new(&receipt)?)?;
            retire::finish(&mut resolution.engine, &resolution.pending, receipt, delta)?;
        } else if resolution.post {
            let receipt = receipt(&resolution, 1, resolution.pending.post.clone())?;
            let delta =
                Delta::outcome(&resolution.engine.catalog.maps, ReceiptRow::new(&receipt)?)?;
            resolution.engine.catalog.append(
                Action {
                    payload: Payload::Outcome {
                        summary: Summary::from_pending(&resolution.pending)?,
                        receipt,
                        disposition: if resolution.pending.intended.owner.is_none() {
                            2
                        } else {
                            1
                        },
                    },
                    delta,
                },
                resolution.pending.intended.clone(),
                PendingChange::Clear,
            )?;
        } else {
            let receipt = receipt(&resolution, 2, resolution.pending.pre.clone())?;
            let delta =
                Delta::outcome(&resolution.engine.catalog.maps, ReceiptRow::new(&receipt)?)?;
            let maps = delta.apply(
                &resolution.engine.catalog.maps,
                RegistryPurpose::None,
                Some(&ReceiptRow::new(&receipt)?),
            )?;
            let mut after = resolution.engine.catalog.state.clone();
            after.pending = None;
            after.current = Some(resolution.pending.pre.clone());
            after.source_policy = if resolution.pending.pre.profile == 1 {
                resolution.pending.pre.policy
            } else {
                None
            };
            after.receipts = maps.digests()?.1;
            resolution.engine.catalog.append(
                Action {
                    payload: Payload::Abort {
                        summary: Summary::from_pending(&resolution.pending)?,
                        receipt,
                        disposition: 1,
                    },
                    delta,
                },
                after,
                PendingChange::Clear,
            )?;
        }
        resolution.engine.book.clear();
        resolution.engine.stamp = None;
        Ok(status(&resolution.engine.catalog))
    }
}
fn verify_resolution(resolution: &mut Resolution) -> Result<()> {
    let pending = resolution.pending.clone();
    let post = resolution.post;
    let actual = resolution
        .engine
        .guard(|engine| pending_verify::verify(engine, &pending, post))?;
    if actual != resolution.cancellations {
        resolution.engine.stop();
        return Err(ManagedError::DerivationMismatch);
    }
    Ok(())
}
fn receipt(resolution: &Resolution, result: u8, foot: StorageFoot) -> Result<Receipt> {
    Ok(Receipt {
        sequence: resolution.pending.sequence,
        operation: resolution.pending.operation,
        purpose: resolution.pending.purpose,
        result,
        foot,
        action: hash(&resolution.pending.action),
        revision: next(resolution.engine.catalog.frontier.revision)?,
        cancellations: if result == 1 {
            resolution.cancellations
        } else {
            [0; 32]
        },
    })
}
fn admission(resolution: &Resolution) -> Result<Input> {
    let pending = &resolution.pending;
    let foot = resolution.engine.world()?.foot.clone();
    Ok(Input {
        installation: resolution.engine.catalog.state.installation,
        scope: foot.known.scope,
        foot: foot.clone(),
        composition: Vec::new(),
        catalog: resolution.engine.catalog.path.clone(),
        delta: Delta::default(),
        operation: Operation::Reconcile {
            sequence: pending.sequence,
            operation: pending.operation,
            purpose: pending.purpose,
            stage: pending.stage,
            pending: pending.digest()?,
            outcome: if resolution.post { 2 } else { 1 },
            verified: foot,
            pre: pending.pre.clone(),
            post: pending.post.clone(),
        },
    })
}
fn status(catalog: &Catalog) -> ReconciledStatus {
    ReconciledStatus {
        frontier: catalog.frontier,
        phase: catalog.state.phase,
        foot: catalog.state.current.clone(),
        pending: catalog.state.pending,
    }
}
