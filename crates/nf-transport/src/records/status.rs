use super::reader::Reader;
use crate::PeerError;
use nf_contract::identity::{EventSeq, OperationId};
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetainedPhase {
    UnknownRequest,
    Pending {
        operation: OperationId,
        binding: [u8; 32],
    },
    Rejected {
        operation: OperationId,
        binding: [u8; 32],
        sequence: EventSeq,
    },
}
pub(super) fn read(r: &mut Reader<'_>) -> Result<RetainedPhase, PeerError> {
    let operation = OperationId::from_bytes(r.array()?);
    let binding = r.array()?;
    let phase = r.u8()?;
    let presence = r.u8()?;
    let sequence = r.u64()?;
    let rejection = r.u8()?;
    match (phase, presence, sequence, rejection) {
        (1, 0, 0, 0) if operation.as_bytes() == &[0; 16] && binding == [0; 32] => {
            Ok(RetainedPhase::UnknownRequest)
        }
        (2, 0, 0, 0) => Ok(RetainedPhase::Pending { operation, binding }),
        (3, 1, s, 1) if s > 0 => Ok(RetainedPhase::Rejected {
            operation,
            binding,
            sequence: EventSeq(s),
        }),
        _ => Err(PeerError::Malformed),
    }
}
pub(super) fn write(b: &mut Vec<u8>, phase: &RetainedPhase) {
    let (operation, binding, code, present, sequence, rejection) = match phase {
        RetainedPhase::UnknownRequest => (OperationId::from_bytes([0; 16]), [0; 32], 1, 0, 0, 0),
        RetainedPhase::Pending { operation, binding } => (*operation, *binding, 2, 0, 0, 0),
        RetainedPhase::Rejected {
            operation,
            binding,
            sequence,
        } => (*operation, *binding, 3, 1, sequence.0, 1),
    };
    b.extend(operation.as_bytes());
    b.extend(binding);
    b.extend([code, present]);
    b.extend(sequence.to_le_bytes());
    b.push(rejection);
}
