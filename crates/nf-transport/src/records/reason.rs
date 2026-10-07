use crate::PeerError;
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedReason {
    KernelOutcome = 1,
    HistorySync = 2,
    SnapshotInstall = 3,
    PubSub = 4,
    Unregistered = 5,
    Offline = 6,
    PolicyDenied = 7,
    Backpressure = 8,
}
impl TryFrom<u8> for UnsupportedReason {
    type Error = PeerError;
    fn try_from(v: u8) -> Result<Self, PeerError> {
        match v {
            1 => Ok(Self::KernelOutcome),
            2 => Ok(Self::HistorySync),
            3 => Ok(Self::SnapshotInstall),
            4 => Ok(Self::PubSub),
            5 => Ok(Self::Unregistered),
            6 => Ok(Self::Offline),
            7 => Ok(Self::PolicyDenied),
            8 => Ok(Self::Backpressure),
            _ => Err(PeerError::Malformed),
        }
    }
}
