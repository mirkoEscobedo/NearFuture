//! A private libp2p key is separate from nf-identity device/account seeds. Missing keys never regenerate.
use crate::{
    PeerError,
    network::{PeerBehaviour, build_lane_swarm},
    records::Lane,
};
use nf_identity::private_storage::PrivateVault;
use zeroize::Zeroizing;
const NAME: &str = "transport-ed25519-v1";
pub struct TransportIdentity {
    key: libp2p::identity::Keypair,
}
impl std::fmt::Debug for TransportIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TransportIdentity(REDACTED)")
    }
}
impl TransportIdentity {
    pub fn create(vault: &PrivateVault) -> Result<Self, PeerError> {
        let key = libp2p::identity::Keypair::generate_ed25519();
        let bytes = Zeroizing::new(key.to_protobuf_encoding().map_err(|_| PeerError::Storage)?);
        vault
            .create_private_blob(NAME, &bytes)
            .map_err(|_| PeerError::Storage)?;
        Ok(Self { key })
    }
    pub fn load(vault: &PrivateVault) -> Result<Self, PeerError> {
        let bytes = Zeroizing::new(
            vault
                .read_private_blob(NAME)
                .map_err(|_| PeerError::Storage)?,
        );
        let key = libp2p::identity::Keypair::from_protobuf_encoding(&bytes)
            .map_err(|_| PeerError::Storage)?;
        if key.key_type() != libp2p::identity::KeyType::Ed25519 {
            return Err(PeerError::Unsupported);
        }
        Ok(Self { key })
    }
    pub fn peer_id(&self) -> libp2p::PeerId {
        self.key.public().to_peer_id()
    }
    pub fn build_lane(&self, lane: Lane) -> Result<libp2p::Swarm<PeerBehaviour>, PeerError> {
        build_lane_swarm(self.key.clone(), lane)
    }
}
