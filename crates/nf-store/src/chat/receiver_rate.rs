//! Trusted LOCAL receiver admission policy; signed ChatPolicy remains exactly 58 bytes.
use super::{ChatStoreError, Result};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChatRatePolicy {
    capacity: u16,
    refill_ms: u32,
}
impl Default for ChatRatePolicy {
    fn default() -> Self {
        Self {
            capacity: 10,
            refill_ms: 2_000,
        }
    }
}
impl ChatRatePolicy {
    pub fn new(capacity: u16, refill_ms: u32) -> Result<Self> {
        if !(1..=1_024).contains(&capacity) || !(1..=86_400_000).contains(&refill_ms) {
            return Err(ChatStoreError::Policy);
        }
        Ok(Self {
            capacity,
            refill_ms,
        })
    }
    pub const fn capacity(self) -> u16 {
        self.capacity
    }
    pub const fn refill_ms(self) -> u32 {
        self.refill_ms
    }
    pub(super) fn interval_ns(self) -> u64 {
        u64::from(self.refill_ms) * 1_000_000
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Balance {
    pub tokens: u16,
    pub fractional_ns: u64,
}
impl Balance {
    pub const fn fresh(policy: ChatRatePolicy) -> Self {
        Self {
            tokens: policy.capacity,
            fractional_ns: 0,
        }
    }
    pub const fn migrated_existing() -> Self {
        Self {
            tokens: 0,
            fractional_ns: 0,
        }
    }
    pub fn validate(self, policy: ChatRatePolicy) -> Result<Self> {
        if self.tokens > policy.capacity
            || self.fractional_ns >= policy.interval_ns()
            || (self.tokens == policy.capacity && self.fractional_ns != 0)
        {
            return Err(ChatStoreError::Corrupt);
        }
        Ok(self)
    }
    pub fn refill(self, policy: ChatRatePolicy, elapsed: Duration) -> Result<Self> {
        self.validate(policy)?;
        let interval = policy.interval_ns();
        let capacity = u64::from(policy.capacity) * interval;
        let retained = u64::from(self.tokens) * interval + self.fractional_ns;
        let added = elapsed.as_nanos().min(u128::from(capacity - retained)) as u64;
        let credit = retained + added;
        Ok(Self {
            tokens: (credit / interval) as u16,
            fractional_ns: credit % interval,
        })
    }
    pub fn take(self, policy: ChatRatePolicy, elapsed: Duration) -> Result<Self> {
        let current = self.refill(policy, elapsed)?;
        if current.tokens == 0 {
            let retry_after_ms =
                (policy.interval_ns() - current.fractional_ns).div_ceil(1_000_000) as u32;
            return Err(ChatStoreError::RateLimited { retry_after_ms });
        }
        Ok(Self {
            tokens: current.tokens - 1,
            ..current
        })
    }
    pub fn change_policy(self, old: ChatRatePolicy, new: ChatRatePolicy) -> Result<Self> {
        self.validate(old)?;
        let tokens = self.tokens.min(new.capacity);
        let fractional_ns = if tokens == new.capacity {
            0
        } else {
            (u128::from(self.fractional_ns) * u128::from(new.interval_ns())
                / u128::from(old.interval_ns())) as u64
        };
        Self {
            tokens,
            fractional_ns,
        }
        .validate(new)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_operator_validation_are_distinct() {
        let defaults = ChatRatePolicy::default();
        assert_eq!((defaults.capacity(), defaults.refill_ms()), (10, 2_000));
        assert_eq!(ChatRatePolicy::new(0, 2_000), Err(ChatStoreError::Policy));
        assert_eq!(
            ChatRatePolicy::new(1_025, 2_000),
            Err(ChatStoreError::Policy)
        );
        assert_eq!(ChatRatePolicy::new(10, 0), Err(ChatStoreError::Policy));
        assert_eq!(
            ChatRatePolicy::new(10, 86_400_001),
            Err(ChatStoreError::Policy)
        );
        assert_eq!(ChatRatePolicy::new(2, 2_000).unwrap().capacity(), 2);
    }
    #[test]
    fn default_burst_boundary_fraction_and_large_elapsed_are_exact() {
        let policy = ChatRatePolicy::default();
        let mut balance = Balance::fresh(policy);
        for _ in 0..10 {
            balance = balance.take(policy, Duration::ZERO).unwrap();
        }
        assert_eq!(balance, Balance::migrated_existing());
        assert_eq!(
            balance.take(policy, Duration::ZERO),
            Err(ChatStoreError::RateLimited {
                retry_after_ms: 2_000
            })
        );
        assert_eq!(
            balance.take(policy, Duration::from_nanos(1_999_999_999)),
            Err(ChatStoreError::RateLimited { retry_after_ms: 1 })
        );
        assert_eq!(
            balance
                .take(policy, Duration::from_nanos(2_000_000_000))
                .unwrap(),
            Balance::migrated_existing()
        );
        assert_eq!(
            balance
                .take(policy, Duration::from_nanos(2_750_000_000))
                .unwrap(),
            Balance {
                tokens: 0,
                fractional_ns: 750_000_000
            }
        );
        assert_eq!(
            balance.refill(policy, Duration::MAX).unwrap(),
            Balance::fresh(policy)
        );
    }
    #[test]
    fn changed_capacity_does_not_fill_and_fraction_is_conserved() {
        let old = ChatRatePolicy::default();
        let next = ChatRatePolicy::new(20, 1_000).unwrap();
        let before = Balance {
            tokens: 3,
            fractional_ns: 1_000_000_000,
        };
        assert_eq!(
            before.change_policy(old, next).unwrap(),
            Balance {
                tokens: 3,
                fractional_ns: 500_000_000
            }
        );
        assert_eq!(
            Balance::fresh(old).change_policy(old, next).unwrap(),
            Balance {
                tokens: 10,
                fractional_ns: 0
            }
        );
        let smaller = ChatRatePolicy::new(2, 2_000).unwrap();
        assert_eq!(
            before.change_policy(old, smaller).unwrap(),
            Balance::fresh(smaller)
        );
        assert_eq!(
            Balance::migrated_existing()
                .change_policy(old, next)
                .unwrap(),
            Balance::migrated_existing()
        );
    }
    #[test]
    fn inconsistent_persisted_balances_are_never_treated_as_fresh() {
        let policy = ChatRatePolicy::default();
        for corrupt in [
            Balance {
                tokens: 11,
                fractional_ns: 0,
            },
            Balance {
                tokens: 0,
                fractional_ns: 2_000_000_000,
            },
            Balance {
                tokens: 10,
                fractional_ns: 1,
            },
        ] {
            assert_eq!(corrupt.validate(policy), Err(ChatStoreError::Corrupt));
            assert_eq!(
                corrupt.refill(policy, Duration::MAX),
                Err(ChatStoreError::Corrupt)
            );
        }
    }
}
