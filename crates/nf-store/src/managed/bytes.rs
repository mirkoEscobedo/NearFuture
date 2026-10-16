use super::error::{ManagedError, Result};
use sha2::{Digest, Sha256};

pub(super) fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub(super) struct Encoder {
    bytes: Vec<u8>,
    cap: usize,
}
impl Encoder {
    pub fn new(domain: &[u8], cap: usize) -> Result<Self> {
        let mut out = Self {
            bytes: Vec::new(),
            cap,
        };
        out.raw(domain)?;
        out.raw(&1_u16.to_le_bytes())?;
        Ok(out)
    }
    pub fn plain(cap: usize) -> Self {
        Self {
            bytes: Vec::new(),
            cap,
        }
    }
    pub fn raw(&mut self, value: &[u8]) -> Result<()> {
        let len = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or(ManagedError::Limit)?;
        if len > self.cap {
            return Err(ManagedError::Limit);
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    pub fn byte(&mut self, value: u8) -> Result<()> {
        self.raw(&[value])
    }
    pub fn u32(&mut self, value: u32) -> Result<()> {
        self.raw(&value.to_le_bytes())
    }
    pub fn u64(&mut self, value: u64) -> Result<()> {
        self.raw(&value.to_be_bytes())
    }
    pub fn blob(&mut self, value: &[u8], cap: usize) -> Result<()> {
        if value.len() > cap {
            return Err(ManagedError::Limit);
        }
        self.u32(value.len().try_into().map_err(|_| ManagedError::Limit)?)?;
        self.raw(value)
    }
    pub fn option<T>(
        &mut self,
        value: Option<T>,
        write: impl FnOnce(&mut Self, T) -> Result<()>,
    ) -> Result<()> {
        match value {
            None => self.byte(0),
            Some(value) => {
                self.byte(1)?;
                write(self, value)
            }
        }
    }
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}
pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8], domain: &[u8], cap: usize) -> Result<Self> {
        let mut input = Self::plain(bytes, cap)?;
        if input.take(domain.len())? != domain || input.take(2)? != 1_u16.to_le_bytes() {
            return Err(ManagedError::Corrupt);
        }
        Ok(input)
    }
    pub fn plain(bytes: &'a [u8], cap: usize) -> Result<Self> {
        if bytes.len() > cap {
            return Err(ManagedError::Limit);
        }
        Ok(Self { bytes, at: 0 })
    }
    pub fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(len).ok_or(ManagedError::Corrupt)?;
        let value = self.bytes.get(self.at..end).ok_or(ManagedError::Corrupt)?;
        self.at = end;
        Ok(value)
    }
    pub fn fixed<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.take(N)?.try_into().map_err(|_| ManagedError::Corrupt)
    }
    pub fn byte(&mut self) -> Result<u8> {
        Ok(self.fixed::<1>()?[0])
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.fixed()?))
    }
    pub fn blob(&mut self, cap: usize) -> Result<&'a [u8]> {
        let len = usize::try_from(self.u32()?).map_err(|_| ManagedError::Limit)?;
        if len > cap {
            return Err(ManagedError::Limit);
        }
        self.take(len)
    }
    pub fn option<T>(&mut self, read: impl FnOnce(&mut Self) -> Result<T>) -> Result<Option<T>> {
        match self.byte()? {
            0 => Ok(None),
            1 => read(self).map(Some),
            _ => Err(ManagedError::Corrupt),
        }
    }
    pub fn finish(self) -> Result<()> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(ManagedError::Corrupt)
        }
    }
}
pub(super) fn same(actual: &[u8], canonical: &[u8]) -> Result<()> {
    if actual == canonical {
        Ok(())
    } else {
        Err(ManagedError::Corrupt)
    }
}
pub(super) fn next(value: u64) -> Result<u64> {
    value.checked_add(1).ok_or(ManagedError::Limit)
}
