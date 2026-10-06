use super::{Error, Profile};
use alloc::{boxed::Box, string::String, vec::Vec};
use sha2::{Digest, Sha256};

mod decode;
mod encode;
mod schema;

pub const MAX_RECORD_BYTES: usize = 1_048_576;
const MAX_FIELD_BYTES: usize = 262_144;
const MAX_ENTRIES: usize = 4096;
const MAX_TOTAL_ENTRIES: usize = 16_384;
const MAX_DEPTH: usize = 32;
pub const MAX_DECODE_ALLOCATION_BYTES: usize = 4_194_304;

/// Closed schema-1 semantic values. Field types and order are checked against the registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub domain: u16,
    pub kind: u16,
    pub fields: Vec<Value>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Id([u8; 16]),
    Digest([u8; 32]),
    U32(u32),
    U64(u64),
    Bool(bool),
    Text(String),
    Bytes(Vec<u8>),
    OptionalU64(Option<u64>),
    IdMap(Vec<([u8; 16], u64)>),
    Set(Vec<u32>),
    Record(Box<Record>),
    Records(Vec<Record>),
    Outcome(Option<Box<Record>>),
    Fixture(Box<Profile>),
}

pub fn encode_record(record: &Record) -> Result<Vec<u8>, Error> {
    encode::encode(record)
}
pub fn decode_record(input: &[u8]) -> Result<Record, Error> {
    decode::decode(input)
}
pub fn record_digest(record: &Record) -> Result<[u8; 32], Error> {
    Ok(Sha256::digest(encode_record(record)?).into())
}

pub(crate) fn validate_semantics(record: &Record) -> Result<usize, Error> {
    use Value::*;
    match (record.domain, record.kind) {
        (1, 2) => {
            let (Record(binding), Record(payload)) = (&record.fields[0], &record.fields[2]) else {
                return Err(Error::InvalidProfile);
            };
            if binding.fields[5] != U32(1) || binding.fields[6] != Digest(record_digest(payload)?) {
                return Err(Error::InvalidProfile);
            }
        }
        (2, 1) => {
            let history = &record.fields[1];
            let Records(statuses) = &record.fields[10] else {
                return Err(Error::InvalidProfile);
            };
            for status in statuses {
                if &status.fields[2] != history {
                    return Err(Error::InvalidProfile);
                }
            }
            let Records(entries) = &record.fields[11] else {
                return Err(Error::InvalidProfile);
            };
            for entry in entries {
                let Record(status) = &entry.fields[2] else {
                    return Err(Error::InvalidProfile);
                };
                if &status.fields[2] != history {
                    return Err(Error::InvalidProfile);
                }
            }
        }
        (4, 1) => {
            let Records(statuses) = &record.fields[6] else {
                return Err(Error::InvalidProfile);
            };
            for status in statuses {
                if status.fields[2] != record.fields[0] {
                    return Err(Error::InvalidProfile);
                }
            }
        }
        (6, 1) => {
            let (U32(1), Bytes(body)) = (&record.fields[0], &record.fields[1]) else {
                return Err(Error::InvalidProfile);
            };
            let profile = super::decode_profile(body, super::Limits::default())?;
            return Ok(profile.ordered_values.len() + profile.map.len());
        }
        (6, 2) => {
            for value in &record.fields {
                let Set(ids) = value else {
                    return Err(Error::InvalidProfile);
                };
                if ids.iter().any(|id| *id != 1) {
                    return Err(Error::InvalidProfile);
                }
            }
        }
        (6, 8) => {
            let (U32(phase), Outcome(outcome), OptionalU64(frontier)) =
                (&record.fields[4], &record.fields[5], &record.fields[6])
            else {
                return Err(Error::InvalidProfile);
            };
            let valid = match (*phase, outcome.as_deref(), frontier) {
                (1 | 2, None, None) => true,
                (3, Some(result), Some(_)) => result.domain == 6 && result.kind == 1,
                (4 | 5, Some(error), _) => error.domain == 6 && error.kind == 10,
                _ => false,
            };
            if !valid {
                return Err(Error::InvalidProfile);
            }
        }
        (3, 1) => {
            if record.fields[2] != U32(1) {
                return Err(Error::InvalidProfile);
            }
        }
        (6, 4) => {
            if record.fields[1] != U32(1) {
                return Err(Error::InvalidProfile);
            }
        }
        (6, 9) => {
            let Record(status) = &record.fields[2] else {
                return Err(Error::InvalidProfile);
            };
            if record.fields[0] != status.fields[0] || record.fields[1] != status.fields[3] {
                return Err(Error::InvalidProfile);
            }
        }
        (6, 10) => {
            let U32(code) = record.fields[0] else {
                return Err(Error::InvalidProfile);
            };
            if !(1..=10).contains(&code) {
                return Err(Error::InvalidProfile);
            }
        }
        _ => {}
    }
    Ok(0)
}
