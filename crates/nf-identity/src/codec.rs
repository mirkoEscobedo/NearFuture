use crate::model::*;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};

const PREFIX: &[u8] = b"NF-CANON-1\0";
pub const MAX_STATE_BYTES: usize = 262_144;
pub const MAX_ACCOUNTS: usize = 128;
pub const MAX_DEVICES: usize = 512;
pub const MAX_CONSUMED: usize = 1024;

pub(crate) struct Writer(pub Vec<u8>);
impl Writer {
    pub(crate) fn new(kind: u16) -> Self {
        let mut value = Self(PREFIX.to_vec());
        value.0.extend_from_slice(&8u16.to_le_bytes());
        value.0.extend_from_slice(&kind.to_le_bytes());
        value.0.extend_from_slice(&1u16.to_le_bytes());
        value
    }
    pub(crate) fn raw(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
    pub(crate) fn u64(&mut self, value: u64) {
        self.raw(&value.to_le_bytes());
    }
    pub(crate) fn count(&mut self, value: usize) {
        self.raw(&(value as u32).to_le_bytes());
    }
    pub(crate) fn scope(&mut self, value: Scope) {
        self.raw(value.universe.as_bytes());
        self.raw(value.history.as_bytes());
    }
    pub(crate) fn peer(&mut self, value: &[u8]) -> Result<(), IdentityError> {
        if value.is_empty() || value.len() > 128 {
            return Err(IdentityError::Limit);
        }
        self.count(value.len());
        self.raw(value);
        Ok(())
    }
    pub(crate) fn public(&mut self, identity: &PublicIdentity) -> Result<(), IdentityError> {
        self.raw(identity.account.as_bytes());
        self.raw(&identity.account_key);
        self.raw(identity.device.as_bytes());
        self.raw(&identity.device_key);
        self.peer(&identity.peer)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], kind: u16) -> Result<Self, IdentityError> {
        if bytes.len() > MAX_STATE_BYTES {
            return Err(IdentityError::Limit);
        }
        let mut value = Self { bytes, offset: 0 };
        if value.take::<11>()? != PREFIX
            || value.take::<2>()? != 8u16.to_le_bytes()
            || value.take::<2>()? != kind.to_le_bytes()
            || value.take::<2>()? != 1u16.to_le_bytes()
        {
            return Err(IdentityError::Malformed);
        }
        Ok(value)
    }
    fn take<const N: usize>(&mut self) -> Result<[u8; N], IdentityError> {
        let end = self.offset.checked_add(N).ok_or(IdentityError::Limit)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(IdentityError::Malformed)?;
        self.offset = end;
        value.try_into().map_err(|_| IdentityError::Malformed)
    }
    fn u64(&mut self) -> Result<u64, IdentityError> {
        Ok(u64::from_le_bytes(self.take()?))
    }
    fn count(&mut self, max: usize) -> Result<usize, IdentityError> {
        let count = u32::from_le_bytes(self.take()?) as usize;
        if count > max {
            return Err(IdentityError::Limit);
        }
        Ok(count)
    }
    fn flag(&mut self) -> Result<bool, IdentityError> {
        match self.take::<1>()?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(IdentityError::Malformed),
        }
    }
    fn scope(&mut self) -> Result<Scope, IdentityError> {
        Ok(Scope {
            universe: UniverseId::from_bytes(self.take()?),
            history: HistoryId::from_bytes(self.take()?),
        })
    }
    fn peer(&mut self) -> Result<Vec<u8>, IdentityError> {
        let count = self.count(128)?;
        if count == 0 {
            return Err(IdentityError::Malformed);
        }
        let end = self.offset.checked_add(count).ok_or(IdentityError::Limit)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(IdentityError::Malformed)?
            .to_vec();
        self.offset = end;
        Ok(value)
    }
    fn public(&mut self) -> Result<PublicIdentity, IdentityError> {
        Ok(PublicIdentity {
            account: AccountId::from_bytes(self.take()?),
            account_key: self.take()?,
            device: DeviceId::from_bytes(self.take()?),
            device_key: self.take()?,
            peer: self.peer()?,
        })
    }
    fn finish(self) -> Result<(), IdentityError> {
        if self.offset != self.bytes.len() {
            return Err(IdentityError::Malformed);
        }
        Ok(())
    }
}
pub fn encode_invitation(value: &Invitation) -> Result<Vec<u8>, IdentityError> {
    let mut out = Writer::new(1);
    out.scope(value.scope);
    out.raw(&value.id);
    out.raw(value.issuer.as_bytes());
    out.public(&value.recipient)?;
    out.raw(&[value.roles.bits()]);
    out.u64(value.expires_at);
    out.u64(value.issued_revision);
    out.raw(&[u8::from(value.reusable)]);
    Ok(out.0)
}
pub fn decode_invitation(bytes: &[u8]) -> Result<Invitation, IdentityError> {
    let mut input = Reader::new(bytes, 1)?;
    let value = Invitation {
        scope: input.scope()?,
        id: input.take()?,
        issuer: AccountId::from_bytes(input.take()?),
        recipient: input.public()?,
        roles: Roles::from_bits(input.take::<1>()?[0])?,
        expires_at: input.u64()?,
        issued_revision: input.u64()?,
        reusable: input.flag()?,
    };
    input.finish()?;
    Ok(value)
}
pub fn validate_state(state: &MembershipState) -> Result<(), IdentityError> {
    if state.accounts.is_empty()
        || state.accounts.len() > MAX_ACCOUNTS
        || state.devices.len() > MAX_DEVICES
        || state.consumed.len() > MAX_CONSUMED
    {
        return Err(IdentityError::Limit);
    }
    if !state.accounts.contains_key(&state.owner) {
        return Err(IdentityError::Malformed);
    }
    for account in state.accounts.values() {
        Roles::from_bits(account.roles.bits())?;
    }
    for device in state.devices.values() {
        if !state.accounts.contains_key(&device.account)
            || device.peer.is_empty()
            || device.peer.len() > 128
        {
            return Err(IdentityError::Malformed);
        }
    }
    Ok(())
}
pub fn encode_state(value: &MembershipState) -> Result<Vec<u8>, IdentityError> {
    validate_state(value)?;
    let mut out = Writer::new(6);
    out.scope(value.scope);
    out.u64(value.revision);
    out.raw(value.owner.as_bytes());
    out.count(value.accounts.len());
    for (id, account) in &value.accounts {
        out.raw(id.as_bytes());
        out.raw(&account.key);
        out.raw(&[account.roles.bits()]);
    }
    out.count(value.devices.len());
    for (id, device) in &value.devices {
        out.raw(id.as_bytes());
        out.raw(device.account.as_bytes());
        out.raw(&device.key);
        out.peer(&device.peer)?;
        out.raw(&[u8::from(device.revoked)]);
    }
    out.count(value.consumed.len());
    for id in &value.consumed {
        out.raw(id);
    }
    if out.0.len() > MAX_STATE_BYTES {
        return Err(IdentityError::Limit);
    }
    Ok(out.0)
}
pub fn decode_state(bytes: &[u8]) -> Result<MembershipState, IdentityError> {
    let mut input = Reader::new(bytes, 6)?;
    let mut state = MembershipState {
        scope: input.scope()?,
        revision: input.u64()?,
        owner: AccountId::from_bytes(input.take()?),
        accounts: Default::default(),
        devices: Default::default(),
        consumed: Default::default(),
    };
    let mut previous = None;
    for _ in 0..input.count(MAX_ACCOUNTS)? {
        let id = AccountId::from_bytes(input.take()?);
        if previous.is_some_and(|old| old >= id) {
            return Err(IdentityError::Malformed);
        }
        previous = Some(id);
        state.accounts.insert(
            id,
            Account {
                key: input.take()?,
                roles: Roles::from_bits(input.take::<1>()?[0])?,
            },
        );
    }
    let mut previous = None;
    for _ in 0..input.count(MAX_DEVICES)? {
        let id = DeviceId::from_bytes(input.take()?);
        if previous.is_some_and(|old| old >= id) {
            return Err(IdentityError::Malformed);
        }
        previous = Some(id);
        state.devices.insert(
            id,
            Device {
                account: AccountId::from_bytes(input.take()?),
                key: input.take()?,
                peer: input.peer()?,
                revoked: input.flag()?,
            },
        );
    }
    let mut previous = None;
    for _ in 0..input.count(MAX_CONSUMED)? {
        let id = input.take()?;
        if previous.is_some_and(|old| old >= id) {
            return Err(IdentityError::Malformed);
        }
        previous = Some(id);
        state.consumed.insert(id);
    }
    input.finish()?;
    validate_state(&state)?;
    Ok(state)
}
