mod budget;
mod profile;
mod records;
use crate::Unavailable;
use nf_nex_boundary::NexWorld;
use sha2::Digest;
/// Equality of declared admitted immutable values only; never native capture or operation authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldCommitment {
    digest: [u8; 32],
}
impl WorldCommitment {
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}
/// Fully preflights borrowed fields before sorting bounded trait references or hashing any byte.
pub fn commit_world(world: &NexWorld) -> Result<WorldCommitment, Unavailable> {
    let mut count = budget::Writer::new(budget::Count);
    profile::profile(&mut count, world, false)?;
    let mut hash = budget::Writer::new(budget::Hash(sha2::Sha256::new()));
    profile::profile(&mut hash, world, true)?;
    Ok(WorldCommitment {
        digest: hash.sink.0.finalize().into(),
    })
}
#[cfg(test)]
mod tests;
