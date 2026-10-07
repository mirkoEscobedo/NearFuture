mod miniature_support;
use nf_contract::identity::*;
use nf_kernel::{AuthorityContext, miniature::*};
#[test]
fn exhausted_event_sequence_cannot_produce_a_pending_resource_hold() {
    let initial = miniature_support::world();
    let mut metadata = initial.metadata();
    metadata.event_sequence = EventSeq(u64::MAX);
    let world = MiniatureWorld::new(metadata, initial.component().clone()).unwrap();
    let result = admit_miniature(
        &world,
        vec![miniature_support::intent(&world)],
        AuthorityContext {
            term: AuthorityTerm(7),
            session: RuntimeSession(9),
        },
        2,
    );
    assert_eq!(result, Err(MiniatureRejection::Overflow));
}
#[test]
fn frontier_caps_duplicates_and_stale_provider_are_checked_before_any_arbitrary_hold() {
    let world = miniature_support::world();
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(9),
    };
    let valid = miniature_support::intent(&world);
    assert_eq!(
        admit_miniature(&world, vec![valid.clone(); 65], authority, 2),
        Err(MiniatureRejection::Limit)
    );
    assert_eq!(
        admit_miniature(&world, vec![valid.clone(); 2], authority, 2),
        Err(MiniatureRejection::DuplicateIdentity)
    );
    let mut stale = valid.clone();
    stale.request = RequestId::from_bytes([27; 16]);
    stale.operation = OperationId::from_bytes([28; 16]);
    stale.job = JobId::from_bytes([29; 16]);
    stale
        .expected
        .insert(world.metadata().provider_aggregate, AggregateRevision(1));
    let frontier = admit_miniature(&world, vec![valid.clone(), stale], authority, 2).unwrap();
    assert_eq!(
        frontier.outcomes()[0].rejection,
        Some(MiniatureRejection::StaleRevision)
    );
    assert_eq!(frontier.reservation().unwrap().operation(), valid.operation);
    let mut exhausted = world.metadata();
    exhausted.tick = WorldTick(u64::MAX);
    exhausted.event_sequence = EventSeq(u64::MAX);
    exhausted.provider_revision = AggregateRevision(u64::MAX);
    let max = MiniatureWorld::new(exhausted, world.component().clone()).unwrap();
    assert_eq!(
        admit_miniature(&max, vec![], authority, 2),
        Err(MiniatureRejection::Overflow)
    );
    let mut unreachable = world.metadata();
    unreachable.tick = WorldTick(1);
    assert!(MiniatureWorld::new(unreachable, world.component().clone()).is_err());
}
#[test]
fn record_lengths_and_closed_versions_fail_before_exposing_typed_values() {
    let world = miniature_support::world();
    let fixtures = [
        miniature_support::hex(include_str!("fixtures/miniature/snapshot-genesis.hex")),
        miniature_support::hex(include_str!("fixtures/miniature/intent-colony.hex")),
        miniature_support::hex(include_str!("fixtures/miniature/frontier-colony.hex")),
        miniature_support::hex(include_str!("fixtures/miniature/batch-cancel.hex")),
    ];
    let decode = |index, bytes: &[u8]| -> bool {
        match index {
            0 => decode_miniature_snapshot(bytes).is_ok(),
            1 => decode_miniature_intent(bytes).is_ok(),
            2 => decode_miniature_frontier(bytes).is_ok(),
            _ => decode_miniature_batch(bytes, &world).is_ok(),
        }
    };
    for (index, original) in fixtures.iter().enumerate() {
        assert!(decode(index, original));
        for offset in 0..original.len() {
            assert!(!decode(index, &original[..offset]));
        }
        for offset in [11, 13, 15] {
            let mut raw = original.clone();
            raw[offset] = 255;
            assert!(!decode(index, &raw));
        }
        let mut trailing = original.clone();
        trailing.push(0);
        assert!(!decode(index, &trailing));
        assert!(!decode(index, &vec![0; MAX_MINIATURE_BYTES + 1]));
    }
    let mut snapshot = fixtures[0].clone();
    snapshot[185..189].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        decode_miniature_snapshot(&snapshot),
        Err(MiniatureRejection::Limit)
    );
    let mut frontier = fixtures[2].clone();
    frontier[81..85].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        decode_miniature_frontier(&frontier),
        Err(MiniatureRejection::Limit)
    );
    let mut batch = fixtures[3].clone();
    batch[122..126].copy_from_slice(&65u32.to_le_bytes());
    assert_eq!(
        decode_miniature_batch(&batch, &world),
        Err(MiniatureRejection::Limit)
    );
    let mut pseudo_random = 0x1234_5678u32;
    for _ in 0..2048 {
        pseudo_random = pseudo_random.wrapping_mul(1664525).wrapping_add(1013904223);
        let index = (pseudo_random % 4) as usize;
        let mut raw = fixtures[index].clone();
        let offset = (pseudo_random as usize >> 2) % raw.len();
        raw[offset] ^= ((pseudo_random >> 24) as u8) | 1;
        if decode(index, &raw) {
            // A mutation may describe a different canonical intent/public policy label;
            // any accepted record must still roundtrip exactly, and a batch must replay.
            let canonical = match index {
                0 => encode_miniature_snapshot(&decode_miniature_snapshot(&raw).unwrap()).unwrap(),
                1 => encode_miniature_intent(&decode_miniature_intent(&raw).unwrap()).unwrap(),
                2 => encode_miniature_frontier(&decode_miniature_frontier(&raw).unwrap()).unwrap(),
                _ => {
                    encode_miniature_batch(&decode_miniature_batch(&raw, &world).unwrap()).unwrap()
                }
            };
            assert_eq!(canonical, raw);
        }
    }
}
