// The shared synthetic fixture assumes the std prelude; this no_std unit scope supplies only its Vec macro.
macro_rules! vec { ($($t:tt)*) => { alloc::vec![$($t)*] } }
#[path = "../../../nf-nex-boundary/tests/support/mod.rs"]
mod boundary_fixture;
use super::{
    budget::{Sink, Writer},
    profile,
};
use alloc::{string::String, vec::Vec};
#[derive(Default)]
struct Bytes(Vec<u8>);
impl Sink for Bytes {
    fn write(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }
}
#[test]
fn streaming_byte_order_matches_independent_synthetic_profile_bytes() {
    let world = nf_nex_boundary::NexWorld::admit(boundary_fixture::snapshot()).unwrap();
    let mut w = Writer::new(Bytes::default());
    profile::profile(&mut w, &world, true).unwrap();
    let hex: String = w.sink.0.iter().map(|b| alloc::format!("{b:02x}")).collect();
    let row = include_str!("../../fixtures/world-v1.tsv")
        .lines()
        .find(|l| l.starts_with("baseline|"))
        .unwrap();
    assert_eq!(hex, row.split('|').nth(1).unwrap());
}
