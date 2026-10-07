#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcError {
    Limit,
    Malformed,
    Incomplete,
    Io,
    Unauthorized,
    HistoryMismatch,
    SessionMismatch,
    PolicyMismatch,
    Unsupported,
    Backpressure,
    ReadOnly,
    CounterOverflow,
    PrivateStorage,
    UnknownOperation,
    Disconnected,
}
