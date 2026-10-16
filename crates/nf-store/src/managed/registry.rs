use super::{
    bytes::{Encoder, Reader, hash, same},
    error::{ManagedError, Result},
    schema,
};
use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::model::Scope;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Manifest {
    pub installation: [u8; 16],
    pub scope: Scope,
    pub kind: u32,
    pub id: [u8; 16],
    pub path: PathBuf,
    pub bytes: u64,
    pub digest: [u8; 32],
    pub profile: u32,
    pub schema: [u8; 32],
    pub provenance: String,
    pub composition: Vec<u8>,
}
impl Manifest {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if !matches!(self.kind, 1..=5)
            || self.bytes == 0
            || self.bytes > 268_435_456
            || self.provenance.len() > 2048
            || self.provenance.as_bytes().contains(&0)
            || !self.path.is_absolute()
        {
            return Err(ManagedError::Limit);
        }
        let path = self.path.to_str().ok_or(ManagedError::Scope)?;
        if path.as_bytes().contains(&0) {
            return Err(ManagedError::Scope);
        }
        match self.kind {
            1 | 2 if self.profile == 1 && self.schema == crate::schema::schema_digest() => {}
            3 if self.profile == 4 && self.schema == schema::active_schema_digest() => {}
            4 | 5 if self.profile == 0 && self.schema == [0; 32] => {}
            _ => return Err(ManagedError::Corrupt),
        }
        if !self.composition.is_empty() {
            crate::migration::staging_record::decode(&self.composition)
                .map_err(|_| ManagedError::Corrupt)?;
        } else if self.kind != 4 {
            return Err(ManagedError::Corrupt);
        }
        let mut w = Encoder::new(b"NF-OWNER-ARTIFACT-1\0", 8192)?;
        w.raw(&self.installation)?;
        w.raw(self.scope.universe.as_bytes())?;
        w.raw(self.scope.history.as_bytes())?;
        w.u32(self.kind)?;
        w.raw(&self.id)?;
        w.blob(path.as_bytes(), 4096)?;
        w.u64(self.bytes)?;
        w.raw(&self.digest)?;
        w.u32(self.profile)?;
        w.raw(&self.schema)?;
        w.blob(self.provenance.as_bytes(), 2048)?;
        w.blob(&self.composition, 1923)?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-ARTIFACT-1\0", 8192)?;
        let installation = r.fixed()?;
        let scope = Scope {
            universe: UniverseId::from_bytes(r.fixed()?),
            history: HistoryId::from_bytes(r.fixed()?),
        };
        let kind = r.u32()?;
        let id = r.fixed()?;
        let path =
            PathBuf::from(std::str::from_utf8(r.blob(4096)?).map_err(|_| ManagedError::Corrupt)?);
        let value = Self {
            installation,
            scope,
            kind,
            id,
            path,
            bytes: r.u64()?,
            digest: r.fixed()?,
            profile: r.u32()?,
            schema: r.fixed()?,
            provenance: std::str::from_utf8(r.blob(2048)?)
                .map_err(|_| ManagedError::Corrupt)?
                .to_string(),
            composition: r.blob(1923)?.to_vec(),
        };
        r.finish()?;
        same(bytes, &value.encode()?)?;
        Ok(value)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RegistryRow {
    pub kind: u32,
    pub artifact: [u8; 16],
    pub path: PathBuf,
    pub bytes: u64,
    pub digest: [u8; 32],
    pub manifest: Vec<u8>,
}
impl RegistryRow {
    pub fn from_manifest(manifest: &Manifest) -> Result<Self> {
        Ok(Self {
            kind: manifest.kind,
            artifact: manifest.id,
            path: manifest.path.clone(),
            bytes: manifest.bytes,
            digest: manifest.digest,
            manifest: manifest.encode()?,
        })
    }
    pub fn write(&self, w: &mut Encoder) -> Result<()> {
        let manifest = Manifest::decode(&self.manifest)?;
        if manifest.kind != self.kind
            || manifest.id != self.artifact
            || manifest.path != self.path
            || manifest.bytes != self.bytes
            || manifest.digest != self.digest
        {
            return Err(ManagedError::Corrupt);
        }
        w.u32(self.kind)?;
        w.raw(&self.artifact)?;
        w.blob(
            self.path.to_str().ok_or(ManagedError::Scope)?.as_bytes(),
            4096,
        )?;
        w.u64(self.bytes)?;
        w.raw(&self.digest)?;
        w.blob(&self.manifest, 8192)
    }
    pub fn read(r: &mut Reader<'_>) -> Result<Self> {
        let value = Self {
            kind: r.u32()?,
            artifact: r.fixed()?,
            path: PathBuf::from(
                std::str::from_utf8(r.blob(4096)?).map_err(|_| ManagedError::Corrupt)?,
            ),
            bytes: r.u64()?,
            digest: r.fixed()?,
            manifest: r.blob(8192)?.to_vec(),
        };
        let mut w = Encoder::plain(12_356);
        value.write(&mut w)?;
        Ok(value)
    }
    pub fn cells(&self) -> Vec<super::rows::Cell> {
        use super::rows::Cell::{Blob, Integer};
        vec![
            Integer(i64::from(self.kind)),
            Blob(self.artifact.to_vec()),
            Blob(self.path.as_os_str().as_encoded_bytes().to_vec()),
            Blob(self.bytes.to_be_bytes().to_vec()),
            Blob(self.digest.to_vec()),
            Blob(self.manifest.clone()),
        ]
    }
    pub fn manifest_digest(&self) -> [u8; 32] {
        hash(&self.manifest)
    }
}
