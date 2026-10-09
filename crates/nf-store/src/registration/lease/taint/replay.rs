use super::{codec, model::*, schema};
use crate::registration::lease::{AdmissionPolicy, KnownAdmissionFrontier, replay as lease_replay};
use crate::registration::{KnownRegistrationFrontier, RegistrationPolicy};
use crate::{persistence::bounded_blob, schema::read_counter};
use nf_contract::identity::RequestId;
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) const MAX_TAINTS: usize = 64;
pub(super) struct Entry {
    pub request: MarkProhibitedManifest,
    pub receipt: TaintRecorded,
}
pub(super) struct State {
    pub leases: lease_replay::State,
    pub requests: BTreeMap<RequestId, Entry>,
    pub lineages: BTreeMap<AuthorityLineageId, TaintRecorded>,
    pub heads: Vec<[u8; 32]>,
    pub revision: u64,
    pub head: [u8; 32],
}
type JournalRow = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
impl State {
    pub fn frontier(&self) -> KnownTaintFrontier {
        KnownTaintFrontier {
            scope: self.leases.membership.scope,
            revision: self.revision,
            head: self.head,
            minimum_membership_revision: self.leases.membership.revision,
        }
    }
}
pub(super) fn receipt(request: &MarkProhibitedManifest, revision: u64) -> TaintRecorded {
    TaintRecorded {
        original_request: request.request,
        binding: request.binding,
        lineage: request.lineage,
        revision,
        cause: TaintCause::SelfReportedProhibitedManifest,
        expected_detector: request.expected_detector,
        expected_detector_version: request.expected_detector_version,
        expected_manifest: request.expected_manifest,
        observed_manifest: request.observed_manifest,
    }
}
pub(super) fn verify(
    connection: &Connection,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
    taint: &TaintPolicy,
) -> Result<State> {
    let profile = schema::verify(connection, registration, admission, taint)?;
    let mut state = State {
        leases: profile.leases,
        requests: BTreeMap::new(),
        lineages: BTreeMap::new(),
        heads: vec![[0; 32]],
        revision: 0,
        head: [0; 32],
    };
    let Some(meta) = profile.taint else {
        return Ok(state);
    };
    let mut statement = connection.prepare("SELECT revision,request,previous,request_digest,head,body FROM taint_journal ORDER BY revision LIMIT 65")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if state.requests.len() == MAX_TAINTS {
            return Err(TaintError::Corrupt);
        }
        let (revision, request_id, previous, digest, head, body): JournalRow = (
            bounded_blob(row, 0, 8)?,
            bounded_blob(row, 1, 16)?,
            bounded_blob(row, 2, 32)?,
            bounded_blob(row, 3, 32)?,
            bounded_blob(row, 4, 32)?,
            bounded_blob(row, 5, 733)?,
        );
        let revision = read_counter(&revision)?;
        let next = state.revision.checked_add(1).ok_or(TaintError::Corrupt)?;
        let request = codec::decode_request(&body)?;
        validate_request(&request, &state, registration, admission, taint)
            .map_err(|_| TaintError::Corrupt)?;
        let expected_digest = codec::request_digest(&request);
        let expected_head = codec::journal_head(&state.head, revision, &expected_digest, &body);
        if revision != next
            || request_id != request.request.as_bytes()
            || previous != state.head
            || digest != expected_digest
            || head != expected_head
        {
            return Err(TaintError::Corrupt);
        }
        let receipt = receipt(&request, revision);
        if state
            .requests
            .insert(request.request, Entry { request, receipt })
            .is_some()
            || state.lineages.insert(request.lineage, receipt).is_some()
        {
            return Err(TaintError::Corrupt);
        }
        state.revision = revision;
        state.head = expected_head;
        state.heads.push(expected_head);
    }
    if state.revision == 0 || state.revision != meta.revision || state.head != meta.head {
        return Err(TaintError::Corrupt);
    }
    verify_lineages(connection, &state)?;
    Ok(state)
}
pub(super) fn check_known(
    state: &State,
    registration: KnownRegistrationFrontier,
    admission: KnownAdmissionFrontier,
    taint: KnownTaintFrontier,
) -> Result<()> {
    lease_replay::check_known(&state.leases, registration, admission)?;
    if taint.scope != state.leases.membership.scope {
        return Err(TaintError::Scope);
    }
    if state.revision < taint.revision
        || state.leases.membership.revision < taint.minimum_membership_revision
    {
        return Err(TaintError::StaleBackup);
    }
    let prefix = usize::try_from(taint.revision)
        .ok()
        .and_then(|index| state.heads.get(index))
        .ok_or(TaintError::StaleBackup)?;
    if prefix != &taint.head {
        return Err(TaintError::Corrupt);
    }
    Ok(())
}
pub(super) fn validate_request(
    request: &MarkProhibitedManifest,
    state: &State,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
    policy: &TaintPolicy,
) -> Result<()> {
    if request.binding.scope != registration.scope {
        return Err(TaintError::Scope);
    }
    check_known(
        state,
        request.known_registration,
        request.known_admission,
        request.known_taint,
    )?;
    let actual = state
        .leases
        .requests
        .get(&request.grant.original_request)
        .ok_or(TaintError::UnknownGrant)?;
    lease_replay::validate_request(&actual.request, &state.leases, registration, admission)?;
    let active = state
        .leases
        .active
        .get(&(request.binding.campaign, request.binding.branch))
        .ok_or(TaintError::UnknownGrant)?;
    if actual.grant != request.grant
        || *active != request.grant
        || request.grant.registration.binding != request.binding
        || request.grant.device != request.device
        || request.grant.session != request.session
        || request.grant.generation != 1
        || request.known_registration.revision < request.grant.registration.revision
        || request.known_admission.revision < request.grant.revision
    {
        return Err(TaintError::UnknownGrant);
    }
    policy.validate(registration, admission)?;
    if request.lineage != policy.lineage(request.binding.campaign)?
        || request.policy_digest != policy.digest()?
        || request.expected_detector != policy.expected_detector
        || request.expected_detector_version != policy.expected_detector_version
        || request.expected_manifest != policy.expected_manifest
        || request.observed_manifest == [0; 32]
        || request.observed_manifest == request.expected_manifest
    {
        return Err(TaintError::Policy);
    }
    Ok(())
}
fn verify_lineages(connection: &Connection, state: &State) -> Result<()> {
    let mut actual = BTreeMap::new();
    let mut statement = connection.prepare(
        "SELECT lineage,original_request,revision FROM taint_lineages ORDER BY lineage LIMIT 65",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if actual.len() == MAX_TAINTS {
            return Err(TaintError::Corrupt);
        }
        let lineage: [u8; 32] = bounded_blob(row, 0, 32)?
            .try_into()
            .map_err(|_| TaintError::Corrupt)?;
        let lineage = AuthorityLineageId::from_bytes(lineage).map_err(|_| TaintError::Corrupt)?;
        let request: [u8; 16] = bounded_blob(row, 1, 16)?
            .try_into()
            .map_err(|_| TaintError::Corrupt)?;
        let revision = read_counter(&bounded_blob(row, 2, 8)?)?;
        if actual
            .insert(lineage, (RequestId::from_bytes(request), revision))
            .is_some()
        {
            return Err(TaintError::Corrupt);
        }
    }
    let expected: BTreeMap<_, _> = state
        .lineages
        .iter()
        .map(|(lineage, receipt)| (*lineage, (receipt.original_request, receipt.revision)))
        .collect();
    if actual != expected {
        return Err(TaintError::Corrupt);
    }
    Ok(())
}
