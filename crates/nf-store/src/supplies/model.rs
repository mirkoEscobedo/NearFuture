use nf_identity::model::Scope;
use nf_kernel::supplies::{BalanceQuery, Burn, Issuance, Reserve, StatusQuery};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownSuppliesFrontiers {
    pub scope: Scope,
    pub revision: u64,
    pub membership_revision: u64,
}
#[derive(Clone, Copy)]
pub enum ChallengeRequest<'a> {
    Issue(&'a Issuance),
    Burn(&'a Burn),
    Reserve(&'a Reserve),
    Status(&'a StatusQuery),
    Balance(&'a BalanceQuery),
}
