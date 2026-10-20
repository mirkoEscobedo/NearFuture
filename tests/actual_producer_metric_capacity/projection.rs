use super::Brackets;
use near_future::application::{
    ForegroundApplication, HostExit, PortableCompletionV1, PortableEvaluationReceipt,
    ProducerMetricClockV1, ProducerMetricScopeV1, ProducerMetricUnitV1, ProducerNativeOriginV1,
    ProducerObservationOriginV1, ProducerObservationSelection, ProducerObservationV1,
    ProducerScopeV1, RegisteredProducerCodeV1, RegisteredProducerKindV1,
    RegisteredProducerMetricV1, RegisteredProducerSeverityV1,
};
use nf_nex_shadow::{
    DiagnosticDroppedCountV1, DiagnosticObservationOriginV1, DiagnosticObservationScopeV1,
    DiagnosticObservationUnavailableV1, Difference, ShadowAuthority,
};

#[derive(Debug, Eq, PartialEq)]
struct Outer {
    counts: [u64; 5],
    decision_records: u64,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub stable: String,
    outer: Outer,
    values: [Option<u64>; 4],
    enclosing_ns: u128,
    capacity: u64,
}
impl Snapshot {
    pub fn actual(&self) -> String {
        format!(
            "{{\"checkpoint\":{},\"quantiles_ns\":[{},{},{},{}],\"maximum_enclosing_call_ns\":{}}}\n",
            self.stable,
            display_ns(self.values[0]),
            display_ns(self.values[1]),
            display_ns(self.values[2]),
            display_ns(self.values[3]),
            self.enclosing_ns,
        )
    }
}
pub struct Omitted {
    pub actual: String,
    outer: Outer,
    metrics_absent: bool,
    l1_absent: bool,
}

pub fn assert_receipt(value: &PortableEvaluationReceipt) {
    assert!(matches!(value.authority(), ShadowAuthority::ShadowOnly));
    assert!(matches!(
        value.scope(),
        ProducerScopeV1::PortableCopiedFactsSinceOwnerStart
    ));
    assert!(matches!(
        value.completion(),
        PortableCompletionV1::Mismatched
    ));
    assert!(matches!(
        value.codes(),
        [RegisteredProducerCodeV1::NexDecisionMismatch]
    ));
}

pub fn observe(
    host: &mut ForegroundApplication,
    include_l1: bool,
    include_metrics: bool,
) -> ProducerObservationV1 {
    host.observe_producer(ProducerObservationSelection {
        required_scope: ProducerScopeV1::PortableCopiedFactsSinceOwnerStart,
        include_l1,
        include_metrics,
    })
    .unwrap_or_else(|_| panic!("actual capacity observation refused"))
}

fn outer(value: &ProducerObservationV1) -> Outer {
    assert!(matches!(
        value.origin(),
        ProducerObservationOriginV1::ExistingOwnerGuardObservation
    ));
    assert!(matches!(
        value.scope(),
        ProducerScopeV1::PortableCopiedFactsSinceOwnerStart
    ));
    assert!(matches!(value.authority(), ShadowAuthority::ShadowOnly));
    assert!(matches!(
        value.native_origin(),
        ProducerNativeOriginV1::Unqualified
    ));
    assert!(value.coalesced_records().is_none());
    assert!(value.historical_records().is_none());
    let counts = value.evaluations();
    let counts = [
        counts.started(),
        counts.completed(),
        counts.unavailable(),
        counts.matched(),
        counts.mismatched(),
    ];
    assert!(value.records().len() <= 1, "closed decision-only fixture");
    let decision_records = match value.records().first() {
        Some(record) => {
            assert!(matches!(
                record.kind(),
                RegisteredProducerKindV1::NexCopiedFactsV1
            ));
            assert!(matches!(
                record.severity(),
                RegisteredProducerSeverityV1::Warning
            ));
            assert!(matches!(
                record.code(),
                RegisteredProducerCodeV1::NexDecisionMismatch
            ));
            assert!(matches!(record.difference(), Some(Difference::Decision)));
            record.count()
        }
        None => 0,
    };
    Outer {
        counts,
        decision_records,
    }
}

