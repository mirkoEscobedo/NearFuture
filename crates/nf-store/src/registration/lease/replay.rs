use super::{codec, model::*, schema};
use crate::{
    persistence::bounded_blob,
    registration::{
        CampaignBinding, KnownRegistrationFrontier, RegisteredBranch, RegistrationPolicy,
        membership, replay as registration_replay,
    },
    schema::read_counter,
};
use nf_contract::identity::{AccountId, BranchId, CampaignId, DeviceId, RequestId};
use nf_identity::model::MembershipState;
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) const MAX_ADMISSIONS: usize = 64;
pub(super) struct Entry {
    pub request: AdmitLease,
    pub grant: LeaseGranted,
}
pub(super) struct State {
    pub registrations: registration_replay::State,
    pub membership: MembershipState,
    pub version: i32,
    pub requests: BTreeMap<RequestId, Entry>,
    pub active: BTreeMap<(CampaignId, BranchId), LeaseGranted>,
    pub heads: Vec<[u8; 32]>,
    pub revision: u64,
    pub head: [u8; 32],
}
type JournalRow = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
impl State {
    pub fn frontier(&self) -> KnownAdmissionFrontier {
        KnownAdmissionFrontier {
            scope: self.membership.scope,
            revision: self.revision,
            head: self.head,
            minimum_membership_revision: self.membership.revision,
        }
    }
}
pub(super) fn verify(
    connection: &Connection,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
) -> Result<State> {
    let profile = schema::verify(connection, registration, admission)?;
    let membership = membership::load(connection, registration.scope)?;
    let mut state = State {
        registrations: profile.registrations,
        membership,
        version: profile.version,
        requests: BTreeMap::new(),
        active: BTreeMap::new(),
        heads: vec![[0; 32]],
        revision: 0,
        head: [0; 32],
    };
    let Some(meta) = profile.admission else {
        return Ok(state);
    };
    let mut statement = connection.prepare("SELECT revision,request,previous,request_digest,head,body FROM admission_journal ORDER BY revision LIMIT 65")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if state.requests.len() == MAX_ADMISSIONS {
            return Err(LeaseError::Corrupt);
        }
        let (revision, request_id, previous, digest, head, body): JournalRow = (
            bounded_blob(row, 0, 8)?,
            bounded_blob(row, 1, 16)?,
            bounded_blob(row, 2, 32)?,
            bounded_blob(row, 3, 32)?,
            bounded_blob(row, 4, 32)?,
            bounded_blob(row, 5, codec::REQUEST_BYTES)?,
        );
        let revision = read_counter(&revision)?;
        let next = state.revision.checked_add(1).ok_or(LeaseError::Corrupt)?;
        let request = codec::decode_request(&body)?;
        validate_request(&request, &state, registration, admission)
            .map_err(|_| LeaseError::Corrupt)?;
        let expected_digest = codec::request_digest(&request);
        let expected_head = codec::journal_head(&state.head, revision, &expected_digest, &body);
        if revision != next
            || request_id != request.request.as_bytes()
            || previous != state.head
            || digest != expected_digest
            || head != expected_head
        {
            return Err(LeaseError::Corrupt);
        }
        let grant = LeaseGranted {
            mode: AdmissionMode::HeadlessLeaseOnly,
            registration: request.registration,
            original_request: request.request,
            device: request.device,
            session: request.session,
            generation: 1,
            revision,
        };
        if state
            .requests
            .insert(request.request, Entry { request, grant })
            .is_some()
            || state
                .active
                .insert((request.binding.campaign, request.binding.branch), grant)
                .is_some()
        {
            return Err(LeaseError::Corrupt);
        }
        state.revision = revision;
        state.head = expected_head;
        state.heads.push(expected_head);
    }
    if state.revision == 0 || state.revision != meta.revision || state.head != meta.head {
        return Err(LeaseError::Corrupt);
    }
    verify_active(connection, registration, &state)?;
    Ok(state)
}

pub(super) fn check_known(
    state: &State,
    registration: KnownRegistrationFrontier,
    admission: KnownAdmissionFrontier,
) -> Result<()> {
    if registration.scope != state.membership.scope || admission.scope != state.membership.scope {
        return Err(LeaseError::Scope);
    }
    registration_replay::check_known(
        &state.registrations,
        registration,
        state.membership.revision,
    )?;
    if state.revision < admission.revision
        || state.membership.revision < admission.minimum_membership_revision
    {
        return Err(LeaseError::StaleBackup);
    }
    let prefix = usize::try_from(admission.revision)
        .ok()
        .and_then(|index| state.heads.get(index))
        .ok_or(LeaseError::StaleBackup)?;
    if prefix != &admission.head {
        return Err(LeaseError::Corrupt);
    }
    Ok(())
}
pub(super) fn validate_request(
    request: &AdmitLease,
    state: &State,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
) -> Result<()> {
    if request.binding.scope != registration.scope {
        return Err(LeaseError::Scope);
    }
    check_known(state, request.known_registration, request.known_admission)?;
    let actual = state
        .registrations
        .requests
        .get(&request.registration.original_request)
        .ok_or(LeaseError::UnknownRegistration)?;
    if actual.receipt != request.registration
        || actual.receipt.binding != request.binding
        || request.known_registration.revision < actual.receipt.revision
    {
        return Err(LeaseError::UnknownRegistration);
    }
    registration.permit(&actual.request)?;
    admission.validate(registration)?;
    if request.policy_digest != admission.digest() {
        return Err(LeaseError::Policy);
    }
    Ok(())
}

fn verify_active(
    connection: &Connection,
    registration: &RegistrationPolicy,
    state: &State,
) -> Result<()> {
    let mut actual = BTreeMap::new();
    let mut statement = connection.prepare("SELECT campaign,branch,account,device,session,registered_request,registered_revision,generation,admission_request,admission_revision FROM admission_active ORDER BY campaign,branch LIMIT 65")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if actual.len() == MAX_ADMISSIONS {
            return Err(LeaseError::Corrupt);
        }
        let bounds = [16, 16, 16, 16, 16, 16, 8, 8, 16, 8];
        let columns: Vec<Vec<u8>> = bounds
            .into_iter()
            .enumerate()
            .map(|(index, bound)| bounded_blob(row, index, bound))
            .collect::<rusqlite::Result<_>>()?;
        let campaign = CampaignId::from_bytes(codec::fixed(&columns[0])?);
        let branch = BranchId::from_bytes(codec::fixed(&columns[1])?);
        let grant = LeaseGranted {
            mode: AdmissionMode::HeadlessLeaseOnly,
            registration: RegisteredBranch {
                binding: CampaignBinding {
                    scope: registration.scope,
                    campaign,
                    branch,
                    account: AccountId::from_bytes(codec::fixed(&columns[2])?),
                },
                original_request: RequestId::from_bytes(codec::fixed(&columns[5])?),
                revision: read_counter(&columns[6])?,
            },
            device: DeviceId::from_bytes(codec::fixed(&columns[3])?),
            session: ClientSessionId::from_bytes(codec::fixed(&columns[4])?)
                .map_err(|_| LeaseError::Corrupt)?,
            generation: read_counter(&columns[7])?,
            original_request: RequestId::from_bytes(codec::fixed(&columns[8])?),
            revision: read_counter(&columns[9])?,
        };
        if actual.insert((campaign, branch), grant).is_some() {
            return Err(LeaseError::Corrupt);
        }
    }
    if actual != state.active {
        return Err(LeaseError::Corrupt);
    }
    Ok(())
}
