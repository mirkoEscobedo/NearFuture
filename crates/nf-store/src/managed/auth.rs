use super::{
    bytes::{Encoder, Reader, hash, same},
    challenge::{Owned, Principal, Times},
    error::{ManagedError, Result},
    proof,
    stamp::OwnerStamp,
};
use nf_identity::model::DeviceProof;

impl Owned {
    pub fn transcript(&self) -> Result<Vec<u8>> {
        if !matches!(self.purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8 | 11 | 12 | 15) {
            return Err(ManagedError::OperationConflict);
        }
        let mut w = Encoder::new(b"NF-OWNER-AUTH-1\0", 4096)?;
        w.raw(self.stamp.canonical_bytes())?;
        w.u32(self.purpose)?;
        w.u64(self.sequence)?;
        w.raw(&self.operation)?;
        w.u64(self.revision)?;
        w.raw(&self.state)?;
        w.raw(&self.head)?;
        w.raw(&self.payload)?;
        w.u64(self.times.auth_sequence)?;
        w.raw(&self.times.nonce)?;
        w.u64(self.times.issued)?;
        w.u64(self.times.expires)?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-AUTH-1\0", 4096)?;
        let value = Self {
            stamp: OwnerStamp::read(&mut r)?,
            purpose: r.u32()?,
            sequence: r.u64()?,
            operation: r.fixed()?,
            revision: r.u64()?,
            state: r.fixed()?,
            head: r.fixed()?,
            payload: r.fixed()?,
            times: Times {
                auth_sequence: r.u64()?,
                nonce: r.fixed()?,
                issued: r.u64()?,
                expires: r.u64()?,
            },
        };
        r.finish()?;
        validate_times(&value.times)?;
        same(bytes, &value.transcript()?)?;
        Ok(value)
    }
}
fn validate_times(times: &Times) -> Result<()> {
    if times.auth_sequence == 0
        || times.expires <= times.issued
        || times.expires - times.issued > 20_000
    {
        return Err(ManagedError::Corrupt);
    }
    Ok(())
}
impl Principal {
    pub fn transcript(&self) -> Result<Vec<u8>> {
        if self.times.expires <= self.times.issued
            || self.times.expires - self.times.issued > 20_000
        {
            return Err(ManagedError::Corrupt);
        }
        let mut w = Encoder::new(b"NF-OWNER-PRINCIPAL-1\0", 4096)?;
        w.raw(self.stamp.canonical_bytes())?;
        w.u64(self.sequence)?;
        w.raw(&self.operation)?;
        w.u64(self.revision)?;
        w.raw(&self.state)?;
        w.raw(&self.head)?;
        w.raw(&self.frontier)?;
        w.raw(self.request.as_bytes())?;
        w.raw(self.device.as_bytes())?;
        w.raw(&self.intent)?;
        w.raw(&self.times.nonce)?;
        w.u64(self.times.issued)?;
        w.u64(self.times.expires)?;
        Ok(w.finish())
    }
}
pub(super) fn action(auth: &Owned, payload: &[u8], proof: &DeviceProof) -> Result<Vec<u8>> {
    if !matches!(auth.purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8)
        || hash(payload) != auth.payload
        || proof.challenge != hash(&auth.transcript()?)
        || proof.account != auth.stamp.account
        || proof.device != auth.stamp.device
        || proof.frontier
            != auth
                .stamp
                .membership_revision
                .ok_or(ManagedError::Corrupt)?
    {
        return Err(ManagedError::Authentication);
    }
    let mut w = Encoder::new(b"NF-ACTIVE-ACTION-1\0", 1_114_112)?;
    w.u32(auth.purpose)?;
    w.raw(auth.stamp.canonical_bytes())?;
    w.u64(auth.sequence)?;
    w.raw(&auth.operation)?;
    w.u64(auth.revision)?;
    w.raw(&auth.state)?;
    w.raw(&auth.head)?;
    w.u64(auth.times.auth_sequence)?;
    w.raw(&auth.times.nonce)?;
    w.u64(auth.times.issued)?;
    w.u64(auth.times.expires)?;
    w.blob(&proof::encode(proof)?, 300)?;
    w.blob(payload, 1_114_112)?;
    Ok(w.finish())
}
pub(super) fn decode_action(bytes: &[u8]) -> Result<(Owned, Vec<u8>, DeviceProof)> {
    let mut r = Reader::new(bytes, b"NF-ACTIVE-ACTION-1\0", 1_114_112)?;
    let purpose = r.u32()?;
    let stamp = OwnerStamp::read(&mut r)?;
    let sequence = r.u64()?;
    let operation = r.fixed()?;
    let revision = r.u64()?;
    let state = r.fixed()?;
    let head = r.fixed()?;
    let times = Times {
        auth_sequence: r.u64()?,
        nonce: r.fixed()?,
        issued: r.u64()?,
        expires: r.u64()?,
    };
    validate_times(&times)?;
    let proof = proof::decode(r.blob(300)?)?;
    let payload = r.blob(1_114_112)?.to_vec();
    r.finish()?;
    let owned = Owned {
        stamp,
        purpose,
        sequence,
        operation,
        revision,
        state,
        head,
        payload: hash(&payload),
        times,
    };
    same(bytes, &action(&owned, &payload, &proof)?)?;
    Ok((owned, payload, proof))
}
