use super::{error::Result, model::*, state::State};
use crate::StoreError;
use nf_contract::identity::*;
use nf_kernel::miniature::*;
use std::collections::BTreeMap;
const HEADER: &[u8] = b"NF-STORE-2\0\x02\0";
pub(crate) fn encode(state: &State) -> Result<Vec<u8>> {
    state.validate()?;
    let mut w = Writer(HEADER.to_vec());
    w.raw(&MINIATURE_IMPLEMENTATION_HASH)?;
    w.bytes(&encode_miniature_snapshot(&state.world)?)?;
    w.byte(u8::from(state.authority.is_some()))?;
    if let Some(a) = state.authority {
        for v in [a.term.0, a.session.0] {
            w.u64(v)?;
        }
        w.raw(a.account.as_bytes())?;
        w.raw(a.device.as_bytes())?;
        w.u64(a.membership_revision)?;
    }
    w.byte(u8::from(state.pending.is_some()))?;
    if let Some(f) = &state.pending {
        w.bytes(&encode_miniature_frontier(f)?)?;
    }
    w.count(state.requests.len())?;
    for (id, r) in &state.requests {
        w.raw(id.as_bytes())?;
        w.bytes(&encode_miniature_intent(&r.intent)?)?;
        match r.status {
            MiniatureRequestStatus::Pending { .. } => w.byte(1)?,
            MiniatureRequestStatus::Committed {
                sequence,
                rejection,
                ..
            } => {
                w.byte(2)?;
                w.u64(sequence.0)?;
                w.byte(code(rejection))?;
            }
        }
    }
    let hold = state.pending.as_ref().and_then(|f| f.reservation());
    w.count(usize::from(hold.is_some()))?;
    if let Some(h) = hold {
        w.raw(h.operation().as_bytes())?;
        w.raw(h.faction().as_bytes())?;
        w.u64(h.credits())?;
        w.u64(h.supplies())?;
    }
    w.count(state.outbox.len())?;
    for (id, o) in &state.outbox {
        w.raw(id.as_bytes())?;
        w.u64(o.sequence.0)?;
        w.raw(&o.batch_digest)?;
        w.byte(code(o.rejection))?;
    }
    Ok(w.0)
}
pub(crate) fn decode(bytes: &[u8]) -> Result<State> {
    if bytes.len() > 1_048_576 {
        return Err(StoreError::Limit.into());
    }
    let mut r = Reader { bytes, pos: 0 };
    if r.take(HEADER.len())? != HEADER || r.take(32)? != MINIATURE_IMPLEMENTATION_HASH {
        return Err(StoreError::UnsupportedSchema.into());
    }
    let world = decode_miniature_snapshot(r.bytes()?)?;
    let authority = if r.presence()? {
        Some(MiniatureAuthority {
            term: AuthorityTerm(r.u64()?),
            session: RuntimeSession(r.u64()?),
            account: AccountId::from_bytes(r.array()?),
            device: DeviceId::from_bytes(r.array()?),
            membership_revision: r.u64()?,
        })
    } else {
        None
    };
    let pending = if r.presence()? {
        Some(decode_miniature_frontier(r.bytes()?)?)
    } else {
        None
    };
    let mut requests = BTreeMap::new();
    let mut last = None;
    for _ in 0..r.count(4096)? {
        let id = RequestId::from_bytes(r.array()?);
        if last.is_some_and(|old| old >= id) {
            return Err(StoreError::Corrupt.into());
        }
        last = Some(id);
        let intent = decode_miniature_intent(r.bytes()?)?;
        let status = match r.byte()? {
            1 => MiniatureRequestStatus::Pending {
                operation: intent.operation,
            },
            2 => MiniatureRequestStatus::Committed {
                operation: intent.operation,
                sequence: EventSeq(r.u64()?),
                rejection: rejection(r.byte()?)?,
            },
            _ => return Err(StoreError::Corrupt.into()),
        };
        let binding_digest = miniature_request_binding(&intent)?.digest();
        requests.insert(
            id,
            MiniatureBoundRequest {
                intent,
                status,
                binding_digest,
            },
        );
    }
    let hold = if r.count(1)? == 1 {
        Some((
            OperationId::from_bytes(r.array()?),
            EntityId::from_bytes(r.array()?),
            r.u64()?,
            r.u64()?,
        ))
    } else {
        None
    };
    if hold
        != pending
            .as_ref()
            .and_then(|f| f.reservation())
            .map(|h| (h.operation(), h.faction(), h.credits(), h.supplies()))
    {
        return Err(StoreError::Corrupt.into());
    }
    let mut outbox = BTreeMap::new();
    let mut last = None;
    for _ in 0..r.count(256)? {
        let id = OperationId::from_bytes(r.array()?);
        if last.is_some_and(|old| old >= id) {
            return Err(StoreError::Corrupt.into());
        }
        last = Some(id);
        outbox.insert(
            id,
            MiniatureOutbox {
                operation: id,
                sequence: EventSeq(r.u64()?),
                batch_digest: r.array()?,
                rejection: rejection(r.byte()?)?,
            },
        );
    }
    if r.pos != bytes.len() {
        return Err(StoreError::Corrupt.into());
    }
    let state = State {
        world,
        authority,
        pending,
        requests,
        outbox,
    };
    state.validate()?;
    if encode(&state)? != bytes {
        return Err(StoreError::Corrupt.into());
    }
    Ok(state)
}
pub(crate) fn code(value: Option<MiniatureRejection>) -> u8 {
    value.map_or(0, |v| v as u8)
}
fn rejection(value: u8) -> Result<Option<MiniatureRejection>> {
    use MiniatureRejection::*;
    Ok(Some(match value {
        0 => return Ok(None),
        1 => InvalidReference,
        2 => DuplicateIdentity,
        3 => Limit,
        4 => InvalidValue,
        5 => UnsupportedProvider,
        6 => Unauthorized,
        7 => StaleRevision,
        8 => Conflict,
        9 => Overflow,
        10 => InvalidProposal,
        11 => ProviderFailed,
        12 => StaleSession,
        13 => FencedAuthority,
        14 => UnknownJob,
        15 => DuplicateJob,
        16 => InvalidFrontier,
        17 => StateMismatch,
        18 => Resources,
        19 => Locked,
        20 => AdmissionChanged,
        21 => Cancelled,
        _ => return Err(StoreError::Corrupt.into()),
    }))
}
struct Writer(Vec<u8>);
impl Writer {
    fn raw(&mut self, b: &[u8]) -> Result<()> {
        if self
            .0
            .len()
            .checked_add(b.len())
            .is_none_or(|n| n > 1_048_576)
        {
            return Err(StoreError::Limit.into());
        }
        self.0.extend_from_slice(b);
        Ok(())
    }
    fn byte(&mut self, v: u8) -> Result<()> {
        self.raw(&[v])
    }
    fn u64(&mut self, v: u64) -> Result<()> {
        self.raw(&v.to_le_bytes())
    }
    fn count(&mut self, n: usize) -> Result<()> {
        self.raw(
            &u32::try_from(n)
                .map_err(|_| StoreError::Limit)?
                .to_le_bytes(),
        )
    }
    fn bytes(&mut self, b: &[u8]) -> Result<()> {
        self.count(b.len())?;
        self.raw(b)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(StoreError::Corrupt)?;
        let b = self.bytes.get(self.pos..end).ok_or(StoreError::Corrupt)?;
        self.pos = end;
        Ok(b)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into().map_err(|_| StoreError::Corrupt)?)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn count(&mut self, max: usize) -> Result<usize> {
        let n = u32::from_le_bytes(self.array()?) as usize;
        if n > max {
            return Err(StoreError::Limit.into());
        }
        Ok(n)
    }
    fn bytes(&mut self) -> Result<&'a [u8]> {
        let n = self.count(1_048_576)?;
        self.take(n)
    }
    fn presence(&mut self) -> Result<bool> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(StoreError::Corrupt.into()),
        }
    }
}
