#[path = "miniature_support/crash_data.rs"]
mod crash_data;
mod miniature_support;
#[path = "miniature_support/process.rs"]
mod process_support;
#[path = "miniature_support/vault.rs"]
mod vault_support;
use nf_contract::identity::*;
use nf_identity::model::MembershipRepository;
use nf_kernel::miniature::*;
use nf_store::{Boundary, KnownFrontiers, miniature::*};
use std::{path::PathBuf, process::Command, time::Duration};
#[test]
fn miniature_crash_worker() {
    let Some(path) = std::env::var_os("NF_MINIATURE_CRASH_DB") else {
        return;
    };
    let phase = std::env::var("NF_MINIATURE_CRASH_PHASE").unwrap();
    let target = std::env::var("NF_MINIATURE_CRASH_BOUNDARY").unwrap();
    let marker = PathBuf::from(std::env::var_os("NF_MINIATURE_CRASH_READY").unwrap());
    let known = MiniatureKnownFrontiers {
        storage: KnownFrontiers::genesis(
            UniverseId::from_bytes([1; 16]),
            HistoryId::from_bytes([2; 16]),
        ),
        minimum_authority_term: AuthorityTerm(0),
    };
    let mut store = MiniatureStore::open_existing(&path, known, AuthConfig::default()).unwrap();
    let vault = PathBuf::from(std::env::var_os("NF_MINIATURE_CRASH_VAULT").unwrap());
    let saves = PathBuf::from(std::env::var_os("NF_MINIATURE_CRASH_SAVES").unwrap());
    let mut signer = vault_support::load_signer(&mut store, &vault, &saves);
    if phase != "claim" {
        vault_support::fresh_claim(&mut store, &mut signer);
    }
    if phase == "advance" {
        vault_support::fresh_resume(&mut store, &mut signer);
        vault_support::activity(&mut store, &mut signer);
    }
    crash_data::write_before(&store, &marker);
    let mut hook = |point: Boundary| {
        if format!("{point:?}") == target {
            process_support::block_at(&marker);
        }
        Ok(())
    };
    match phase.as_str() {
        "claim" => {
            let g = store.world().component().genesis();
            let member = store
                .load_membership(nf_identity::model::Scope {
                    universe: g.universe,
                    history: g.history,
                })
                .unwrap()
                .unwrap();
            let device = *member
                .devices
                .iter()
                .find(|(_, d)| d.account == g.accounts[0] && !d.revoked)
                .unwrap()
                .0;
            let proof = vault_support::owner_attempt(
                &mut store,
                &mut signer,
                ChallengeRequest::Claim {
                    actor: g.accounts[0],
                    device,
                },
            );
            store.claim_authority_with_hook(proof, &mut hook).unwrap();
        }
        "prepare" => {
            let a = store.authority().unwrap();
            let public = store.current_identity(a.account, a.device).unwrap();
            let i = miniature_support::intent(store.world(), &public, 40);
            let proof = vault_support::owner_attempt(
                &mut store,
                &mut signer,
                ChallengeRequest::Prepare(&i),
            );
            store
                .prepare_with_hook(vec![i], vec![proof], &mut hook)
                .unwrap();
        }
        "resume" => {
            let ids: Vec<_> = store
                .pending()
                .unwrap()
                .intents()
                .iter()
                .map(|i| i.request)
                .collect();
            let proofs = ids
                .into_iter()
                .map(|id| {
                    vault_support::owner_attempt(
                        &mut store,
                        &mut signer,
                        ChallengeRequest::Resume(id),
                    )
                })
                .collect();
            store.resume_pending_with_hook(proofs, &mut hook).unwrap();
        }
        "advance" => {
            let batch = settle_miniature(
                store.world(),
                store.pending().unwrap(),
                store.authority().unwrap().context(),
            )
            .unwrap();
            let proof =
                vault_support::owner_attempt(&mut store, &mut signer, ChallengeRequest::Commit);
            store.commit_with_hook(&batch, proof, &mut hook).unwrap();
        }
        "cancel" => {
            let proof =
                vault_support::owner_attempt(&mut store, &mut signer, ChallengeRequest::Cancel);
            store
                .cancel_pending_with_hook(MiniatureCancelCause::Cancelled, proof, &mut hook)
                .unwrap();
        }
        _ => panic!("closed owned crash phase"),
    }
    std::fs::write(marker.with_extension("ack"), b"acknowledged").unwrap();
}
#[test]
fn killed_owned_processes_cover_all25_mutation_boundaries_with_no_premature_ack() {
    let mut f = vault_support::VaultFixture::new();
    for phase in ["claim", "prepare", "resume", "advance", "cancel"] {
        for point in [
            "BeforeTransaction",
            "AfterWrites",
            "BeforeCommit",
            "AfterCommit",
            "BeforeAcknowledgement",
        ] {
            let db = f.scratch.0.join(format!("{phase}-{point}.sqlite"));
            let mut store = f.create(&db);
            if matches!(phase, "resume" | "advance" | "cancel") {
                vault_support::fresh_claim(&mut store, &mut f.signer);
                let i = miniature_support::intent(store.world(), &f.policy.owner, 40);
                let proof = vault_support::owner_attempt(
                    &mut store,
                    &mut f.signer,
                    ChallengeRequest::Prepare(&i),
                );
                store.prepare(vec![i], vec![proof]).unwrap();
                let mut next = f.membership.clone();
                next.revision += 1;
                next.consumed.insert([99; 16]);
                store
                    .commit_membership(Some(f.membership.revision), &next)
                    .unwrap();
            }
            drop(store);
            let marker = db.with_extension("ready");
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "miniature_crash_worker",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("NF_MINIATURE_CRASH_DB", &db)
                .env("NF_MINIATURE_CRASH_PHASE", phase)
                .env("NF_MINIATURE_CRASH_BOUNDARY", point)
                .env("NF_MINIATURE_CRASH_READY", &marker)
                .env("NF_MINIATURE_CRASH_VAULT", &f.vault)
                .env("NF_MINIATURE_CRASH_SAVES", &f.saves);
            let mut child = process_support::OwnedChild::spawn(&mut command);
            child.await_marker(&marker, Duration::from_secs(30));
            child.kill_and_reap();
            assert!(
                !marker.with_extension("ack").exists(),
                "no public acknowledgement before {phase}/{point}"
            );
            let before = crash_data::known(&marker);
            let store = MiniatureStore::open_existing(&db, before, f.policy.auth).unwrap();
            let committed = matches!(point, "AfterCommit" | "BeforeAcknowledgement");
            if !committed {
                assert_eq!(
                    store.snapshot_bytes().unwrap(),
                    process_support::read_bounded(
                        &marker.with_extension("before.envelope"),
                        1_048_576
                    )
                    .unwrap(),
                    "rollback {phase}/{point}"
                );
                assert_eq!(store.known_frontiers().unwrap(), before);
            } else {
                assert_eq!(
                    store.known_frontiers().unwrap().storage.store_revision,
                    before.storage.store_revision + 1
                );
                let world = decode_miniature_snapshot(
                    &process_support::read_bounded(
                        &marker.with_extension("before.world"),
                        1_048_576,
                    )
                    .unwrap(),
                )
                .unwrap();
                let a = crash_data::authority(&marker);
                match phase {
                    "claim" => {
                        let current = store.authority().unwrap();
                        assert_eq!(current.term.0, before.minimum_authority_term.0 + 1);
                        assert_ne!(current.session.0, 0);
                        assert_ne!(Some(current.session), a.map(|a| a.session));
                        assert_eq!(store.world(), &world);
                    }
                    "prepare" => {
                        let expected = admit_miniature(
                            &world,
                            vec![miniature_support::intent(&world, &f.policy.owner, 40)],
                            a.unwrap().context(),
                            before.storage.membership_revision.unwrap(),
                        )
                        .unwrap();
                        assert_eq!(store.pending(), Some(&expected));
                        assert_eq!(store.world(), &world);
                    }
                    "resume" => {
                        let old = decode_miniature_frontier(
                            &process_support::read_bounded(
                                &marker.with_extension("before.pending"),
                                1_048_576,
                            )
                            .unwrap(),
                        )
                        .unwrap();
                        let expected = admit_miniature(
                            &world,
                            old.intents().to_vec(),
                            a.unwrap().context(),
                            before.storage.membership_revision.unwrap(),
                        )
                        .unwrap();
                        assert_eq!(store.pending(), Some(&expected));
                        assert_eq!(store.world(), &world);
                    }
                    "advance" | "cancel" => {
                        let old = decode_miniature_frontier(
                            &process_support::read_bounded(
                                &marker.with_extension("before.pending"),
                                1_048_576,
                            )
                            .unwrap(),
                        )
                        .unwrap();
                        let a = a.unwrap();
                        let batch = if phase == "advance" {
                            settle_miniature(&world, &old, a.context()).unwrap()
                        } else {
                            cancel_miniature(
                                &world,
                                &old,
                                a.context(),
                                before.storage.membership_revision.unwrap(),
                                MiniatureRejection::Cancelled,
                            )
                            .unwrap()
                        };
                        assert_eq!(store.world(), &replay_miniature(&world, &batch).unwrap());
                        assert!(store.pending().is_none());
                        assert!(matches!(
                            store
                                .query_bound(
                                    RequestId::from_bytes([40; 16]),
                                    f.policy.owner.account,
                                    f.policy.owner.device,
                                    f.policy.scope
                                )
                                .unwrap()
                                .unwrap()
                                .status,
                            MiniatureRequestStatus::Committed { .. }
                        ));
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
