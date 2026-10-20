//! Closed NF-CANON-1 interoperability record; never a Protobuf serialization hash.
use alloc::{string::String, vec::Vec};
use text::is_canonical_text;

pub(crate) const PREFIX: &[u8] = b"NF-CANON-1\0";
const HEADER: &[u8] = b"NF-CANON-1\0\xff\0\x01\0\x01\0";

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub record_bytes: usize,
    pub field_bytes: usize,
    pub entries: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            record_bytes: 1_048_576,
            field_bytes: 262_144,
            entries: 4096,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let maximum = Self::default();
        if self.record_bytes > maximum.record_bytes
            || self.field_bytes > maximum.field_bytes
            || self.entries > maximum.entries
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Profile {
    pub optional_bytes: Option<Vec<u8>>,
    pub optional_text: Option<String>,
    pub ordered_values: Vec<i64>,
    /// Maps arrive in canonical order. Duplicate or unordered keys must reject.
    pub map: Vec<(String, u64)>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Limit,
    InvalidProfile,
}

fn field_size(bytes: &[u8], limits: Limits) -> Result<usize, Error> {
    if bytes.len() > limits.field_bytes || u32::try_from(bytes.len()).is_err() {
        return Err(Error::Limit);
    }
    bytes.len().checked_add(4).ok_or(Error::Limit)
}
fn append_field(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    output.extend_from_slice(bytes);
}
fn encoded_size(profile: &Profile, limits: Limits) -> Result<usize, Error> {
    if profile.ordered_values.len() > limits.entries
        || profile.map.len() > limits.entries
        || u32::try_from(profile.ordered_values.len()).is_err()
        || u32::try_from(profile.map.len()).is_err()
    {
        return Err(Error::Limit);
    }
    if profile
        .map
        .windows(2)
        .any(|pair| pair[0].0.as_bytes() >= pair[1].0.as_bytes())
    {
        return Err(Error::InvalidProfile);
    }
    let mut size = HEADER.len() + 10;
    if let Some(bytes) = &profile.optional_bytes {
        size = size
            .checked_add(field_size(bytes, limits)?)
            .ok_or(Error::Limit)?;
    }
    if let Some(text) = &profile.optional_text {
        if text.len() > 4096 {
            return Err(Error::Limit);
        }
        if !is_canonical_text(text) {
            return Err(Error::InvalidProfile);
        }
        size = size
            .checked_add(field_size(text.as_bytes(), limits)?)
            .ok_or(Error::Limit)?;
    }
    size = size
        .checked_add(
            profile
                .ordered_values
                .len()
                .checked_mul(8)
                .ok_or(Error::Limit)?,
        )
        .ok_or(Error::Limit)?;
    for (key, _) in &profile.map {
        if key.len() > 4096 {
            return Err(Error::Limit);
        }
        if !is_canonical_text(key) {
            return Err(Error::InvalidProfile);
        }
        size = size
            .checked_add(field_size(key.as_bytes(), limits)? + 8)
            .ok_or(Error::Limit)?;
    }
    if size > limits.record_bytes {
        return Err(Error::Limit);
    }
    Ok(size)
}
pub fn encode_profile(profile: &Profile, limits: Limits) -> Result<Vec<u8>, Error> {
    limits.validate()?;
    let mut output = Vec::with_capacity(encoded_size(profile, limits)?);
    output.extend_from_slice(HEADER);
    match &profile.optional_bytes {
        None => output.push(0),
        Some(bytes) => {
            output.push(1);
            append_field(&mut output, bytes);
        }
    }
    match &profile.optional_text {
        None => output.push(0),
        Some(text) => {
            output.push(1);
            append_field(&mut output, text.as_bytes());
        }
    }
    output.extend_from_slice(&(profile.ordered_values.len() as u32).to_le_bytes());
    for value in &profile.ordered_values {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output.extend_from_slice(&(profile.map.len() as u32).to_le_bytes());
    for (key, value) in &profile.map {
        append_field(&mut output, key.as_bytes());
        output.extend_from_slice(&value.to_le_bytes());
    }
    Ok(output)
}

struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
    limits: Limits,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.offset.checked_add(count).ok_or(Error::Limit)?;
        let bytes = self
            .input
            .get(self.offset..end)
            .ok_or(Error::InvalidProfile)?;
        self.offset = end;
        Ok(bytes)
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
    fn field(&mut self) -> Result<&'a [u8], Error> {
        let count = self.number32()? as usize;
        if count > self.limits.field_bytes {
            return Err(Error::Limit);
        }
        self.take(count)
    }
    fn optional_field(&mut self) -> Result<Option<&'a [u8]>, Error> {
        match self.take(1)?[0] {
            0 => Ok(None),
            1 => self.field().map(Some),
            _ => Err(Error::InvalidProfile),
        }
    }
    fn text(&self, bytes: &[u8]) -> Result<String, Error> {
        if bytes.len() > 4096 {
            return Err(Error::Limit);
        }
        let value = core::str::from_utf8(bytes).map_err(|_| Error::InvalidProfile)?;
        if !is_canonical_text(value) {
            return Err(Error::InvalidProfile);
        }
        Ok(String::from(value))
    }
    fn count(&mut self, minimum_item_bytes: usize) -> Result<usize, Error> {
        let count = self.number32()? as usize;
        if count > self.limits.entries {
            return Err(Error::Limit);
        }
        let minimum = count.checked_mul(minimum_item_bytes).ok_or(Error::Limit)?;
        if minimum > self.input.len() - self.offset {
            return Err(Error::InvalidProfile);
        }
        Ok(count)
    }
}

/// Decode a closed, flat profile under caller-supplied byte/field/entry limits.
/// Unknown profiles, trailing bytes and noncanonical forms fail before acceptance.
pub fn decode_profile(input: &[u8], limits: Limits) -> Result<Profile, Error> {
    limits.validate()?;
    if input.len() > limits.record_bytes {
        return Err(Error::Limit);
    }
    let mut reader = Reader {
        input,
        offset: 0,
        limits,
    };
    if reader.take(HEADER.len())? != HEADER {
        return Err(Error::InvalidProfile);
    }
    let optional_bytes = reader.optional_field()?.map(Vec::from);
    let optional_text = reader
        .optional_field()?
        .map(|bytes| reader.text(bytes))
        .transpose()?;
    let count = reader.count(8)?;
    let mut ordered_values = Vec::with_capacity(count);
    for _ in 0..count {
        ordered_values.push(reader.number64()? as i64);
    }
    let count = reader.count(12)?;
    let mut map = Vec::with_capacity(count);
    for _ in 0..count {
        let key = reader.field()?;
        let key = reader.text(key)?;
        if map
            .last()
            .is_some_and(|(previous, _): &(String, u64)| previous.as_bytes() >= key.as_bytes())
        {
            return Err(Error::InvalidProfile);
        }
        map.push((key, reader.number64()?));
    }
    if reader.offset != input.len() {
        return Err(Error::InvalidProfile);
    }
    Ok(Profile {
        optional_bytes,
        optional_text,
        ordered_values,
        map,
    })
}
pub mod binding;
pub mod records;

pub mod text;

pub mod replica_budget;
