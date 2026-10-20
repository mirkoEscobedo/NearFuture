use super::{primitives::*, *};
use nf_contract::canonical::binding::RequestBinding;
use std::str::Split;
fn next<'a>(tokens: &mut Split<'a, char>) -> Result<&'a str, PeerError> {
    tokens.next().ok_or(PeerError::Malformed)
}
pub(super) fn parse(text: &str, config: &PortalConfig) -> Result<OriginalConfig, PeerError> {
    let mut tokens = text.split(' ');
    let anchored = config.mode == PortalMode::Watch;
    let key = if anchored { "anchor" } else { "original" };
    if next(&mut tokens)? != key {
        return Err(PeerError::Malformed);
    }
    let slot = number(next(&mut tokens)?, 7)? as u8;
    let head = if anchored {
        let generation = number(next(&mut tokens)?, 7)? as u8;
        let digest = hex(next(&mut tokens)?)?;
        if digest == [0; 32] {
            return Err(PeerError::Malformed);
        }
        Some((generation, digest))
    } else {
        None
    };
    let request_id = RequestId::from_bytes(id(next(&mut tokens)?)?);
    let operation = OperationId::from_bytes(id(next(&mut tokens)?)?);
    let kind = number(next(&mut tokens)?, 3)? as u8;
    let payload_digest = hex(next(&mut tokens)?)?;
    let minimum = SourceMinima {
        event: EventSeq(number(next(&mut tokens)?, u64::MAX)?),
        store_revision: number(next(&mut tokens)?, u64::MAX)?,
        membership_revision: number(next(&mut tokens)?, u64::MAX)?,
    };
    if tokens.next().is_some()
        || kind == 0
        || minimum.membership_revision < config.server.minimum_membership
    {
        return Err(PeerError::Malformed);
    }
    let original = OriginalReceipt::new(
        operation,
        RequestBinding {
            request_id,
            account_id: config.local_account,
            device_id: config.local_device,
            universe_id: config.scope().universe,
            history_id: config.scope().history,
            operation_kind: u32::from(kind),
            payload_digest,
        },
        config.server,
    )?;
    Ok(OriginalConfig {
        slot,
        original,
        minimum,
        head,
    })
}
