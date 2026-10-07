use crate::*;
use alloc::vec::Vec;
use nf_contract::identity::*;
pub const MAX_COMPONENT_BYTES: usize = 131072;
mod read;
mod write;
pub use read::decode_component;
pub use write::encode_component;
pub(super) struct Reader<'a> {
    input: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Result<Self, WorldError> {
        if input.len() > MAX_COMPONENT_BYTES {
            return Err(WorldError::Limit);
        }
        Ok(Self { input, position: 0 })
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], WorldError> {
        let end = self.position.checked_add(N).ok_or(WorldError::Limit)?;
        let value = self
            .input
            .get(self.position..end)
            .ok_or(WorldError::Malformed)?
            .try_into()
            .map_err(|_| WorldError::Malformed)?;
        self.position = end;
        Ok(value)
    }
    fn id(&mut self) -> Result<EntityId, WorldError> {
        Ok(EntityId::from_bytes(self.array()?))
    }
    fn u8(&mut self) -> Result<u8, WorldError> {
        Ok(self.array::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, WorldError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, WorldError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn count(&mut self, max: usize, exact: Option<usize>) -> Result<usize, WorldError> {
        let n = u32::from_le_bytes(self.array()?) as usize;
        if n > max {
            return Err(WorldError::Limit);
        }
        if exact.is_some_and(|e| n != e) {
            return Err(WorldError::InvalidValue);
        }
        Ok(n)
    }
    fn industry(&mut self) -> Result<IndustryKind, WorldError> {
        match self.u8()? {
            1 => Ok(IndustryKind::Farming),
            2 => Ok(IndustryKind::Workshop),
            _ => Err(WorldError::Unsupported),
        }
    }
}
pub(super) fn count(b: &mut Vec<u8>, n: usize) {
    b.extend((n as u32).to_le_bytes());
}
