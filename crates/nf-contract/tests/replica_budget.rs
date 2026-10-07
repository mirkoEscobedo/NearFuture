use nf_contract::canonical::replica_budget::*;

#[test]
fn limits_are_lower_only_with_root_depth_and_zero_work_allowed() {
    assert_eq!(
        ReplicaDecodeLimits::new(0, 0, 0),
        Err(ReplicaBudgetError::Depth)
    );
    assert_eq!(
        ReplicaDecodeLimits::new(33, 0, 0),
        Err(ReplicaBudgetError::Depth)
    );
    assert_eq!(
        ReplicaDecodeLimits::new(1, 32769, 0),
        Err(ReplicaBudgetError::Entries)
    );
    assert_eq!(
        ReplicaDecodeLimits::new(1, 0, 4194305),
        Err(ReplicaBudgetError::CopiedBytes)
    );
    let zero = ReplicaDecodeLimits::new(1, 0, 0).unwrap();
    assert_eq!(
        zero.with_scope::<_, (), _>(|s| {
            s.charge_entries(0).map_err(ScopedError::Budget)?;
            s.charge_copied(0).map_err(ScopedError::Budget)?;
            Ok(s.usage())
        })
        .unwrap(),
        ReplicaUsage {
            entries: 0,
            copied_bytes: 0,
            depth: 1
        }
    );
}

#[test]
fn an_actual_bounded_consumer_refuses_before_the_second_copy() {
    let mut copied = Vec::new();
    let result = ReplicaDecodeLimits::new(2, 2, 3)
        .unwrap()
        .with_scope::<_, (), _>(|s| {
            for field in [b"ab".as_slice(), b"cd".as_slice()] {
                s.charge_entries(1).map_err(ScopedError::Budget)?;
                s.charge_copied(field.len() as u64)
                    .map_err(ScopedError::Budget)?;
                copied.push(field.to_vec());
            }
            Ok(())
        });
    assert_eq!(
        result,
        Err(ScopedError::Budget(ReplicaBudgetError::CopiedBytes))
    );
    assert_eq!(copied, vec![b"ab".to_vec()]);
}

#[test]
fn nested_consumers_share_counters_and_enforce_root_depth() {
    let result = ReplicaDecodeLimits::new(2, 1, 10)
        .unwrap()
        .with_scope::<_, (), _>(|s| {
            s.charge_entries(1).map_err(ScopedError::Budget)?;
            s.nested(|child| {
                assert_eq!(child.usage().depth, 2);
                child.charge_entries(1).map_err(ScopedError::Budget)
            })
        });
    assert_eq!(
        result,
        Err(ScopedError::Budget(ReplicaBudgetError::Entries))
    );
    let result = ReplicaDecodeLimits::new(1, 1, 1)
        .unwrap()
        .with_scope::<_, (), _>(|s| s.nested(|_| Ok(())));
    assert_eq!(result, Err(ScopedError::Budget(ReplicaBudgetError::Depth)));
}

#[test]
fn first_failure_is_sticky_even_when_a_consumer_swallows_it() {
    let result = ReplicaDecodeLimits::new(2, 0, 0)
        .unwrap()
        .with_scope::<_, (), _>(|s| {
            assert_eq!(s.charge_entries(1), Err(ReplicaBudgetError::Entries));
            assert_eq!(s.charge_copied(0), Err(ReplicaBudgetError::Entries));
            assert_eq!(
                s.nested::<(), (), _>(|_| Ok(())),
                Err(ScopedError::Budget(ReplicaBudgetError::Entries))
            );
            assert_eq!(s.failure(), Some(ReplicaBudgetError::Entries));
            Ok(())
        });
    assert_eq!(
        result,
        Err(ScopedError::Budget(ReplicaBudgetError::Entries))
    );
}

#[test]
fn ordinary_semantic_rejection_does_not_poison_or_refund_the_scope() {
    let result = ReplicaDecodeLimits::new(2, 2, 8)
        .unwrap()
        .with_scope::<_, &str, _>(|s| {
            let rejected = s.nested::<(), _, _>(|child| {
                child.charge_copied(2).map_err(ScopedError::Budget)?;
                Err(ScopedError::Semantic("ordinary rejected intent"))
            });
            assert_eq!(
                rejected,
                Err(ScopedError::Semantic("ordinary rejected intent"))
            );
            assert_eq!(s.failure(), None);
            assert_eq!(
                s.usage(),
                ReplicaUsage {
                    entries: 0,
                    copied_bytes: 2,
                    depth: 1
                }
            );
            s.nested(|child| {
                child.charge_copied(1).map_err(ScopedError::Budget)?;
                Ok(())
            })?;
            Ok(s.usage())
        })
        .unwrap();
    assert_eq!(result.copied_bytes, 3);
}
