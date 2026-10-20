#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeerError {
    Limit,
    RateLimited { retry_after_ms: u32 },
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
