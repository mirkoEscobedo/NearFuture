use super::{bytes::{Reader, hash, next, same}, error::{ManagedError, Result},
    mirror_rows::{b, i, u}, rows::{self, Row}, schema::Profile, state::FullState,
    upgrade::{self, Descriptor}};
use crate::KnownFrontiers;
use nf_contract::identity::{EventSeq, HistoryId, UniverseId};
use nf_identity::model::Scope;
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) struct Origin {
    pub state: FullState,
    pub revision: u64,
    pub head: [u8; 32],
    pub lineage: [u8; 32],
    pub action: Vec<u8>,
    pub before: [u8; 32],
    pub to: Descriptor,
}
impl Origin {
    pub fn read(c: &Connection) -> Result<Self> {
        let row = rows::get(c, rows::table("migration_lineage")?, &vec![i(1)])?.ok_or(ManagedError::Corrupt)?;
        let record = rows::blob(&row, 1, 16_384)?.to_vec(); let lineage = hash(&record);
        if rows::blob(&row, 2, 32)? != lineage { return Err(ManagedError::Corrupt); }
        let mut r = Reader::new(&record, b"NF-ACTIVE-LINEAGE-1\0", 16_384)?;
        let binding = r.blob(256)?.to_vec(); let action = r.blob(8192)?.to_vec(); let revision = r.u64()?;
        let before = r.fixed()?; let after: [u8; 32] = r.fixed()?; r.finish()?;
        let mut a = Reader::new(&action, b"NF-ACTIVE-UPGRADE-1\0", 8192)?;
        if a.blob(256)? != binding { return Err(ManagedError::Corrupt); }
        let from = Descriptor::read(a.blob(1923)?)?; let to = Descriptor::read(a.blob(1923)?)?; a.finish()?;
        let mut bnd = Reader::new(&binding, b"NF-ACTIVE-SOURCE-1\0", 256)?;
        let scope = Scope { universe: UniverseId::from_bytes(bnd.fixed()?), history: HistoryId::from_bytes(bnd.fixed()?) };
        let known = KnownFrontiers { scope, store_revision: bnd.u64()?, event_sequence: EventSeq(bnd.u64()?),
            membership_revision: bnd.option(Reader::u64)? };
        let source_head = bnd.fixed()?; let core_sha: [u8; 32] = bnd.fixed()?;
        let member_sha: Option<[u8; 32]> = bnd.option(Reader::fixed)?; let quota = bnd.u32()?;
        let base_rows: [u8; 32] = bnd.fixed()?; let schema: [u8; 32] = bnd.fixed()?; bnd.finish()?;
        if schema != crate::schema::schema_digest() || member_sha.is_none()
            || rows::digest(c, &rows::tables(Profile::Source)?, &BTreeMap::new())? != base_rows
        { return Err(ManagedError::SourceMismatch); }
        let (core, source_revision, actual_head) = crate::recovery::recover(c, known)?;
        let member = crate::identity::load(c, known.scope)?.ok_or(ManagedError::Unauthorized)?;
        if source_revision != known.store_revision || actual_head != source_head
            || core.world.view().event_seq() != known.event_sequence || Some(member.revision) != known.membership_revision
            || hash(&crate::codec::encode(&core)?) != core_sha
            || Some(hash(&nf_identity::codec::encode_state(&member)?)) != member_sha
        { return Err(ManagedError::SourceMismatch); }
        let source = FullState { core, member: Some(member), quota, policy: None, cancellations: BTreeMap::new() };
        source.validate(source_revision)?; upgrade::validate(&from, &to, &source, &from.providers)?;
        if revision != next(source_revision)? || before != hash(&source.encode()?) {
            return Err(ManagedError::DerivationMismatch);
        }
        let mut state = source.clone(); state.core.world = source.core.world.with_ruleset_hash(to.digest())?;
        state.validate(revision)?;
        if after != hash(&state.encode()?) { return Err(ManagedError::DerivationMismatch); }
        same(&action, &upgrade::initial_action(&binding, &from, &to)?)?;
        same(&record, &upgrade::lineage(&binding, &action, revision, before, after)?)?;
        let head = upgrade::head(source_head, revision, 4, &action, before, after, lineage)?;
        Ok(Self { state, revision, head, lineage, action, before, to })
    }
    pub fn journal(&self) -> Result<Row> {
        let state = self.state.encode()?;
        Ok(vec![u(self.revision), i(4), b(self.before), b(hash(&state)), b(self.head), b(&self.action), b(state)])
    }
}
