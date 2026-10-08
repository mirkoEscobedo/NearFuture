use nf_nex_shadow::{MakePeaceEligibility, make_peace_action_eligible};

#[test]
fn make_peace_without_target_is_eligible_when_diplomacy_and_concern_allow_it() {
    let facts = MakePeaceEligibility {
        diplomacy_enabled: true,
        concern_can_make_peace: true,
        target_hostile: None,
        faction_diplomacy_disabled: false,
    };
    let original = facts;

    assert_eq!(make_peace_action_eligible(&facts), Ok(true));
    assert_eq!(facts, original);
}
