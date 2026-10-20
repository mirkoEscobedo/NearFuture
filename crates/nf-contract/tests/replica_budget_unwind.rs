use nf_contract::canonical::replica_budget::*;

#[test]
fn charged_copy_overflow_is_fatal_and_does_not_wrap() {
    let result = ReplicaDecodeLimits::new(2, 2, 8)
        .unwrap()
        .with_scope::<_, (), _>(|s| {
            s.charge_copied(1).map_err(ScopedError::Budget)?;
            s.charge_copied(u64::MAX).map_err(ScopedError::Budget)?;
            Ok(())
        });
    assert_eq!(
        result,
        Err(ScopedError::Budget(ReplicaBudgetError::Overflow))
    );
}

#[test]
fn caught_unwind_restores_lexical_depth_without_refunding_copy_work() {
    let usage = ReplicaDecodeLimits::new(2, 2, 8)
        .unwrap()
        .with_scope::<_, (), _>(|s| {
            let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _: ScopedResult<(), ()> = s.nested(|child| {
                    assert_eq!(child.usage().depth, 2);
                    child.charge_copied(1).map_err(ScopedError::Budget)?;
                    panic!("owned consumer unwind");
                });
            }));
            assert!(unwind.is_err());
            assert_eq!(s.usage().depth, 1);
            s.nested(|child| {
                assert_eq!(child.usage().depth, 2);
                child.charge_copied(1).map_err(ScopedError::Budget)?;
                Ok(())
            })?;
            Ok(s.usage())
        })
        .unwrap();
    assert_eq!(usage.copied_bytes, 2);
}
