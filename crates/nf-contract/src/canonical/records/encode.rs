use super::{
    Error, MAX_DEPTH, MAX_ENTRIES, MAX_FIELD_BYTES, MAX_RECORD_BYTES, MAX_TOTAL_ENTRIES, Record,
    Value,
    schema::{self, Field},
    validate_semantics,
};
use crate::canonical::text::is_canonical_text;
use alloc::vec::Vec;

pub(super) fn encode(record: &Record) -> Result<Vec<u8>, Error> {
    let mut writer = Writer {
        output: Vec::new(),
        total_entries: 0,
    };
    writer.record(record, 0)?;
    Ok(writer.output)
}
struct Writer {
    output: Vec<u8>,
    total_entries: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let size = self
            .output
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::Limit)?;
        if size > MAX_RECORD_BYTES {
            return Err(Error::Limit);
        }
        self.output.extend_from_slice(bytes);
        Ok(())
    }
    fn count(&mut self, count: usize, maximum: usize) -> Result<(), Error> {
        self.total_entries = self.total_entries.checked_add(count).ok_or(Error::Limit)?;
        if count > maximum || self.total_entries > MAX_TOTAL_ENTRIES {
            return Err(Error::Limit);
        }
        self.put(&(count as u32).to_le_bytes())
    }
    fn bytes(&mut self, bytes: &[u8], maximum: usize) -> Result<(), Error> {
        if bytes.len() > maximum {
            return Err(Error::Limit);
        }
        self.put(&(bytes.len() as u32).to_le_bytes())?;
        self.put(bytes)
    }
    fn nested(
        &mut self,
        record: &Record,
        domain: u16,
        kind: u16,
        depth: usize,
    ) -> Result<(), Error> {
        if record.domain != domain || record.kind != kind {
            return Err(Error::InvalidProfile);
        }
        self.record(record, depth + 1)
    }
    fn record(&mut self, record: &Record, depth: usize) -> Result<(), Error> {
        if depth >= MAX_DEPTH {
            return Err(Error::Limit);
        }
        if (record.domain, record.kind) == (255, 1) {
            let [Value::Fixture(profile)] = record.fields.as_slice() else {
                return Err(Error::InvalidProfile);
            };
            return self.put(&super::super::encode_profile(
                profile,
                super::super::Limits::default(),
            )?);
        }
        let fields = schema::fields(record.domain, record.kind)?;
        if fields.len() != record.fields.len() {
            return Err(Error::InvalidProfile);
        }
        self.put(super::super::PREFIX)?;
        self.put(&record.domain.to_le_bytes())?;
        self.put(&record.kind.to_le_bytes())?;
        self.put(&1_u16.to_le_bytes())?;
        for (spec, value) in fields.iter().zip(&record.fields) {
            self.value(*spec, value, depth)?;
        }
        self.total_entries = self
            .total_entries
            .checked_add(validate_semantics(record)?)
            .ok_or(Error::Limit)?;
        if self.total_entries > MAX_TOTAL_ENTRIES {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn value(&mut self, spec: Field, value: &Value, depth: usize) -> Result<(), Error> {
        match (spec, value) {
            (Field::Id, Value::Id(bytes)) => self.put(bytes),
            (Field::Digest, Value::Digest(bytes)) => self.put(bytes),
            (Field::U32, Value::U32(value)) => self.put(&value.to_le_bytes()),
            (Field::U64, Value::U64(value)) => self.put(&value.to_le_bytes()),
            (Field::Bool, Value::Bool(value)) => self.put(&[u8::from(*value)]),
            (Field::Text, Value::Text(text)) => {
                if !is_canonical_text(text) {
                    return Err(Error::InvalidProfile);
                }
                self.bytes(text.as_bytes(), 512)
            }
            (Field::Bytes, Value::Bytes(bytes)) => self.bytes(bytes, MAX_FIELD_BYTES),
            (Field::OptionalU64, Value::OptionalU64(value)) => {
                self.put(&[u8::from(value.is_some())])?;
                if let Some(value) = value {
                    self.put(&value.to_le_bytes())?;
                }
                Ok(())
            }
            (Field::IdMap, Value::IdMap(map)) => {
                if map.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
                    return Err(Error::InvalidProfile);
                }
                self.count(map.len(), MAX_ENTRIES)?;
                for (key, value) in map {
                    self.put(key)?;
                    self.put(&value.to_le_bytes())?;
                }
                Ok(())
            }
            (Field::Set, Value::Set(set)) => {
                if set.contains(&0) || set.windows(2).any(|pair| pair[0] >= pair[1]) {
                    return Err(Error::InvalidProfile);
                }
                self.count(set.len(), 64)?;
                for value in set {
                    self.put(&value.to_le_bytes())?;
                }
                Ok(())
            }
            (Field::Record(domain, kind), Value::Record(record)) => {
                self.nested(record, domain, kind, depth)
            }
            (Field::Records(domain, kind, sort), Value::Records(records)) => {
                self.count(records.len(), MAX_ENTRIES)?;
                let mut previous = None;
                for record in records {
                    if let Some(index) = sort {
                        let Some(Value::Id(key)) = record.fields.get(index) else {
                            return Err(Error::InvalidProfile);
                        };
                        if previous.is_some_and(|previous| previous >= *key) {
                            return Err(Error::InvalidProfile);
                        }
                        previous = Some(*key);
                    }
                    self.nested(record, domain, kind, depth)?;
                }
                Ok(())
            }
            (Field::Outcome, Value::Outcome(outcome)) => match outcome.as_deref() {
                None => self.put(&[0]),
                Some(record) if record.domain == 6 && record.kind == 1 => {
                    self.put(&[1])?;
                    self.nested(record, 6, 1, depth)
                }
                Some(record) if record.domain == 6 && record.kind == 10 => {
                    self.put(&[2])?;
                    self.nested(record, 6, 10, depth)
                }
                _ => Err(Error::InvalidProfile),
            },
            _ => Err(Error::InvalidProfile),
        }
    }
}
