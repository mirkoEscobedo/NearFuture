//! Closed NF-SPT-1 topology summaries. Data validity never grants publication or game authority.
use crate::{
    canonical::text::is_canonical_text,
    identity::{AccountId, EntityId, HistoryId, UniverseId},
};
use alloc::{string::String, vec::Vec};
use sha2::{Digest, Sha256};

const HEADER: &[u8] = b"NF-SPT-1\0\x01\0";
pub const MAX_ENTITIES: usize = 128;
pub const MAX_DEPENDENCIES: usize = 64;
pub const MAX_LABEL_BYTES: usize = 128;
pub const MAX_DOCUMENT_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Invalid,
    Limit,
    Cycle,
    Privacy,
}
pub type Result<T> = core::result::Result<T, Error>;
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub entities: usize,
    pub dependencies: usize,
    pub label_bytes: usize,
    pub document_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entities: MAX_ENTITIES,
            dependencies: MAX_DEPENDENCIES,
            label_bytes: MAX_LABEL_BYTES,
            document_bytes: MAX_DOCUMENT_BYTES,
        }
    }
}
impl Limits {
    fn checked(self) -> Result<Self> {
        if self.entities > MAX_ENTITIES
            || self.dependencies > MAX_DEPENDENCIES
            || self.label_bytes > MAX_LABEL_BYTES
            || self.document_bytes > MAX_DOCUMENT_BYTES
        {
            return Err(Error::Limit);
        }
        Ok(self)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Star,
    Planet,
    Station,
}
impl Kind {
    fn tag(self) -> u8 {
        match self {
            Self::Star => 1,
            Self::Planet => 2,
            Self::Station => 3,
        }
    }
    fn read(tag: u8) -> Result<Self> {
        match tag {
            1 => Ok(Self::Star),
            2 => Ok(Self::Planet),
            3 => Ok(Self::Station),
            _ => Err(Error::Invalid),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
    pub semantic_key: [u8; 32],
    pub required: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub kind: Kind,
    pub parent: Option<EntityId>,
    pub dependency: Option<u16>,
    pub label: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Topology {
    pub universe: UniverseId,
    pub history: HistoryId,
    pub owner: AccountId,
    pub system: EntityId,
    pub revision: u64,
    pub campaign_day_millis: u64,
    pub dependencies: Vec<Dependency>,
    pub entities: Vec<Entity>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentStatus {
    Known,
    Abstract(Vec<[u8; 32]>),
}
fn nonzero(bytes: &[u8]) -> bool {
    bytes.iter().any(|b| *b != 0)
}
fn keys_checked(keys: &[[u8; 32]], limits: Limits) -> Result<()> {
    if keys.len() > limits.dependencies {
        return Err(Error::Limit);
    }
    if keys.iter().any(|k| !nonzero(k)) || keys.windows(2).any(|w| w[0] >= w[1]) {
        return Err(Error::Invalid);
    }
    Ok(())
}
/// Iterative logical parent depth is bounded by 128; this is not recursive NF-CANON-1 depth.
pub fn validate(value: &Topology, limits: Limits) -> Result<()> {
    let limits = limits.checked()?;
    if value.entities.is_empty()
        || value.entities.len() > limits.entities
        || value.dependencies.len() > limits.dependencies
    {
        return Err(Error::Limit);
    }
    if value.revision == 0
        || !nonzero(value.universe.as_bytes())
        || !nonzero(value.history.as_bytes())
        || !nonzero(value.owner.as_bytes())
        || !nonzero(value.system.as_bytes())
    {
        return Err(Error::Invalid);
    }
    if value.dependencies.iter().any(|d| !nonzero(&d.semantic_key))
        || value
            .dependencies
            .windows(2)
            .any(|w| w[0].semantic_key >= w[1].semantic_key)
        || value.entities.windows(2).any(|w| w[0].id >= w[1].id)
    {
        return Err(Error::Invalid);
    }
    let mut parents = [None; MAX_ENTITIES];
    let mut stars = 0;
    for (index, entity) in value.entities.iter().enumerate() {
        if !nonzero(entity.id.as_bytes()) {
            return Err(Error::Invalid);
        }
        if let Some(label) = &entity.label {
            if label.len() > limits.label_bytes {
                return Err(Error::Limit);
            }
            if !is_canonical_text(label) {
                return Err(Error::Invalid);
            }
        }
        if entity
            .dependency
            .is_some_and(|d| usize::from(d) >= value.dependencies.len())
        {
            return Err(Error::Invalid);
        }
        match (entity.kind, entity.parent) {
            (Kind::Star, None) => stars += 1,
            (Kind::Planet | Kind::Station, Some(id)) => {
                parents[index] = Some(
                    value
                        .entities
                        .binary_search_by_key(&id, |e| e.id)
                        .map_err(|_| Error::Invalid)?,
                )
            }
            _ => return Err(Error::Invalid),
        }
    }
    if stars != 1 {
        return Err(Error::Invalid);
    }
    for start in 0..value.entities.len() {
        let mut seen = [false; MAX_ENTITIES];
        let mut next = Some(start);
        let mut depth = 0;
        while let Some(index) = next {
            if seen[index] {
                return Err(Error::Cycle);
            }
            depth += 1;
            if depth > MAX_ENTITIES {
                return Err(Error::Limit);
            }
            seen[index] = true;
            next = parents[index];
        }
    }
    if measured(value) > limits.document_bytes {
        return Err(Error::Limit);
    }
    Ok(())
}
fn measured(value: &Topology) -> usize {
    96 + value.dependencies.len() * 33
        + value
            .entities
            .iter()
            .map(|e| {
                20 + if e.parent.is_some() { 16 } else { 0 }
                    + if e.dependency.is_some() { 2 } else { 0 }
                    + e.label.as_ref().map_or(0, |s| 2 + s.len())
            })
            .sum::<usize>()
}
/// Exact selection rejects dangling private parents instead of importing hidden ancestors.
pub fn select(
    source: &Topology,
    selected: &[EntityId],
    allowed_keys: &[[u8; 32]],
    limits: Limits,
) -> Result<Topology> {
    validate(source, limits)?;
    keys_checked(allowed_keys, limits)?;
    if selected.is_empty() || selected.len() > limits.entities {
        return Err(Error::Privacy);
    }
    if selected.windows(2).any(|w| w[0] >= w[1]) {
        return Err(Error::Invalid);
    }
    let mut entities = Vec::with_capacity(selected.len());
    let mut used = [false; MAX_DEPENDENCIES];
    for id in selected {
        let index = source
            .entities
            .binary_search_by_key(id, |e| e.id)
            .map_err(|_| Error::Privacy)?;
        let entity = &source.entities[index];
        if entity
            .parent
            .is_some_and(|p| selected.binary_search(&p).is_err())
        {
            return Err(Error::Privacy);
        }
        if let Some(dependency) = entity.dependency {
            let key = source.dependencies[usize::from(dependency)].semantic_key;
            if allowed_keys.binary_search(&key).is_err() {
                return Err(Error::Privacy);
            }
            used[usize::from(dependency)] = true;
        }
        entities.push(entity.clone());
    }
    let dependencies: Vec<_> = source
        .dependencies
        .iter()
        .enumerate()
        .filter(|(i, _)| used[*i])
        .map(|(_, d)| d.clone())
        .collect();
    for entity in &mut entities {
        if let Some(old) = entity.dependency {
            let key = source.dependencies[usize::from(old)].semantic_key;
            let index = dependencies
                .binary_search_by_key(&key, |d| d.semantic_key)
                .map_err(|_| Error::Invalid)?;
            entity.dependency = Some(u16::try_from(index).map_err(|_| Error::Limit)?);
        }
    }
    let output = Topology {
        universe: source.universe,
        history: source.history,
        owner: source.owner,
        system: source.system,
        revision: source.revision,
        campaign_day_millis: source.campaign_day_millis,
        dependencies,
        entities,
    };
    validate(&output, limits)?;
    Ok(output)
}
pub fn diagnose_content(
    value: &Topology,
    known_keys: &[[u8; 32]],
    limits: Limits,
) -> Result<ContentStatus> {
    validate(value, limits)?;
    keys_checked(known_keys, limits)?;
    let missing: Vec<_> = value
        .dependencies
        .iter()
        .filter(|d| d.required && known_keys.binary_search(&d.semantic_key).is_err())
        .map(|d| d.semantic_key)
        .collect();
    Ok(if missing.is_empty() {
        ContentStatus::Known
    } else {
        ContentStatus::Abstract(missing)
    })
}
pub fn encode(value: &Topology, limits: Limits) -> Result<Vec<u8>> {
    validate(value, limits)?;
    let mut output = Vec::with_capacity(measured(value));
    output.extend_from_slice(HEADER);
    for id in [
        value.universe.as_bytes(),
        value.history.as_bytes(),
        value.owner.as_bytes(),
        value.system.as_bytes(),
    ] {
        output.extend_from_slice(id);
    }
    output.extend_from_slice(&value.revision.to_le_bytes());
    output.extend_from_slice(&value.campaign_day_millis.to_le_bytes());
    output.push(1);
    output.extend_from_slice(&(value.dependencies.len() as u16).to_le_bytes());
    for d in &value.dependencies {
        output.extend_from_slice(&d.semantic_key);
        output.push(u8::from(d.required));
    }
    output.extend_from_slice(&(value.entities.len() as u16).to_le_bytes());
    for e in &value.entities {
        output.extend_from_slice(e.id.as_bytes());
        output.push(e.kind.tag());
        output.push(u8::from(e.parent.is_some()));
        if let Some(p) = e.parent {
            output.extend_from_slice(p.as_bytes());
        }
        output.push(u8::from(e.dependency.is_some()));
        if let Some(d) = e.dependency {
            output.extend_from_slice(&d.to_le_bytes());
        }
        output.push(u8::from(e.label.is_some()));
        if let Some(label) = &e.label {
            output.extend_from_slice(&(label.len() as u16).to_le_bytes());
            output.extend_from_slice(label.as_bytes());
        }
    }
    Ok(output)
}
pub fn digest(value: &Topology, limits: Limits) -> Result<[u8; 32]> {
    Ok(Sha256::digest(encode(value, limits)?).into())
}
struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
    limits: Limits,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self.offset.checked_add(count).ok_or(Error::Limit)?;
        let value = self.input.get(self.offset..end).ok_or(Error::Invalid)?;
        self.offset = end;
        Ok(value)
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.take(N)?.try_into().map_err(|_| Error::Invalid)
    }
    fn number16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.fixed()?))
    }
    fn flag(&mut self) -> Result<bool> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::Invalid),
        }
    }
    fn count(&mut self, max: usize, minimum: usize) -> Result<usize> {
        let n = usize::from(self.number16()?);
        if n > max {
            return Err(Error::Limit);
        }
        if n > (self.input.len() - self.offset) / minimum {
            return Err(Error::Invalid);
        }
        Ok(n)
    }
}
pub fn decode(input: &[u8], limits: Limits) -> Result<Topology> {
    let limits = limits.checked()?;
    if input.len() > limits.document_bytes {
        return Err(Error::Limit);
    }
    let mut reader = Reader {
        input,
        offset: 0,
        limits,
    };
    if reader.take(HEADER.len())? != HEADER {
        return Err(Error::Invalid);
    }
    let universe = UniverseId::from_bytes(reader.fixed()?);
    let history = HistoryId::from_bytes(reader.fixed()?);
    let owner = AccountId::from_bytes(reader.fixed()?);
    let system = EntityId::from_bytes(reader.fixed()?);
    let revision = u64::from_le_bytes(reader.fixed()?);
    let campaign_day_millis = u64::from_le_bytes(reader.fixed()?);
    if reader.take(1)? != [1] {
        return Err(Error::Invalid);
    }
    let count = reader.count(limits.dependencies, 33)?;
    let mut dependencies = Vec::with_capacity(count);
    for _ in 0..count {
        dependencies.push(Dependency {
            semantic_key: reader.fixed()?,
            required: reader.flag()?,
        });
    }
    let count = reader.count(limits.entities, 20)?;
    let mut entities = Vec::with_capacity(count);
    for _ in 0..count {
        let id = EntityId::from_bytes(reader.fixed()?);
        let kind = Kind::read(reader.take(1)?[0])?;
        let parent = if reader.flag()? {
            Some(EntityId::from_bytes(reader.fixed()?))
        } else {
            None
        };
        let dependency = if reader.flag()? {
            Some(reader.number16()?)
        } else {
            None
        };
        let label = if reader.flag()? {
            let n = usize::from(reader.number16()?);
            if n > reader.limits.label_bytes {
                return Err(Error::Limit);
            }
            Some(String::from(
                core::str::from_utf8(reader.take(n)?).map_err(|_| Error::Invalid)?,
            ))
        } else {
            None
        };
        entities.push(Entity {
            id,
            kind,
            parent,
            dependency,
            label,
        });
    }
    if reader.offset != input.len() {
        return Err(Error::Invalid);
    }
    let value = Topology {
        universe,
        history,
        owner,
        system,
        revision,
        campaign_day_millis,
        dependencies,
        entities,
    };
    validate(&value, limits)?;
    Ok(value)
}