pub fn snapshot(value: &ProducerObservationV1, checkpoint: u64, enclosing_ns: u128) -> Snapshot {
    let outer = outer(value);
    let port = value
        .metrics()
        .expect("actual registered metric selection required");
    assert!(matches!(
        port.origin(),
        ProducerObservationOriginV1::ExistingOwnerGuardObservation
    ));
    assert!(matches!(
        port.scope(),
        ProducerMetricScopeV1::RegisteredOwnerLast64CompletedEvaluationSamples
    ));
    assert!(matches!(
        port.clock(),
        ProducerMetricClockV1::LocalOwnerMonotonicElapsedUnaligned
    ));
    assert!(port.complete_within_scope());
    assert_eq!(port.window_capacity(), 64);
    assert_eq!(port.metrics().len(), 1);
    assert_eq!(
        format!("{port:?}"),
        "ProducerMetricsObservationV1(data_only,evidence_is_not_authority)"
    );
    let row = &port.metrics()[0];
    assert!(matches!(
        row.metric(),
        RegisteredProducerMetricV1::RustShadowEvaluationDuration
    ));
    assert!(matches!(row.unit(), ProducerMetricUnitV1::Nanoseconds));
    assert_eq!(
        format!("{row:?}"),
        "ProducerMetricObservationV1(data_only,evidence_is_not_authority)"
    );
    let values = [row.p50(), row.p95(), row.p99(), row.p999()];
    let percentiles = if checkpoint == 0 && values.iter().all(Option::is_none) {
        "all_none"
    } else if checkpoint != 0
        && values.iter().all(Option::is_some)
        && values.windows(2).all(|pair| pair[0] <= pair[1])
        && values
            .iter()
            .all(|value| value.is_some_and(|value| u128::from(value) <= enclosing_ns))
    {
        "all_present_ordered_within_actual_outer_bound"
    } else {
        "invalid"
    };
    let queue = value
        .l1()
        .expect("actual same owned queue selection required");
    assert!(matches!(
        queue.origin(),
        DiagnosticObservationOriginV1::ExternallySuppliedUnverified
    ));
    assert!(matches!(
        queue.scope(),
        DiagnosticObservationScopeV1::QueuedStreamContentsOnly
    ));
    assert!(queue.complete_within_scope());
    assert!(queue.coalesced_records().is_none());
    assert!(queue.historical_records().is_none());
    assert!(matches!(
        queue.unavailable(),
        [
            DiagnosticObservationUnavailableV1::ProducerBinding,
            DiagnosticObservationUnavailableV1::ClockAlignment,
            DiagnosticObservationUnavailableV1::Metrics,
            DiagnosticObservationUnavailableV1::HistoricalRecords,
            DiagnosticObservationUnavailableV1::CoalescedRecords,
        ]
    ));
    assert_eq!(value.dropped(), Some(queue.dropped()));
    assert!(queue.difference_counts().len() <= 1);
    let queue_decisions = match queue.difference_counts().first() {
        Some(row) => {
            assert!(matches!(row.difference(), Difference::Decision));
            row.count()
        }
        None => 0,
    };
    let dropped = match queue.dropped() {
        DiagnosticDroppedCountV1::Exact(count) => Some(count),
        DiagnosticDroppedCountV1::AtLeast(_) => None,
    };
    let counts = outer.counts;
    let mut stable = format!(
        concat!(
            "{{\"started\":{},\"completed\":{},\"unavailable\":{},\"matched\":{},\"mismatched\":{},",
            "\"completed_samples\":{},\"sample_count\":{},\"evicted_samples\":{},\"percentiles\":\"{}\""
        ),
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        counts[4],
        row.completed_samples(),
        row.sample_count(),
        row.evicted_samples(),
        percentiles,
    );
    if checkpoint == 0 {
        assert!(value.records().is_empty());
        assert!(queue.difference_counts().is_empty());
        assert_eq!(outer.decision_records, 0);
        assert_eq!(queue_decisions, 0);
        assert_eq!(dropped, Some(0));
        stable.push_str(&format!(",\"queued_records\":{}}}", queue.queued_records()));
    } else {
        stable.push_str(&format!(
            ",\"owner_decision_record_count\":{},\"queued_records\":{},\"queue_decision_count\":{},\"dropped_exact\":{}}}",
            outer.decision_records, queue.queued_records(), queue_decisions, display_ns(dropped),
        ));
    }
    assert!(stable.is_ascii() && stable.len() <= 2048);
    Snapshot {
        stable,
        outer,
        values,
        enclosing_ns,
        capacity: port.window_capacity(),
    }
}

