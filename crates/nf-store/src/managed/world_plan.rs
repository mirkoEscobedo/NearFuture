use super::{bytes::hash, error::{ManagedError, Result}, foot::{CheckpointFoot, StorageFoot},
    rows::{self, Patch}, schema::Profile, state::FullState};
use crate::KnownFrontiers;
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
pub(super) struct Plan {
    pub state: FullState,
    pub revision: u64,
    pub head: [u8; 32],
    pub checkpoint: CheckpointFoot,
    pub patches: BTreeMap<String, Patch>,
}
impl Plan {
    pub fn foot(&self, c: &Connection, profile: Profile, lineage: Option<[u8; 32]>,
        absent: &BTreeSet<&str>) -> Result<StorageFoot> {
        self.state.validate(self.revision)?;
        let member = self.state.member.as_ref().map(nf_identity::codec::encode_state).transpose()?;
        StorageFoot::from_parts(profile.version(), hash(profile.ddl().as_bytes()), KnownFrontiers {
            scope: self.state.core.scope(), store_revision: self.revision,
            event_sequence: self.state.core.world.view().event_seq(),
            membership_revision: self.state.member.as_ref().map(|m| m.revision),
        }, hash(&crate::codec::encode(&self.state.core)?), member.as_deref().map(hash), hash(&self.state.encode()?),
            self.head, lineage, self.state.quota, self.state.policy, self.state.cancellation_digest()?,
            self.checkpoint.clone(), rows::digest_with_absent(c, &rows::tables(profile)?, &self.patches, absent)?)
    }
    pub fn apply(&self, c: &Connection) -> Result<()> {
        for (name, patch) in &self.patches { patch.apply(c, rows::table(name)?)?; }
        Ok(())
    }
}
pub(super) fn new_checkpoint(state: &FullState, revision: u64, head: [u8; 32]) -> Result<CheckpointFoot> {
    state.validate(revision)?;
    Ok(CheckpointFoot { revision, event: state.core.world.view().event_seq().0,
        membership: state.member.as_ref().map(|m| m.revision), state: hash(&state.encode()?), head, quota: state.quota })
}
pub(super) fn fixed32(bytes: &[u8]) -> Result<[u8; 32]> {
    bytes.try_into().map_err(|_| ManagedError::Corrupt)
}
