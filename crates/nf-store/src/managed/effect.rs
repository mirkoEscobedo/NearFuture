use super::{bytes::{Encoder, Reader, same}, error::{ManagedError, Result}, foot::StorageFoot,
    rows::{self, Patch, Table}, schema::Profile};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TableEffect {
    pub name: String,
    pub pre_count: u64,
    pub pre: [u8; 32],
    pub post_count: u64,
    pub post: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Effect {
    pub purpose: u32,
    pub pre_profile: u32,
    pub post_profile: u32,
    pub pre_quota: u32,
    pub post_quota: u32,
    pub tables: Vec<TableEffect>,
    pub pre_checkpoint: Vec<u8>,
    pub post_checkpoint: Vec<u8>,
}
fn profile(value: u32) -> Result<Profile> {
    match value { 1 => Ok(Profile::Source), 4 => Ok(Profile::Active), 5 => Ok(Profile::Retired),
        _ => Err(ManagedError::UnsupportedFormat) }
}
impl Effect {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if !matches!(self.purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8)
            || (self.purpose == 8 && (self.pre_profile, self.post_profile) != (1, 5))
            || (self.purpose != 8 && (self.pre_profile != self.post_profile || !matches!(self.pre_profile, 1 | 4)))
            || !(1..=65_536).contains(&self.pre_quota) || !(1..=65_536).contains(&self.post_quota)
        { return Err(ManagedError::Corrupt); }
        let names = rows::tables(profile(self.pre_profile)?)?.into_iter()
            .chain(rows::tables(profile(self.post_profile)?)?).map(|t| t.name).collect::<BTreeSet<_>>();
        if self.tables.iter().map(|t| t.name.as_str()).collect::<Vec<_>>() != names.iter().copied().collect::<Vec<_>>() {
            return Err(ManagedError::Corrupt);
        }
        let mut w = Encoder::new(b"NF-OWNER-EFFECT-1\0", 8192)?;
        for n in [self.purpose, self.pre_profile, self.post_profile, self.pre_quota, self.post_quota] { w.u32(n)?; }
        w.u32(self.tables.len().try_into().map_err(|_| ManagedError::Limit)?)?;
        let mut pre_rows = 0_u64; let mut post_rows = 0_u64;
        for table in &self.tables {
            pre_rows = pre_rows.checked_add(table.pre_count).ok_or(ManagedError::Limit)?;
            post_rows = post_rows.checked_add(table.post_count).ok_or(ManagedError::Limit)?;
            if pre_rows > 1_048_576 || post_rows > 1_048_576 { return Err(ManagedError::Limit); }
            w.blob(table.name.as_bytes(), 256)?; w.u64(table.pre_count)?; w.raw(&table.pre)?;
            w.u64(table.post_count)?; w.raw(&table.post)?;
        }
        for p in [self.pre_profile, self.post_profile] { w.u32(0x4e46_5354)?; w.u32(p)?; w.u32(4096)?; }
        for checkpoint in [&self.pre_checkpoint, &self.post_checkpoint] {
            let mut r = Reader::plain(checkpoint, 93)?;
            super::foot::CheckpointFoot::read(&mut r)?; r.finish()?;
            w.blob(checkpoint, 93)?;
        }
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-EFFECT-1\0", 8192)?;
        let purpose = r.u32()?; let pre_profile = r.u32()?; let post_profile = r.u32()?;
        let pre_quota = r.u32()?; let post_quota = r.u32()?; let count = r.u32()?;
        if count > 15 { return Err(ManagedError::Limit); }
        let mut tables = Vec::new();
        for _ in 0..count {
            tables.push(TableEffect { name: std::str::from_utf8(r.blob(256)?).map_err(|_| ManagedError::Corrupt)?.to_string(),
                pre_count: r.u64()?, pre: r.fixed()?, post_count: r.u64()?, post: r.fixed()? });
        }
        for p in [pre_profile, post_profile] {
            if (r.u32()?, r.u32()?, r.u32()?) != (0x4e46_5354, p, 4096) { return Err(ManagedError::Corrupt); }
        }
        let value = Self { purpose, pre_profile, post_profile, pre_quota, post_quota, tables,
            pre_checkpoint: r.blob(93)?.to_vec(), post_checkpoint: r.blob(93)?.to_vec() };
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
    pub fn check_feet(&self, pre: &StorageFoot, post: &StorageFoot) -> Result<()> {
        if (self.pre_profile, self.post_profile, self.pre_quota, self.post_quota) !=
            (pre.profile, post.profile, pre.quota, post.quota)
            || self.pre_checkpoint != pre.checkpoint.encode()? || self.post_checkpoint != post.checkpoint.encode()?
        { return Err(ManagedError::DerivationMismatch); }
        for table in &self.tables {
            if (pre.profile == 4 && !table.name.starts_with("active_"))
                || (self.purpose == 8 && table.name != "owner_retirement")
            {
                if (table.pre_count, table.pre) != (table.post_count, table.post) { return Err(ManagedError::DerivationMismatch); }
            }
            if self.purpose == 8 && table.name == "owner_retirement"
                && (table.pre_count != 0 || table.post_count != 1 || table.pre != rows::empty_digest(rows::table("owner_retirement")?)?)
            { return Err(ManagedError::DerivationMismatch); }
        }
        Ok(())
    }
}
pub(super) fn count(c: &Connection, table: Table, patch: Option<&Patch>) -> Result<u64> {
    let raw: i64 = c.query_row(&format!("SELECT count(*) FROM (SELECT 1 FROM {} LIMIT 1048577)", table.name), [], |r| r.get(0))?;
    let mut count = u64::try_from(raw).map_err(|_| ManagedError::Corrupt)?;
    if let Some(patch) = patch {
        for (key, (before, after)) in &patch.0 {
            if rows::get(c, table, key)? != *before { return Err(ManagedError::DerivationMismatch); }
            match (before.is_some(), after.is_some()) {
                (false, true) => count = count.checked_add(1).ok_or(ManagedError::Limit)?,
                (true, false) => count = count.checked_sub(1).ok_or(ManagedError::Corrupt)?,
                _ => {},
            }
        }
    }
    if count > 1_048_576 { return Err(ManagedError::Limit); }
    Ok(count)
}
pub(super) fn derive(c: &Connection, pre: &StorageFoot, post: &StorageFoot, purpose: u32,
    patches: &BTreeMap<String, Patch>) -> Result<Effect> {
    let mut effects = Vec::new();
    for table in rows::tables(profile(pre.profile)?)? {
        let post_patches = patches.get(table.name).map(|p| BTreeMap::from([(table.name.to_string(), p.clone())])).unwrap_or_default();
        effects.push(TableEffect { name: table.name.to_string(), pre_count: count(c, table, None)?,
            pre: rows::digest(c, &[table], &BTreeMap::new())?, post_count: count(c, table, patches.get(table.name))?,
            post: rows::digest(c, &[table], &post_patches)? });
    }
    if purpose == 8 {
        let table = rows::table("owner_retirement")?;
        let patch = patches.get(table.name).ok_or(ManagedError::DerivationMismatch)?;
        if patch.0.len() != 1 { return Err(ManagedError::DerivationMismatch); }
        let (before, after) = patch.0.values().next().ok_or(ManagedError::DerivationMismatch)?;
        if before.is_some() { return Err(ManagedError::DerivationMismatch); }
        let row = after.clone().ok_or(ManagedError::DerivationMismatch)?;
        effects.push(TableEffect { name: table.name.to_string(), pre_count: 0, pre: rows::empty_digest(table)?,
            post_count: 1, post: rows::value_digest(table, &[row])? });
        effects.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    }
    let effect = Effect { purpose, pre_profile: pre.profile, post_profile: post.profile,
        pre_quota: pre.quota, post_quota: post.quota, tables: effects,
        pre_checkpoint: pre.checkpoint.encode()?, post_checkpoint: post.checkpoint.encode()? };
    effect.encode()?; effect.check_feet(pre, post)?; Ok(effect)
}
