use super::{
    codec,
    model::*,
    replay::{MAX_ADMISSIONS, State},
    schema,
};
use crate::{registration::RegistrationPolicy, schema::counter};
use rusqlite::{Transaction, params};

pub(super) enum Prepared {
    Retry(LeaseGranted),
    New {
        request: Box<AdmitLease>,
        body: Vec<u8>,
        digest: [u8; 32],
        head: [u8; 32],
        grant: LeaseGranted,
    },
}
impl Prepared {
    pub fn grant(&self) -> LeaseGranted {
        match self {
            Self::Retry(grant) | Self::New { grant, .. } => *grant,
        }
    }
    pub fn frontier(&self, state: &State) -> KnownAdmissionFrontier {
        match self {
            Self::Retry(_) => state.frontier(),
            Self::New { grant, head, .. } => KnownAdmissionFrontier {
                revision: grant.revision,
                head: *head,
                ..state.frontier()
            },
        }
    }
}
/// All pure request, bound, generation, revision and canonical checks precede any DDL/DML.
pub(super) fn prepare(request: &AdmitLease, state: &State) -> Result<Prepared> {
    let body = codec::request_bytes(request);
    if codec::decode_request(&body)? != *request {
        return Err(LeaseError::Corrupt);
    }
    if let Some(original) = state.requests.get(&request.request) {
        if codec::request_bytes(&original.request) != body {
            return Err(LeaseError::Conflict);
        }
        return Ok(Prepared::Retry(original.grant));
    }
    let key = (request.binding.campaign, request.binding.branch);
    if state.active.contains_key(&key) {
        return Err(LeaseError::Conflict);
    }
    if state.requests.len() >= MAX_ADMISSIONS || state.active.len() >= MAX_ADMISSIONS {
        return Err(LeaseError::Limit);
    }
    let revision = state.revision.checked_add(1).ok_or(LeaseError::Limit)?;
    let generation = state
        .active
        .get(&key)
        .map_or(0, |grant| grant.generation)
        .checked_add(1)
        .ok_or(LeaseError::Limit)?;
    if generation != 1 {
        return Err(LeaseError::Corrupt);
    }
    let digest = codec::request_digest(request);
    let head = codec::journal_head(&state.head, revision, &digest, &body);
    let grant = LeaseGranted {
        mode: AdmissionMode::HeadlessLeaseOnly,
        registration: request.registration,
        original_request: request.request,
        device: request.device,
        session: request.session,
        generation,
        revision,
    };
    Ok(Prepared::New {
        request: Box::new(*request),
        body,
        digest,
        head,
        grant,
    })
}
pub(super) fn apply(
    tx: &Transaction<'_>,
    prepared: &Prepared,
    state: &State,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
) -> Result<()> {
    let Prepared::New {
        request,
        body,
        digest,
        head,
        grant,
    } = prepared
    else {
        return Ok(());
    };
    if state.version == 1 {
        schema::activate(tx, registration, admission)?;
    }
    tx.execute(
        "INSERT INTO admission_journal VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            counter(grant.revision),
            request.request.as_bytes(),
            state.head,
            digest,
            head,
            body
        ],
    )?;
    tx.execute(
        "INSERT INTO admission_active VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            request.binding.campaign.as_bytes(),
            request.binding.branch.as_bytes(),
            request.binding.account.as_bytes(),
            request.device.as_bytes(),
            request.session.as_bytes(),
            request.registration.original_request.as_bytes(),
            counter(request.registration.revision),
            counter(grant.generation),
            request.request.as_bytes(),
            counter(grant.revision)
        ],
    )?;
    let changed = tx.execute("UPDATE admission_meta SET revision=?1,head=?2 WHERE singleton=1 AND revision=?3 AND head=?4",
        params![counter(grant.revision), head, counter(state.revision), state.head])?;
    if changed != 1 {
        return Err(LeaseError::Corrupt);
    }
    Ok(())
}
