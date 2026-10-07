mod supplies_property_support;
mod supplies_support;
use nf_kernel::supplies::SUPPLIES_CONTENT;
use supplies_property_support::{Call, Driver, Kind, allow, call, op, origins, req};
use supplies_support::{Fixture, Scratch};

#[test]
fn unique_event_conservation_survives_drained_retries_and_recovery_prefixes() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let key = (fixture.beneficiary.account, SUPPLIES_CONTENT, origins()[2]);
    let mut trace = Driver::new(&scratch, &fixture);
    trace.apply(call(&fixture, Kind::Issue, key, op(1), req(101), 25));
    let reserve = call(&fixture, Kind::Reserve, key, op(2), req(102), 8);
    trace.apply(reserve);
    trace.recover(false);
    trace.apply(reserve);
    trace.recover(true);
    trace.apply(reserve.alias(req(103)));
    trace.apply(call(&fixture, Kind::Burn, key, op(3), req(104), 17));
    trace.apply(call(&fixture, Kind::Reserve, key, op(4), req(105), 1));
    trace.apply(call(&fixture, Kind::Burn, key, op(5), req(106), 1));
    trace.apply(call(&fixture, Kind::Issue, key, op(6), req(107), 5));
    trace.apply(call(&fixture, Kind::Reserve, key, op(7), req(108), 5));
    trace.recover(true);
    assert_eq!(trace.totals(key), [0, 13, 0, 0, 30, 17]);
    // Five accepted asset effects plus two immutable insufficient-stock decisions.
    assert_eq!(trace.revision(), 7);
}
#[test]
fn conservation_is_partitioned_by_real_owner_trust_and_lineage() {
    let scratch = Scratch::new();
    let mut fixture = Fixture::new();
    let origins = origins();
    allow(&mut fixture, &origins, 1000);
    let mut trace = Driver::new(&scratch, &fixture);
    for owner in [fixture.beneficiary.account, fixture.issuer.account] {
        for origin in origins {
            trace.watch((owner, SUPPLIES_CONTENT, origin));
        }
    }
    let mut absent = origins[2];
    absent.lineage = [99; 32];
    trace.watch((fixture.beneficiary.account, SUPPLIES_CONTENT, absent));
    for (i, amount) in [11, 17, 23, 29].into_iter().enumerate() {
        let key = (fixture.beneficiary.account, SUPPLIES_CONTENT, origins[i]);
        trace.apply(call(
            &fixture,
            Kind::Issue,
            key,
            op(10 + i as u32),
            req(110 + i as u32),
            amount,
        ));
    }
    let canonical = (fixture.beneficiary.account, SUPPLIES_CONTENT, origins[2]);
    trace.apply(call(
        &fixture,
        Kind::Reserve,
        canonical,
        op(14),
        req(114),
        7,
    ));
    trace.apply(call(&fixture, Kind::Burn, canonical, op(15), req(115), 5));
    let authority = (fixture.issuer.account, SUPPLIES_CONTENT, origins[2]);
    trace.apply(call(&fixture, Kind::Issue, authority, op(16), req(116), 13));
    trace.recover(true);
    assert_eq!(trace.totals(canonical), [11, 7, 0, 0, 23, 5]);
    assert_eq!(trace.totals(authority), [13, 0, 0, 0, 13, 0]);
    let mut total = [0_u128; 6];
    for owner in [fixture.beneficiary.account, fixture.issuer.account] {
        for origin in origins {
            let fields = trace.totals((owner, SUPPLIES_CONTENT, origin));
            for (sum, value) in total.iter_mut().zip(fields) {
                *sum += value;
            }
        }
    }
    assert_eq!(total, [81, 7, 0, 0, 93, 5]);
    assert_eq!(trace.revision(), 7);
}
#[test]
fn seeded_public_sequences_preserve_independent_conservation_and_global_outcomes() {
    let mut fixture = Fixture::new();
    let all = origins();
    let selected = [all[0], all[2], all[3]];
    allow(&mut fixture, &selected, 1000);
    let mut keys = Vec::new();
    for owner in [fixture.beneficiary.account, fixture.issuer.account] {
        for origin in selected {
            keys.push((owner, SUPPLIES_CONTENT, origin));
        }
    }
    // One real signed membership/key pair is reused across three separate test-owned histories.
    for (index, mut seed) in [1_u32, 0x5eed, 0xd00d].into_iter().enumerate() {
        let scratch = Scratch::new();
        let mut trace = Driver::new(&scratch, &fixture);
        let base = 1000 + index as u32 * 100;
        for (i, &key) in keys.iter().enumerate() {
            trace.apply(call(
                &fixture,
                Kind::Issue,
                key,
                op(base + i as u32),
                req(base + i as u32),
                12,
            ));
        }
        for step in 0..24_u32 {
            // Dependency-free deterministic command selection; no product state-transition or random helper.
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let key = keys[(seed as usize) % keys.len()];
            let amount = u64::from(seed % 5 + 1);
            let operation = op(base + 10 + step);
            let request = req(base + 10 + step);
            match step % 8 {
                0 => trace.apply(call(&fixture, Kind::Issue, key, operation, request, amount)),
                1 => trace.apply(call(
                    &fixture,
                    Kind::Reserve,
                    key,
                    operation,
                    request,
                    amount,
                )),
                2 => trace.apply(call(&fixture, Kind::Burn, key, operation, request, amount)),
                3..=6 => {
                    let originals = trace.originals();
                    let old = originals[(seed as usize) % originals.len()];
                    match step % 8 {
                        3 => trace.apply(old),
                        4 => trace.apply(old.alias(request)),
                        5 => {
                            let altered = if step < 16 {
                                old.alias(request).changed_amount(old.amount() + 1)
                            } else {
                                let i = selected
                                    .iter()
                                    .position(|&origin| origin == old.key().2)
                                    .unwrap();
                                old.alias(request)
                                    .changed_origin(selected[(i + 1) % selected.len()])
                            };
                            trace.apply(altered);
                        }
                        _ => {
                            let other_kind = match old {
                                Call::Burn(_) => Kind::Reserve,
                                _ => Kind::Burn,
                            };
                            let (operation, request) = if step < 16 {
                                (old.operation(), request)
                            } else {
                                (operation, old.request())
                            };
                            trace.apply(call(
                                &fixture,
                                other_kind,
                                old.key(),
                                operation,
                                request,
                                old.amount(),
                            ));
                        }
                    }
                }
                _ => trace.recover(step != 15),
            }
        }
        trace.recover(true);
        trace.assert_state();
    }
}
#[test]
fn u64_boundary_refusals_are_atomic_against_independent_u128_totals() {
    let scratch = Scratch::new();
    let mut fixture = Fixture::new();
    let origin = origins()[2];
    allow(&mut fixture, &[origin], u64::MAX);
    let key = (fixture.beneficiary.account, SUPPLIES_CONTENT, origin);
    let mut trace = Driver::new(&scratch, &fixture);
    trace.apply(call(&fixture, Kind::Issue, key, op(21), req(121), u64::MAX));
    let reserve = call(&fixture, Kind::Reserve, key, op(22), req(122), u64::MAX);
    trace.apply(reserve);
    trace.apply(call(&fixture, Kind::Burn, key, op(23), req(123), 1));
    trace.apply(call(&fixture, Kind::Reserve, key, op(24), req(124), 1));
    trace.apply(call(&fixture, Kind::Issue, key, op(25), req(125), 1));
    trace.recover(true);
    trace.apply(reserve.alias(req(126)));
    assert_eq!(
        trace.totals(key),
        [0, u128::from(u64::MAX), 0, 0, u128::from(u64::MAX), 0]
    );
    // IssueMax and ReserveMax plus two insufficient decisions; Issue overflow is not a journal decision.
    assert_eq!(trace.revision(), 4);
}
