use super::{
    book,
    bytes::{hash, next},
    challenge::{Binding, Challenge, Principal},
    engine::{Engine, ExecuteFailure, PrepareReservation},
    error::{ManagedError, Result},
    failure::SettlementFailure,
    input::{PrepareInput, SettlementInput},
    mutation::Mutation,
    observations::OwnerData,
    prepare::{Prepare, PrincipalEntry},
    receipt::Receipt,
    stamp::OwnerStamp,
    state::PendingCut,
};
use nf_contract::identity::RequestId;
use nf_identity::model::{DeviceProof, ProtectedOperation, PublicIdentity};
use nf_kernel::{Frontier, JobResult};

pub(super) fn principal(
    engine: &mut Engine,
    frontier: &Frontier,
    request: RequestId,
    public: &PublicIdentity,
    cutoff: u64,
) -> Result<Challenge> {
    engine.gate(false)?;
    if frontier.authority() != engine.authority()? || frontier.jobs().len() > 64 {
        return Err(ManagedError::StaleSession);
    }
    let intent = frontier
        .jobs()
        .iter()
        .find(|job| job.intent().request == request)
        .ok_or(ManagedError::OperationConflict)?
        .intent();
    let member = engine.member()?;
    let account = member
        .accounts
        .get(&public.account)
        .ok_or(ManagedError::Unauthorized)?;
    let device = member
        .devices
        .get(&public.device)
        .ok_or(ManagedError::Unauthorized)?;
    if intent.actor != public.account
        || account.key != public.account_key
        || device.account != public.account
        || device.key != public.device_key
        || device.peer != public.peer
        || device.revoked
        || public.peer.len() > 128
    {
        return Err(ManagedError::Unauthorized);
    }
    reserve_prepare(engine, frontier)?;
    let frontier_digest = hash(&nf_kernel::encode_frontier(frontier)?);
    let stamp = engine.owned_stamp()?.clone();
    let foot = engine.world()?.foot.clone();
    let reservation = engine
        .prepare_reservation
        .as_ref()
        .ok_or(ManagedError::Corrupt)?;
    let (sequence, operation) = (reservation.sequence, reservation.operation);
    let times = engine.book.times(cutoff, engine.catalog.state.utc)?;
    let principal = Principal {
        stamp,
        sequence,
        operation,
        revision: foot.known.store_revision,
        state: foot.state,
        head: foot.head,
        frontier: frontier_digest,
        request,
        actor: public.account,
        device: public.device,
        intent: nf_kernel::intent_digest(intent)?,
        times: times.window(),
    };
    engine.book.issue(
        Binding::Principal(principal.clone()),
        &times,
        principal.transcript()?,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}
fn reserve_prepare(engine: &mut Engine, frontier: &Frontier) -> Result<()> {
    engine.gate(false)?;
    if frontier.authority() != engine.authority()? || frontier.jobs().len() > 64 {
        return Err(ManagedError::StaleSession);
    }
    let frontier_digest = hash(&nf_kernel::encode_frontier(frontier)?);
    let stamp = engine.owned_stamp()?.clone();
    let foot = engine.world()?.foot.clone();
    if let Some(reservation) = &engine.prepare_reservation {
        if reservation.frontier != frontier_digest
            || reservation.stamp != stamp
            || reservation.state != foot.state
            || reservation.revision != foot.known.store_revision
            || reservation.head != foot.head
        {
            return Err(ManagedError::OperationConflict);
        }
    } else {
        engine.prepare_reservation = Some(PrepareReservation {
            sequence: next(engine.catalog.state.consumed)?,
            operation: book::random()?,
            frontier: frontier_digest,
            stamp: stamp.clone(),
            state: foot.state,
            revision: foot.known.store_revision,
            head: foot.head,
        });
    }
    Ok(())
}
fn prepare_payload(engine: &mut Engine, input: &PrepareInput<'_>) -> Result<Prepare> {
    engine.gate(false)?;
    let reservation = engine
        .prepare_reservation
        .as_ref()
        .ok_or(ManagedError::OperationConflict)?;
    let world = engine.world()?;
    if input.principals.len() > 64
        || input.devices.len() != input.principals.len()
        || reservation.frontier != hash(&nf_kernel::encode_frontier(input.frontier)?)
        || reservation.stamp != *engine.owned_stamp()?
        || reservation.state != world.foot.state
        || input.admitted_at_ms < engine.catalog.state.utc
        || input.admitted_at_ms > book::now_ms()?
        || input.cutoff_ms <= input.admitted_at_ms
    {
        return Err(ManagedError::OperationConflict);
    }
    let mut principals = Vec::new();
    for signed in input.principals {
        let value = engine.book.inspect_principal(signed)?;
        if value.sequence != reservation.sequence
            || value.operation != reservation.operation
            || value.stamp != reservation.stamp
            || value.frontier != reservation.frontier
            || value.state != reservation.state
            || value.revision != reservation.revision
            || value.head != reservation.head
            || input.admitted_at_ms < value.times.issued
            || input.admitted_at_ms >= value.times.expires
        {
            return Err(ManagedError::StaleSession);
        }
        principals.push(PrincipalEntry {
            request: value.request,
            window: value.times,
            proof: signed.proof.clone(),
        });
    }
    principals.sort_unstable_by_key(|p| p.request);
    let mut devices = input.devices.to_vec();
    devices.sort_unstable_by_key(|d| d.request);
    let mut reservations = input.reservations.to_vec();
    reservations.sort_unstable_by_key(|r| r.operation);
    let payload = Prepare {
        frontier: input.frontier.clone(),
        devices,
        reservations,
        principals,
        cut: PendingCut {
            policy: input.policy,
            admitted: input.admitted_at_ms,
            cutoff: input.cutoff_ms,
        },
    };
    payload.encode()?;
    Ok(payload)
}
pub(super) fn issue_prepare(
    engine: &mut Engine,
    input: &PrepareInput<'_>,
    public: &PublicIdentity,
    cutoff: u64,
) -> Result<Challenge> {
    if input.frontier.jobs().is_empty() {
        engine.gate(false)?;
        if !input.principals.is_empty()
            || !input.devices.is_empty()
            || !input.reservations.is_empty()
        {
            return Err(ManagedError::OperationConflict);
        }
        reserve_prepare(engine, input.frontier)?;
    }
    let payload = prepare_payload(engine, input)?.encode()?;
    let reservation = engine
        .prepare_reservation
        .as_ref()
        .ok_or(ManagedError::OperationConflict)?;
    engine.issue_owned(
        1,
        payload,
        public,
        cutoff,
        Some((reservation.sequence, reservation.operation)),
    )
}
pub(super) fn prepare(
    engine: &mut Engine,
    data: &mut OwnerData,
    input: PrepareInput<'_>,
    proof: DeviceProof,
) -> Result<Receipt> {
    let payload = prepare_payload(engine, &input)?;
    let bytes = payload.encode()?;
    let (owned, _, _) =
        engine.consume_owned(&proof, 1, Some(&bytes), ProtectedOperation::Economic)?;
    let member = engine.member()?.clone();
    for signed in input.principals {
        engine.book.consume_principal(signed, &member)?;
    }
    finish(engine, data, &Mutation::Prepare(payload), &owned, &proof).map_err(|e| match e {
        SettlementFailure::Refused(error) => error,
        SettlementFailure::Paused(_) => ManagedError::Corrupt,
    })
}
pub(super) fn issue_settlement(
    engine: &mut Engine,
    input: &SettlementInput,
    public: &PublicIdentity,
    cutoff: u64,
) -> Result<Challenge> {
    engine.issue_owned(
        2,
        Mutation::Settlement {
            results: input.results.clone(),
            missing: input.missing_policy,
        }
        .encode()?,
        public,
        cutoff,
        None,
    )
}
pub(super) fn settle(
    engine: &mut Engine,
    data: &mut OwnerData,
    input: SettlementInput,
    proof: DeviceProof,
) -> std::result::Result<Receipt, SettlementFailure> {
    execute(
        engine,
        data,
        Mutation::Settlement {
            results: input.results,
            missing: input.missing_policy,
        },
        proof,
    )
}
pub(super) fn execute(
    engine: &mut Engine,
    data: &mut OwnerData,
    mutation: Mutation,
    proof: DeviceProof,
) -> std::result::Result<Receipt, SettlementFailure> {
    let (owned, _, _) = engine.consume_owned(
        &proof,
        mutation.purpose(),
        Some(&mutation.encode()?),
        mutation.role(),
    )?;
    finish(engine, data, &mutation, &owned, &proof)
}
fn finish(
    engine: &mut Engine,
    data: &mut OwnerData,
    mutation: &Mutation,
    owned: &super::challenge::Owned,
    proof: &DeviceProof,
) -> std::result::Result<Receipt, SettlementFailure> {
    let result = engine.execute(mutation, owned, proof);
    if let Err(error) = data.refresh(engine) {
        engine.stop();
        return Err(error.into());
    }
    match result {
        Ok(receipt) => Ok(receipt),
        Err(ExecuteFailure::Paused(ids)) => Err(SettlementFailure::Paused(ids)),
        Err(ExecuteFailure::Refused(error)) => Err(SettlementFailure::Refused(error)),
    }
}
pub(super) fn worker_reply(
    engine: &mut Engine,
    stamp: &OwnerStamp,
    result: &JobResult,
) -> Result<()> {
    if engine.owned_stamp()? != stamp {
        return Err(ManagedError::StaleSession);
    }
    engine.gate(false)?;
    let frontier = engine
        .world()?
        .state
        .core
        .pending
        .as_ref()
        .ok_or(ManagedError::StaleSession)?;
    if frontier.authority() != engine.authority()?
        || !frontier.jobs().iter().any(|job| job.id() == result.job)
    {
        return Err(ManagedError::StaleSession);
    }
    Ok(())
}
