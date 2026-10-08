use super::model::{AdmissionMode, AdmissionPolicy, AdmitLease, KnownAdmissionFrontier};
use crate::registration::{CampaignBinding, KnownRegistrationFrontier};
use nf_identity::model::Scope;

pub(super) const REQUEST_BYTES: usize = 452;

pub(super) fn policy_bytes(policy: &AdmissionPolicy) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(b"NF-HEADLESS-LEASE-POLICY-1\0");
    bytes.push(match policy.mode {
        AdmissionMode::HeadlessLeaseOnly => 1,
    });
    bytes.extend_from_slice(&policy.registration_policy_digest);
    bytes
}
fn scope(bytes: &mut Vec<u8>, scope: Scope) {
    bytes.extend_from_slice(scope.universe.as_bytes());
    bytes.extend_from_slice(scope.history.as_bytes());
}
fn binding(bytes: &mut Vec<u8>, binding: CampaignBinding) {
    scope(bytes, binding.scope);
    bytes.extend_from_slice(binding.campaign.as_bytes());
    bytes.extend_from_slice(binding.branch.as_bytes());
    bytes.extend_from_slice(binding.account.as_bytes());
}
fn registration_frontier(bytes: &mut Vec<u8>, frontier: KnownRegistrationFrontier) {
    scope(bytes, frontier.scope);
    bytes.extend_from_slice(&frontier.revision.to_be_bytes());
    bytes.extend_from_slice(&frontier.head);
    bytes.extend_from_slice(&frontier.minimum_membership_revision.to_be_bytes());
}
fn admission_frontier(bytes: &mut Vec<u8>, frontier: KnownAdmissionFrontier) {
    scope(bytes, frontier.scope);
    bytes.extend_from_slice(&frontier.revision.to_be_bytes());
    bytes.extend_from_slice(&frontier.head);
    bytes.extend_from_slice(&frontier.minimum_membership_revision.to_be_bytes());
}
pub(super) fn request_bytes(request: &AdmitLease) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(REQUEST_BYTES);
    bytes.extend_from_slice(b"NF-HEADLESS-LEASE-REQUEST-1\0");
    bytes.extend_from_slice(request.request.as_bytes());
    binding(&mut bytes, request.binding);
    bytes.extend_from_slice(request.device.as_bytes());
    bytes.extend_from_slice(request.session.as_bytes());
    binding(&mut bytes, request.registration.binding);
    bytes.extend_from_slice(request.registration.original_request.as_bytes());
    bytes.extend_from_slice(&request.registration.revision.to_be_bytes());
    registration_frontier(&mut bytes, request.known_registration);
    admission_frontier(&mut bytes, request.known_admission);
    bytes.extend_from_slice(&request.policy_digest);
    bytes
}
pub(super) fn request_digest(request: &AdmitLease) -> [u8; 32] {
    crate::schema::hash(&request_bytes(request))
}

pub(super) fn fixed<const N: usize>(bytes: &[u8]) -> super::model::Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| super::model::LeaseError::Corrupt)
}
struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Cursor<'_> {
    fn take<const N: usize>(&mut self) -> super::model::Result<[u8; N]> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(super::model::LeaseError::Corrupt)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(super::model::LeaseError::Corrupt)?;
        self.offset = end;
        fixed(bytes)
    }
    fn counter(&mut self) -> super::model::Result<u64> {
        Ok(u64::from_be_bytes(self.take()?))
    }
    fn scope(&mut self) -> super::model::Result<Scope> {
        Ok(Scope {
            universe: nf_contract::identity::UniverseId::from_bytes(self.take()?),
            history: nf_contract::identity::HistoryId::from_bytes(self.take()?),
        })
    }
    fn binding(&mut self) -> super::model::Result<CampaignBinding> {
        Ok(CampaignBinding {
            scope: self.scope()?,
            campaign: nf_contract::identity::CampaignId::from_bytes(self.take()?),
            branch: nf_contract::identity::BranchId::from_bytes(self.take()?),
            account: nf_contract::identity::AccountId::from_bytes(self.take()?),
        })
    }
}
pub(super) fn decode_request(bytes: &[u8]) -> super::model::Result<AdmitLease> {
    use super::model::{ClientSessionId, LeaseError};
    use crate::registration::RegisteredBranch;
    use nf_contract::identity::{DeviceId, RequestId};
    const DOMAIN: &[u8] = b"NF-HEADLESS-LEASE-REQUEST-1\0";
    if bytes.len() != REQUEST_BYTES || !bytes.starts_with(DOMAIN) {
        return Err(LeaseError::Corrupt);
    }
    let mut cursor = Cursor {
        bytes,
        offset: DOMAIN.len(),
    };
    let request = AdmitLease {
        request: RequestId::from_bytes(cursor.take()?),
        binding: cursor.binding()?,
        device: DeviceId::from_bytes(cursor.take()?),
        session: ClientSessionId::from_bytes(cursor.take()?).map_err(|_| LeaseError::Corrupt)?,
        registration: RegisteredBranch {
            binding: cursor.binding()?,
            original_request: RequestId::from_bytes(cursor.take()?),
            revision: cursor.counter()?,
        },
        known_registration: KnownRegistrationFrontier {
            scope: cursor.scope()?,
            revision: cursor.counter()?,
            head: cursor.take()?,
            minimum_membership_revision: cursor.counter()?,
        },
        known_admission: KnownAdmissionFrontier {
            scope: cursor.scope()?,
            revision: cursor.counter()?,
            head: cursor.take()?,
            minimum_membership_revision: cursor.counter()?,
        },
        policy_digest: cursor.take()?,
    };
    if cursor.offset != REQUEST_BYTES || request_bytes(&request) != bytes {
        return Err(LeaseError::Corrupt);
    }
    Ok(request)
}
pub(super) fn journal_head(
    previous: &[u8; 32],
    revision: u64,
    digest: &[u8; 32],
    body: &[u8],
) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(544);
    bytes.extend_from_slice(b"NF-HEADLESS-LEASE-JOURNAL-1\0");
    bytes.extend_from_slice(previous);
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(digest);
    bytes.extend_from_slice(body);
    crate::schema::hash(&bytes)
}
