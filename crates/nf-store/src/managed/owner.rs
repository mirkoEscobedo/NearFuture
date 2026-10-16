use super::{
    backup,
    challenge::Challenge,
    engine::Engine,
    error::{ManagedError, Result},
    failure::{PrecommitFailure, SettlementFailure},
    input::{FilePin, PrepareInput, SettlementInput},
    mutation::Mutation,
    observations::{self, ManagedOutboxRecord, ManagedRequestStatus, ManagedSnapshot, OwnerData},
    owner_ops,
    receipt::Receipt,
    sealed::ManagedDrainedSource,
    stamp::{CatalogFrontier, OwnerStamp},
    state::QuiescePolicy,
};
use crate::{DurableAck, KnownFrontiers};
use nf_contract::identity::{AccountId, DeviceId, OperationId, RequestId};
use nf_identity::model::{DeviceProof, MembershipState, PublicIdentity, Scope};
use nf_kernel::{AuthorityContext, Frontier, JobResult, World};
use std::{cell::RefCell, path::PathBuf};

pub(super) struct Owner {
    pub engine: RefCell<Engine>,
    pub data: OwnerData,
}
pub struct SourceOwner {
    pub(super) inner: Owner,
}
pub struct ActiveOwner {
    pub(super) inner: Owner,
}
pub struct CurrentOwner {
    pub(super) inner: Owner,
}
impl Owner {
    fn new(mut engine: Engine) -> Result<Self> {
        engine.gate(false)?;
        let data = OwnerData::from_engine(&engine)?;
        Ok(Self {
            engine: RefCell::new(engine),
            data,
        })
    }
    fn gate(&mut self) -> Result<()> {
        self.engine.get_mut().gate(false)
    }
    fn execute(&mut self, mutation: Mutation, proof: DeviceProof) -> Result<Receipt> {
        self.gate()?;
        owner_ops::execute(self.engine.get_mut(), &mut self.data, mutation, proof).map_err(
            |failure| match failure {
                SettlementFailure::Refused(error) => error,
                SettlementFailure::Paused(_) => ManagedError::OperationConflict,
            },
        )
    }
    fn issue(
        &mut self,
        mutation: Mutation,
        public: &PublicIdentity,
        cutoff: u64,
    ) -> Result<Challenge> {
        self.gate()?;
        self.engine.get_mut().issue_owned(
            mutation.purpose(),
            mutation.encode()?,
            public,
            cutoff,
            None,
        )
    }
}
fn ack(receipt: &Receipt) -> DurableAck {
    DurableAck {
        revision: receipt.foot.known.store_revision,
        sequence: receipt.foot.known.event_sequence,
        state_hash: receipt.foot.core,
    }
}
macro_rules! owner_surface {
    ($kind:ident) => {
        impl $kind {
            /// Bounded last positively verified data, never a fresh current-owner or health witness.
            pub fn world(&self) -> &World {
                &self.inner.data.state.core.world
            }
            pub fn core_bytes(&self) -> &[u8] {
                &self.inner.data.core
            }
            pub fn envelope_bytes(&self) -> &[u8] {
                &self.inner.data.envelope
            }
            pub fn known_frontiers(&self) -> KnownFrontiers {
                self.inner.data.foot.known
            }
            pub fn storage_foot(&self) -> &super::foot::StorageFoot {
                &self.inner.data.foot
            }
            pub fn owner_stamp(&self) -> &OwnerStamp {
                &self.inner.data.stamp
            }
            pub fn catalog_frontier(&self) -> CatalogFrontier {
                self.inner.data.frontier
            }
            pub fn authority(&self) -> AuthorityContext {
                AuthorityContext {
                    term: nf_contract::identity::AuthorityTerm(self.inner.data.stamp.term),
                    session: nf_contract::identity::RuntimeSession(self.inner.data.stamp.session),
                }
            }
            pub fn outbox(&self) -> impl Iterator<Item = ManagedOutboxRecord> + '_ {
                observations::outbox(&self.inner.data)
            }
            pub fn read_snapshot(&mut self) -> Result<ManagedSnapshot> {
                self.inner.gate()?;
                observations::snapshot(self.inner.engine.get_mut())
            }
            pub fn query_bound(
                &self,
                request: RequestId,
                account: AccountId,
                device: DeviceId,
                scope: Scope,
            ) -> Result<Option<ManagedRequestStatus>> {
                // The private engine borrow never escapes. Immutable public data access still
                // performs a fresh actual gate and can clear all live authority on failure.
                let mut engine = self
                    .inner
                    .engine
                    .try_borrow_mut()
                    .map_err(|_| ManagedError::Busy)?;
                observations::query(&mut engine, request, account, device, scope)
            }
            pub fn read_catalog(
                &self,
            ) -> Result<super::catalog_observation::ManagedCatalogObservation> {
                let mut engine = self
                    .inner
                    .engine
                    .try_borrow_mut()
                    .map_err(|_| ManagedError::Busy)?;
                super::catalog_observation::read(&mut engine)
            }
            pub fn operation_receipt(
                &self,
                sequence: u64,
                operation: [u8; 16],
            ) -> Result<Option<Receipt>> {
                self.read_catalog()?.exact_receipt(sequence, operation)
            }
            pub fn admit_worker_reply(
                &mut self,
                stamp: &OwnerStamp,
                result: &JobResult,
            ) -> Result<()> {
                self.inner.gate()?;
                if &self.inner.data.stamp != stamp {
                    return Err(ManagedError::StaleSession);
                }
                owner_ops::worker_reply(self.inner.engine.get_mut(), stamp, result)
            }
            pub fn issue_principal_challenge(
                &mut self,
                frontier: &Frontier,
                request: RequestId,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner.gate()?;
                owner_ops::principal(
                    self.inner.engine.get_mut(),
                    frontier,
                    request,
                    public,
                    cutoff,
                )
            }
            pub fn issue_prepare_challenge(
                &mut self,
                input: &PrepareInput<'_>,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner.gate()?;
                owner_ops::issue_prepare(self.inner.engine.get_mut(), input, public, cutoff)
            }
            pub fn prepare(
                &mut self,
                input: PrepareInput<'_>,
                proof: DeviceProof,
            ) -> Result<DurableAck> {
                self.inner.gate()?;
                owner_ops::prepare(
                    self.inner.engine.get_mut(),
                    &mut self.inner.data,
                    input,
                    proof,
                )
                .map(|receipt| ack(&receipt))
            }
            pub fn issue_settlement_challenge(
                &mut self,
                input: &SettlementInput,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner.gate()?;
                owner_ops::issue_settlement(self.inner.engine.get_mut(), input, public, cutoff)
            }
            pub fn settle_current(
                &mut self,
                input: SettlementInput,
                proof: DeviceProof,
            ) -> std::result::Result<DurableAck, SettlementFailure> {
                self.inner.gate()?;
                owner_ops::settle(
                    self.inner.engine.get_mut(),
                    &mut self.inner.data,
                    input,
                    proof,
                )
                .map(|receipt| ack(&receipt))
            }
            pub fn issue_drain_challenge(
                &mut self,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner.issue(Mutation::Drain, public, cutoff)
            }
            pub fn drain_persisted_locally(&mut self, proof: DeviceProof) -> Result<DurableAck> {
                self.inner
                    .execute(Mutation::Drain, proof)
                    .map(|receipt| ack(&receipt))
            }
            pub fn issue_delivery_challenge(
                &mut self,
                operation: OperationId,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner
                    .issue(Mutation::Delivered(operation), public, cutoff)
            }
            pub fn deliver_outbox(
                &mut self,
                operation: OperationId,
                proof: DeviceProof,
            ) -> Result<Receipt> {
                self.inner.execute(Mutation::Delivered(operation), proof)
            }
            pub fn issue_membership_challenge(
                &mut self,
                expected: Option<u64>,
                member: &MembershipState,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner.issue(
                    Mutation::Membership {
                        expected,
                        member: member.clone(),
                    },
                    public,
                    cutoff,
                )
            }
            pub fn commit_membership(
                &mut self,
                expected: Option<u64>,
                member: MembershipState,
                proof: DeviceProof,
            ) -> Result<Receipt> {
                self.inner
                    .execute(Mutation::Membership { expected, member }, proof)
            }
            pub fn issue_quota_challenge(
                &mut self,
                quota: u32,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner.issue(Mutation::Quota(quota), public, cutoff)
            }
            pub fn lower_page_limit(&mut self, quota: u32, proof: DeviceProof) -> Result<Receipt> {
                self.inner.execute(Mutation::Quota(quota), proof)
            }
            pub fn issue_compact_challenge(
                &mut self,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner.issue(Mutation::Compact, public, cutoff)
            }
            pub fn compact(&mut self, proof: DeviceProof) -> Result<Receipt> {
                self.inner.execute(Mutation::Compact, proof)
            }
            pub fn issue_backup_challenge(
                &mut self,
                path: PathBuf,
                minimum: KnownFrontiers,
                public: &PublicIdentity,
                cutoff: u64,
            ) -> Result<Challenge> {
                self.inner.gate()?;
                backup::issue(self.inner.engine.get_mut(), path, minimum, public, cutoff)
            }
            pub fn bounded_backup(&mut self, proof: DeviceProof) -> Result<FilePin> {
                self.inner.gate()?;
                backup::execute(self.inner.engine.get_mut(), proof)
            }
            pub fn graceful_close(mut self) -> Result<CatalogFrontier> {
                self.inner.gate()?;
                let mut engine = self.inner.engine.into_inner();
                let world = engine
                    .world
                    .take()
                    .ok_or(ManagedError::ReconciliationRequired)?;
                if let Err(error) = world.close() {
                    engine.stop();
                    return Err(error);
                }
                engine.book.clear();
                engine.stamp = None;
                engine.catalog.close()
            }
        }
    };
}
owner_surface!(SourceOwner);
owner_surface!(ActiveOwner);
owner_surface!(CurrentOwner);
impl SourceOwner {
    pub(super) fn from_engine(engine: Engine) -> Result<Self> {
        if engine.world()?.foot.profile != 1 {
            return Err(ManagedError::UnsupportedFormat);
        }
        Ok(Self {
            inner: Owner::new(engine)?,
        })
    }
    pub fn issue_quiesce_challenge(
        &mut self,
        policy: QuiescePolicy,
        public: &PublicIdentity,
        cutoff: u64,
    ) -> Result<Challenge> {
        super::sealed::issue_quiesce(self.inner.engine.get_mut(), policy, public, cutoff)
    }
    pub fn quiesce_and_seal(
        self,
        policy: QuiescePolicy,
        minimum: KnownFrontiers,
        proof: DeviceProof,
    ) -> std::result::Result<ManagedDrainedSource, PrecommitFailure<Self>> {
        super::sealed::quiesce(self, policy, minimum, proof)
    }
}
impl ActiveOwner {
    pub fn issue_cancel_challenge(
        &mut self,
        requested: u64,
        public: &PublicIdentity,
        cutoff: u64,
    ) -> Result<Challenge> {
        self.inner
            .issue(Mutation::Cancel { requested }, public, cutoff)
    }
    pub fn cancel_pending_admin(&mut self, requested: u64, proof: DeviceProof) -> Result<Receipt> {
        self.inner.execute(Mutation::Cancel { requested }, proof)
    }
}
impl CurrentOwner {
    pub(super) fn from_engine(engine: Engine) -> Result<Self> {
        Ok(Self {
            inner: Owner::new(engine)?,
        })
    }
    pub fn into_source(mut self) -> std::result::Result<SourceOwner, Self> {
        if self.inner.gate().is_err() {
            return Err(self);
        }
        if self.inner.data.foot.profile == 1 {
            Ok(SourceOwner { inner: self.inner })
        } else {
            Err(self)
        }
    }
    pub fn into_active(mut self) -> std::result::Result<ActiveOwner, Self> {
        if self.inner.gate().is_err() {
            return Err(self);
        }
        if self.inner.data.foot.profile == 4 {
            Ok(ActiveOwner { inner: self.inner })
        } else {
            Err(self)
        }
    }
    pub fn issue_cancel_challenge(
        &mut self,
        requested: u64,
        public: &PublicIdentity,
        cutoff: u64,
    ) -> Result<Challenge> {
        self.inner.gate()?;
        if self.inner.data.foot.profile != 4 {
            return Err(ManagedError::UnsupportedFormat);
        }
        self.inner
            .issue(Mutation::Cancel { requested }, public, cutoff)
    }
    pub fn cancel_pending_admin(&mut self, requested: u64, proof: DeviceProof) -> Result<Receipt> {
        self.inner.gate()?;
        if self.inner.data.foot.profile != 4 {
            return Err(ManagedError::UnsupportedFormat);
        }
        self.inner.execute(Mutation::Cancel { requested }, proof)
    }
}
