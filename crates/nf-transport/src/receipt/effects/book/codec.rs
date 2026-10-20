use super::{reader::Reader, *};
use crate::{
    PeerError,
    auth::ServerPin,
    receipt::{OriginalReceipt, ReceiptPhase, SourceMinima},
};
use nf_contract::{canonical::binding::RequestBinding, identity::*};
fn nonzero<const N: usize>(value: [u8; N]) -> Result<[u8; N], PeerError> {
    if value == [0; N] {
        Err(PeerError::Malformed)
    } else {
        Ok(value)
    }
}
pub fn decode_book(bytes: &[u8]) -> Result<BookRecord, PeerError> {
    if bytes.len() != BOOK_BYTES {
        return Err(PeerError::Limit);
    }
    let mut r = Reader { bytes, at: 0 };
    if &r.array::<18>()? != b"NF-RECEIPT-BOOK-1\0" {
        return Err(PeerError::Malformed);
    }
    if r.u16()? != 1 || r.u16()? != 1 {
        return Err(PeerError::Unsupported);
    }
    let generation = r.u8()?;
    let previous = r.array()?;
    let universe_id = UniverseId::from_bytes(nonzero(r.array()?)?);
    let history_id = HistoryId::from_bytes(nonzero(r.array()?)?);
    let ruleset = r.array()?;
    let content = r.array()?;
    let account_id = AccountId::from_bytes(nonzero(r.array()?)?);
    let device_id = DeviceId::from_bytes(nonzero(r.array()?)?);
    let account = AccountId::from_bytes(nonzero(r.array()?)?);
    let device = DeviceId::from_bytes(nonzero(r.array()?)?);
    let n = r.u8()? as usize;
    if !(1..=128).contains(&n) {
        return Err(PeerError::Limit);
    }
    let padded = r.array::<128>()?;
    if padded[n..].iter().any(|&b| b != 0) {
        return Err(PeerError::Malformed);
    }
    let peer = libp2p::PeerId::from_bytes(&padded[..n]).map_err(|_| PeerError::Malformed)?;
    if peer.to_bytes() != padded[..n] {
        return Err(PeerError::Malformed);
    }
    let request_id = RequestId::from_bytes(nonzero(r.array()?)?);
    let operation = OperationId::from_bytes(nonzero(r.array()?)?);
    let original = RequestBinding {
        request_id,
        account_id,
        device_id,
        universe_id,
        history_id,
        operation_kind: r.u32()?,
        payload_digest: r.array()?,
    };
    let binding = r.array::<32>()?;
    if original.digest() != binding {
        return Err(PeerError::Malformed);
    }
    let minimum = SourceMinima {
        event: EventSeq(r.u64()?),
        store_revision: r.u64()?,
        membership_revision: r.u64()?,
    };
    let phase = r.u8()?;
    let present = r.u8()?;
    let sequence = EventSeq(r.u64()?);
    let rejection = r.u8()?;
    let phase = match (phase, present, sequence.0, rejection) {
        (0, 0, 0, 0) => BookPhase::Unobserved,
        (1, 0, 0, 0) => BookPhase::Receipt(ReceiptPhase::Unknown),
        (2, 0, 0, 0) => BookPhase::Receipt(ReceiptPhase::Pending),
        (3, 1, 1.., 1) => BookPhase::Receipt(ReceiptPhase::Rejected { sequence }),
        (4, 1, 1.., 0) => BookPhase::Receipt(ReceiptPhase::Committed { sequence }),
        _ => return Err(PeerError::Malformed),
    };
    let record = BookRecord {
        generation,
        previous,
        original: OriginalReceipt::new(
            operation,
            original,
            ServerPin {
                peer,
                account,
                device,
                minimum_membership: 0,
            },
        )?,
        ruleset,
        content,
        minimum,
        phase,
    };
    record.validate()?;
    Ok(record)
}
pub fn encode_book(record: &BookRecord) -> Result<[u8; BOOK_BYTES], PeerError> {
    record.validate()?;
    let mut b = Vec::with_capacity(BOOK_BYTES);
    b.extend(b"NF-RECEIPT-BOOK-1\0");
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.push(record.generation);
    b.extend(record.previous);
    let o = record.original.original();
    let p = record.original.source();
    b.extend(o.universe_id.as_bytes());
    b.extend(o.history_id.as_bytes());
    b.extend(record.ruleset);
    b.extend(record.content);
    b.extend(o.account_id.as_bytes());
    b.extend(o.device_id.as_bytes());
    b.extend(p.account.as_bytes());
    b.extend(p.device.as_bytes());
    crate::records::fields::write_peer(&mut b, &p.peer.to_bytes())?;
    b.extend(o.request_id.as_bytes());
    b.extend(record.original.operation().as_bytes());
    b.extend(o.operation_kind.to_le_bytes());
    b.extend(o.payload_digest);
    b.extend(o.digest());
    b.extend(record.minimum.event.0.to_le_bytes());
    b.extend(record.minimum.store_revision.to_le_bytes());
    b.extend(record.minimum.membership_revision.to_le_bytes());
    let (phase, present, seq, rejection) = match record.phase {
        BookPhase::Unobserved => (0, 0, 0, 0),
        BookPhase::Receipt(ReceiptPhase::Unknown) => (1, 0, 0, 0),
        BookPhase::Receipt(ReceiptPhase::Pending) => (2, 0, 0, 0),
        BookPhase::Receipt(ReceiptPhase::Rejected { sequence }) => (3, 1, sequence.0, 1),
        BookPhase::Receipt(ReceiptPhase::Committed { sequence }) => (4, 1, sequence.0, 0),
    };
    b.push(phase);
    b.push(present);
    b.extend(seq.to_le_bytes());
    b.push(rejection);
    b.try_into().map_err(|_| PeerError::Limit)
}
