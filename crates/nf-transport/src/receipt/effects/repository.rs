use super::{book::*, model::*};
use crate::{
    PeerError,
    auth::ServerPin,
    identity::TransportIdentity,
    receipt::{OriginalReceipt, SourceMinima},
};
use nf_contract::identity::{AccountId, DeviceId};
use nf_identity::{
    keys::SecretSeed,
    model::{MembershipRepository, MembershipState, PublicIdentity, Scope},
    private_storage::PrivateVault,
};
use nf_store::Store;
#[derive(Clone, Copy, Debug)]
pub struct ReceiptConfig {
    pub scope: Scope,
    pub ruleset: [u8; 32],
    pub content: [u8; 32],
    pub local_account: AccountId,
    pub local_device: DeviceId,
    pub server_pin: ServerPin,
    pub minimum_membership: u64,
}
/// Exactly one repository owns SQLite, credentials and observational recovery metadata.
/// No wire handler receives a Store handle or mutation closure.
pub struct ReceiptRepo {
    pub(super) store: Store,
    pub(super) vault: PrivateVault,
    pub(super) transport: TransportIdentity,
    pub(super) local: PublicIdentity,
    pub(super) device_key: SecretSeed,
    pub(super) config: ReceiptConfig,
    pub(super) anchors: Vec<BookAnchor>,
}
impl ReceiptRepo {
    /// Loads existing identity only. Book recovery is explicit and never creates missing roots.
    pub fn open_owned(
        mut store: Store,
        vault: PrivateVault,
        config: ReceiptConfig,
        anchors: &[BookAnchor],
    ) -> Result<Self, PeerError> {
        if anchors.len() > 8 {
            return Err(PeerError::Limit);
        }
        let transport = TransportIdentity::load(&vault)?;
        let identity = vault
            .load_identity(transport.peer_id().to_bytes())
            .map_err(|_| PeerError::Storage)?;
        if identity.public.account != config.local_account
            || identity.public.device != config.local_device
        {
            return Err(PeerError::Unauthorized);
        }
        let state = store
            .load_membership(config.scope)
            .map_err(|_| PeerError::Storage)?
            .ok_or(PeerError::Policy)?;
        let source = source(&store, &state, config.ruleset)?;
        validate(&config, &identity.public, &state, source)?;
        for anchor in anchors {
            validate_original(&config, &identity.public, &anchor.original)?;
            if anchor.ruleset != config.ruleset || anchor.content != config.content {
                return Err(PeerError::Scope);
            }
        }
        let catalog = recover_book(&vault, anchors)?;
        Ok(Self {
            store,
            vault,
            transport,
            local: identity.public,
            device_key: identity.device_key,
            config,
            anchors: catalog.anchors()?,
        })
    }
    pub fn local_public(&self) -> &PublicIdentity {
        &self.local
    }
    pub fn config(&self) -> ReceiptConfig {
        self.config
    }
    pub fn book_anchors(&self) -> &[BookAnchor] {
        &self.anchors
    }
    pub fn recover_observations(&self) -> Result<BookCatalog, PeerError> {
        self.validate_book_context()?;
        recover_book(&self.vault, &self.anchors)
    }
    /// Explicit registration before network timing starts. The entire64-name inventory must be empty.
    pub fn initialize_originals(
        &mut self,
        originals: &[(u8, OriginalReceipt, SourceMinima)],
    ) -> Result<Vec<BookAnchor>, PeerError> {
        self.initialize_originals_inner(originals)
            .map_err(BookFailure::peer_error)
    }
    fn initialize_originals_inner(
        &mut self,
        originals: &[(u8, OriginalReceipt, SourceMinima)],
    ) -> BookResult<Vec<BookAnchor>> {
        if !self.anchors.is_empty() {
            return Err(PeerError::Backpressure.into());
        }
        if originals.is_empty() || originals.len() > 8 {
            return Err(PeerError::Limit.into());
        }
        let mut records = Vec::with_capacity(originals.len());
        for (slot, original, minimum) in originals {
            validate_original(&self.config, &self.local, original)?;
            records.push((
                *slot,
                BookRecord {
                    generation: 0,
                    previous: [0; 32],
                    original: original.clone(),
                    ruleset: self.config.ruleset,
                    content: self.config.content,
                    minimum: *minimum,
                    phase: BookPhase::Unobserved,
                },
            ));
        }
        let catalog = initialize_book_detailed(&self.vault, &records)?;
        self.anchors = catalog.anchors()?;
        Ok(self.anchors.clone())
    }
    #[cfg(test)]
    pub(crate) fn initialize_originals_for_test(
        &mut self,
        originals: &[(u8, OriginalReceipt, SourceMinima)],
    ) -> BookResult<Vec<BookAnchor>> {
        self.initialize_originals_inner(originals)
    }
    pub(super) fn validate_book_context(&self) -> Result<(), PeerError> {
        if self
            .anchors
            .iter()
            .any(|a| a.ruleset != self.config.ruleset || a.content != self.config.content)
        {
            return Err(PeerError::Scope);
        }
        Ok(())
    }
    /// Unit-test-only fixed 5.1s synchronous delay after a selected genuine current SQL read.
    #[cfg(test)]
    pub(crate) fn delay_current_read_for_test(skip: usize) -> super::read_delay::Guard {
        super::read_delay::arm(skip)
    }
    pub(crate) fn with_current_read<R>(
        &mut self,
        f: impl for<'a> FnOnce(CurrentRead<'a>) -> Result<R, PeerError>,
    ) -> Result<R, PeerError> {
        let state = self
            .store
            .load_membership(self.config.scope)
            .map_err(|_| PeerError::Storage)?
            .ok_or(PeerError::Policy)?;
        let source = source(&self.store, &state, self.config.ruleset)?;
        validate(&self.config, &self.local, &state, source)?;
        #[cfg(test)]
        super::read_delay::after_current_read();
        f(CurrentRead {
            membership: &state,
            local: &self.local,
            device_key: &self.device_key,
            source,
        })
    }
    /// Builds the separate bounded receipt lane from this repository's already loaded opaque identity.
    /// A constructed Noise lane grants no application or retained-request authority.
    /// Builds a separate notification backend using the already loaded opaque Noise identity.
    pub fn build_notification_lane(
        &self,
    ) -> Result<libp2p::Swarm<crate::notification_effects::NotifyBehaviour>, PeerError> {
        self.transport().build_notification_lane()
    }
    pub fn build_receipt_lane(&self) -> Result<libp2p::Swarm<super::ReceiptBehaviour>, PeerError> {
        self.transport().build_receipt_lane()
    }
    pub(crate) fn transport(&self) -> &TransportIdentity {
        &self.transport
    }
    pub(super) fn validate_original(&self, original: &OriginalReceipt) -> Result<(), PeerError> {
        validate_original(&self.config, &self.local, original)
    }
}
fn source(
    store: &Store,
    state: &MembershipState,
    ruleset: [u8; 32],
) -> Result<SourceMinima, PeerError> {
    if store.world().to_spec().ruleset_hash != ruleset {
        return Err(PeerError::Scope);
    }
    let known = store.known_frontiers().map_err(|_| PeerError::Storage)?;
    if known.scope != state.scope || known.membership_revision != Some(state.revision) {
        return Err(PeerError::Policy);
    }
    Ok(SourceMinima {
        event: known.event_sequence,
        store_revision: known.store_revision,
        membership_revision: state.revision,
    })
}
fn validate(
    config: &ReceiptConfig,
    local: &PublicIdentity,
    state: &MembershipState,
    current: SourceMinima,
) -> Result<(), PeerError> {
    nf_identity::codec::validate_state(state).map_err(|_| PeerError::Policy)?;
    if state.scope != config.scope
        || state.owner != config.server_pin.account
        || current.membership_revision
            < config
                .minimum_membership
                .max(config.server_pin.minimum_membership)
    {
        return Err(PeerError::Policy);
    }
    role(
        state,
        config.server_pin.account,
        config.server_pin.device,
        config.server_pin.peer,
    )?;
    let peer = libp2p::PeerId::from_bytes(&local.peer).map_err(|_| PeerError::Unauthorized)?;
    role(state, local.account, local.device, peer)?;
    if state
        .accounts
        .get(&local.account)
        .is_none_or(|a| a.key != local.account_key)
        || state
            .devices
            .get(&local.device)
            .is_none_or(|d| d.key != local.device_key)
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
fn validate_original(
    config: &ReceiptConfig,
    local: &PublicIdentity,
    original: &OriginalReceipt,
) -> Result<(), PeerError> {
    let o = original.original();
    let p = original.source();
    let expected = config.server_pin;
    if o.account_id != local.account
        || o.device_id != local.device
        || o.universe_id != config.scope.universe
        || o.history_id != config.scope.history
        || p.account != expected.account
        || p.device != expected.device
        || p.peer != expected.peer
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
