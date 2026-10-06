use crate::{
    error::{Result, StoreError},
    model::*,
};
use nf_contract::identity::*;
use nf_kernel::Rejection;
use std::collections::BTreeMap;
const PREFIX: &[u8] = b"NF-STORE-1\0\x01\0";
struct Writer {
    bytes: Vec<u8>,
}
impl Writer {
    fn raw(&mut self, bytes: &[u8]) -> Result<()> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > MAX_STATE_BYTES)
        {
            return Err(StoreError::Limit);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn u32(&mut self, value: u32) -> Result<()> {
        self.raw(&value.to_le_bytes())
    }
    fn u64(&mut self, value: u64) -> Result<()> {
        self.raw(&value.to_le_bytes())
    }
    fn byte(&mut self, value: u8) -> Result<()> {
        self.raw(&[value])
    }
    fn bytes(&mut self, value: &[u8]) -> Result<()> {
        self.u32(value.len().try_into().map_err(|_| StoreError::Limit)?)?;
        self.raw(value)
    }
    fn count(&mut self, value: usize) -> Result<()> {
        self.u32(value.try_into().map_err(|_| StoreError::Limit)?)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    entries: usize,
}
impl<'a> Reader<'a> {
    fn raw(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self.offset.checked_add(count).ok_or(StoreError::Corrupt)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(StoreError::Corrupt)?;
        self.offset = end;
        Ok(bytes)
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.raw(N)?.try_into().map_err(|_| StoreError::Corrupt)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.fixed()?))
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.fixed::<1>()?[0])
    }
    fn bytes(&mut self) -> Result<&'a [u8]> {
        let count = self.u32()? as usize;
        if count > MAX_STATE_BYTES {
            return Err(StoreError::Limit);
        }
        self.raw(count)
    }
    fn count(&mut self, max: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        self.entries = self.entries.checked_add(n).ok_or(StoreError::Limit)?;
        if n > max || self.entries > 8192 {
            return Err(StoreError::Limit);
        }
        Ok(n)
    }
}
pub(crate) fn rejection_code(rejection: Option<Rejection>) -> u32 {
    rejection.map_or(0, |value| value as u32 + 1)
}
fn rejection(code: u32) -> Result<Option<Rejection>> {
    use Rejection::*;
    let codes = [
        InvalidReference,
        DuplicateIdentity,
        Limit,
        InvalidValue,
        UnsupportedProvider,
        Unauthorized,
        StaleRevision,
        Conflict,
        Overflow,
        InvalidProposal,
        ProviderFailed,
        StaleSession,
        FencedAuthority,
        UnknownJob,
        DuplicateJob,
        InvalidFrontier,
        StateMismatch,
    ];
    if code == 0 {
        Ok(None)
    } else {
        codes
            .get(code as usize - 1)
            .copied()
            .map(Some)
            .ok_or(StoreError::Corrupt)
    }
}
pub(crate) fn encode(state: &State) -> Result<Vec<u8>> {
    state.validate()?;
    let mut w = Writer { bytes: Vec::new() };
    w.raw(PREFIX)?;
    w.bytes(&nf_kernel::encode_snapshot(&state.world)?)?;
    match &state.pending {
        None => w.byte(0)?,
        Some(frontier) => {
            w.byte(1)?;
            w.bytes(&nf_kernel::encode_frontier(frontier)?)?
        }
    }
    w.count(state.requests.len())?;
    for record in state.requests.values() {
        w.bytes(&nf_kernel::encode_intent(&record.intent)?)?;
        w.raw(record.device.as_bytes())?;
        match record.status {
            RequestStatus::Pending { .. } => w.byte(1)?,
            RequestStatus::Committed {
                sequence,
                rejection,
                ..
            } => {
                w.byte(2)?;
                w.u64(sequence.0)?;
                w.u32(rejection_code(rejection))?
            }
        }
    }
    w.count(state.reservations.len())?;
    for value in state.reservations.values() {
        w.raw(value.operation.as_bytes())?;
        w.raw(value.market.as_bytes())?;
        w.u64(value.amount)?;
    }
    w.count(state.outbox.len())?;
    for value in state.outbox.values() {
        w.raw(value.operation.as_bytes())?;
        w.u64(value.sequence.0)?;
        w.raw(&value.batch_digest)?;
        w.u32(rejection_code(value.rejection))?;
    }
    Ok(w.bytes)
}
pub(crate) fn decode(bytes: &[u8]) -> Result<State> {
    if bytes.len() > MAX_STATE_BYTES {
        return Err(StoreError::Limit);
    }
    let mut r = Reader {
        bytes,
        offset: 0,
        entries: 0,
    };
    if r.raw(PREFIX.len())? != PREFIX {
        return Err(StoreError::UnsupportedSchema);
    }
    let world = nf_kernel::decode_snapshot(r.bytes()?).map_err(|_| StoreError::Corrupt)?;
    let pending = match r.byte()? {
        0 => None,
        1 => Some(nf_kernel::decode_frontier(r.bytes()?).map_err(|_| StoreError::Corrupt)?),
        _ => return Err(StoreError::Corrupt),
    };
    let mut requests = BTreeMap::new();
    let mut previous = None;
    for _ in 0..r.count(MAX_REQUESTS)? {
        let intent = nf_kernel::decode_intent(r.bytes()?).map_err(|_| StoreError::Corrupt)?;
        if previous.is_some_and(|prior| prior >= intent.request) {
            return Err(StoreError::Corrupt);
        }
        previous = Some(intent.request);
        let device = DeviceId::from_bytes(r.fixed()?);
        let status = match r.byte()? {
            1 => RequestStatus::Pending {
                operation: intent.operation,
            },
            2 => RequestStatus::Committed {
                operation: intent.operation,
                sequence: EventSeq(r.u64()?),
                rejection: rejection(r.u32()?)?,
            },
            _ => return Err(StoreError::Corrupt),
        };
        requests.insert(
            intent.request,
            RequestRecord {
                intent,
                device,
                status,
            },
        );
    }
    let mut reservations = BTreeMap::new();
    let mut previous = None;
    for _ in 0..r.count(MAX_RESERVATIONS)? {
        let operation = OperationId::from_bytes(r.fixed()?);
        if previous.is_some_and(|prior| prior >= operation) {
            return Err(StoreError::Corrupt);
        }
        previous = Some(operation);
        reservations.insert(
            operation,
            Reservation {
                operation,
                market: EntityId::from_bytes(r.fixed()?),
                amount: r.u64()?,
            },
        );
    }
    let mut outbox = BTreeMap::new();
    let mut previous = None;
    for _ in 0..r.count(MAX_OUTBOX)? {
        let operation = OperationId::from_bytes(r.fixed()?);
        if previous.is_some_and(|prior| prior >= operation) {
            return Err(StoreError::Corrupt);
        }
        previous = Some(operation);
        outbox.insert(
            operation,
            OutboxRecord {
                operation,
                sequence: EventSeq(r.u64()?),
                batch_digest: r.fixed()?,
                rejection: rejection(r.u32()?)?,
            },
        );
    }
    if r.offset != bytes.len() {
        return Err(StoreError::Corrupt);
    }
    let state = State {
        world,
        pending,
        requests,
        reservations,
        outbox,
    };
    state.validate().map_err(|_| StoreError::Corrupt)?;
    if encode(&state)? != bytes {
        return Err(StoreError::Corrupt);
    }
    Ok(state)
}
