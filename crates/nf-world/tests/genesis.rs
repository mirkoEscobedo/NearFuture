use nf_contract::identity::*;
use nf_world::*;
#[test]
fn miniature_genesis_has_exact_topology_ownership_resources_and_repeatable_ids() {
    let g = Genesis {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
        seed: [3; 32],
        accounts: [
            AccountId::from_bytes([4; 16]),
            AccountId::from_bytes([5; 16]),
            AccountId::from_bytes([6; 16]),
        ],
    };
    let s = generate(g).expect("supported miniature genesis");
    assert_eq!(
        (
            s.systems().len(),
            s.factions().len(),
            s.markets().len(),
            s.fleets().len()
        ),
        (3, 3, 6, 3)
    );
    assert_eq!(s.markets().iter().filter(|m| m.owner.is_some()).count(), 3);
    assert!(s.factions().iter().all(|f| (
        f.credits,
        f.supplies,
        f.spent_credits,
        f.spent_supplies
    ) == (200, 100, 0, 0)));
    assert_eq!(generate(g).unwrap(), s);
    let mut changed = g;
    changed.seed[0] ^= 1;
    assert_ne!(generate(changed).unwrap().systems(), s.systems());
}