pub fn omitted(value: &ProducerObservationV1) -> Omitted {
    let outer = outer(value);
    let metrics_absent = value.metrics().is_none();
    let l1_absent = value.l1().is_none();
    assert!(value.dropped().is_none());
    let counts = outer.counts;
    let actual = format!(
        concat!(
            "{{\"started\":{},\"completed\":{},\"unavailable\":{},\"matched\":{},\"mismatched\":{},",
            "\"owner_decision_record_count\":{},\"metrics_absent\":{},\"l1_absent\":{}}}\n"
        ),
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        counts[4],
        outer.decision_records,
        metrics_absent,
        l1_absent,
    );
    Omitted {
        actual,
        outer,
        metrics_absent,
        l1_absent,
    }
}

fn display_ns(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

pub fn captures(
    snapshots: [&Snapshot; 5],
    omitted: &Omitted,
    brackets: &Brackets,
    snapshot64_unchanged: bool,
    originals_unchanged: bool,
    exit: HostExit,
    after: &[u8],
) -> (String, Vec<(String, String)>) {
    let [empty, full, first, second, last] = snapshots;
    let later_values_equal = first.values == second.values && second.values == last.values;
    let nonmutation = first == second && second == last && omitted.outer == first.outer;
    let stop = match exit {
        HostExit::Stopped => "stopped",
        HostExit::Expired => "expired",
    };
    let actual = format!(
        concat!(
            "{{\"purpose\":\"PROPOSED_TEST_PRIVATE_STABLE_PREDICATES_NOT_PRODUCT_FORMAT\",",
            "\"scope\":\"portable_copied_facts_since_owner_start\",\"authority\":\"shadow_only\",",
            "\"native_origin\":\"unqualified\",\"metric\":\"rust_shadow_evaluation_duration\",",
            "\"unit\":\"ns\",\"clock\":\"local_owner_monotonic_elapsed_unaligned\",",
            "\"window_capacity\":{},\"empty\":{},\"after64\":{},\"after65\":{},",
            "\"accepted_receipts\":{},\"receipt_codes\":\"nex_decision_mismatch\",",
            "\"selection_sequence\":[[true,true],[true,true],[false,false],[true,true]],",
            "\"omitted_metrics\":{},\"omitted_l1\":{},\"later65_metric_values_equal\":{},",
            "\"snapshot64_still_equals_original\":{},\"counts_and_queue_unchanged_by_observation\":{},",
            "\"coalesced_records\":null,\"historical_records\":null,\"stop\":\"{}\",",
            "\"owned_originals_and_public_fixture_unchanged\":{}}}\n"
        ),
        first.capacity,
        empty.stable,
        full.stable,
        first.stable,
        brackets.len(),
        if omitted.metrics_absent {
            "null"
        } else {
            "\"unexpected\""
        },
        if omitted.l1_absent {
            "null"
        } else {
            "\"unexpected\""
        },
        later_values_equal,
        snapshot64_unchanged,
        nonmutation,
        stop,
        originals_unchanged,
    );
    let mut files = Vec::with_capacity(11);
    for (name, snapshot) in [
        ("empty", empty),
        ("64", full),
        ("65.first", first),
        ("65.second", second),
        ("65.final", last),
    ] {
        files.push((format!("actual.{name}.metrics.json"), snapshot.actual()));
    }
    files.push(("actual.65.omitted.json".to_owned(), omitted.actual.clone()));
    files.push(("actual.outer-and-l1.json".to_owned(), actual.clone()));
    files.push((
        "actual.typed-receipts.txt".to_owned(),
        format!(
            "actual_validated_receipts={} completion=mismatched code=nex_decision_mismatch\n",
            brackets.len(),
        ),
    ));
    files.push((
        "actual.public-call-brackets.tsv".to_owned(),
        brackets.capture(),
    ));
    files.push(("actual.snapshot-preservation.txt".to_owned(), format!(
        "snapshot64_unchanged={snapshot64_unchanged}\nowned_originals_unchanged={originals_unchanged}\n",
    )));
    files.push((
        "public-ram-fixture.after.txt".to_owned(),
        String::from_utf8(after.to_vec()).unwrap(),
    ));
    assert_eq!(files.len(), 11);
    for (_, body) in &files {
        assert!(
            body.is_ascii() && body.len() <= 16_384,
            "bounded closed capture"
        );
    }
    (actual, files)
}
