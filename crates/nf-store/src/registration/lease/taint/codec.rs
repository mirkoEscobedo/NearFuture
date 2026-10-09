use super::model::{MarkProhibitedManifest, TaintMode, TaintPolicy};
use crate::registration::lease::{AdmissionMode, LeaseGranted};
use crate::registration::{CampaignBinding, RegisteredBranch};
use nf_identity::model::Scope;

// Domain28 + request16 + binding80 + device16 + session16 + grant169 + lineage32
// + three frontiers(80 each) + policy32 + detector32 + version8 + manifests64 = 733.
const REQUEST_BYTES: usize = 733;
pub(super) fn policy_bytes(policy: &TaintPolicy) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4000);
    bytes.extend_from_slice(b"NF-HEADLESS-TAINT-POLICY-1\0");
    bytes.push(match policy.mode {
        TaintMode::HeadlessTaintOnly => 1,
    });
    bytes.extend_from_slice(&policy.registration_policy_digest);
    bytes.extend_from_slice(&policy.admission_policy_digest);
    bytes.extend_from_slice(&policy.expected_detector);
    bytes.extend_from_slice(&policy.expected_detector_version.to_be_bytes());
    bytes.extend_from_slice(&policy.expected_manifest);
    bytes.extend_from_slice(&(policy.campaigns.len() as u64).to_be_bytes());
    for entry in &policy.campaigns {
        bytes.extend_from_slice(entry.campaign.as_bytes());
        bytes.extend_from_slice(entry.lineage.as_bytes());
    }
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
fn registered(bytes: &mut Vec<u8>, registration: RegisteredBranch) {
    binding(bytes, registration.binding);
    bytes.extend_from_slice(registration.original_request.as_bytes());
    bytes.extend_from_slice(&registration.revision.to_be_bytes());
}
fn grant(bytes: &mut Vec<u8>, grant: LeaseGranted) {
    bytes.push(match grant.mode {
        AdmissionMode::HeadlessLeaseOnly => 1,
    });
    registered(bytes, grant.registration);
    bytes.extend_from_slice(grant.original_request.as_bytes());
    bytes.extend_from_slice(grant.device.as_bytes());
    bytes.extend_from_slice(grant.session.as_bytes());
    bytes.extend_from_slice(&grant.generation.to_be_bytes());
    bytes.extend_from_slice(&grant.revision.to_be_bytes());
}
fn frontier(
    bytes: &mut Vec<u8>,
    frontier_scope: Scope,
    revision: u64,
    head: &[u8; 32],
    member: u64,
) {
    scope(bytes, frontier_scope);
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(head);
    bytes.extend_from_slice(&member.to_be_bytes());
}
pub(super) fn request_bytes(request: &MarkProhibitedManifest) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(REQUEST_BYTES);
    bytes.extend_from_slice(b"NF-HEADLESS-TAINT-REQUEST-1\0");
    bytes.extend_from_slice(request.request.as_bytes());
    binding(&mut bytes, request.binding);
    bytes.extend_from_slice(request.device.as_bytes());
    bytes.extend_from_slice(request.session.as_bytes());
    grant(&mut bytes, request.grant);
    bytes.extend_from_slice(request.lineage.as_bytes());
    let known = request.known_registration;
    frontier(
        &mut bytes,
        known.scope,
        known.revision,
        &known.head,
        known.minimum_membership_revision,
    );
    let known = request.known_admission;
    frontier(
        &mut bytes,
        known.scope,
        known.revision,
        &known.head,
        known.minimum_membership_revision,
    );
    let known = request.known_taint;
    frontier(
        &mut bytes,
        known.scope,
        known.revision,
        &known.head,
        known.minimum_membership_revision,
    );
    bytes.extend_from_slice(&request.policy_digest);
    bytes.extend_from_slice(&request.expected_detector);
    bytes.extend_from_slice(&request.expected_detector_version.to_be_bytes());
    bytes.extend_from_slice(&request.expected_manifest);
    bytes.extend_from_slice(&request.observed_manifest);
    bytes
}
pub(super) fn request_digest(request: &MarkProhibitedManifest) -> [u8; 32] {
    crate::schema::hash(&request_bytes(request))
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
            .ok_or(super::model::TaintError::Corrupt)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(super::model::TaintError::Corrupt)?;
        self.offset = end;
        bytes
            .try_into()
            .map_err(|_| super::model::TaintError::Corrupt)
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
    fn registration(&mut self) -> super::model::Result<RegisteredBranch> {
        Ok(RegisteredBranch {
            binding: self.binding()?,
            original_request: nf_contract::identity::RequestId::from_bytes(self.take()?),
            revision: self.counter()?,
        })
    }
    fn grant(&mut self) -> super::model::Result<LeaseGranted> {
        if self.take::<1>()? != [1] {
            return Err(super::model::TaintError::Corrupt);
        }
        Ok(LeaseGranted {
            mode: AdmissionMode::HeadlessLeaseOnly,
            registration: self.registration()?,
            original_request: nf_contract::identity::RequestId::from_bytes(self.take()?),
            device: nf_contract::identity::DeviceId::from_bytes(self.take()?),
            session: crate::registration::lease::ClientSessionId::from_bytes(self.take()?)
                .map_err(|_| super::model::TaintError::Corrupt)?,
            generation: self.counter()?,
            revision: self.counter()?,
        })
    }
    fn registration_frontier(
        &mut self,
    ) -> super::model::Result<crate::registration::KnownRegistrationFrontier> {
        Ok(crate::registration::KnownRegistrationFrontier {
            scope: self.scope()?,
            revision: self.counter()?,
            head: self.take()?,
            minimum_membership_revision: self.counter()?,
        })
    }
    fn admission_frontier(
        &mut self,
    ) -> super::model::Result<crate::registration::lease::KnownAdmissionFrontier> {
        Ok(crate::registration::lease::KnownAdmissionFrontier {
            scope: self.scope()?,
            revision: self.counter()?,
            head: self.take()?,
            minimum_membership_revision: self.counter()?,
        })
    }
    fn taint_frontier(&mut self) -> super::model::Result<super::model::KnownTaintFrontier> {
        Ok(super::model::KnownTaintFrontier {
            scope: self.scope()?,
            revision: self.counter()?,
            head: self.take()?,
            minimum_membership_revision: self.counter()?,
        })
    }
}
pub(super) fn decode_request(bytes: &[u8]) -> super::model::Result<MarkProhibitedManifest> {
    const DOMAIN: &[u8] = b"NF-HEADLESS-TAINT-REQUEST-1\0";
    if bytes.len() != REQUEST_BYTES || !bytes.starts_with(DOMAIN) {
        return Err(super::model::TaintError::Corrupt);
    }
    let mut cursor = Cursor {
        bytes,
        offset: DOMAIN.len(),
    };
    let request = MarkProhibitedManifest {
        request: nf_contract::identity::RequestId::from_bytes(cursor.take()?),
        binding: cursor.binding()?,
        device: nf_contract::identity::DeviceId::from_bytes(cursor.take()?),
        session: crate::registration::lease::ClientSessionId::from_bytes(cursor.take()?)
            .map_err(|_| super::model::TaintError::Corrupt)?,
        grant: cursor.grant()?,
        lineage: super::model::AuthorityLineageId::from_bytes(cursor.take()?)
            .map_err(|_| super::model::TaintError::Corrupt)?,
        known_registration: cursor.registration_frontier()?,
        known_admission: cursor.admission_frontier()?,
        known_taint: cursor.taint_frontier()?,
        policy_digest: cursor.take()?,
        expected_detector: cursor.take()?,
        expected_detector_version: cursor.counter()?,
        expected_manifest: cursor.take()?,
        observed_manifest: cursor.take()?,
    };
    if cursor.offset != REQUEST_BYTES || request_bytes(&request) != bytes {
        return Err(super::model::TaintError::Corrupt);
    }
    Ok(request)
}
pub(super) fn journal_head(
    previous: &[u8; 32],
    revision: u64,
    digest: &[u8; 32],
    body: &[u8],
) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(833);
    bytes.extend_from_slice(b"NF-HEADLESS-TAINT-JOURNAL-1\0");
    bytes.extend_from_slice(previous);
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(digest);
    bytes.extend_from_slice(body);
    crate::schema::hash(&bytes)
}
