use super::{
    NotifyBody, NotifyLimits, NotifyRecord, PROTOCOL,
    fields::{write_limits, write_selector},
    validation::validate,
};
use crate::PeerError;
pub fn encode_body(
    record: &NotifyRecord,
    actual_protocol: &str,
    limits: NotifyLimits,
) -> Result<Vec<u8>, PeerError> {
    if actual_protocol != PROTOCOL {
        return Err(PeerError::Unsupported);
    }
    let kind = validate(record, limits)?;
    let mut b = Vec::with_capacity(545);
    b.extend(b"NF-NOTIFY-1\0");
    b.extend(1u16.to_le_bytes());
    b.extend([kind, 3]);
    b.extend(record.context.session);
    b.extend(record.context.scope.universe.as_bytes());
    b.extend(record.context.scope.history.as_bytes());
    b.extend(record.context.ruleset);
    b.extend(record.context.content);
    match &record.body {
        NotifyBody::Hello {
            account,
            device,
            nonce,
            required,
            optional,
            offered,
        } => {
            b.extend(account.as_bytes());
            b.extend(device.as_bytes());
            b.extend(nonce);
            b.extend(required.to_le_bytes());
            b.extend(optional.to_le_bytes());
            write_limits(&mut b, *offered);
        }
        NotifyBody::ServerHello {
            nonce,
            available,
            selected_caps,
            server_limits,
            selected,
            proof,
        } => {
            b.extend(nonce);
            b.extend(available.to_le_bytes());
            b.extend(selected_caps.to_le_bytes());
            write_limits(&mut b, *server_limits);
            write_limits(&mut b, *selected);
            crate::records::fields::write_proof(&mut b, proof)?;
        }
        NotifyBody::ClientProof(p) | NotifyBody::Finished(p) => {
            crate::records::fields::write_proof(&mut b, p)?
        }
        NotifyBody::BeginSubscribe {
            subscription,
            selector,
            nonce,
            minimum_membership,
            lifetime,
        } => {
            b.extend(subscription);
            b.push(1);
            write_selector(&mut b, selector);
            b.extend(nonce);
            b.extend(minimum_membership.to_le_bytes());
            b.extend(lifetime.to_le_bytes());
        }
        NotifyBody::SubscribeChallenge {
            subscription,
            client_nonce,
            server_nonce,
            frontier,
            challenge,
        } => {
            b.extend(subscription);
            b.extend(client_nonce);
            b.extend(server_nonce);
            b.extend(frontier.to_le_bytes());
            b.extend(challenge);
        }
        NotifyBody::ProveSubscribe {
            subscription,
            nonce,
            proof,
        } => {
            b.extend(subscription);
            b.extend(nonce);
            crate::records::fields::write_proof(&mut b, proof)?;
        }
        NotifyBody::Subscribed {
            subscription,
            selector,
            first_sequence,
            lifetime,
            proof,
        } => {
            b.extend(subscription);
            b.push(1);
            write_selector(&mut b, selector);
            b.extend(first_sequence.to_le_bytes());
            b.extend(lifetime.to_le_bytes());
            crate::records::fields::write_proof(&mut b, proof)?;
        }
        NotifyBody::Notice {
            subscription,
            sequence,
            selector,
            nonce,
            proof,
        } => {
            b.extend(subscription);
            b.extend(sequence.to_le_bytes());
            write_selector(&mut b, selector);
            b.extend(nonce);
            crate::records::fields::write_proof(&mut b, proof)?;
        }
        NotifyBody::NoticeAck {
            subscription,
            sequence,
            notice_digest,
            proof,
        } => {
            b.extend(subscription);
            b.extend(sequence.to_le_bytes());
            b.extend(notice_digest);
            b.push(1);
            crate::records::fields::write_proof(&mut b, proof)?;
        }
    }
    if b.len() > usize::from(limits.frame) {
        return Err(PeerError::Limit);
    }
    Ok(b)
}
