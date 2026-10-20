use super::{
    ChatRatePolicy, ChatStoreError, ReceiverClock, Result, rate_sql::Snapshot,
    receiver_rate::Balance,
};
use nf_contract::identity::AccountId;
use nf_identity::model::MembershipState;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

pub(super) struct Runtime {
    policy: ChatRatePolicy,
    generation: u64,
    clock: Arc<dyn ReceiverClock>,
    baselines: BTreeMap<AccountId, Duration>,
}
pub(super) struct Debit {
    pub account: AccountId,
    pub before: Balance,
    pub after: Balance,
    cut: Duration,
}
pub(super) struct Reload {
    pub policy: ChatRatePolicy,
    pub generation: u64,
    pub rows: BTreeMap<AccountId, (Balance, Balance)>,
    cut: Duration,
}
impl Runtime {
    // Called only after verified exclusive ownership and complete durable state validation.
    pub fn restore(snapshot: &Snapshot, clock: Arc<dyn ReceiverClock>) -> Self {
        let cut = clock.now();
        Self {
            policy: snapshot.policy,
            generation: snapshot.generation,
            clock,
            baselines: snapshot
                .accounts
                .keys()
                .map(|account| (*account, cut))
                .collect(),
        }
    }
    pub fn policy(&self) -> ChatRatePolicy {
        self.policy
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn prepare(&self, account: AccountId, before: Balance) -> Result<Debit> {
        let baseline = self
            .baselines
            .get(&account)
            .ok_or(ChatStoreError::Corrupt)?;
        let cut = self.clock.now();
        let elapsed = cut.checked_sub(*baseline).ok_or(ChatStoreError::Corrupt)?;
        Ok(Debit {
            account,
            before,
            after: before.take(self.policy, elapsed)?,
            cut,
        })
    }
    // Invoked only after the transaction COMMIT succeeds; SQL error/refusal/retry never calls it.
    pub fn commit(&mut self, debit: Debit) -> Result<()> {
        let baseline = self
            .baselines
            .get_mut(&debit.account)
            .ok_or(ChatStoreError::Corrupt)?;
        *baseline = debit.cut;
        Ok(())
    }
    pub fn new_members(&self, next: &MembershipState) -> Result<(Vec<AccountId>, Duration)> {
        if next.accounts.len() > 128
            || self
                .baselines
                .keys()
                .any(|id| !next.accounts.contains_key(id))
        {
            return Err(ChatStoreError::Corrupt);
        }
        let cut = self.clock.now();
        if self.baselines.values().any(|baseline| cut < *baseline) {
            return Err(ChatStoreError::Corrupt);
        }
        let added = next
            .accounts
            .keys()
            .filter(|id| !self.baselines.contains_key(*id))
            .copied()
            .collect();
        Ok((added, cut))
    }
    pub fn commit_members(&mut self, added: Vec<AccountId>, cut: Duration) -> Result<()> {
        if added.iter().any(|id| self.baselines.contains_key(id))
            || self.baselines.len() + added.len() > 128
        {
            return Err(ChatStoreError::Corrupt);
        }
        for account in added {
            self.baselines.insert(account, cut);
        }
        Ok(())
    }
    pub fn prepare_reload(
        &self,
        snapshot: Snapshot,
        next: ChatRatePolicy,
    ) -> Result<Option<Reload>> {
        if snapshot.policy != self.policy
            || snapshot.generation != self.generation
            || snapshot.accounts.len() != self.baselines.len()
            || snapshot
                .accounts
                .keys()
                .any(|id| !self.baselines.contains_key(id))
        {
            return Err(ChatStoreError::Corrupt);
        }
        // An unchanged policy does not even sample or reset the volatile clock baselines.
        if next == self.policy {
            return Ok(None);
        }
        let cut = self.clock.now();
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(ChatStoreError::Limit)?;
        let mut rows = BTreeMap::new();
        for (account, before) in snapshot.accounts {
            let baseline = self
                .baselines
                .get(&account)
                .ok_or(ChatStoreError::Corrupt)?;
            let elapsed = cut.checked_sub(*baseline).ok_or(ChatStoreError::Corrupt)?;
            let after = before
                .refill(self.policy, elapsed)?
                .change_policy(self.policy, next)?;
            rows.insert(account, (before, after));
        }
        Ok(Some(Reload {
            policy: next,
            generation,
            rows,
            cut,
        }))
    }
    pub fn commit_reload(&mut self, reload: Reload) {
        self.policy = reload.policy;
        self.generation = reload.generation;
        for baseline in self.baselines.values_mut() {
            *baseline = reload.cut;
        }
    }
}
