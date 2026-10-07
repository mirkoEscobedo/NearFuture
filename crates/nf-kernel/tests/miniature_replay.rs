mod miniature_support;
use nf_contract::identity::*;
use nf_kernel::{AuthorityContext, miniature::*};
#[test]
fn empty_tick_advances_only_one_clock_and_provider_revision_then_replays_exactly() {
    let world = miniature_support::world();
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(9),
    };
    let frontier = admit_miniature(&world, vec![], authority, 2).unwrap();
    let batch = settle_miniature(&world, &frontier, authority).expect("supported advancing batch");
    assert_eq!(batch.disposition(), MiniatureDisposition::AdvanceTick);
    assert_eq!(
        (batch.tick(), batch.event_sequence()),
        (WorldTick(1), EventSeq(1))
    );
    let next = replay_miniature(&world, &batch).unwrap();
    assert_eq!(next.metadata().provider_revision, AggregateRevision(1));
    assert_eq!(next.component(), world.component());
    assert_eq!(next.component().revision(), AggregateRevision(0));
    assert_eq!(batch.after_hash(), miniature_state_hash(&next).unwrap());
    assert!(replay_miniature(&next, &batch).is_err());
    assert_eq!(
        settle_miniature(
            &world,
            &frontier,
            AuthorityContext {
                term: AuthorityTerm(8),
                ..authority
            }
        ),
        Err(MiniatureRejection::FencedAuthority)
    );
    assert_eq!(
        settle_miniature(
            &world,
            &frontier,
            AuthorityContext {
                session: RuntimeSession(10),
                ..authority
            }
        ),
        Err(MiniatureRejection::StaleSession)
    );
}
#[test]
fn fresh_policy_cancellation_is_tick_neutral_and_preserves_original_binding_without_economic_effect()
 {
    let world = miniature_support::world();
    let intent = miniature_support::intent(&world);
    let old = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(9),
    };
    let current = AuthorityContext {
        term: AuthorityTerm(8),
        session: RuntimeSession(10),
    };
    let frontier = admit_miniature(&world, vec![intent.clone()], old, 2).unwrap();
    assert_eq!(frontier.reservation().unwrap().credits(), 100);
    let batch = cancel_miniature(
        &world,
        &frontier,
        current,
        3,
        MiniatureRejection::AdmissionChanged,
    )
    .expect("supported cancellation under fresh policy");
    assert_eq!(batch.disposition(), MiniatureDisposition::CancelPending);
    assert_eq!(
        (batch.tick(), batch.event_sequence()),
        (WorldTick(0), EventSeq(1))
    );
    assert_eq!(batch.membership_revision(), 3);
    assert_eq!(batch.intents(), core::slice::from_ref(&intent));
    assert!(batch.action_event().is_none());
    assert!(batch.due_events().is_empty());
    let next = replay_miniature(&world, &batch).unwrap();
    assert_eq!(next.component(), world.component());
    assert_eq!(next.metadata().provider_revision, AggregateRevision(0));
    assert_eq!(
        next.outcomes()[0].rejection,
        Some(MiniatureRejection::AdmissionChanged)
    );
    assert_eq!(
        miniature_request_binding(&batch.intents()[0]).unwrap(),
        miniature_request_binding(&intent).unwrap()
    );
    assert!(
        cancel_miniature(&world, &frontier, current, 3, MiniatureRejection::Resources).is_err()
    );
}
