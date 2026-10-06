use crate::Rejection;
use alloc::vec::Vec;
pub(super) const MAX_BYTES: usize = 1_048_576;
#[derive(Default)]
pub(super) struct EntryBudget {
    used: usize,
}
impl EntryBudget {
    fn reserve(&mut self, entries: usize) -> Result<(), Rejection> {
        let next = self.used.checked_add(entries).ok_or(Rejection::Limit)?;
        if next > 16384 {
            return Err(Rejection::Limit);
        }
        self.used = next;
        Ok(())
    }
}
pub(super) struct Writer<'b> {
    output: Vec<u8>,
    pub budget: &'b mut EntryBudget,
}
impl<'b> Writer<'b> {
    pub fn header(kind: u16, budget: &'b mut EntryBudget) -> Self {
        let mut w = Self {
            output: Vec::new(),
            budget,
        };
        w.output.extend_from_slice(b"NF-CANON-1\0");
        w.u16(7);
        w.u16(kind);
        w.u16(1);
        w
    }
    pub fn u16(&mut self, v: u16) {
        self.output.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.output.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.output.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i64(&mut self, v: i64) {
        self.output.extend_from_slice(&v.to_le_bytes());
    }
    pub fn fixed(&mut self, v: &[u8]) {
        self.output.extend_from_slice(v);
    }
    pub fn count(&mut self, n: usize) -> Result<(), Rejection> {
        self.budget.reserve(n)?;
        self.u32(n.try_into().map_err(|_| Rejection::Limit)?);
        Ok(())
    }
    pub fn bytes(&mut self, bytes: &[u8]) -> Result<(), Rejection> {
        if bytes.len() > 262144 {
            return Err(Rejection::Limit);
        }
        self.u32(bytes.len() as u32);
        self.fixed(bytes);
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<u8>, Rejection> {
        if self.output.len() > MAX_BYTES {
            Err(Rejection::Limit)
        } else {
            Ok(self.output)
        }
    }
}
pub(super) struct Reader<'a, 'b> {
    bytes: &'a [u8],
    offset: usize,
    pub budget: &'b mut EntryBudget,
}
impl<'a, 'b> Reader<'a, 'b> {
    pub fn new(bytes: &'a [u8], kind: u16, budget: &'b mut EntryBudget) -> Result<Self, Rejection> {
        if bytes.len() > MAX_BYTES {
            return Err(Rejection::Limit);
        }
        let mut r = Self {
            bytes,
            offset: 0,
            budget,
        };
        if r.fixed::<11>()? != *b"NF-CANON-1\0"
            || r.u16()? != 7
            || r.u16()? != kind
            || r.u16()? != 1
        {
            return Err(Rejection::InvalidValue);
        }
        Ok(r)
    }
    pub fn fixed<const N: usize>(&mut self) -> Result<[u8; N], Rejection> {
        let end = self.offset.checked_add(N).ok_or(Rejection::Limit)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Rejection::InvalidValue)?;
        let mut out = [0; N];
        out.copy_from_slice(bytes);
        self.offset = end;
        Ok(out)
    }
    pub fn u16(&mut self) -> Result<u16, Rejection> {
        Ok(u16::from_le_bytes(self.fixed()?))
    }
    pub fn u32(&mut self) -> Result<u32, Rejection> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }
    pub fn u64(&mut self) -> Result<u64, Rejection> {
        Ok(u64::from_le_bytes(self.fixed()?))
    }
    pub fn i64(&mut self) -> Result<i64, Rejection> {
        Ok(i64::from_le_bytes(self.fixed()?))
    }
    pub fn count(&mut self, max: usize) -> Result<usize, Rejection> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(Rejection::Limit);
        }
        self.budget.reserve(n)?;
        Ok(n)
    }
    pub fn bytes(&mut self) -> Result<&'a [u8], Rejection> {
        let n = self.u32()? as usize;
        if n > 262144 {
            return Err(Rejection::Limit);
        }
        let end = self.offset.checked_add(n).ok_or(Rejection::Limit)?;
        let out = self
            .bytes
            .get(self.offset..end)
            .ok_or(Rejection::InvalidValue)?;
        self.offset = end;
        Ok(out)
    }
    pub fn done(self) -> Result<(), Rejection> {
        if self.offset != self.bytes.len() {
            Err(Rejection::InvalidValue)
        } else {
            Ok(())
        }
    }
}
