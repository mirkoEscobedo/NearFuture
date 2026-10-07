use super::{NotifyBody, NotifyContext, NotifyLimits, NotifyRecord, NotifySelector};
use crate::PeerError;
use nf_identity::model::DeviceProof;
pub(super) fn nonzero<const N: usize>(v: &[u8; N]) -> Result<(), PeerError> {
    if v == &[0; N] {
        return Err(PeerError::Malformed);
    }
    Ok(())
}
pub(super) fn selector(value: &NotifySelector) -> Result<(), PeerError> {
    nonzero(value.request.as_bytes())?;
    nonzero(value.operation.as_bytes())
}
pub(super) fn context(value: NotifyContext, hello: bool) -> Result<(), PeerError> {
    nonzero(value.scope.universe.as_bytes())?;
    nonzero(value.scope.history.as_bytes())?;
    if (value.session == [0; 16]) != hello {
        return Err(PeerError::Session);
    }
    Ok(())
}
fn proof(value: &DeviceProof, context: NotifyContext) -> Result<(), PeerError> {
    if value.scope != context.scope {
        return Err(PeerError::Scope);
    }
    nonzero(value.account.as_bytes())?;
    nonzero(value.device.as_bytes())?;
    if value.peer.is_empty() || value.peer.len() > 128 {
        return Err(PeerError::Limit);
    }
    let peer = libp2p::PeerId::from_bytes(&value.peer).map_err(|_| PeerError::Malformed)?;
    if peer.to_bytes() != value.peer {
        return Err(PeerError::Malformed);
    }
    Ok(())
}
fn lifetime(value: u16) -> Result<(), PeerError> {
    if !(1..=30).contains(&value) {
        return Err(PeerError::Limit);
    }
    Ok(())
}
pub(super) fn validate(record: &NotifyRecord, limits: NotifyLimits) -> Result<u8, PeerError> {
    limits.validate()?;
    context(
        record.context,
        matches!(record.body, NotifyBody::Hello { .. }),
    )?;
    Ok(match &record.body {
        NotifyBody::Hello {
            account,
            device,
            nonce,
            required,
            optional,
            offered,
        } => {
            nonzero(account.as_bytes())?;
            nonzero(device.as_bytes())?;
            nonzero(nonce)?;
            offered.validate()?;
            if *required != 1 || *optional != 0 {
                return Err(PeerError::Unsupported);
            }
            1
        }
        NotifyBody::ServerHello {
            nonce,
            available,
            selected_caps,
            server_limits,
            selected,
            proof: p,
        } => {
            nonzero(nonce)?;
            server_limits.validate()?;
            selected.validate()?;
            proof(p, record.context)?;
            if *available != 1 || *selected_caps != 1 {
                return Err(PeerError::Unsupported);
            }
            if selected.negotiate(*server_limits)? != *selected {
                return Err(PeerError::Limit);
            }
            2
        }
        NotifyBody::ClientProof(p) => {
            proof(p, record.context)?;
            3
        }
        NotifyBody::Finished(p) => {
            proof(p, record.context)?;
            4
        }
        NotifyBody::BeginSubscribe {
            subscription,
            selector: s,
            nonce,
            lifetime: l,
            ..
        } => {
            nonzero(subscription)?;
            selector(s)?;
            nonzero(nonce)?;
            lifetime(*l)?;
            5
        }
        NotifyBody::SubscribeChallenge {
            subscription,
            client_nonce,
            server_nonce,
            ..
        } => {
            nonzero(subscription)?;
            nonzero(client_nonce)?;
            nonzero(server_nonce)?;
            6
        }
        NotifyBody::ProveSubscribe {
            subscription,
            nonce,
            proof: p,
        } => {
            nonzero(subscription)?;
            nonzero(nonce)?;
            proof(p, record.context)?;
            7
        }
        NotifyBody::Subscribed {
            subscription,
            selector: s,
            first_sequence,
            lifetime: l,
            proof: p,
        } => {
            nonzero(subscription)?;
            selector(s)?;
            lifetime(*l)?;
            proof(p, record.context)?;
            if *first_sequence != 1 {
                return Err(PeerError::Malformed);
            }
            8
        }
        NotifyBody::Notice {
            subscription,
            sequence,
            selector: s,
            nonce,
            proof: p,
        } => {
            nonzero(subscription)?;
            selector(s)?;
            nonzero(nonce)?;
            proof(p, record.context)?;
            if *sequence == 0 {
                return Err(PeerError::Malformed);
            }
            9
        }
        NotifyBody::NoticeAck {
            subscription,
            sequence,
            proof: p,
            ..
        } => {
            nonzero(subscription)?;
            proof(p, record.context)?;
            if *sequence == 0 {
                return Err(PeerError::Malformed);
            }
            10
        }
    })
}
