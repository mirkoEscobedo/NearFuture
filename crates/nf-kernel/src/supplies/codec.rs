use super::*;
use alloc::vec::Vec;
use nf_contract::canonical::binding::RequestBinding;
use sha2::{Digest, Sha256};
fn origin(bytes: &mut Vec<u8>, value: Origin) {
    bytes.extend_from_slice(
        &(match value.trust {
            TrustClass::Sandbox => 1_u32,
            TrustClass::CooperativeAudited => 2,
            TrustClass::Canonical => 3,
        })
        .to_le_bytes(),
    );
    bytes.extend_from_slice(&value.lineage);
}
fn reason(bytes: &mut Vec<u8>, value: IssuanceReason) {
    bytes.extend_from_slice(
        &(match value {
            IssuanceReason::AuthorityGrant => 1_u32,
        })
        .to_le_bytes(),
    );
}
pub fn policy_bytes(policy: &SuppliesPolicy) -> Result<Vec<u8>, SuppliesRejection> {
    policy.validate()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"NF-SUPPLIES-POLICY-2\0");
    bytes.extend_from_slice(policy.universe.as_bytes());
    bytes.extend_from_slice(policy.history.as_bytes());
    bytes.extend_from_slice(&policy.ruleset);
    let count = u32::try_from(policy.issuers.len()).map_err(|_| SuppliesRejection::Limit)?;
    bytes.extend_from_slice(&count.to_le_bytes());
    for rule in &policy.issuers {
        bytes.extend_from_slice(rule.issuer.as_bytes());
        bytes.extend_from_slice(&rule.content);
        origin(&mut bytes, rule.origin);
        reason(&mut bytes, rule.reason);
        bytes.extend_from_slice(&rule.maximum.to_le_bytes());
    }
    let count = u32::try_from(policy.burners.len()).map_err(|_| SuppliesRejection::Limit)?;
    bytes.extend_from_slice(&count.to_le_bytes());
    for rule in &policy.burners {
        bytes.extend_from_slice(rule.issuer.as_bytes());
        bytes.extend_from_slice(&rule.content);
        origin(&mut bytes, rule.origin);
        bytes.extend_from_slice(
            &(match rule.reason {
                BurnReason::AuthorityDestruction => 1_u32,
            })
            .to_le_bytes(),
        );
        bytes.extend_from_slice(&rule.maximum.to_le_bytes());
    }
    Ok(bytes)
}
pub fn policy_digest(policy: &SuppliesPolicy) -> Result<[u8; 32], SuppliesRejection> {
    Ok(Sha256::digest(policy_bytes(policy)?).into())
}
pub fn issuance_bytes(value: &Issuance) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"NF-SUPPLIES-ISSUANCE-1\0");
    for id in [
        value.request.as_bytes(),
        value.issuance.as_bytes(),
        value.actor.as_bytes(),
        value.device.as_bytes(),
        value.beneficiary.as_bytes(),
        value.universe.as_bytes(),
        value.history.as_bytes(),
    ] {
        bytes.extend_from_slice(id);
    }
    bytes.extend_from_slice(&value.policy);
    bytes.extend_from_slice(&value.content);
    origin(&mut bytes, value.origin);
    reason(&mut bytes, value.reason);
    bytes.extend_from_slice(&value.amount.to_le_bytes());
    bytes
}
pub fn issuance_digest(value: &Issuance) -> [u8; 32] {
    Sha256::digest(issuance_bytes(value)).into()
}
pub fn issuance_binding(value: &Issuance) -> RequestBinding {
    RequestBinding {
        request_id: value.request,
        account_id: value.actor,
        device_id: value.device,
        universe_id: value.universe,
        history_id: value.history,
        operation_kind: ISSUE_OPERATION,
        payload_digest: issuance_digest(value),
    }
}
pub fn balance_digest(value: &BalanceQuery) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"NF-SUPPLIES-BALANCE-1\0");
    for id in [
        value.actor.as_bytes(),
        value.device.as_bytes(),
        value.owner.as_bytes(),
        value.universe.as_bytes(),
        value.history.as_bytes(),
    ] {
        bytes.extend_from_slice(id);
    }
    bytes.extend_from_slice(&value.content);
    origin(&mut bytes, value.origin);
    Sha256::digest(bytes).into()
}

pub fn status_digest(value: &StatusQuery) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"NF-SUPPLIES-STATUS-1\0");
    for id in [
        value.actor.as_bytes(),
        value.device.as_bytes(),
        value.owner.as_bytes(),
        value.universe.as_bytes(),
        value.history.as_bytes(),
        value.request.as_bytes(),
    ] {
        hash.update(id);
    }
    hash.finalize().into()
}
