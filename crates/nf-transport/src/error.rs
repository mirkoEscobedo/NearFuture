#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeerError {
    Limit,
    Malformed,
    Unsupported,
    Scope,
    Policy,
    Session,
    Unauthorized,
    Offline,
    Backpressure,
    Storage,
    Replay,
    Entropy,
}
