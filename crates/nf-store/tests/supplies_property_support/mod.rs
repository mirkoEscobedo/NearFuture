mod driver;
mod reference;
use crate::supplies_support::Fixture;
pub use driver::Driver;
use nf_contract::identity::{AccountId, DeviceId, OperationId, RequestId};
use nf_kernel::supplies::*;

pub type Key = (AccountId, [u8; 32], Origin);
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Issue,
    Reserve,
    Burn,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Call {
    Issue(Issuance),
    Reserve(Reserve),
    Burn(Burn),
}
impl Call {
    pub fn key(self) -> Key {
        match self {
            Self::Issue(v) => (v.beneficiary, v.content, v.origin),
            Self::Reserve(v) => (v.owner, v.content, v.origin),
            Self::Burn(v) => (v.owner, v.content, v.origin),
        }
    }
    pub fn request(self) -> RequestId {
        match self {
            Self::Issue(v) => v.request,
            Self::Reserve(v) => v.request,
            Self::Burn(v) => v.request,
        }
    }
    pub fn operation(self) -> OperationId {
        match self {
            Self::Issue(v) => v.issuance,
            Self::Reserve(v) => v.reservation,
            Self::Burn(v) => v.burn,
        }
    }
    pub fn amount(self) -> u64 {
        match self {
            Self::Issue(v) => v.amount,
            Self::Reserve(v) => v.amount,
            Self::Burn(v) => v.amount,
        }
    }
    pub fn alias(mut self, request: RequestId) -> Self {
        match &mut self {
            Self::Issue(v) => v.request = request,
            Self::Reserve(v) => v.request = request,
            Self::Burn(v) => v.request = request,
        }
        self
    }
    pub fn changed_amount(mut self, amount: u64) -> Self {
        match &mut self {
            Self::Issue(v) => v.amount = amount,
            Self::Reserve(v) => v.amount = amount,
            Self::Burn(v) => v.amount = amount,
        }
        self
    }
    pub fn changed_origin(mut self, origin: Origin) -> Self {
        match &mut self {
            Self::Issue(v) => v.origin = origin,
            Self::Reserve(v) => v.origin = origin,
            Self::Burn(v) => v.origin = origin,
        }
        self
    }
    // Independent semantic equality includes every typed field and the variant, excluding only RequestId.
    pub fn economic(self) -> Self {
        self.alias(RequestId::from_bytes([0; 16]))
    }
    pub fn outcome(self, revision: u64) -> RequestOutcome {
        match self {
            Self::Issue(v) => RequestOutcome::Issued(IssuanceOutcome {
                issuance: v.issuance,
                revision,
            }),
            Self::Reserve(v) => RequestOutcome::Reserved(ReserveOutcome {
                reservation: v.reservation,
                revision,
            }),
            Self::Burn(v) => RequestOutcome::Burned(BurnOutcome {
                burn: v.burn,
                revision,
            }),
        }
    }
}
pub fn op(n: u32) -> OperationId {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&n.to_le_bytes());
    bytes[15] = 0xa1;
    OperationId::from_bytes(bytes)
}
pub fn req(n: u32) -> RequestId {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&n.to_le_bytes());
    bytes[15] = 0xb2;
    RequestId::from_bytes(bytes)
}
pub fn origins() -> [Origin; 4] {
    [
        Origin {
            trust: TrustClass::Sandbox,
            lineage: [4; 32],
        },
        Origin {
            trust: TrustClass::CooperativeAudited,
            lineage: [4; 32],
        },
        Origin {
            trust: TrustClass::Canonical,
            lineage: [4; 32],
        },
        Origin {
            trust: TrustClass::Canonical,
            lineage: [5; 32],
        },
    ]
}
pub fn allow(fixture: &mut Fixture, origins: &[Origin], maximum: u64) {
    fixture.policy.issuers.clear();
    fixture.policy.burners.clear();
    for &origin in origins {
        fixture.policy.issuers.push(IssuerRule {
            issuer: fixture.issuer.account,
            content: SUPPLIES_CONTENT,
            origin,
            reason: IssuanceReason::AuthorityGrant,
            maximum,
        });
        fixture.policy.burners.push(BurnRule {
            issuer: fixture.issuer.account,
            content: SUPPLIES_CONTENT,
            origin,
            reason: BurnReason::AuthorityDestruction,
            maximum,
        });
    }
}
pub fn own_device(fixture: &Fixture, owner: AccountId) -> DeviceId {
    if owner == fixture.issuer.account {
        fixture.issuer.device
    } else {
        assert_eq!(owner, fixture.beneficiary.account);
        fixture.beneficiary.device
    }
}
pub fn call(
    f: &Fixture,
    kind: Kind,
    key: Key,
    operation: OperationId,
    request: RequestId,
    amount: u64,
) -> Call {
    // Product digest is used solely to authenticate the real call, never for the expected arithmetic or identity.
    let policy = f.issuance().policy;
    let scope = f.query();
    match kind {
        Kind::Issue => Call::Issue(Issuance {
            request,
            issuance: operation,
            actor: f.issuer.account,
            device: f.issuer.device,
            beneficiary: key.0,
            universe: scope.universe,
            history: scope.history,
            policy,
            content: key.1,
            origin: key.2,
            reason: IssuanceReason::AuthorityGrant,
            amount,
        }),
        Kind::Reserve => Call::Reserve(Reserve {
            request,
            reservation: operation,
            actor: key.0,
            device: own_device(f, key.0),
            owner: key.0,
            universe: scope.universe,
            history: scope.history,
            policy,
            content: key.1,
            origin: key.2,
            amount,
        }),
        Kind::Burn => Call::Burn(Burn {
            request,
            burn: operation,
            actor: f.issuer.account,
            device: f.issuer.device,
            owner: key.0,
            universe: scope.universe,
            history: scope.history,
            policy,
            content: key.1,
            origin: key.2,
            reason: BurnReason::AuthorityDestruction,
            amount,
        }),
    }
}
