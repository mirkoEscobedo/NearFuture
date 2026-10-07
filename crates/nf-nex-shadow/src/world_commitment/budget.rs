use crate::Unavailable;
use nf_nex_boundary::{DoubleBits, FloatBits};
pub(super) trait Sink {
    fn write(&mut self, bytes: &[u8]);
}
pub(super) struct Count;
impl Sink for Count {
    fn write(&mut self, _: &[u8]) {}
}
pub(super) struct Hash(pub sha2::Sha256);
impl Sink for Hash {
    fn write(&mut self, bytes: &[u8]) {
        use sha2::Digest;
        self.0.update(bytes);
    }
}
pub(super) struct Writer<S> {
    pub sink: S,
    pub bytes: usize,
    entries: usize,
    text: usize,
}
impl<S: Sink> Writer<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            bytes: 0,
            entries: 0,
            text: 0,
        }
    }
    pub fn raw(&mut self, bytes: &[u8]) -> Result<(), Unavailable> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or(Unavailable::Limit)?;
        if self.bytes > 4_194_304 {
            return Err(Unavailable::Limit);
        }
        self.sink.write(bytes);
        Ok(())
    }
    pub fn byte(&mut self, n: u8) -> Result<(), Unavailable> {
        self.raw(&[n])
    }
    pub fn boolean(&mut self, b: bool) -> Result<(), Unavailable> {
        self.byte(u8::from(b))
    }
    pub fn u32(&mut self, n: u32) -> Result<(), Unavailable> {
        self.raw(&n.to_le_bytes())
    }
    pub fn u64(&mut self, n: u64) -> Result<(), Unavailable> {
        self.raw(&n.to_le_bytes())
    }
    pub fn float(&mut self, f: FloatBits) -> Result<(), Unavailable> {
        self.u32(f.bits())
    }
    pub fn double(&mut self, f: DoubleBits) -> Result<(), Unavailable> {
        self.u64(f.bits())
    }
    pub fn count(&mut self, n: usize, cap: usize) -> Result<(), Unavailable> {
        if n > cap {
            return Err(Unavailable::Limit);
        }
        self.entries = self.entries.checked_add(n).ok_or(Unavailable::Limit)?;
        if self.entries > 8192 {
            return Err(Unavailable::Limit);
        }
        self.u32(u32::try_from(n).map_err(|_| Unavailable::Limit)?)
    }
    pub fn text(&mut self, s: &str) -> Result<(), Unavailable> {
        if s.is_empty() || s.len() > 256 {
            return Err(Unavailable::Limit);
        }
        self.text = self.text.checked_add(s.len()).ok_or(Unavailable::Limit)?;
        if self.text > 1_048_576 {
            return Err(Unavailable::Limit);
        }
        if s.chars().any(char::is_control) || !nf_contract::canonical::text::is_canonical_text(s) {
            return Err(Unavailable::Unsupported);
        }
        self.u32(u32::try_from(s.len()).map_err(|_| Unavailable::Limit)?)?;
        self.raw(s.as_bytes())
    }
    pub fn optional_text(&mut self, s: Option<&str>) -> Result<(), Unavailable> {
        self.boolean(s.is_some())?;
        if let Some(s) = s {
            self.text(s)?;
        }
        Ok(())
    }
}
