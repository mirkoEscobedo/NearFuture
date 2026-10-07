use crate::{
    PeerError,
    portal_config::{PortalConfig, PortalMode},
    receipt::OriginalReceipt,
    receipt_effects::ReceiptRepo,
};
use std::time::Duration;

fn same_original(a: &OriginalReceipt, b: &OriginalReceipt) -> bool {
    let x = a.source();
    let y = b.source();
    a.original() == b.original()
        && a.operation() == b.operation()
        && x.peer == y.peer
        && x.account == y.account
        && x.device == y.device
        && x.minimum_membership == y.minimum_membership
}

pub(super) fn duration(duration: Duration) -> Result<(), PeerError> {
    if !(Duration::from_millis(500)..=Duration::from_secs(60)).contains(&duration) {
        return Err(PeerError::Limit);
    }
    Ok(())
}
pub(super) fn prepare(
    repo: &mut ReceiptRepo,
    config: &PortalConfig,
    slot: u8,
) -> Result<(), PeerError> {
    if config.mode != PortalMode::Watch {
        return Err(PeerError::Malformed);
    }
    if config.originals.is_empty() || config.originals.len() > 8 || slot >= 8 {
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
        || config.protected_membership()? != c.minimum_membership
        || config.local_account.as_bytes() == &[0; 16]
        || config.local_device.as_bytes() == &[0; 16]
        || config.scope().universe.as_bytes() == &[0; 16]
        || config.scope().history.as_bytes() == &[0; 16]
    {
        return Err(PeerError::Unauthorized);
    }
    if config.addresses.is_some() {
        super::super::configuration::addresses(config, PortalMode::Watch)?;
    }
    let catalog = repo.recover_observations()?;
    if catalog.occupied() != config.originals.len() {
        return Err(PeerError::Storage);
    }
    let mut configured = [false; 8];
    for row in &config.originals {
        let Some(occupied) = configured.get_mut(row.slot as usize) else {
            return Err(PeerError::Malformed);
        };
        if *occupied {
            return Err(PeerError::Malformed);
        }
        *occupied = true;
        let head = catalog.head(row.slot)?;
        if !same_original(&row.original, &head.original)
            || row.minimum != head.minimum
            || row.head != Some((head.generation, head.digest()?))
        {
            return Err(PeerError::Unauthorized);
        }
        repo.selector(&row.original)?;
    }
    config.selected_original(slot)?;
    current(repo, config)
}
pub(super) fn current(repo: &mut ReceiptRepo, config: &PortalConfig) -> Result<(), PeerError> {
    repo.with_current_read(|cut| {
        if config.local.event_sequence > cut.source.event
            || config.local.store_revision > cut.source.store_revision
            || config.protected_membership()? > cut.membership.revision
        {
            return Err(PeerError::Policy);
        }
        Ok(())
    })
}
