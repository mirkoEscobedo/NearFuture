mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn canonical_frontier_reserves_only_first_legal_winner_despite_invalid_earlier_candidate() {
    let s = support::state();
    let first = support::colony(&s, 1, 2);
    let second = support::colony(&s, 1, 3);
    let mut invalid = support::colony(&s, 1, 1);
    invalid.actor = AccountId::from_bytes([99; 16]);
    let expected = [
        Some(WorldError::Unauthorized),
        None,
        Some(WorldError::Conflict),
    ];
    for items in [
        [invalid.clone(), first.clone(), second.clone()],
        [second.clone(), invalid.clone(), first.clone()],
        [first.clone(), second.clone(), invalid.clone()],
    ] {
        let plan = plan_reservations(&s, WorldTick(1), &items).expect("trusted frontier plan");
        assert_eq!(
            plan.outcomes()
                .iter()
                .map(|o| o.rejection)
                .collect::<Vec<_>>(),
            expected
        );
        let winner = plan.winner().unwrap();
        assert_eq!(winner.candidate().operation, first.operation);
        assert_eq!(
            (
                winner.reservation().credits(),
                winner.reservation().supplies()
            ),
            (100, 40)
        );
    }
}
