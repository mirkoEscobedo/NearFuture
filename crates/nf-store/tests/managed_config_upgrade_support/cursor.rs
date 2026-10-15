//! Bounded canonical byte observations; this never implements admission or authority.
#[derive(Clone)]
pub struct Cursor<'a> {
    pub bytes: &'a [u8],
    pub at: usize,
}
impl<'a> Cursor<'a> {
    pub fn new(bytes: &'a [u8], domain: &[u8], cap: usize) -> Self {
        assert!(bytes.len() <= cap);
        assert!(bytes.starts_with(domain));
        let mut value = Self {
            bytes,
            at: domain.len(),
        };
        assert_eq!(value.take(2), 1u16.to_le_bytes());
        value
    }
    pub fn take(&mut self, size: usize) -> &'a [u8] {
        let end = self.at.checked_add(size).unwrap();
        let value = self.bytes.get(self.at..end).unwrap();
        self.at = end;
        value
    }
    pub fn byte(&mut self) -> u8 {
        self.take(1)[0]
    }
    pub fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().unwrap())
    }
    pub fn u64(&mut self) -> u64 {
        u64::from_be_bytes(self.take(8).try_into().unwrap())
    }
    pub fn blob(&mut self, cap: usize) -> &'a [u8] {
        let size = usize::try_from(self.u32()).unwrap();
        assert!(size <= cap);
        self.take(size)
    }
    pub fn option(&mut self, size: usize) -> Option<&'a [u8]> {
        match self.byte() {
            0 => None,
            1 => Some(self.take(size)),
            _ => panic!("noncanonical Option"),
        }
    }
    pub fn finish(self) {
        assert_eq!(self.at, self.bytes.len());
    }
}
