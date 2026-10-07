//! Inert, single-owner preparation; only consuming start creates a runtime deadline.
use super::{PortalClient, configuration};
use crate::{
    PeerError,
    notification::NotifyLimits,
    notification_effects::NotifyLane,
    portal_config::{PortalConfig, PortalMode},
    receipt_effects::{ReceiptLane, ReceiptRepo},
};
use libp2p::Multiaddr;
use std::time::{Duration, Instant};

pub struct PreparedPortalClient {
    repo: ReceiptRepo,
    config: PortalConfig,
    slot: u8,
    receipt: ReceiptLane,
    notify: NotifyLane,
}
impl PreparedPortalClient {
    pub(super) fn new(
        mut repo: ReceiptRepo,
        config: PortalConfig,
        slot: u8,
    ) -> Result<Self, PeerError> {
        configuration::prepare(&mut repo, &config, slot)?;
        // Existing lane constructors retain their complete protected-book recovery.
        let receipt = ReceiptLane::client(&repo, slot)?;
        let notify = NotifyLane::client(&repo, slot, 30, NotifyLimits::default())?;
        Ok(Self {
            repo,
            config,
            slot,
            receipt,
            notify,
        })
    }
    /// Consumes preparation once, checks all three explicit pins and current SQL,
    /// then creates the first finite owner deadline before either actual dial.
    pub fn start(
        mut self,
        addresses: [Multiaddr; 3],
        duration: Duration,
    ) -> Result<PortalClient, PeerError> {
        configuration::duration(duration)?;
        if self
            .config
            .addresses
            .as_ref()
            .is_some_and(|expected| expected != &addresses)
        {
            return Err(PeerError::Unauthorized);
        }
        self.config.selected_original(self.slot)?;
        self.config.addresses = Some(addresses);
        let addresses = super::super::configuration::addresses(&self.config, PortalMode::Watch)?;
        configuration::current(&mut self.repo, &self.config)?;
        let until = Instant::now()
            .checked_add(duration)
            .ok_or(PeerError::Limit)?;
        self.receipt.dial(addresses[0].clone())?;
        self.notify.dial(addresses[2].clone())?;
        Ok(PortalClient {
            repo: self.repo,
            receipt: self.receipt,
            notify: self.notify,
            until,
            wake: Box::pin(tokio::time::sleep(Duration::from_millis(25))),
            cursor: 0,
            receipt_authenticated: false,
            subscribed: false,
            query_in_flight: false,
            closed: false,
        })
    }
}
