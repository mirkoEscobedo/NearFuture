use crate::PeerError;
use libp2p::{Multiaddr, PeerId};
use std::{net::Ipv4Addr, str::FromStr};
pub(super) fn number(text: &str, max: u64) -> Result<u64, PeerError> {
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(PeerError::Malformed);
    }
    let value = text.parse::<u64>().map_err(|_| PeerError::Malformed)?;
    if value > max {
        return Err(PeerError::Limit);
    }
    Ok(value)
}
pub(super) fn hex<const N: usize>(text: &str) -> Result<[u8; N], PeerError> {
    if text.len() != N * 2 {
        return Err(PeerError::Malformed);
    }
    let mut out = [0; N];
    for (i, pair) in text.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        out[i] = nibble(pair[0])? * 16 + nibble(pair[1])?;
    }
    Ok(out)
}
fn nibble(b: u8) -> Result<u8, PeerError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(PeerError::Malformed),
    }
}
pub(super) fn id(text: &str) -> Result<[u8; 16], PeerError> {
    let bytes = hex(text)?;
    if bytes == [0; 16] {
        return Err(PeerError::Malformed);
    }
    Ok(bytes)
}
pub(super) fn peer(text: &str) -> Result<PeerId, PeerError> {
    let value = PeerId::from_str(text).map_err(|_| PeerError::Malformed)?;
    if value.to_bytes().is_empty() || value.to_bytes().len() > 128 || value.to_string() != text {
        return Err(PeerError::Malformed);
    }
    Ok(value)
}
pub(super) fn address(text: &str, remote: Option<PeerId>) -> Result<Multiaddr, PeerError> {
    let mut parts = text.split('/');
    if parts.next() != Some("") || parts.next() != Some("ip4") {
        return Err(PeerError::Malformed);
    }
    let ip_text = parts.next().ok_or(PeerError::Malformed)?;
    let ip = Ipv4Addr::from_str(ip_text).map_err(|_| PeerError::Malformed)?;
    if !ip.is_loopback() || ip.to_string() != ip_text || parts.next() != Some("tcp") {
        return Err(PeerError::Malformed);
    }
    let port_text = parts.next().ok_or(PeerError::Malformed)?;
    let port = number(port_text, u16::MAX.into())?;
    if let Some(expected) = remote
        && (port == 0
            || parts.next() != Some("p2p")
            || peer(parts.next().ok_or(PeerError::Malformed)?)? != expected)
    {
        return Err(PeerError::Malformed);
    }
    if parts.next().is_some() {
        return Err(PeerError::Malformed);
    }
    let value = Multiaddr::from_str(text).map_err(|_| PeerError::Malformed)?;
    if value.to_string() != text {
        return Err(PeerError::Malformed);
    }
    Ok(value)
}
