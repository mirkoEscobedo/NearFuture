use crate::PeerError;
pub(super) struct Reader<'a> {
    pub input: &'a [u8],
    pub position: usize,
}
impl Reader<'_> {
    pub fn take(&mut self, n: usize) -> Result<&[u8], PeerError> {
        let end = self.position.checked_add(n).ok_or(PeerError::Limit)?;
        let value = self
            .input
            .get(self.position..end)
            .ok_or(PeerError::Malformed)?;
        self.position = end;
        Ok(value)
    }
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], PeerError> {
        self.take(N)?.try_into().map_err(|_| PeerError::Malformed)
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
    pub fn finish(&self) -> Result<(), PeerError> {
        if self.position == self.input.len() {
            Ok(())
        } else {
            Err(PeerError::Malformed)
        }
    }
}
pub(super) struct Writer {
    pub bytes: Vec<u8>,
}
impl Writer {
    pub fn raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }
    pub fn u8(&mut self, value: u8) {
        self.raw(&[value]);
    }
    pub fn u16(&mut self, value: u16) {
        self.raw(&value.to_le_bytes());
    }
    pub fn u32(&mut self, value: u32) {
        self.raw(&value.to_le_bytes());
    }
    pub fn u64(&mut self, value: u64) {
        self.raw(&value.to_le_bytes());
    }
}
