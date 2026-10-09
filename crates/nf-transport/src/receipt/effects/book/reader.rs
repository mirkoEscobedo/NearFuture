use crate::PeerError;
pub(super) struct Reader<'a> {
    pub bytes: &'a [u8],
    pub at: usize,
}
impl Reader<'_> {
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], PeerError> {
        let end = self.at.checked_add(N).ok_or(PeerError::Limit)?;
        let value = self.bytes.get(self.at..end).ok_or(PeerError::Malformed)?;
        self.at = end;
        value.try_into().map_err(|_| PeerError::Malformed)
    }
    pub fn u8(&mut self) -> Result<u8, PeerError> {
        Ok(self.array::<1>()?[0])
    }
    pub fn u16(&mut self) -> Result<u16, PeerError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    pub fn u32(&mut self) -> Result<u32, PeerError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    pub fn u64(&mut self) -> Result<u64, PeerError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
}
