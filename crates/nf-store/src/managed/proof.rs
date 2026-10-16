use super::{
    bytes::{Encoder, Reader, same},
    error::{ManagedError, Result},
};
use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::{DeviceProof, Scope};

pub(super) fn encode(proof: &DeviceProof) -> Result<Vec<u8>> {
    let mut w = Encoder::plain(300);
    if proof.peer.len() > 128 {
        return Err(ManagedError::Limit);
    }
    w.raw(proof.scope.universe.as_bytes())?;
    w.raw(proof.scope.history.as_bytes())?;
    w.raw(proof.account.as_bytes())?;
    w.raw(proof.device.as_bytes())?;
    w.u64(proof.frontier)?;
    w.blob(&proof.peer, 128)?;
    w.raw(&proof.challenge)?;
    w.raw(&proof.signature)?;
    Ok(w.finish())
}
pub(super) fn decode(bytes: &[u8]) -> Result<DeviceProof> {
    let mut r = Reader::plain(bytes, 300)?;
    let value = DeviceProof {
        scope: Scope {
            universe: UniverseId::from_bytes(r.fixed()?),
            history: HistoryId::from_bytes(r.fixed()?),
        },
        account: AccountId::from_bytes(r.fixed()?),
        device: DeviceId::from_bytes(r.fixed()?),
        frontier: r.u64()?,
        peer: r.blob(128)?.to_vec(),
        challenge: r.fixed()?,
        signature: r.fixed()?,
    };
    r.finish()?;
    same(bytes, &encode(&value)?)?;
    Ok(value)
}
