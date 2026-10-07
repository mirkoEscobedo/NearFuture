#[path = "support/community.rs"]
mod community;
#[path = "support/query_pair.rs"]
mod query_pair;
#[path = "support/retained_store.rs"]
mod retained_store;
#[path = "support/scratch.rs"]
mod scratch;
use nf_contract::identity::RequestId;
use nf_identity::model::Roles;
use nf_transport::{
    query::QueryResult,
    records::{RetainedPhase, UnsupportedReason},
};
use std::time::Instant;
#[test]
fn exact_retained_pending_rejected_and_success_unsupported_never_change_durable_world() {
    for (delta, settle, kind) in [(1, false, 0), (-1000, true, 1), (1, true, 2)] {
        let c = community::Community::new(Roles::PLAYER);
        let tmp = scratch::Scratch::new();
        let store = retained_store::create(&c, &tmp.0.join("server.sqlite"), delta, settle);
        let before = store.revision();
        let known = store.known_frontiers().unwrap();
        drop(store);
        let store = nf_store::Store::open_existing(tmp.0.join("server.sqlite"), known).unwrap();
        let (mut client, mut server, cp, sp, id) = query_pair::with_server(&c, &tmp.0, store);
        for _ in 0..2 {
            let now = Instant::now();
            let begin = client.begin(RequestId::from_bytes([9; 16]), now).unwrap();
            let challenge = server.begin(begin, cp, id, now).unwrap();
            let proof = client
                .challenge(challenge, sp, id, &c.client_device, now)
                .unwrap();
            let reply = server.prove(proof, cp, id, &c.server_device, now).unwrap();
            let result = client.reply(reply, sp, id, now).unwrap();
            match (kind, result) {
                (0, QueryResult::Status(RetainedPhase::Pending { operation, binding })) => {
                    assert_eq!(operation.as_bytes(), &[9; 16]);
                    assert_ne!(binding, [0; 32]);
                }
                (
                    1,
                    QueryResult::Status(RetainedPhase::Rejected {
                        operation,
                        sequence,
                        ..
                    }),
                ) => {
                    assert_eq!(operation.as_bytes(), &[9; 16]);
                    assert!(sequence.0 > 0);
                }
                (2, QueryResult::Unsupported(UnsupportedReason::KernelOutcome)) => {}
                _ => panic!("exact retained status"),
            };
            assert_eq!(server.owner_store_mut().revision(), before);
            assert_eq!(server.owner_store_mut().known_frontiers().unwrap(), known);
        }
    }
}
