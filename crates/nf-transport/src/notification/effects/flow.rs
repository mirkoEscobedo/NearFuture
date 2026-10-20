#[cfg(test)]
use super::NotifyQueueStats;
use super::{NotifyEmission, quota::PermitTag};
use super::{NotifyPermit, NotifyQuota};
use crate::notification::{NotifyBody, NotifyRecord, PROTOCOL, encode_body};
use crate::{PeerError, notification::NotifyLimits};
use sha2::{Digest, Sha256};
use std::time::Instant;
struct Staged {
    digest: [u8; 32],
    permit: NotifyPermit,
}

struct Bucket {
    rate: u16,
    burst: u16,
    units: u64,
    sampled: Instant,
}
impl Bucket {
    fn new(limits: NotifyLimits) -> Self {
        Self {
            rate: limits.rate,
            burst: limits.burst,
            units: u64::from(limits.burst) * 1_000_000_000,
            sampled: Instant::now(),
        }
    }
    fn take(&mut self) -> Result<(), PeerError> {
        let now = Instant::now();
        let elapsed = now
            .checked_duration_since(self.sampled)
            .ok_or(PeerError::Session)?;
        let capacity = u64::from(self.burst)
            .checked_mul(1_000_000_000)
            .ok_or(PeerError::Limit)?;
        // Cap only after enough elapsed time to refill this selected burst at this selected rate.
        let seconds_to_fill = u64::from(self.burst).div_ceil(u64::from(self.rate));
        if elapsed.as_secs() >= seconds_to_fill {
            self.units = capacity;
        } else {
            let nanos = elapsed
                .as_secs()
                .checked_mul(1_000_000_000)
                .and_then(|n| n.checked_add(u64::from(elapsed.subsec_nanos())))
                .ok_or(PeerError::Limit)?;
            let refill = nanos
                .checked_mul(u64::from(self.rate))
                .ok_or(PeerError::Limit)?;
            self.units = self
                .units
                .checked_add(refill)
                .ok_or(PeerError::Limit)?
                .min(capacity);
        }
        self.sampled = now;
        self.units = self
            .units
            .checked_sub(1_000_000_000)
            .ok_or(PeerError::Backpressure)?;
        Ok(())
    }
}
/// Connection-owned accounting for every queued frame in each direction.
/// Kept across authentication and subscription; a wire record cannot reset it.
pub(crate) struct NotifyFlow {
    inbound: NotifyQuota,
    outbound: NotifyQuota,
    in_rate: Bucket,
    out_rate: Bucket,
    closed: bool,
    limits: NotifyLimits,
    staged: [Option<Staged>; 16],
    emitted: [Option<(PermitTag, u8)>; 16],
}
impl NotifyFlow {
    pub(crate) fn new(limits: NotifyLimits) -> Result<Self, PeerError> {
        limits.validate()?;
        Ok(Self {
            inbound: NotifyQuota::new(limits)?,
            outbound: NotifyQuota::new(limits)?,
            in_rate: Bucket::new(limits),
            out_rate: Bucket::new(limits),
            closed: false,
            limits,
            staged: std::array::from_fn(|_| None),
            emitted: [None; 16],
        })
    }
    pub(crate) fn reserve_outbound(&mut self, body: usize) -> Result<NotifyPermit, PeerError> {
        if self.closed {
            return Err(PeerError::Session);
        }
        self.out_rate
            .take()
            .and_then(|()| self.outbound.reserve(body))
    }
    pub(crate) fn admit_inbound(&mut self, body: usize) -> Result<NotifyPermit, PeerError> {
        if self.closed {
            return Err(PeerError::Session);
        }
        let result = self
            .in_rate
            .take()
            .and_then(|()| self.inbound.reserve(body));
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    #[cfg(test)]
    pub(crate) fn complete_outbound(&mut self, p: NotifyPermit) -> Result<(), PeerError> {
        self.outbound.complete(p)
    }
    pub(crate) fn complete_inbound(&mut self, p: NotifyPermit) -> Result<(), PeerError> {
        self.inbound.complete(p)
    }
    #[cfg(test)]
    pub(crate) fn outbound_stats(&self) -> NotifyQueueStats {
        self.outbound.stats()
    }
    #[cfg(test)]
    pub(crate) fn inbound_stats(&self) -> NotifyQueueStats {
        self.inbound.stats()
    }
    pub(crate) fn invalidate(&mut self) {
        self.closed = true;
        self.staged = std::array::from_fn(|_| None);
        self.emitted = [None; 16];
        self.inbound.invalidate();
        self.outbound.invalidate();
    }
}

impl NotifyFlow {
    pub(super) fn tighten(&mut self, limits: NotifyLimits) -> Result<(), PeerError> {
        if self.limits.negotiate(limits)? != limits {
            return Err(PeerError::Limit);
        }
        self.inbound.tighten(limits)?;
        self.outbound.tighten(limits)?;
        self.limits = limits;
        for b in [&mut self.in_rate, &mut self.out_rate] {
            b.rate = limits.rate;
            b.burst = limits.burst;
            b.units = b.units.min(u64::from(limits.burst) * 1_000_000_000);
        }
        Ok(())
    }
    pub(super) fn receive(&mut self, r: &NotifyRecord) -> Result<NotifyPermit, PeerError> {
        let n = encode_body(r, PROTOCOL, self.limits)?.len();
        self.admit_inbound(n)
    }
    pub(super) fn stage(&mut self, r: &NotifyRecord, p: NotifyPermit) -> Result<(), PeerError> {
        let bytes = encode_body(r, PROTOCOL, self.limits)?;
        if bytes.len() != usize::from(p.body()) || !self.outbound.owns(&p) {
            return Err(PeerError::Replay);
        }
        let slot = self
            .staged
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(PeerError::Backpressure)?;
        *slot = Some(Staged {
            digest: Sha256::digest(bytes).into(),
            permit: p,
        });
        Ok(())
    }
    pub(super) fn output(&mut self, r: NotifyRecord) -> Result<NotifyEmission, PeerError> {
        let digest: [u8; 32] = Sha256::digest(encode_body(&r, PROTOCOL, self.limits)?).into();
        let slot = self
            .staged
            .iter_mut()
            .find(|s| s.as_ref().is_some_and(|s| s.digest == digest))
            .ok_or(PeerError::Replay)?;
        let staged = slot.take().ok_or(PeerError::Replay)?;
        let emitted = self
            .emitted
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(PeerError::Backpressure)?;
        *emitted = Some((staged.permit.tag(), kind(&r.body)));
        Ok(NotifyEmission {
            record: r,
            permit: staged.permit,
        })
    }
    pub(super) fn finish_emission(&mut self, e: NotifyEmission) -> Result<u8, PeerError> {
        let (record, permit) = e.into_parts();
        let tag = permit.tag();
        let k = kind(&record.body);
        let slot = self
            .emitted
            .iter_mut()
            .find(|s| s.is_some_and(|(t, v)| t == tag && v == k))
            .ok_or(PeerError::Replay)?;
        self.outbound.complete(permit)?;
        *slot = None;
        Ok(k)
    }
}
fn kind(b: &NotifyBody) -> u8 {
    match b {
        NotifyBody::Hello { .. } => 1,
        NotifyBody::ServerHello { .. } => 2,
        NotifyBody::ClientProof(_) => 3,
        NotifyBody::Finished(_) => 4,
        NotifyBody::BeginSubscribe { .. } => 5,
        NotifyBody::SubscribeChallenge { .. } => 6,
        NotifyBody::ProveSubscribe { .. } => 7,
        NotifyBody::Subscribed { .. } => 8,
        NotifyBody::Notice { .. } => 9,
        NotifyBody::NoticeAck { .. } => 10,
    }
}
