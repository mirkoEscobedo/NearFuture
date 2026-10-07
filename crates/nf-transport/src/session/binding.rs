use super::*;
use nf_identity::model::Roles;
pub(crate) struct Binding<'a> {
    pub context: &'a HandshakeContext,
    pub local: &'a PublicIdentity,
    pub connection: libp2p::swarm::ConnectionId,
    pub revision: u64,
    pub minimum: u64,
}
impl Binding<'_> {
    pub(crate) fn current(&self, s: &MembershipState) -> Result<(), PeerError> {
        nf_identity::codec::validate_state(s).map_err(|_| PeerError::Unauthorized)?;
        if s.scope != self.context.context.scope
            || s.revision != self.revision
            || s.revision < self.minimum
            || s.owner != self.context.server_account
        {
            return Err(PeerError::Unauthorized);
        }
        for (account, device, peer) in [
            (
                self.context.server_account,
                self.context.server_device,
                self.context.server_peer,
            ),
            (
                self.context.client_account,
                self.context.client_device,
                self.context.client_peer,
            ),
        ] {
            let a = s.accounts.get(&account).ok_or(PeerError::Unauthorized)?;
            let d = s.devices.get(&device).ok_or(PeerError::Unauthorized)?;
            if d.revoked
                || d.account != account
                || d.peer != peer.to_bytes()
                || !a.roles.contains(Roles::PLAYER)
            {
                return Err(PeerError::Unauthorized);
            }
        }
        let a = s
            .accounts
            .get(&self.local.account)
            .ok_or(PeerError::Unauthorized)?;
        let d = s
            .devices
            .get(&self.local.device)
            .ok_or(PeerError::Unauthorized)?;
        if a.key != self.local.account_key
            || d.key != self.local.device_key
            || d.peer != self.local.peer
        {
            return Err(PeerError::Unauthorized);
        }
        Ok(())
    }
    pub(crate) fn record(
        &self,
        r: &PeerRecord,
        actual_peer: libp2p::PeerId,
        connection: libp2p::swarm::ConnectionId,
        expected_peer: libp2p::PeerId,
    ) -> Result<(), PeerError> {
        if actual_peer != expected_peer
            || connection != self.connection
            || r.context != self.context.context
        {
            return Err(PeerError::Session);
        }
        encode_body(r, self.context.lane, self.context.selected)?;
        Ok(())
    }
}
pub(crate) fn sign_challenge(
    context: PeerContext,
    challenge: [u8; 32],
    current: &MembershipState,
    local: &PublicIdentity,
    key: &SecretSeed,
    minimum: u64,
) -> Result<DeviceProof, PeerError> {
    if context.scope != current.scope || key.public_key() != local.device_key {
        return Err(PeerError::Unauthorized);
    }
    let mut p = DeviceProof {
        scope: current.scope,
        account: local.account,
        device: local.device,
        frontier: current.revision,
        peer: local.peer.clone(),
        challenge,
        signature: [0; 64],
    };
    p.signature = key.sign(&device_digest(&p).map_err(|_| PeerError::Unauthorized)?);
    current
        .authorize(
            &p,
            &local.peer,
            &challenge,
            minimum,
            ProtectedOperation::Economic,
        )
        .map_err(|_| PeerError::Unauthorized)?;
    Ok(p)
}
