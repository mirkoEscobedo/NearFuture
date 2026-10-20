use super::{
    codec,
    model::*,
    replay::{MAX_TAINTS, State, receipt},
    schema,
};
use crate::registration::lease::AdmissionPolicy;
use crate::{registration::RegistrationPolicy, schema::counter};
use rusqlite::{Transaction, params};

pub(super) enum Prepared {
    Retry(TaintRecorded),
    New {
        request: Box<MarkProhibitedManifest>,
        body: Vec<u8>,
        digest: [u8; 32],
        head: [u8; 32],
        receipt: TaintRecorded,
    },
}
impl Prepared {
    pub fn receipt(&self) -> TaintRecorded {
        match self {
            Self::Retry(receipt) | Self::New { receipt, .. } => *receipt,
        }
    }
    pub fn frontier(&self, state: &State) -> KnownTaintFrontier {
        match self {
            Self::Retry(_) => state.frontier(),
            Self::New { receipt, head, .. } => KnownTaintFrontier {
                revision: receipt.revision,
                head: *head,
                ..state.frontier()
            },
        }
    }
}
/// Canonical roundtrip, dedupe, lineage/capacity/revision checks precede all DDL/DML.
/// This first cause profile never replaces or clears an existing lineage cause.
pub(super) fn prepare(request: &MarkProhibitedManifest, state: &State) -> Result<Prepared> {
    let body = codec::request_bytes(request);
    if codec::decode_request(&body)? != *request {
        return Err(TaintError::Corrupt);
    }
    if let Some(original) = state.requests.get(&request.request) {
        if codec::request_bytes(&original.request) != body {
            return Err(TaintError::Conflict);
        }
        return Ok(Prepared::Retry(original.receipt));
    }
    if state.lineages.contains_key(&request.lineage) {
        return Err(TaintError::Conflict);
    }
    if state.requests.len() >= MAX_TAINTS || state.lineages.len() >= MAX_TAINTS {
        return Err(TaintError::Limit);
    }
    let revision = state.revision.checked_add(1).ok_or(TaintError::Limit)?;
    let digest = codec::request_digest(request);
    let head = codec::journal_head(&state.head, revision, &digest, &body);
    let receipt = receipt(request, revision);
    Ok(Prepared::New {
        request: Box::new(*request),
        body,
        digest,
        head,
        receipt,
    })
}
pub(super) fn apply(
    tx: &Transaction<'_>,
    prepared: &Prepared,
    state: &State,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
    taint: &TaintPolicy,
) -> Result<()> {
    let Prepared::New {
        request,
        body,
        digest,
        head,
        receipt,
    } = prepared
    else {
        return Ok(());
    };
    if state.leases.version == 2 {
        schema::activate(tx, registration, admission, taint)?;
    }
    tx.execute(
        "INSERT INTO taint_journal VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            counter(receipt.revision),
            request.request.as_bytes(),
            state.head,
            digest,
            head,
            body
        ],
    )?;
    tx.execute(
        "INSERT INTO taint_lineages VALUES(?1,?2,?3)",
        params![
            request.lineage.as_bytes(),
            request.request.as_bytes(),
            counter(receipt.revision)
        ],
    )?;
    let changed = tx.execute(
        "UPDATE taint_meta SET revision=?1,head=?2 WHERE singleton=1 AND revision=?3 AND head=?4",
        params![
            counter(receipt.revision),
            head,
            counter(state.revision),
            state.head
        ],
    )?;
    if changed != 1 {
        return Err(TaintError::Corrupt);
    }
    Ok(())
}
