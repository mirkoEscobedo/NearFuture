use crate::{
    PeerError,
    portal_config::{PortalConfig, PortalMode},
    receipt_effects::ReceiptRepo,
};
use libp2p::{Multiaddr, multiaddr::Protocol};
use std::time::Duration;
pub(super) fn addresses(
    config: &PortalConfig,
    mode: PortalMode,
) -> Result<[Multiaddr; 3], PeerError> {
    if config.mode != mode || mode == PortalMode::InitializeBook {
        return Err(PeerError::Malformed);
    }
    let values = config.addresses.as_ref().ok_or(PeerError::Malformed)?;
    let mut ports = [0; 3];
    for (i, a) in values.iter().enumerate() {
        if a.len() > 4096 {
            return Err(PeerError::Limit);
        }
        let mut parts = a.iter();
        let (Some(Protocol::Ip4(ip)), Some(Protocol::Tcp(port))) = (parts.next(), parts.next())
        else {
            return Err(PeerError::Malformed);
        };
        if !ip.is_loopback() {
            return Err(PeerError::Unauthorized);
        }
        match mode {
            PortalMode::Serve => {
                if parts.next().is_some() {
                    return Err(PeerError::Malformed);
                }
            }
            PortalMode::Watch => {
                if port == 0
                    || parts.next() != Some(Protocol::P2p(config.server.peer))
                    || parts.next().is_some()
                {
                    return Err(PeerError::Unauthorized);
                }
            }
            PortalMode::InitializeBook => return Err(PeerError::Malformed),
        }
        ports[i] = port;
    }
    for i in 0..3 {
        for j in i + 1..3 {
            if ports[i] != 0 && ports[j] != 0 && values[i] == values[j] {
                return Err(PeerError::Malformed);
            }
        }
    }
    Ok(values.clone())
}
pub(super) fn server(
    repo: &mut ReceiptRepo,
    config: &PortalConfig,
    duration: Duration,
) -> Result<[Multiaddr; 3], PeerError> {
    if !(Duration::from_millis(500)..=Duration::from_secs(60)).contains(&duration) {
        return Err(PeerError::Limit);
    }
    let c = repo.config();
    if config.scope() != c.scope
        || config.ruleset != c.ruleset
        || config.content != c.content
        || config.local_account != c.local_account
        || config.local_device != c.local_device
        || config.server.peer != c.server_pin.peer
        || config.server.account != c.server_pin.account
        || config.server.device != c.server_pin.device
        || config.server.minimum_membership != c.server_pin.minimum_membership
        || config.protected_membership() != Ok(c.minimum_membership)
        || !config.originals.is_empty()
        || config.local_account.as_bytes() == &[0; 16]
        || config.local_device.as_bytes() == &[0; 16]
        || config.scope().universe.as_bytes() == &[0; 16]
        || config.scope().history.as_bytes() == &[0; 16]
    {
        return Err(PeerError::Unauthorized);
    }
    let values = addresses(config, PortalMode::Serve)?;
    if !repo.book_anchors().is_empty() || repo.recover_observations()?.occupied() != 0 {
        return Err(PeerError::Storage);
    }
    repo.with_current_read(|cut| {
        if config.local.event_sequence > cut.source.event
            || config.local.store_revision > cut.source.store_revision
            || config.protected_membership()? > cut.membership.revision
            || cut.membership.owner != config.local_account
            || cut.local.peer != config.server.peer.to_bytes()
        {
            return Err(PeerError::Policy);
        }
        Ok(())
    })?;
    Ok(values)
}
