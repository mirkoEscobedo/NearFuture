use libp2p::Multiaddr;
use nf_contract::identity::{AccountId, DeviceId, EventSeq, HistoryId, UniverseId};
use nf_identity::model::Scope;
use nf_store::KnownFrontiers;
use nf_transport::{PeerError, auth::ServerPin, records::PeerLimits, session::SessionPolicy};
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};
pub struct Config {
    pub policy: SessionPolicy,
    pub known: KnownFrontiers,
    pub remote: Option<(ServerPin, Multiaddr)>,
}
pub fn hex<const N: usize>(text: &str) -> Result<[u8; N], PeerError> {
    if text.len() != N * 2 {
        return Err(PeerError::Malformed);
    }
    let mut out = [0; N];
    for (i, pair) in text.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        fn nibble(b: u8) -> Result<u8, PeerError> {
            match b {
                b'0'..=b'9' => Ok(b - b'0'),
                b'a'..=b'f' => Ok(b - b'a' + 10),
                _ => Err(PeerError::Malformed),
            }
        }
        out[i] = nibble(pair[0])? * 16 + nibble(pair[1])?;
    }
    Ok(out)
}
impl Config {
    pub fn read(path: &Path, query: bool) -> Result<Self, PeerError> {
        let mut f = File::open(path).map_err(|_| PeerError::Storage)?;
        if !f.metadata().map_err(|_| PeerError::Storage)?.is_file() {
            return Err(PeerError::Malformed);
        }
        let mut bytes = [0; 4097];
        let mut n = 0;
        while n < bytes.len() {
            let got = f.read(&mut bytes[n..]).map_err(|_| PeerError::Storage)?;
            if got == 0 {
                break;
            }
            n += got;
        }
        if n > 4096 {
            return Err(PeerError::Limit);
        }
        let text = std::str::from_utf8(&bytes[..n]).map_err(|_| PeerError::Malformed)?;
        let mut lines = text.lines();
        if lines.next() != Some("NF-PEER-CONFIG-1") {
            return Err(PeerError::Unsupported);
        }
        let base = [
            "universe",
            "history",
            "ruleset",
            "content",
            "event_sequence",
            "store_revision",
            "membership_revision",
        ];
        let extra = [
            "server_peer",
            "server_account",
            "server_device",
            "control_address",
        ];
        let mut fields = BTreeMap::new();
        for line in lines {
            let (k, v) = line.split_once('=').ok_or(PeerError::Malformed)?;
            if !base.contains(&k) && !(query && extra.contains(&k))
                || v.is_empty()
                || fields.contains_key(k)
            {
                return Err(PeerError::Malformed);
            }
            if fields.len() >= 11 {
                return Err(PeerError::Limit);
            }
            fields.insert(k, v);
        }
        if fields.len() != base.len() + if query { extra.len() } else { 0 } {
            return Err(PeerError::Malformed);
        }
        let get = |k| fields.get(k).copied().ok_or(PeerError::Malformed);
        let number = |k| {
            let v = get(k)?;
            if v.len() > 20 || !v.bytes().all(|v| v.is_ascii_digit()) {
                return Err(PeerError::Malformed);
            }
            v.parse::<u64>().map_err(|_| PeerError::Malformed)
        };
        let scope = Scope {
            universe: UniverseId::from_bytes(hex(get("universe")?)?),
            history: HistoryId::from_bytes(hex(get("history")?)?),
        };
        let member = number("membership_revision")?;
        let policy = SessionPolicy {
            scope,
            ruleset: hex(get("ruleset")?)?,
            content: hex(get("content")?)?,
            limits: PeerLimits::default(),
            minimum_membership: member,
        };
        let known = KnownFrontiers {
            scope,
            event_sequence: EventSeq(number("event_sequence")?),
            store_revision: number("store_revision")?,
            membership_revision: Some(member),
        };
        let remote = if query {
            let peer_text = get("server_peer")?;
            let addr = get("control_address")?;
            if peer_text.len() > 128 || addr.len() > 256 {
                return Err(PeerError::Limit);
            }
            let peer = peer_text
                .parse::<libp2p::PeerId>()
                .map_err(|_| PeerError::Malformed)?;
            if peer.to_string() != peer_text {
                return Err(PeerError::Malformed);
            }
            Some((
                ServerPin {
                    peer,
                    account: AccountId::from_bytes(hex(get("server_account")?)?),
                    device: DeviceId::from_bytes(hex(get("server_device")?)?),
                    minimum_membership: member,
                },
                addr.parse::<Multiaddr>()
                    .map_err(|_| PeerError::Malformed)?,
            ))
        } else {
            None
        };
        Ok(Self {
            policy,
            known,
            remote,
        })
    }
}
