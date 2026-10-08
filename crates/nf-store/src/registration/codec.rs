use super::model::{
    RegisterBranch, RegistrationError, RegistrationMode, RegistrationPolicy, Result,
};
use nf_contract::identity::{
    AccountId, BranchId, CampaignId, DeviceId, HistoryId, RequestId, UniverseId,
};
use nf_identity::model::Scope;

const REQUEST_DOMAIN: &[u8; 21] = b"NF-BRANCH-REGISTER-1\0";
pub(super) const REQUEST_BYTES: usize = 165;

pub(super) fn policy_bytes(policy: &RegistrationPolicy) -> Result<Vec<u8>> {
    policy.validate()?;
    let mut bytes = Vec::with_capacity(4096);
    bytes.extend_from_slice(b"NF-BRANCH-POLICY-1\0");
    bytes.extend_from_slice(policy.scope.universe.as_bytes());
    bytes.extend_from_slice(policy.scope.history.as_bytes());
    bytes.push(match policy.mode {
        RegistrationMode::HeadlessRegistrationOnly => 1,
    });
    bytes.extend_from_slice(&(policy.allowed.len() as u16).to_be_bytes());
    for binding in &policy.allowed {
        bytes.extend_from_slice(binding.campaign.as_bytes());
        bytes.extend_from_slice(binding.branch.as_bytes());
        bytes.extend_from_slice(binding.account.as_bytes());
    }
    Ok(bytes)
}

pub(super) fn request_bytes(request: &RegisterBranch) -> [u8; REQUEST_BYTES] {
    let mut bytes = [0; REQUEST_BYTES];
    bytes[..21].copy_from_slice(REQUEST_DOMAIN);
    let ids = [
        request.request.as_bytes(),
        request.scope.universe.as_bytes(),
        request.scope.history.as_bytes(),
        request.campaign.as_bytes(),
        request.branch.as_bytes(),
        request.account.as_bytes(),
        request.device.as_bytes(),
    ];
    for (index, id) in ids.into_iter().enumerate() {
        let start = 21 + index * 16;
        bytes[start..start + 16].copy_from_slice(id);
    }
    bytes[133..].copy_from_slice(&request.policy_digest);
    bytes
}

pub(super) fn request_digest(request: &RegisterBranch) -> [u8; 32] {
    crate::schema::hash(&request_bytes(request))
}

pub(super) fn fixed<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes.try_into().map_err(|_| RegistrationError::Corrupt)
}

pub(super) fn decode_request(bytes: &[u8]) -> Result<RegisterBranch> {
    if bytes.len() != REQUEST_BYTES || &bytes[..21] != REQUEST_DOMAIN {
        return Err(RegistrationError::Corrupt);
    }
    let request = RegisterBranch {
        request: RequestId::from_bytes(fixed(&bytes[21..37])?),
        scope: Scope {
            universe: UniverseId::from_bytes(fixed(&bytes[37..53])?),
            history: HistoryId::from_bytes(fixed(&bytes[53..69])?),
        },
        campaign: CampaignId::from_bytes(fixed(&bytes[69..85])?),
        branch: BranchId::from_bytes(fixed(&bytes[85..101])?),
        account: AccountId::from_bytes(fixed(&bytes[101..117])?),
        device: DeviceId::from_bytes(fixed(&bytes[117..133])?),
        policy_digest: fixed(&bytes[133..165])?,
    };
    if request_bytes(&request).as_slice() != bytes {
        return Err(RegistrationError::Corrupt);
    }
    Ok(request)
}

pub(super) fn journal_head(
    previous: &[u8; 32],
    revision: u64,
    digest: &[u8; 32],
    body: &[u8; REQUEST_BYTES],
) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(272);
    bytes.extend_from_slice(b"NF-BRANCH-JOURNAL-1\0");
    bytes.extend_from_slice(previous);
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(digest);
    bytes.extend_from_slice(body);
    crate::schema::hash(&bytes)
}
