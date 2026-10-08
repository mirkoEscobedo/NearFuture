use super::{
    codec,
    model::*,
    replay::{MAX_REGISTRATIONS, State},
};
use crate::schema::counter;
use rusqlite::{Transaction, params};

/// Runs under the caller's current authenticated immediate transaction.
pub(super) fn register(
    tx: &Transaction<'_>,
    request: &RegisterBranch,
    state: &State,
) -> Result<RegisteredBranch> {
    if let Some(original) = state.requests.get(&request.request) {
        if codec::request_bytes(&original.request) != codec::request_bytes(request) {
            return Err(RegistrationError::Conflict);
        }
        return Ok(original.receipt);
    }
    if state
        .branches
        .contains_key(&(request.campaign, request.branch))
    {
        return Err(RegistrationError::Conflict);
    }
    if state.requests.len() >= MAX_REGISTRATIONS {
        return Err(RegistrationError::Limit);
    }
    let revision = state
        .revision
        .checked_add(1)
        .ok_or(RegistrationError::Limit)?;
    let body = codec::request_bytes(request);
    let digest = codec::request_digest(request);
    let head = codec::journal_head(&state.head, revision, &digest, &body);
    let receipt = RegisteredBranch {
        binding: CampaignBinding {
            scope: request.scope,
            campaign: request.campaign,
            branch: request.branch,
            account: request.account,
        },
        original_request: request.request,
        revision,
    };
    tx.execute(
        "INSERT INTO registration_journal VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            counter(revision),
            request.request.as_bytes(),
            state.head,
            digest,
            head,
            body.as_slice()
        ],
    )?;
    tx.execute(
        "INSERT INTO registration_branches VALUES(?1,?2,?3,?4,?5)",
        params![
            request.campaign.as_bytes(),
            request.branch.as_bytes(),
            request.account.as_bytes(),
            request.request.as_bytes(),
            counter(revision)
        ],
    )?;
    let changed = tx.execute("UPDATE registration_meta SET revision=?1,head=?2 WHERE singleton=1 AND revision=?3 AND head=?4",
        params![counter(revision), head, counter(state.revision), state.head])?;
    if changed != 1 {
        return Err(RegistrationError::Corrupt);
    }
    Ok(receipt)
}
