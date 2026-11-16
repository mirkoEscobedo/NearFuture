//! Supplied canonical bytes only. No Store, commit, world advancement or authentication claim.
use crate::{Context, Error, MAX_PAYLOAD, Module};
pub(crate) struct Snapshot;
impl Module for Snapshot {
    fn invoke(&self, payload: &[u8], context: &Context<'_>) -> Result<Vec<u8>, Error> {
        context.check()?;
        if payload.len() > MAX_PAYLOAD {
            return Err(Error::Limit);
        }
        let world = nf_kernel::decode_snapshot(payload).map_err(|_| Error::InvalidSnapshot)?;
        context.check()?;
        let digest = nf_kernel::state_hash(&world).map_err(|_| Error::InvalidSnapshot)?;
        context.check()?;
        let mut reply = Vec::with_capacity(34);
        reply.extend_from_slice(&1u16.to_be_bytes());
        reply.extend_from_slice(&digest);
        Ok(reply)
    }
}
