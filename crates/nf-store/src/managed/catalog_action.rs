use super::{bytes::{Encoder, Reader, hash, same}, catalog_payload::Payload,
    challenge::Owned, delta::Delta, error::{ManagedError, Result}};
#[derive(Clone)]
pub(super) struct Action { pub payload: Payload, pub delta: Delta }
impl Action {
    pub fn identity(&self) -> Result<(u64, [u8; 16])> {
        match &self.payload {
            Payload::Pending(s) | Payload::Attempt(s) | Payload::Outcome { summary: s, .. }
                | Payload::Abort { summary: s, .. } => Ok((s.sequence, s.operation)),
            Payload::Quiesce { transcript, .. } | Payload::Register { transcript, .. } => {
                let auth = Owned::decode(transcript)?; Ok((auth.sequence, auth.operation))
            },
            Payload::Owner { .. } | Payload::Select { .. } | Payload::Checkpoint { .. } => Ok((0, [0; 16])),
        }
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let delta = self.delta.encode()?; let payload = self.payload.encode(hash(&delta))?;
        let (sequence, operation) = self.identity()?;
        let mut w = Encoder::new(b"NF-OWNER-CATALOG-ACTION-1\0", 131_072)?;
        w.u32(self.payload.kind())?; w.u64(sequence)?; w.raw(&operation)?;
        w.blob(&payload, 131_072)?; w.blob(&delta, 131_072)?; Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-CATALOG-ACTION-1\0", 131_072)?;
        let kind = r.u32()?; let sequence = r.u64()?; let operation: [u8; 16] = r.fixed()?;
        let payload = r.blob(131_072)?.to_vec(); let delta = r.blob(131_072)?; r.finish()?;
        let value = Self { payload: Payload::decode(kind, &payload, hash(delta))?, delta: Delta::decode(delta)? };
        if value.identity()? != (sequence, operation) { return Err(ManagedError::Corrupt); }
        same(bytes, &value.encode()?)?; Ok(value)
    }
}
