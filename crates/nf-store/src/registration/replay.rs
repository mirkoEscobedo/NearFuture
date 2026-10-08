use super::{codec, model::*, schema};
use crate::{persistence::bounded_blob, schema::read_counter};
use nf_contract::identity::{AccountId, BranchId, CampaignId, RequestId};
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) const MAX_REGISTRATIONS: usize = 64;
pub(super) struct Entry {
    pub request: RegisterBranch,
    pub receipt: RegisteredBranch,
}
pub(super) struct State {
    pub requests: BTreeMap<RequestId, Entry>,
    pub branches: BTreeMap<(CampaignId, BranchId), RegisteredBranch>,
    pub heads: Vec<[u8; 32]>,
    pub revision: u64,
    pub head: [u8; 32],
}
type JournalRow = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
type BranchRow = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);

pub(super) fn verify(connection: &Connection, policy: &RegistrationPolicy) -> Result<State> {
    let meta = schema::verify(connection, policy)?;
    from_validated_meta(connection, policy, meta)
}

/// Private row replay; metadata comes only from an exact-profile schema validation.
pub(super) fn from_validated_meta(
    connection: &Connection,
    policy: &RegistrationPolicy,
    meta: schema::Meta,
) -> Result<State> {
    let mut state = State {
        requests: BTreeMap::new(),
        branches: BTreeMap::new(),
        heads: vec![[0; 32]],
        revision: 0,
        head: [0; 32],
    };
    let mut statement = connection.prepare("SELECT revision,request,previous,request_digest,head,body FROM registration_journal ORDER BY revision LIMIT 65")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if state.requests.len() == MAX_REGISTRATIONS {
            return Err(RegistrationError::Corrupt);
        }
        let (revision, request_id, previous, digest, head, body): JournalRow = (
            bounded_blob(row, 0, 8)?,
            bounded_blob(row, 1, 16)?,
            bounded_blob(row, 2, 32)?,
            bounded_blob(row, 3, 32)?,
            bounded_blob(row, 4, 32)?,
            bounded_blob(row, 5, 256)?,
        );
        let revision = read_counter(&revision)?;
        let next = state
            .revision
            .checked_add(1)
            .ok_or(RegistrationError::Corrupt)?;
        let request = codec::decode_request(&body)?;
        policy
            .permit(&request)
            .map_err(|_| RegistrationError::Corrupt)?;
        let canonical = codec::request_bytes(&request);
        let expected_digest = codec::request_digest(&request);
        let expected_head =
            codec::journal_head(&state.head, revision, &expected_digest, &canonical);
        if revision != next
            || request_id != request.request.as_bytes()
            || previous != state.head
            || digest != expected_digest
            || head != expected_head
        {
            return Err(RegistrationError::Corrupt);
        }
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
        if state
            .requests
            .insert(request.request, Entry { request, receipt })
            .is_some()
            || state
                .branches
                .insert((request.campaign, request.branch), receipt)
                .is_some()
        {
            return Err(RegistrationError::Corrupt);
        }
        state.revision = revision;
        state.head = expected_head;
        state.heads.push(expected_head);
    }
    if state.revision != meta.revision || state.head != meta.head {
        return Err(RegistrationError::Corrupt);
    }
    verify_branches(connection, policy, &state)?;
    Ok(state)
}

fn verify_branches(
    connection: &Connection,
    policy: &RegistrationPolicy,
    state: &State,
) -> Result<()> {
    let mut actual = BTreeMap::new();
    let mut statement = connection.prepare("SELECT campaign,branch,account,original_request,revision FROM registration_branches ORDER BY campaign,branch LIMIT 65")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if actual.len() == MAX_REGISTRATIONS {
            return Err(RegistrationError::Corrupt);
        }
        let (campaign, branch, account, request, revision): BranchRow = (
            bounded_blob(row, 0, 16)?,
            bounded_blob(row, 1, 16)?,
            bounded_blob(row, 2, 16)?,
            bounded_blob(row, 3, 16)?,
            bounded_blob(row, 4, 8)?,
        );
        let campaign = CampaignId::from_bytes(codec::fixed(&campaign)?);
        let branch = BranchId::from_bytes(codec::fixed(&branch)?);
        let receipt = RegisteredBranch {
            binding: CampaignBinding {
                scope: policy.scope,
                campaign,
                branch,
                account: AccountId::from_bytes(codec::fixed(&account)?),
            },
            original_request: RequestId::from_bytes(codec::fixed(&request)?),
            revision: read_counter(&revision)?,
        };
        if actual.insert((campaign, branch), receipt).is_some() {
            return Err(RegistrationError::Corrupt);
        }
    }
    if actual != state.branches {
        return Err(RegistrationError::Corrupt);
    }
    Ok(())
}

pub(super) fn check_known(
    state: &State,
    known: KnownRegistrationFrontier,
    membership_revision: u64,
) -> Result<()> {
    if state.revision < known.revision || membership_revision < known.minimum_membership_revision {
        return Err(RegistrationError::StaleBackup);
    }
    let prefix = usize::try_from(known.revision)
        .ok()
        .and_then(|index| state.heads.get(index))
        .ok_or(RegistrationError::StaleBackup)?;
    if prefix != &known.head {
        return Err(RegistrationError::Corrupt);
    }
    Ok(())
}
