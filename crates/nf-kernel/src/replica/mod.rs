//! Explicit scoped data owners; default APIs retain their existing behavior.
mod provider;
mod world_copy;
pub use world_copy::clone_world_with_scope;
mod admission;
pub use admission::admit_with_scope;
mod snapshot;
#[cfg(test)]
mod tests;
