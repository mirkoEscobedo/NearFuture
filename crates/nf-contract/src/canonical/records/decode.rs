use super::{
    Error, MAX_DECODE_ALLOCATION_BYTES, MAX_DEPTH, MAX_ENTRIES, MAX_FIELD_BYTES, MAX_RECORD_BYTES,
    MAX_TOTAL_ENTRIES, Record, Value,
    schema::{self, Field},
    validate_semantics,
};
use crate::canonical::text::is_canonical_text;
use alloc::{boxed::Box, string::String, vec::Vec};

pub(super) fn decode(input: &[u8]) -> Result<Record, Error> {
    if input.len() > MAX_RECORD_BYTES {
        return Err(Error::Limit);
    }
    let mut reader = Reader {
        input,
        offset: 0,
        total_entries: 0,
        allocated: 0,
    };
    let record = reader.record(0)?;
    if reader.offset != input.len() {
        return Err(Error::InvalidProfile);
    }
    Ok(record)
}
struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
    total_entries: usize,
    allocated: usize,
}
impl<'a> Reader<'a> {
    fn reserve<T>(&mut self, count: usize) -> Result<(), Error> {
        self.allocated = self
            .allocated
            .checked_add(
                count
                    .checked_mul(core::mem::size_of::<T>())
                    .ok_or(Error::Limit)?,
            )
            .ok_or(Error::Limit)?;
        if self.allocated > MAX_DECODE_ALLOCATION_BYTES {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.offset.checked_add(count).ok_or(Error::Limit)?;
        let bytes = self
            .input
            .get(self.offset..end)
            .ok_or(Error::InvalidProfile)?;
        self.offset = end;
        Ok(bytes)
    }
    fn number16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| Error::InvalidProfile)?,
        ))
    }
    fn number32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| Error::InvalidProfile)?,
        ))
    }
    fn number64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| Error::InvalidProfile)?,
        ))
    }
    fn count(&mut self, maximum: usize, minimum_bytes: usize) -> Result<usize, Error> {
        let count = self.number32()? as usize;
        self.total_entries = self.total_entries.checked_add(count).ok_or(Error::Limit)?;
        if count > maximum || self.total_entries > MAX_TOTAL_ENTRIES {
            return Err(Error::Limit);
        }
        if count.checked_mul(minimum_bytes).ok_or(Error::Limit)? > self.input.len() - self.offset {
            return Err(Error::InvalidProfile);
        }
        Ok(count)
    }
    fn bytes(&mut self, maximum: usize) -> Result<&'a [u8], Error> {
        let length = self.number32()? as usize;
        if length > maximum {
            return Err(Error::Limit);
        }
        self.take(length)
    }
    fn nested(&mut self, domain: u16, kind: u16, depth: usize) -> Result<Record, Error> {
        let record = self.record(depth + 1)?;
        if record.domain != domain || record.kind != kind {
            return Err(Error::InvalidProfile);
        }
        Ok(record)
    }
    fn record(&mut self, depth: usize) -> Result<Record, Error> {
        if depth >= MAX_DEPTH {
            return Err(Error::Limit);
        }
        let start = self.offset;
        if self.take(11)? != super::super::PREFIX {
            return Err(Error::InvalidProfile);
        }
        let domain = self.number16()?;
        let kind = self.number16()?;
        if self.number16()? != 1 {
            return Err(Error::InvalidProfile);
        }
        if (domain, kind) == (255, 1) {
            if depth != 0 {
                return Err(Error::InvalidProfile);
            }
            let profile = super::super::decode_profile(
                &self.input[start..],
                super::super::Limits::default(),
            )?;
            self.offset = self.input.len();
            return Ok(Record {
                domain,
                kind,
                fields: alloc::vec![Value::Fixture(Box::new(profile))],
            });
        }
        let schema = schema::fields(domain, kind)?;
        self.reserve::<Value>(schema.len())?;
        let mut fields = Vec::with_capacity(schema.len());
        for spec in schema {
            fields.push(self.value(*spec, depth)?);
        }
        let record = Record {
            domain,
            kind,
            fields,
        };
        self.total_entries = self
            .total_entries
            .checked_add(validate_semantics(&record)?)
            .ok_or(Error::Limit)?;
        if self.total_entries > MAX_TOTAL_ENTRIES {
            return Err(Error::Limit);
        }
        Ok(record)
    }
    fn value(&mut self, spec: Field, depth: usize) -> Result<Value, Error> {
        Ok(match spec {
            Field::Id => Value::Id(
                self.take(16)?
                    .try_into()
                    .map_err(|_| Error::InvalidProfile)?,
            ),
            Field::Digest => Value::Digest(
                self.take(32)?
                    .try_into()
                    .map_err(|_| Error::InvalidProfile)?,
            ),
            Field::U32 => Value::U32(self.number32()?),
            Field::U64 => Value::U64(self.number64()?),
            Field::Bool => Value::Bool(match self.take(1)?[0] {
                0 => false,
                1 => true,
                _ => return Err(Error::InvalidProfile),
            }),
            Field::Text => {
                let bytes = self.bytes(512)?;
                let text = core::str::from_utf8(bytes).map_err(|_| Error::InvalidProfile)?;
                if !is_canonical_text(text) {
                    return Err(Error::InvalidProfile);
                }
                self.reserve::<u8>(text.len())?;
                Value::Text(String::from(text))
            }
            Field::Bytes => {
                let bytes = self.bytes(MAX_FIELD_BYTES)?;
                self.reserve::<u8>(bytes.len())?;
                Value::Bytes(bytes.to_vec())
            }
            Field::OptionalU64 => Value::OptionalU64(match self.take(1)?[0] {
                0 => None,
                1 => Some(self.number64()?),
                _ => return Err(Error::InvalidProfile),
            }),
            Field::IdMap => {
                let count = self.count(MAX_ENTRIES, 24)?;
                self.reserve::<([u8; 16], u64)>(count)?;
                let mut map = Vec::with_capacity(count);
                for _ in 0..count {
                    let key: [u8; 16] = self
                        .take(16)?
                        .try_into()
                        .map_err(|_| Error::InvalidProfile)?;
                    if map
                        .last()
                        .is_some_and(|(previous, _): &([u8; 16], u64)| previous >= &key)
                    {
                        return Err(Error::InvalidProfile);
                    }
                    map.push((key, self.number64()?));
                }
                Value::IdMap(map)
            }
            Field::Set => {
                let count = self.count(64, 4)?;
                self.reserve::<u32>(count)?;
                let mut set = Vec::with_capacity(count);
                for _ in 0..count {
                    let value = self.number32()?;
                    if value == 0 || set.last().is_some_and(|previous| *previous >= value) {
                        return Err(Error::InvalidProfile);
                    }
                    set.push(value);
                }
                Value::Set(set)
            }
            Field::Record(domain, kind) => {
                self.reserve::<Record>(1)?;
                Value::Record(Box::new(self.nested(domain, kind, depth)?))
            }
            Field::Records(domain, kind, sort) => {
                let count = self.count(MAX_ENTRIES, 17)?;
                self.reserve::<Record>(count)?;
                let mut records = Vec::with_capacity(count);
                let mut previous = None;
                for _ in 0..count {
                    let record = self.nested(domain, kind, depth)?;
                    if let Some(index) = sort {
                        let Some(Value::Id(key)) = record.fields.get(index) else {
                            return Err(Error::InvalidProfile);
                        };
                        if previous.is_some_and(|previous| previous >= *key) {
                            return Err(Error::InvalidProfile);
                        }
                        previous = Some(*key);
                    }
                    records.push(record);
                }
                Value::Records(records)
            }
            Field::Outcome => Value::Outcome(match self.take(1)?[0] {
                0 => None,
                1 => {
                    self.reserve::<Record>(1)?;
                    Some(Box::new(self.nested(6, 1, depth)?))
                }
                2 => {
                    self.reserve::<Record>(1)?;
                    Some(Box::new(self.nested(6, 10, depth)?))
                }
                _ => return Err(Error::InvalidProfile),
            }),
        })
    }
}
