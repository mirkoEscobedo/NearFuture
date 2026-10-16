use super::{
    bytes::{Encoder, Reader, same},
    error::{ManagedError, Result},
};
use nf_contract::identity::{AccountId, DeviceId, JobId, OperationId, RequestId};

/// Durable administrative cause, separate from the underlying kernel projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdministrativeCancellation {
    pub(super) request: RequestId,
    pub(super) operation: OperationId,
    pub(super) job: JobId,
    pub(super) account: AccountId,
    pub(super) device: DeviceId,
    pub(super) term: u64,
    pub(super) session: u64,
    pub(super) sequence: u64,
    pub(super) revision: u64,
    pub(super) policy: u8,
    pub(super) admitted: u64,
    pub(super) cutoff: u64,
    pub(super) requested: u64,
}
impl AdministrativeCancellation {
    pub fn request(&self) -> RequestId {
        self.request
    }
    pub fn operation(&self) -> OperationId {
        self.operation
    }
    pub fn job(&self) -> JobId {
        self.job
    }
    pub fn requested_by_account(&self) -> AccountId {
        self.account
    }
    pub fn requested_by_device(&self) -> DeviceId {
        self.device
    }
    pub fn authority_term(&self) -> u64 {
        self.term
    }
    pub fn runtime_session(&self) -> u64 {
        self.session
    }
    pub fn catalog_operation_sequence(&self) -> u64 {
        self.sequence
    }
    pub fn store_revision(&self) -> u64 {
        self.revision
    }
    pub fn original_admitted_at_ms(&self) -> u64 {
        self.admitted
    }
    pub fn original_cutoff_ms(&self) -> u64 {
        self.cutoff
    }
    pub fn requested_at_ms(&self) -> u64 {
        self.requested
    }
    pub fn classification(&self) -> AdministrativeCancellationKind {
        AdministrativeCancellationKind::AdministrativeCancelled
    }
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(138);
        out.extend_from_slice(self.request.as_bytes());
        out.extend_from_slice(self.operation.as_bytes());
        out.extend_from_slice(self.job.as_bytes());
        out.push(1);
        out.extend_from_slice(self.account.as_bytes());
        out.extend_from_slice(self.device.as_bytes());
        for n in [self.term, self.session, self.sequence, self.revision] {
            out.extend_from_slice(&n.to_be_bytes());
        }
        out.push(self.policy);
        for n in [self.admitted, self.cutoff, self.requested] {
            out.extend_from_slice(&n.to_be_bytes());
        }
        out
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::plain(bytes, 138)?;
        let request = RequestId::from_bytes(r.fixed()?);
        let operation = OperationId::from_bytes(r.fixed()?);
        let job = JobId::from_bytes(r.fixed()?);
        if r.byte()? != 1 {
            return Err(ManagedError::Corrupt);
        }
        let value = Self {
            request,
            operation,
            job,
            account: AccountId::from_bytes(r.fixed()?),
            device: DeviceId::from_bytes(r.fixed()?),
            term: r.u64()?,
            session: r.u64()?,
            sequence: r.u64()?,
            revision: r.u64()?,
            policy: r.byte()?,
            admitted: r.u64()?,
            cutoff: r.u64()?,
            requested: r.u64()?,
        };
        r.finish()?;
        if !matches!(value.policy, 1 | 2)
            || value.cutoff <= value.admitted
            || value.term == 0
            || value.session == 0
            || value.sequence == 0
            || value.revision == 0
            || value.requested < value.admitted
        {
            return Err(ManagedError::Corrupt);
        }
        same(bytes, &value.canonical_bytes())?;
        Ok(value)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdministrativeCancellationKind {
    AdministrativeCancelled,
}

pub(super) fn encode_map<'a>(
    records: impl ExactSizeIterator<Item = &'a AdministrativeCancellation>,
) -> Result<Vec<u8>> {
    let count = records.len();
    if count > 4096 {
        return Err(ManagedError::Limit);
    }
    let mut w = Encoder::plain(4 + 4096 * 138);
    w.u32(count.try_into().map_err(|_| ManagedError::Limit)?)?;
    for record in records {
        w.raw(&record.canonical_bytes())?;
    }
    Ok(w.finish())
}
