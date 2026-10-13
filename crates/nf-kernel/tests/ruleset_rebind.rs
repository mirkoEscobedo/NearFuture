//! Lossless ruleset value changes preserve committed history; no activation grant.
use nf_contract::identity::{AuthorityTerm, EventSeq, JobId, RuntimeSession, WorldTick};
use nf_kernel::{AuthorityContext, JobResult, MissingPolicy, Settlement, World};
mod support;

// Independent composition fixtures: same schema/content/providers, config 2 -> 9.
// Exact 243-byte descriptor preimages and digests are retained in the source packet.
const OLD_COMPOSITION: [u8; 32] = [
    0x54, 0xa5, 0xd3, 0xe5, 0xe1, 0x98, 0x6c, 0xab, 0x46, 0x8f, 0x92, 0x4a, 0x37, 0x25, 0xc7, 0x8e,
    0x0a, 0x94, 0x45, 0xe0, 0x77, 0x61, 0x2a, 0x51, 0xb2, 0x23, 0x1d, 0xbe, 0x79, 0xbf, 0x6a, 0x60,
];
const NEW_COMPOSITION: [u8; 32] = [
    0xd2, 0x73, 0x45, 0x8f, 0x46, 0x4a, 0xfa, 0x5f, 0x03, 0x6d, 0x93, 0xb2, 0xdb, 0x8c, 0xf0, 0xee,
    0x57, 0x7b, 0x4c, 0xa0, 0xf4, 0xd4, 0x19, 0x9a, 0x3c, 0x4b, 0xe5, 0x08, 0xa9, 0x9f, 0x72, 0xd8,
];

#[test]
fn ruleset_value_rebind_preserves_a_real_committed_outcome_and_original_world() {
    let mut spec = support::fixture().to_spec();
    spec.ruleset_hash = OLD_COMPOSITION;
    for owner in &mut spec.ownership {
        owner.ruleset_hash = OLD_COMPOSITION;
    }
    let initial = World::new(spec).unwrap();
    let initial_before = initial.clone();
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(11),
    };
    let intent = support::market_intent(&initial, 1, 5);
    let frontier = nf_kernel::admit(&initial, vec![intent], authority).unwrap();
    let proposal = nf_kernel::evaluate(&frontier, JobId::from_bytes([1; 16])).unwrap();
    let Settlement::Committed { world, batch } = nf_kernel::settle(
        &initial,
        &frontier,
        vec![JobResult {
            job: proposal.job,
            result: Ok(proposal),
        }],
        authority,
        MissingPolicy::Pause,
    )
    .unwrap() else {
        panic!("the real market proposal must commit before testing the value rebind");
    };
    assert_eq!(
        world.view().market(support::entity(30)).unwrap().credits,
        105
    );
    assert_eq!(world.view().event_seq(), EventSeq(1));
    assert_eq!(world.to_spec().tick, WorldTick(1));
    assert_eq!(world.outcomes(), batch.outcomes.as_slice());
    assert_eq!(world.outcomes().len(), 1);
    assert_eq!(world.outcomes()[0].rejection, None);
    assert_eq!(initial, initial_before);

    let committed_before = world.clone();
    let prior_bytes = nf_kernel::encode_snapshot(&world).unwrap();
    let rebound = world.with_ruleset_hash(NEW_COMPOSITION).unwrap();
    let mut expected = world.to_spec();
    expected.ruleset_hash = NEW_COMPOSITION;
    for owner in &mut expected.ownership {
        owner.ruleset_hash = NEW_COMPOSITION;
    }
    // Full equality protects IDs, counters, revisions, registry and provider state.
    assert_eq!(rebound.to_spec(), expected);
    assert_eq!(rebound.outcomes(), world.outcomes());
    assert_ne!(
        nf_kernel::state_hash(&rebound).unwrap(),
        nf_kernel::state_hash(&world).unwrap()
    );
    assert_eq!(
        nf_kernel::decode_snapshot(&nf_kernel::encode_snapshot(&rebound).unwrap()).unwrap(),
        rebound
    );
    assert_eq!(world.with_ruleset_hash(OLD_COMPOSITION).unwrap(), *world);
    assert_eq!(world, committed_before);
    assert_eq!(nf_kernel::encode_snapshot(&world).unwrap(), prior_bytes);
}
