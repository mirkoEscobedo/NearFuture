use super::{fields, reader::Reader, *};
use crate::PeerError;
use nf_contract::identity::*;
pub(super) fn read(r: &mut Reader<'_>) -> Result<ReceiptStatus, PeerError> {
    let request = RequestId::from_bytes(fields::nonzero(r.array()?)?);
    let account = AccountId::from_bytes(fields::nonzero(r.array()?)?);
    let device = DeviceId::from_bytes(fields::nonzero(r.array()?)?);
    let operation = OperationId::from_bytes(r.array()?);
    let binding = r.array()?;
    let phase = r.u8()?;
    let present = r.u8()?;
    let sequence = EventSeq(r.u64()?);
    let rejection = r.u8()?;
    fields::profile(r)?;
    let current = fields::current(r)?;
    let phase = match (phase, present, sequence.0, rejection) {
        (1, 0, 0, 0) if operation.as_bytes() == &[0; 16] && binding == [0; 32] => {
            ReceiptPhase::Unknown
        }
        (2, 0, 0, 0) => ReceiptPhase::Pending,
        (3, 1, 1.., 1) => ReceiptPhase::Rejected { sequence },
        (4, 1, 1.., 0) => ReceiptPhase::Committed { sequence },
        _ => return Err(PeerError::Malformed),
    };
    if phase != ReceiptPhase::Unknown {
        ReceiptTarget {
            request,
            operation,
            binding,
        }
        .validate()?
    }
    Ok(ReceiptStatus {
        request,
        account,
        device,
        operation,
        binding,
        phase,
        current,
    })
}
