use crate::BoundaryError;

/// Exact finite Java float bits. No conversion to fixed point or policy calculation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FloatBits(u32);
impl FloatBits {
    pub fn new(bits: u32) -> Result<Self, BoundaryError> {
        if bits & 0x7f80_0000 == 0x7f80_0000 {
            Err(BoundaryError::NonFinite)
        } else {
            Ok(Self(bits))
        }
    }
    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// Exact Java double bits for supplied Math.random draws, without narrowing to float.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DoubleBits(u64);
impl DoubleBits {
    pub fn new(bits: u64) -> Result<Self, BoundaryError> {
        if bits & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000 {
            Err(BoundaryError::NonFinite)
        } else {
            Ok(Self(bits))
        }
    }
    pub const fn bits(self) -> u64 {
        self.0
    }
}
