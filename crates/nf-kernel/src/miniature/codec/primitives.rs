use super::super::*;
use alloc::vec::Vec;
#[derive(Default)]
pub(super) struct Budget {
    entries: usize,
}
impl Budget {
    pub fn charge(&mut self, n: usize) -> MiniatureResult<()> {
        self.entries = self
            .entries
            .checked_add(n)
            .ok_or(MiniatureRejection::Limit)?;
        if self.entries > 16384 {
            Err(MiniatureRejection::Limit)
        } else {
            Ok(())
        }
    }
}
pub(super) struct Writer<'b> {
    bytes: Vec<u8>,
    pub budget: &'b mut Budget,
}
impl<'b> Writer<'b> {
    pub fn new(kind: u16, budget: &'b mut Budget) -> MiniatureResult<Self> {
        let mut w = Self {
            bytes: Vec::new(),
            budget,
        };
        w.raw(b"NF-CANON-1\0")?;
        w.raw(&7u16.to_le_bytes())?;
        w.raw(&kind.to_le_bytes())?;
        w.raw(&2u16.to_le_bytes())?;
        Ok(w)
    }
    pub fn raw(&mut self, bytes: &[u8]) -> MiniatureResult<()> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > MAX_MINIATURE_BYTES)
        {
            return Err(MiniatureRejection::Limit);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub fn byte(&mut self, n: u8) -> MiniatureResult<()> {
        self.raw(&[n])
    }
    pub fn u32(&mut self, n: u32) -> MiniatureResult<()> {
        self.raw(&n.to_le_bytes())
    }
    pub fn u64(&mut self, n: u64) -> MiniatureResult<()> {
        self.raw(&n.to_le_bytes())
    }
    pub fn count(&mut self, n: usize) -> MiniatureResult<()> {
        self.budget.charge(n)?;
        self.u32(n.try_into().map_err(|_| MiniatureRejection::Limit)?)
    }
    pub fn blob(&mut self, bytes: &[u8], max: usize) -> MiniatureResult<()> {
        if bytes.len() > max {
            return Err(MiniatureRejection::Limit);
        }
        self.u32(
            bytes
                .len()
                .try_into()
                .map_err(|_| MiniatureRejection::Limit)?,
        )?;
        self.raw(bytes)
    }
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}
pub(super) struct Reader<'a, 'b> {
    bytes: &'a [u8],
    offset: usize,
    pub budget: &'b mut Budget,
}
impl<'a, 'b> Reader<'a, 'b> {
    pub fn new(bytes: &'a [u8], kind: u16, budget: &'b mut Budget) -> MiniatureResult<Self> {
        if bytes.len() > MAX_MINIATURE_BYTES {
            return Err(MiniatureRejection::Limit);
        }
        let mut r = Self {
            bytes,
            offset: 0,
            budget,
        };
        if r.fixed::<11>()? != *b"NF-CANON-1\0"
            || r.fixed::<2>()? != 7u16.to_le_bytes()
            || r.fixed::<2>()? != kind.to_le_bytes()
            || r.fixed::<2>()? != 2u16.to_le_bytes()
        {
            return Err(MiniatureRejection::UnsupportedProvider);
        }
        Ok(r)
    }
    fn raw(&mut self, n: usize) -> MiniatureResult<&'a [u8]> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or(MiniatureRejection::Limit)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(MiniatureRejection::InvalidValue)?;
        self.offset = end;
        Ok(bytes)
    }
    pub fn fixed<const N: usize>(&mut self) -> MiniatureResult<[u8; N]> {
        self.raw(N)?
            .try_into()
            .map_err(|_| MiniatureRejection::InvalidValue)
    }
    pub fn byte(&mut self) -> MiniatureResult<u8> {
        Ok(self.fixed::<1>()?[0])
    }
    pub fn u32(&mut self) -> MiniatureResult<u32> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }
    pub fn u64(&mut self) -> MiniatureResult<u64> {
        Ok(u64::from_le_bytes(self.fixed()?))
    }
    pub fn count(&mut self, max: usize) -> MiniatureResult<usize> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(MiniatureRejection::Limit);
        }
        self.budget.charge(n)?;
        Ok(n)
    }
    pub fn blob(&mut self, max: usize) -> MiniatureResult<&'a [u8]> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(MiniatureRejection::Limit);
        }
        self.raw(n)
    }
    pub fn done(self) -> MiniatureResult<()> {
        if self.offset != self.bytes.len() {
            Err(MiniatureRejection::InvalidValue)
        } else {
            Ok(())
        }
    }
}
