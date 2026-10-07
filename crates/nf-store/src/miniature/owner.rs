use super::{auth::AuthRuntime, error::Result, model::*};
use crate::schema::{counter, hash};
use nf_identity::{codec::encode_state, model::MembershipState};
use nf_kernel::miniature::MiniatureWorld;
use rusqlite::{Connection, params};
use std::path::Path;
pub struct MiniatureStore {
    pub(crate) connection: Connection,
    pub(crate) state: super::state::State,
    pub(crate) revision: u64,
    pub(crate) head: [u8; 32],
    pub(crate) quarantined: bool,
    pub(crate) runtime: AuthRuntime,
}
impl MiniatureStore {
    pub fn create(
        path: impl AsRef<Path>,
        spec: MiniatureGenesisSpec,
        membership: &[u8],
        policy: BootstrapPolicy,
        signer: &mut impl BootstrapSigner,
    ) -> Result<Self> {
        let mut runtime = AuthRuntime::new(policy.auth)?;
        let (world, membership, evidence) =
            super::bootstrap::genesis(spec, membership, &policy, signer, &mut runtime)?;
        let state = super::state::State::initial(world.clone());
        let body = super::codec::encode(&state)?;
        crate::schema::reserve(path.as_ref())?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        crate::schema::configure(&connection)?;
        initialize(&mut connection, &world, &membership, &body, &evidence)?;
        Ok(Self {
            connection,
            state,
            revision: 0,
            head: [0; 32],
            quarantined: false,
            runtime,
        })
    }
    pub fn world(&self) -> &MiniatureWorld {
        &self.state.world
    }
    /// Bounded public persisted data; these bytes grant no authorization.
    pub fn snapshot_bytes(&self) -> Result<Vec<u8>> {
        self.ensure()?;
        super::codec::encode(&self.state)
    }
    pub fn authority(&self) -> Option<MiniatureAuthority> {
        self.state.authority
    }
    pub fn pending(&self) -> Option<&nf_kernel::miniature::MiniatureFrontier> {
        self.state.pending.as_ref()
    }
    pub fn known_frontiers(&self) -> Result<MiniatureKnownFrontiers> {
        self.ensure()?;
        Ok(MiniatureKnownFrontiers {
            storage: crate::KnownFrontiers {
                scope: self.state.scope(),
                event_sequence: self.world().metadata().event_sequence,
                store_revision: self.revision,
                membership_revision: crate::identity::revision(
                    &self.connection,
                    self.state.scope(),
                )?,
            },
            minimum_authority_term: self
                .authority()
                .map_or(nf_contract::identity::AuthorityTerm(0), |a| a.term),
        })
    }
    pub fn snapshot(&self) -> Result<MiniatureSnapshot> {
        Ok(MiniatureSnapshot {
            world: self.world().clone(),
            authority: self.authority(),
            pending: self.state.pending.clone(),
            known: self.known_frontiers()?,
        })
    }
    pub fn current_identity(
        &self,
        account: nf_contract::identity::AccountId,
        device: nf_contract::identity::DeviceId,
    ) -> Result<nf_identity::model::PublicIdentity> {
        self.ensure()?;
        let membership = crate::identity::load(&self.connection, self.state.scope())?
            .ok_or(super::error::MiniatureStoreError::Unauthorized)?;
        super::auth::identity(&membership, account, device)
    }
    pub fn open_existing(
        path: impl AsRef<Path>,
        known: MiniatureKnownFrontiers,
        auth: AuthConfig,
    ) -> Result<Self> {
        let metadata =
            std::fs::metadata(path.as_ref()).map_err(|_| crate::StoreError::MissingHistory)?;
        if !metadata.is_file() || metadata.len() < 100 || metadata.len() > 268_435_456 {
            return Err(crate::StoreError::MissingHistory.into());
        }
        let connection = crate::schema::connection(path.as_ref())?;
        super::schema::verify(&connection)?; // Before any modifying pragma.
        crate::schema::configure(&connection)?;
        let (state, revision, head) = super::recovery::recover(&connection, known)?;
        Ok(Self {
            connection,
            state,
            revision,
            head,
            quarantined: false,
            runtime: AuthRuntime::new(auth)?,
        })
    }
    pub(crate) fn ensure(&self) -> Result<()> {
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        Ok(())
    }
}
fn initialize(
    connection: &mut Connection,
    world: &MiniatureWorld,
    membership: &MembershipState,
    bytes: &[u8],
    evidence: &super::auth::Evidence,
) -> Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(&super::schema::schema())?;
    transaction.pragma_update(None, "application_id", crate::schema::APPLICATION_ID)?;
    transaction.pragma_update(None, "user_version", 2)?;
    let g = world.component().genesis();
    transaction.execute(
        "INSERT INTO history_meta VALUES (1,?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            g.universe.as_bytes(),
            g.history.as_bytes(),
            counter(0),
            counter(0),
            bytes,
            hash(bytes),
            [0u8; 32],
            super::schema::digest()
        ],
    )?;
    transaction.execute(
        "INSERT INTO snapshots VALUES (?1,?2,?3,?4)",
        params![counter(0), bytes, hash(bytes), [0u8; 32]],
    )?;
    let m = world.metadata();
    for aggregate in [m.aggregate, m.provider_aggregate] {
        transaction.execute(
            "INSERT INTO aggregate_state VALUES (?1,?2)",
            params![aggregate.as_bytes(), counter(0)],
        )?;
    }
    transaction.execute(
        "INSERT INTO module_state VALUES (?1,?2,?3)",
        params![m.provider.as_bytes(), counter(0), counter(0)],
    )?;
    let public = encode_state(membership)?;
    transaction.execute(
        "INSERT INTO membership VALUES (?1,?2,?3,?4,?5)",
        params![
            g.universe.as_bytes(),
            g.history.as_bytes(),
            counter(membership.revision),
            public,
            hash(&public)
        ],
    )?;
    let actual =
        crate::identity::load(&transaction, membership.scope)?.ok_or(crate::StoreError::Corrupt)?;
    super::auth::check(evidence, evidence.context, &actual, true)?;
    transaction.commit()?;
    Ok(())
}
