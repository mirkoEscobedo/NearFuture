use super::{primitives::*, *};
use std::{collections::BTreeSet, str::SplitTerminator};
struct Lines<'a>(SplitTerminator<'a, char>);
impl<'a> Lines<'a> {
    fn next(&mut self) -> Result<&'a str, PeerError> {
        self.0.next().ok_or(PeerError::Malformed)
    }
    fn value(&mut self, key: &str) -> Result<&'a str, PeerError> {
        let (actual, value) = self.next()?.split_once(' ').ok_or(PeerError::Malformed)?;
        if actual != key || value.is_empty() || value.contains(' ') {
            return Err(PeerError::Malformed);
        }
        Ok(value)
    }
}
pub(super) fn parse(bytes: &[u8], expected: PortalMode) -> Result<PortalConfig, PeerError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(PeerError::Limit);
    }
    if !bytes.ends_with(b"\n") || bytes.iter().any(|b| *b != b'\n' && !(32..=126).contains(b)) {
        return Err(PeerError::Malformed);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| PeerError::Malformed)?;
    if text
        .split_terminator('\n')
        .any(|line| line.is_empty() || line.ends_with(' ') || line.contains("  "))
    {
        return Err(PeerError::Malformed);
    }
    let mut lines = Lines(text.split_terminator('\n'));
    if lines.next()? != "NF-PORTAL-CONFIG-1" {
        return Err(PeerError::Malformed);
    }
    let mode = match lines.value("mode")? {
        "serve" => PortalMode::Serve,
        "watch" => PortalMode::Watch,
        "init-book" => PortalMode::InitializeBook,
        _ => return Err(PeerError::Malformed),
    };
    if mode != expected {
        return Err(PeerError::Malformed);
    }
    let scope = Scope {
        universe: UniverseId::from_bytes(id(lines.value("universe")?)?),
        history: HistoryId::from_bytes(id(lines.value("history")?)?),
    };
    let ruleset = hex(lines.value("ruleset")?)?;
    let content = hex(lines.value("content")?)?;
    let local_account = AccountId::from_bytes(id(lines.value("local_account")?)?);
    let local_device = DeviceId::from_bytes(id(lines.value("local_device")?)?);
    let local = KnownFrontiers {
        scope,
        event_sequence: EventSeq(number(lines.value("local_event_min")?, u64::MAX)?),
        store_revision: number(lines.value("local_store_min")?, u64::MAX)?,
        membership_revision: Some(number(lines.value("local_membership_min")?, u64::MAX)?),
    };
    let server = ServerPin {
        peer: peer(lines.value("server_peer")?)?,
        account: AccountId::from_bytes(id(lines.value("server_account")?)?),
        device: DeviceId::from_bytes(id(lines.value("server_device")?)?),
        minimum_membership: number(lines.value("server_membership_min")?, u64::MAX)?,
    };
    let mut config = PortalConfig {
        mode,
        local,
        ruleset,
        content,
        local_account,
        local_device,
        server,
        addresses: None,
        originals: Vec::new(),
    };
    tail(&mut lines, &mut config)?;
    if lines.next()? != "END" || lines.0.next().is_some() {
        return Err(PeerError::Malformed);
    }
    Ok(config)
}
fn tail(lines: &mut Lines<'_>, config: &mut PortalConfig) -> Result<(), PeerError> {
    let mode = config.mode;
    if mode != PortalMode::InitializeBook {
        let keys = if mode == PortalMode::Serve {
            ["receipt_listen", "bulk_listen", "notification_listen"]
        } else {
            ["receipt_address", "bulk_address", "notification_address"]
        };
        let remote = (mode == PortalMode::Watch).then_some(config.server.peer);
        let addresses = [
            address(lines.value(keys[0])?, remote)?,
            address(lines.value(keys[1])?, remote)?,
            address(lines.value(keys[2])?, remote)?,
        ];
        for (i, first) in addresses.iter().enumerate() {
            for second in &addresses[i + 1..] {
                if first == second
                    && (mode == PortalMode::Watch || !first.to_string().ends_with("/tcp/0"))
                {
                    return Err(PeerError::Malformed);
                }
            }
        }
        config.addresses = Some(addresses);
    }
    let key = if mode == PortalMode::InitializeBook {
        "original_count"
    } else {
        "anchor_count"
    };
    let count = number(lines.value(key)?, 8)? as usize;
    if (mode == PortalMode::Serve && count != 0) || (mode != PortalMode::Serve && count == 0) {
        return Err(PeerError::Malformed);
    }
    let mut requests = BTreeSet::new();
    for _ in 0..count {
        let row = super::rows::parse(lines.next()?, config)?;
        if config
            .originals
            .last()
            .is_some_and(|previous| previous.slot >= row.slot)
            || !requests.insert(row.original.request())
        {
            return Err(PeerError::Malformed);
        }
        config.originals.push(row);
    }
    Ok(())
}
