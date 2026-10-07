use crate::{Authenticator, IpcError, LocalPrincipal, SecretToken, SessionConfig};
use nf_contract::identity::{AccountId, DeviceId};
use nf_identity::private_storage::PrivateVault;
use nf_wire::generated as g;
use std::{
    io::{Cursor, Read},
    net::{Ipv4Addr, SocketAddr},
};
use zeroize::Zeroizing;
const MAGIC: &[u8] = b"NF-IPC-R2\0";
const SIZE: usize = 218;
pub fn default_limits() -> g::ResourceLimits {
    g::ResourceLimits {
        control_frame_bytes: 16_384,
        chunk_bytes: 8192,
        inflight_bytes: 262_144,
        inflight_items: 32,
        decoded_bytes: 16_384,
        collection_items: 256,
        nesting_depth: 16,
        transfer_bytes: 1_048_576,
    }
}
/// Owner-private ephemeral descriptor. No Display/Debug/Serialize exposes the secret.
pub struct DiscoveryRecord {
    config: SessionConfig,
    principal: LocalPrincipal,
    address: SocketAddr,
    bulk_address: Option<SocketAddr>,
    token: SecretToken,
}
impl DiscoveryRecord {
    pub fn read(vault: &PrivateVault, name: &str) -> Result<Self, IpcError> {
        let bytes = Zeroizing::new(
            vault
                .read_private_blob(name)
                .map_err(|_| IpcError::PrivateStorage)?,
        );
        Self::decode(&bytes)
    }
    pub fn bulk_address(&self) -> Option<SocketAddr> {
        self.bulk_address
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
    pub fn principal(&self) -> LocalPrincipal {
        self.principal
    }
    pub fn auth_binding(&self, role: crate::EndpointRole) -> Result<crate::AuthBinding, IpcError> {
        let address = match role {
            crate::EndpointRole::Control => self.address,
            crate::EndpointRole::Bulk => self.bulk_address.ok_or(IpcError::Unsupported)?,
        };
        Ok(crate::AuthBinding {
            config: self.config.clone(),
            principal: self.principal,
            role,
            port: address.port(),
        })
    }
    pub fn authenticator(&self, role: crate::EndpointRole) -> Result<Authenticator, IpcError> {
        Authenticator::new(
            self.auth_binding(role)?,
            SecretToken::from_bytes(*self.token.private_bytes()),
        )
    }
    pub fn client(&self, role: crate::EndpointRole) -> Result<crate::ClientHello, IpcError> {
        crate::ClientHello::start(
            self.auth_binding(role)?,
            SecretToken::from_bytes(*self.token.private_bytes()),
            self.config.limits,
        )
    }
    fn encode(&self) -> Zeroizing<Vec<u8>> {
        let mut b = Zeroizing::new(Vec::with_capacity(SIZE));
        let c = &self.config;
        b.extend_from_slice(MAGIC);
        b.extend_from_slice(&self.address.port().to_le_bytes());
        b.extend_from_slice(&self.bulk_address.map_or(0, |a| a.port()).to_le_bytes());
        b.extend_from_slice(&c.runtime_session.to_le_bytes());
        for field in [
            &c.universe[..],
            &c.history[..],
            &c.ruleset[..],
            &c.content_policy[..],
            self.principal.account.as_bytes(),
            self.principal.device.as_bytes(),
            self.token.private_bytes(),
        ] {
            b.extend_from_slice(field);
        }
        let l = &c.limits;
        for n in [
            l.control_frame_bytes,
            l.chunk_bytes,
            l.inflight_bytes,
            l.inflight_items,
            l.decoded_bytes,
            l.collection_items,
            l.nesting_depth,
        ] {
            b.extend_from_slice(&n.to_le_bytes());
        }
        b.extend_from_slice(&l.transfer_bytes.to_le_bytes());
        b
    }
    fn decode(bytes: &[u8]) -> Result<Self, IpcError> {
        if bytes.len() != SIZE || &bytes[..MAGIC.len()] != MAGIC {
            return Err(IpcError::Malformed);
        }
        let mut r = Cursor::new(&bytes[MAGIC.len()..]);
        let port = u16::from_le_bytes(take(&mut r)?);
        if port == 0 {
            return Err(IpcError::Malformed);
        }
        let bulk_port = u16::from_le_bytes(take(&mut r)?);
        let bulk_address =
            (bulk_port != 0).then(|| SocketAddr::from((Ipv4Addr::LOCALHOST, bulk_port)));
        let session = u64::from_le_bytes(take(&mut r)?);
        let universe = take(&mut r)?;
        let history = take(&mut r)?;
        let ruleset = take(&mut r)?;
        let content_policy = take(&mut r)?;
        let account = AccountId::from_bytes(take(&mut r)?);
        let device = DeviceId::from_bytes(take(&mut r)?);
        let token = SecretToken::from_bytes(take(&mut r)?);
        let limits = g::ResourceLimits {
            control_frame_bytes: u32::from_le_bytes(take(&mut r)?),
            chunk_bytes: u32::from_le_bytes(take(&mut r)?),
            inflight_bytes: u32::from_le_bytes(take(&mut r)?),
            inflight_items: u32::from_le_bytes(take(&mut r)?),
            decoded_bytes: u32::from_le_bytes(take(&mut r)?),
            collection_items: u32::from_le_bytes(take(&mut r)?),
            nesting_depth: u32::from_le_bytes(take(&mut r)?),
            transfer_bytes: u64::from_le_bytes(take(&mut r)?),
        };
        let config = SessionConfig {
            universe,
            history,
            runtime_session: session,
            ruleset,
            content_policy,
            limits,
        };
        config.validate()?;
        Ok(Self {
            config,
            principal: LocalPrincipal { account, device },
            token,
            address: SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
            bulk_address,
        })
    }
}
fn take<const N: usize>(r: &mut Cursor<&[u8]>) -> Result<[u8; N], IpcError> {
    let mut value = [0; N];
    r.read_exact(&mut value).map_err(|_| IpcError::Malformed)?;
    Ok(value)
}
/// Immutable publication under one exclusive cooperative owner-local lifecycle.
/// Callers must not remove, replace or reuse this name concurrently until close completes.
/// The exact-byte guard rejects an already replaced record; comparison/unlink are not atomic.
pub struct PublishedRendezvous<'a> {
    vault: &'a PrivateVault,
    name: String,
    record: DiscoveryRecord,
    expected: Zeroizing<Vec<u8>>,
    closed: bool,
}
impl<'a> PublishedRendezvous<'a> {
    pub fn publish(
        vault: &'a PrivateVault,
        name: &str,
        config: SessionConfig,
        principal: LocalPrincipal,
        address: SocketAddr,
    ) -> Result<Self, IpcError> {
        Self::publish_with_bulk(vault, name, config, principal, address, None)
    }
    pub fn publish_with_bulk(
        vault: &'a PrivateVault,
        name: &str,
        config: SessionConfig,
        principal: LocalPrincipal,
        address: SocketAddr,
        bulk_address: Option<SocketAddr>,
    ) -> Result<Self, IpcError> {
        config.validate()?;
        if bulk_address.is_some_and(|a| a.ip() != Ipv4Addr::LOCALHOST || a.port() == 0) {
            return Err(IpcError::Unauthorized);
        }
        if address.ip() != Ipv4Addr::LOCALHOST || address.port() == 0 {
            return Err(IpcError::Unauthorized);
        }
        let record = DiscoveryRecord {
            config,
            principal,
            address,
            bulk_address,
            token: SecretToken::generate()?,
        };
        let expected = record.encode();
        vault
            .create_private_blob(name, &expected)
            .map_err(|_| IpcError::PrivateStorage)?;
        Ok(Self {
            vault,
            name: name.into(),
            record,
            expected,
            closed: false,
        })
    }
    pub fn record(&self) -> &DiscoveryRecord {
        &self.record
    }
    pub fn close(mut self) -> Result<(), IpcError> {
        let result = self.remove_owned();
        self.closed = true;
        result
    }
    fn remove_owned(&self) -> Result<(), IpcError> {
        let observed = Zeroizing::new(
            self.vault
                .read_private_blob(&self.name)
                .map_err(|_| IpcError::PrivateStorage)?,
        );
        if *observed != *self.expected {
            return Err(IpcError::PrivateStorage);
        }
        self.vault
            .remove_private_blob(&self.name)
            .map_err(|_| IpcError::PrivateStorage)
    }
}
impl Drop for PublishedRendezvous<'_> {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.remove_owned();
        }
    }
}
