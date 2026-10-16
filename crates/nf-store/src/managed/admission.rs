use super::{
    bytes::{Encoder, Reader, same},
    delta::Delta,
    error::{ManagedError, Result},
    foot::StorageFoot,
};
use crate::migration::ProviderVersion;
use nf_contract::identity::{HistoryId, ProviderId, UniverseId};
use nf_identity::model::Scope;
use std::path::PathBuf;

#[derive(Clone)]
pub(super) enum Operation {
    Adopt {
        source: [u8; 16],
        supported: Vec<ProviderVersion>,
    },
    Claim {
        generation: u64,
        term: u64,
        session: u64,
        incarnation: [u8; 16],
    },
    Select {
        source: [u8; 16],
        target: [u8; 16],
        retired: StorageFoot,
        initial: StorageFoot,
        retirement: [u8; 32],
        binary: [u8; 32],
        generation: u64,
        term: u64,
        session: u64,
        incarnation: [u8; 16],
    },
    Reconcile {
        sequence: u64,
        operation: [u8; 16],
        purpose: u32,
        stage: u32,
        pending: [u8; 32],
        outcome: u8,
        verified: StorageFoot,
        pre: StorageFoot,
        post: StorageFoot,
    },
}
impl Operation {
    pub fn purpose(&self) -> u32 {
        match self {
            Self::Adopt { .. } => 9,
            Self::Claim { .. } => 10,
            Self::Select { .. } => 13,
            Self::Reconcile { .. } => 14,
        }
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Encoder::new(b"NF-OWNER-OPERATION-1\0", 32_768)?;
        w.u32(self.purpose())?;
        match self {
            Self::Adopt { source, supported } => {
                if supported.is_empty()
                    || supported.len() > 32
                    || supported.windows(2).any(|p| p[0].provider >= p[1].provider)
                    || supported
                        .iter()
                        .any(|p| p.version == 0 || p.state_schema == 0)
                {
                    return Err(ManagedError::Corrupt);
                }
                w.raw(source)?;
                w.u32(
                    supported
                        .len()
                        .try_into()
                        .map_err(|_| ManagedError::Limit)?,
                )?;
                for p in supported {
                    w.raw(p.provider.as_bytes())?;
                    w.raw(&p.implementation)?;
                    w.u32(p.version)?;
                    w.u32(p.state_schema)?;
                }
            }
            Self::Claim {
                generation,
                term,
                session,
                incarnation,
            } => {
                w.u64(*generation)?;
                w.u64(*term)?;
                w.u64(*session)?;
                w.raw(incarnation)?;
            }
            Self::Select {
                source,
                target,
                retired,
                initial,
                retirement,
                binary,
                generation,
                term,
                session,
                incarnation,
            } => {
                w.raw(source)?;
                w.raw(target)?;
                w.blob(retired.canonical_bytes(), 2048)?;
                w.blob(initial.canonical_bytes(), 2048)?;
                w.raw(retirement)?;
                w.raw(binary)?;
                w.u64(*generation)?;
                w.u64(*term)?;
                w.u64(*session)?;
                w.raw(incarnation)?;
            }
            Self::Reconcile {
                sequence,
                operation,
                purpose,
                stage,
                pending,
                outcome,
                verified,
                pre,
                post,
            } => {
                if *sequence == 0
                    || !matches!(*purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8)
                    || !matches!(*stage, 1 | 2)
                    || (*stage == 2 && *purpose != 8)
                    || !matches!(*outcome, 1 | 2)
                    || (*outcome == 1 && verified != pre)
                    || (*outcome == 2 && verified != post)
                {
                    return Err(ManagedError::Corrupt);
                }
                w.u64(*sequence)?;
                w.raw(operation)?;
                w.u32(*purpose)?;
                w.u32(*stage)?;
                w.raw(pending)?;
                w.byte(*outcome)?;
                w.blob(verified.canonical_bytes(), 2048)?;
                w.blob(pre.canonical_bytes(), 2048)?;
                w.blob(post.canonical_bytes(), 2048)?;
            }
        }
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-OPERATION-1\0", 32_768)?;
        let value = match r.u32()? {
            9 => {
                let source = r.fixed()?;
                let n = r.u32()?;
                if n == 0 || n > 32 {
                    return Err(ManagedError::Limit);
                }
                let mut supported = Vec::new();
                for _ in 0..n {
                    supported.push(ProviderVersion {
                        provider: ProviderId::from_bytes(r.fixed()?),
                        implementation: r.fixed()?,
                        version: r.u32()?,
                        state_schema: r.u32()?,
                    });
                }
                Self::Adopt { source, supported }
            }
            10 => Self::Claim {
                generation: r.u64()?,
                term: r.u64()?,
                session: r.u64()?,
                incarnation: r.fixed()?,
            },
            13 => Self::Select {
                source: r.fixed()?,
                target: r.fixed()?,
                retired: StorageFoot::decode(r.blob(2048)?)?,
                initial: StorageFoot::decode(r.blob(2048)?)?,
                retirement: r.fixed()?,
                binary: r.fixed()?,
                generation: r.u64()?,
                term: r.u64()?,
                session: r.u64()?,
                incarnation: r.fixed()?,
            },
            14 => Self::Reconcile {
                sequence: r.u64()?,
                operation: r.fixed()?,
                purpose: r.u32()?,
                stage: r.u32()?,
                pending: r.fixed()?,
                outcome: r.byte()?,
                verified: StorageFoot::decode(r.blob(2048)?)?,
                pre: StorageFoot::decode(r.blob(2048)?)?,
                post: StorageFoot::decode(r.blob(2048)?)?,
            },
            _ => return Err(ManagedError::OperationConflict),
        };
        r.finish()?;
        same(bytes, &value.encode()?)?;
        Ok(value)
    }
}
#[derive(Clone)]
pub(super) struct Input {
    pub installation: [u8; 16],
    pub scope: Scope,
    pub foot: StorageFoot,
    pub composition: Vec<u8>,
    pub catalog: PathBuf,
    pub delta: Delta,
    pub operation: Operation,
}
impl Input {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let purpose = self.operation.purpose();
        let path = self.catalog.to_str().ok_or(ManagedError::Scope)?;
        if !self.catalog.is_absolute()
            || path.as_bytes().contains(&0)
            || self.scope != self.foot.known.scope
            || (matches!(purpose, 10 | 14) && !self.composition.is_empty())
            || (purpose != 9 && self.delta != Delta::default())
        {
            return Err(ManagedError::Corrupt);
        }
        if matches!(purpose, 9 | 13) {
            super::upgrade::Descriptor::read(&self.composition)?;
        }
        let mut w = Encoder::new(b"NF-OWNER-ADMISSION-1\0", 172_002)?;
        w.u32(purpose)?;
        w.raw(&self.installation)?;
        w.raw(self.scope.universe.as_bytes())?;
        w.raw(self.scope.history.as_bytes())?;
        w.blob(self.foot.canonical_bytes(), 2048)?;
        w.blob(&self.composition, 1923)?;
        w.blob(path.as_bytes(), 4096)?;
        w.blob(&self.delta.encode()?, 131_072)?;
        w.blob(&self.operation.encode()?, 32_768)?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-ADMISSION-1\0", 172_002)?;
        let purpose = r.u32()?;
        let installation = r.fixed()?;
        let scope = Scope {
            universe: UniverseId::from_bytes(r.fixed()?),
            history: HistoryId::from_bytes(r.fixed()?),
        };
        let value = Self {
            installation,
            scope,
            foot: StorageFoot::decode(r.blob(2048)?)?,
            composition: r.blob(1923)?.to_vec(),
            catalog: PathBuf::from(
                std::str::from_utf8(r.blob(4096)?).map_err(|_| ManagedError::Corrupt)?,
            ),
            delta: Delta::decode(r.blob(131_072)?)?,
            operation: Operation::decode(r.blob(32_768)?)?,
        };
        r.finish()?;
        if value.operation.purpose() != purpose {
            return Err(ManagedError::Corrupt);
        }
        same(bytes, &value.encode()?)?;
        Ok(value)
    }
}
