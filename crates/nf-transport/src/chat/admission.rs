//! Foreground resource admission only; current proof/store authority remains unchanged.
use crate::PeerError;
use libp2p::PeerId;
use nf_contract::identity::AccountId;
use nf_identity::model::{MembershipState, Roles};
use nf_store::chat::Author;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
const FRAME_AGE: Duration = Duration::from_secs(5);
const ACCOUNT_AGE: Duration = Duration::from_secs(30);
const GLOBAL_FRAMES: u16 = 128;
const CONNECTION_FRAMES: u16 = 64;
const ACCOUNT_CHALLENGES: u16 = 8;
const ACCOUNT_ENTRIES: usize = 128;
pub(super) struct Window {
    started: Instant,
    used: u16,
    age: Duration,
    limit: u16,
}
impl Window {
    fn new(now: Instant, age: Duration, limit: u16) -> Self {
        Self {
            started: now,
            used: 0,
            age,
            limit,
        }
    }
    pub(super) fn connection(now: Instant) -> Self {
        Self::new(now, FRAME_AGE, CONNECTION_FRAMES)
    }
    fn expired(&self, now: Instant) -> bool {
        now.checked_duration_since(self.started)
            .is_some_and(|elapsed| elapsed >= self.age)
    }
    fn blocked(&self, now: Instant) -> bool {
        now.checked_duration_since(self.started).is_none()
            || (!self.expired(now) && self.used >= self.limit)
    }
    fn take(&mut self, now: Instant) -> Result<(), PeerError> {
        if now.checked_duration_since(self.started).is_none() {
            return Err(PeerError::Limit);
        }
        if self.expired(now) {
            self.started = now;
            self.used = 0;
        }
        if self.used >= self.limit {
            return Err(PeerError::Limit);
        }
        self.used += 1;
        Ok(())
    }
}
pub(super) struct Admission {
    global: Window,
    accounts: BTreeMap<AccountId, Window>,
}
impl Admission {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            global: Window::new(now, FRAME_AGE, GLOBAL_FRAMES),
            accounts: BTreeMap::new(),
        }
    }
    pub(super) fn frame(&mut self, connection: &mut Window, now: Instant) -> Result<(), PeerError> {
        // Every reached packet spends global capacity, including connection refusals.
        self.global.take(now)?;
        connection.take(now)
    }
    pub(super) fn blocked(&self, account: AccountId, now: Instant) -> bool {
        self.accounts
            .get(&account)
            .is_some_and(|window| window.blocked(now))
    }
    pub(super) fn challenge(&mut self, account: AccountId, now: Instant) -> Result<(), PeerError> {
        self.accounts.retain(|_, window| !window.expired(now));
        if !self.accounts.contains_key(&account) {
            // Active entries are never evicted: reconnect/churn cannot reset their budget.
            if self.accounts.len() >= ACCOUNT_ENTRIES {
                return Err(PeerError::Limit);
            }
            self.accounts
                .insert(account, Window::new(now, ACCOUNT_AGE, ACCOUNT_CHALLENGES));
        }
        self.accounts
            .get_mut(&account)
            .ok_or(PeerError::Limit)?
            .take(now)
    }
}
/// Noise/current membership selects a chargeable account, not message authority.
pub(super) fn peer_account(
    current: &MembershipState,
    peer: PeerId,
) -> Result<AccountId, PeerError> {
    let bytes = peer.to_bytes();
    let mut selected = None;
    let mut active = false;
    for device in current
        .devices
        .values()
        .filter(|device| device.peer == bytes)
    {
        // Revocation cannot erase another account's possession of the same physical Noise key.
        if selected.is_some_and(|account| account != device.account) {
            return Err(PeerError::Unauthorized);
        }
        selected = Some(device.account);
        active |= !device.revoked;
    }
    let account = selected.ok_or(PeerError::Unauthorized)?;
    if !active
        || !current
            .accounts
            .get(&account)
            .is_some_and(|entry| entry.roles.contains(Roles::PLAYER))
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(account)
}
pub(super) fn actor_bound(
    current: &MembershipState,
    peer: PeerId,
    account: AccountId,
    actor: Author,
) -> Result<(), PeerError> {
    let device = current
        .devices
        .get(&actor.device)
        .ok_or(PeerError::Unauthorized)?;
    if actor.account != account
        || device.account != account
        || device.revoked
        || device.peer != peer.to_bytes()
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monotonic_window_keeps_exact_boundary_and_does_not_reset_on_rollback() {
        let start = Instant::now();
        let mut window = Window::new(start, Duration::from_secs(30), 2);
        assert_eq!(window.take(start), Ok(()));
        assert_eq!(window.take(start + Duration::from_secs(1)), Ok(()));
        assert_eq!(
            window.take(start + Duration::from_millis(29999)),
            Err(PeerError::Limit)
        );
        assert_eq!(
            window.take(start - Duration::from_millis(1)),
            Err(PeerError::Limit)
        );
        assert_eq!(window.take(start + Duration::from_secs(30)), Ok(()));
        assert_eq!(window.take(start + Duration::from_secs(30)), Ok(()));
        assert_eq!(
            window.take(start + Duration::from_secs(30)),
            Err(PeerError::Limit)
        );
    }
}

#[cfg(test)]
mod retained_account_cap_controls {
    use super::*;
    #[test]
    fn one_hundred_twenty_eight_active_accounts_are_retained_until_the_exact_window_boundary() {
        let now = Instant::now();
        let mut admission = Admission::new(now);
        for id in 1..=128u8 {
            assert_eq!(
                admission.challenge(AccountId::from_bytes([id; 16]), now),
                Ok(())
            );
        }
        let first = AccountId::from_bytes([1; 16]);
        let new = AccountId::from_bytes([129; 16]);
        assert_eq!(admission.challenge(new, now), Err(PeerError::Limit));
        for _ in 1..8 {
            assert_eq!(admission.challenge(first, now), Ok(()));
        }
        assert_eq!(admission.challenge(first, now), Err(PeerError::Limit));
        assert_eq!(
            admission.challenge(new, now + Duration::from_millis(29999)),
            Err(PeerError::Limit)
        );
        assert_eq!(
            admission.challenge(new, now + Duration::from_secs(30)),
            Ok(())
        );
    }
}
