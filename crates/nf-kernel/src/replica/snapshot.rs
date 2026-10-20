//! Scoped canonical snapshot owner used only by explicit admission.
use crate::{Rejection, World};
use alloc::vec::Vec;
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedResult};
use sha2::{Digest, Sha256};
mod entities;
mod ordering;
mod preflight;
mod registry;
mod remainder;
mod writer;
use writer::Writer;

pub(super) fn hash(
    world: &World,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<[u8; 32], Rejection> {
    // SHA256 state and digest are fixed stack values; retained digest is charged by Frontier.
    Ok(Sha256::digest(encode(world, scope)?).into())
}

pub(super) fn encode(
    world: &World,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<Vec<u8>, Rejection> {
    scope.nested(|record| {
        preflight::snapshot(world)?;
        let mut writer = Writer::header(record)?;
        remainder::fixed(world, &mut writer, record)?;
        entities::write(world, &mut writer, record)?;
        registry::write(world, &mut writer, record)?;
        remainder::collections(world, &mut writer, record)?;
        Ok(writer.finish())
    })
}
