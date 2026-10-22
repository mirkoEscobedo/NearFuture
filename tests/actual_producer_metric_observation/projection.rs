use near_future::application::{
    PortableCompletionV1, PortableEvaluationReceipt, ProducerNativeOriginV1,
    ProducerObservationOriginV1, ProducerObservationV1, ProducerScopeV1, RegisteredProducerCodeV1,
    RegisteredProducerKindV1, RegisteredProducerSeverityV1,
};
use nf_nex_shadow::{
    DiagnosticDroppedCountV1, DiagnosticObservationOriginV1, DiagnosticObservationScopeV1,
    DiagnosticObservationUnavailableV1, Difference, ShadowAuthority,
};

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

// Test-private getters only. Both expected documents are independent literal constants.
pub fn project(value: &ProducerObservationV1) -> (String, String) {
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
    let counts = value.evaluations();
    assert_eq!(counts.started(), 1);
    assert_eq!(
        (
            counts.completed(),
            counts.unavailable(),
            counts.matched(),
            counts.mismatched()
        ),
        (1, 0, 0, 1)
    );
    let records = value.records();
    assert_eq!(records.len(), 1, "one actual accepted comparison facet");
    let record = &records[0];
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
    assert_eq!(record.count(), 1);
    assert!(value.coalesced_records().is_none());
    assert!(value.historical_records().is_none());
    let dropped = match value.dropped() {
        Some(DiagnosticDroppedCountV1::Exact(count)) => count,
        _ => panic!("required actual stream exact drop count"),
    };
    assert_eq!(dropped, 0);
    let queue = value.l1().expect("same owned stream observation required");
    assert!(matches!(
        queue.origin(),
        DiagnosticObservationOriginV1::ExternallySuppliedUnverified
    ));
    assert!(matches!(
        queue.scope(),
        DiagnosticObservationScopeV1::QueuedStreamContentsOnly
    ));
    assert!(queue.complete_within_scope());
    assert_eq!(queue.queued_records(), 1);
    let differences = queue.difference_counts();
    assert_eq!(differences.len(), 1, "bounded first queue differences");
    assert!(matches!(differences[0].difference(), Difference::Decision));
    assert_eq!(differences[0].count(), 1);
    assert!(matches!(
        queue.dropped(),
        DiagnosticDroppedCountV1::Exact(0)
    ));
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
    let outer = format!(
        concat!(
            "{{\"schema\":\"NF-ACTUAL-RUST-PRODUCER-OBSERVATION-1\",",
            "\"origin\":\"existing_owner_guard_observation\",",
            "\"scope\":\"portable_copied_facts_since_owner_start\",",
            "\"authority\":\"shadow_only\",\"native_origin\":\"unqualified\",",
            "\"evaluations\":{{\"completed\":{},\"unavailable\":{},\"matched\":{},\"mismatched\":{}}},",
            "\"records\":[{{\"kind\":\"nex_copied_facts_v1\",\"severity\":\"warning\",",
            "\"code\":\"nex_decision_mismatch\",\"difference\":\"decision\",\"count\":{}}}],",
            "\"dropped\":{{\"count\":{},\"semantics\":\"exact\"}},",
            "\"coalesced\":null,\"historical_records\":null}}"
        ),
        counts.completed(),
        counts.unavailable(),
        counts.matched(),
        counts.mismatched(),
        record.count(),
        dropped,
    );
    let retained = format!(
        concat!(
            "{{\"queued_records\":{},\"difference_counts\":[{{\"difference\":\"decision\",\"count\":{}}}],",
            "\"dropped\":{{\"count\":{},\"semantics\":\"exact\"}},\"coalesced\":null,\"historical_records\":null}}"
        ),
        queue.queued_records(),
        differences[0].count(),
        dropped,
    );
    assert!(outer.is_ascii() && outer.len() <= 2048);
    assert!(retained.is_ascii() && retained.len() <= 2048);
    (outer, retained)
}

// This test-private projection checks the selected public data API, not an exporter.
use near_future::application::{
    ForegroundApplication, ProducerMetricClockV1, ProducerMetricObservationV1,
    ProducerMetricScopeV1, ProducerMetricUnitV1, ProducerMetricsObservationV1,
    ProducerObservationSelection, RegisteredProducerMetricV1,
};

#[derive(Debug, Eq, PartialEq)]
pub struct MetricProjection {
    pub stable: String,
    pub actual: String,
    pub values: [Option<u64>; 4],
    pub enclosing_ns: u128,
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
    .unwrap_or_else(|_| panic!("actual metric observation refused"))
}

fn port(value: &ProducerObservationV1) -> &ProducerMetricsObservationV1 {
    let port = value
        .metrics()
        .expect("registered metric port must be observed");
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
    assert_eq!(port.window_capacity(), 64);
    assert!(port.complete_within_scope());
    assert_eq!(port.metrics().len(), 1);
    assert_eq!(
        format!("{port:?}"),
        "ProducerMetricsObservationV1(data_only,evidence_is_not_authority)"
    );
    port
}

fn row(value: &ProducerObservationV1) -> &ProducerMetricObservationV1 {
    let row = &port(value).metrics()[0];
    assert!(matches!(
        row.metric(),
        RegisteredProducerMetricV1::RustShadowEvaluationDuration
    ));
    assert!(matches!(row.unit(), ProducerMetricUnitV1::Nanoseconds));
    assert_eq!(
        format!("{row:?}"),
        "ProducerMetricObservationV1(data_only,evidence_is_not_authority)"
    );
    row
}

pub fn empty(value: &ProducerObservationV1) -> String {
    let port = port(value);
    let row = row(value);
    assert_eq!(value.evaluations().started(), 0);
    assert_eq!(value.evaluations().completed(), 0);
    assert_eq!(value.evaluations().unavailable(), 0);
    assert_eq!(value.evaluations().matched(), 0);
    assert_eq!(value.evaluations().mismatched(), 0);
    assert!(value.records().is_empty());
    assert_eq!(row.completed_samples(), 0);
    assert_eq!(row.sample_count(), 0);
    assert_eq!(row.evicted_samples(), 0);
    let quantiles = [row.p50(), row.p95(), row.p99(), row.p999()];
    assert!(quantiles.iter().all(Option::is_none));
    let queue = value.l1().expect("actual empty registered queue required");
    assert_eq!(queue.queued_records(), 0);
    assert!(queue.difference_counts().is_empty());
    assert!(matches!(
        value.dropped(),
        Some(DiagnosticDroppedCountV1::Exact(0))
    ));
    format!(
        "observed_registered_empty capacity={} completed={} retained={} evicted={} all_percentiles_none={}\n",
        port.window_capacity(),
        row.completed_samples(),
        row.sample_count(),
        row.evicted_samples(),
        quantiles.iter().all(Option::is_none),
    )
}

pub fn one(value: &ProducerObservationV1, enclosing_ns: u128) -> MetricProjection {
    let port = port(value);
    let row = row(value);
    let quantiles = [row.p50(), row.p95(), row.p99(), row.p999()];
    let all_present = quantiles.iter().all(Option::is_some);
    let equal = all_present && quantiles.iter().all(|value| *value == quantiles[0]);
    // Retain observed values, including honest nulls, before literal/bracket assertions.
    let values = quantiles;
    let queue = value.l1().expect("same actual queue required");
    let stable = format!(
        concat!(
            "{{\"schema\":\"NF-ACTUAL-RUST-PRODUCER-METRIC-ASSERTIONS-1\",",
            "\"origin\":\"existing_owner_guard_observation\",",
            "\"producer_scope\":\"portable_copied_facts_since_owner_start\",",
            "\"authority\":\"shadow_only\",\"native_origin\":\"unqualified\",",
            "\"metric\":\"rust_shadow_evaluation_duration\",\"unit\":\"ns\",",
            "\"metric_scope\":\"registered_owner_last_64_completed_evaluation_samples\",",
            "\"clock\":\"local_owner_monotonic_elapsed_unaligned\",",
            "\"window_capacity\":{},\"completed_samples\":{},\"sample_count\":{},",
            "\"evicted_samples\":{},\"all_percentiles_present\":{},",
            "\"one_sample_percentiles_equal\":{},\"completed_evaluations\":{},",
            "\"mismatched_evaluations\":{},\"l1_queued_records\":{}}}"
        ),
        port.window_capacity(),
        row.completed_samples(),
        row.sample_count(),
        row.evicted_samples(),
        all_present,
        equal,
        value.evaluations().completed(),
        value.evaluations().mismatched(),
        queue.queued_records(),
    );
    let actual = format!(
        concat!(
            "scope=accepted_evaluate_accept_compare\nunit=ns\n",
            "p50={}\np95={}\np99={}\np999={}\nenclosing_public_call_ns={}\n"
        ),
        display_ns(values[0]),
        display_ns(values[1]),
        display_ns(values[2]),
        display_ns(values[3]),
        enclosing_ns,
    );
    assert!(stable.is_ascii() && stable.len() <= 2048);
    assert!(actual.is_ascii() && actual.len() <= 2048);
    MetricProjection {
        stable,
        actual,
        values,
        enclosing_ns,
    }
}

pub fn not_selected(value: &ProducerObservationV1) {
    assert!(
        value.metrics().is_none(),
        "not selected is not an empty metric window"
    );
    assert!(value.l1().is_none());
    assert!(value.dropped().is_none());
    assert_eq!(value.evaluations().started(), 1);
    assert_eq!(value.evaluations().completed(), 1);
    assert_eq!(value.evaluations().unavailable(), 0);
    assert_eq!(value.evaluations().matched(), 0);
    assert_eq!(value.evaluations().mismatched(), 1);
    assert!(value.coalesced_records().is_none());
    assert!(value.historical_records().is_none());
}

fn display_ns(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

pub fn captures(
    metrics: [&MetricProjection; 3],
    public: [&(String, String); 3],
    empty: &str,
) -> Vec<(String, String)> {
    let mut files = vec![("actual.empty.metrics.txt".to_owned(), empty.to_owned())];
    for ((label, metric), public) in ["first", "second", "final"]
        .into_iter()
        .zip(metrics)
        .zip(public)
    {
        files.push((
            format!("actual.{label}.metrics.json"),
            metric.stable.clone(),
        ));
        files.push((
            format!("actual.{label}.metric-values.txt"),
            metric.actual.clone(),
        ));
        files.push((format!("actual.{label}.outer.json"), public.0.clone()));
        files.push((format!("actual.{label}.queue.json"), public.1.clone()));
    }
    files.push((
        "actual.typed-receipt.txt".to_owned(),
        "shadow_only portable_copied_facts_since_owner_start mismatched nex_decision_mismatch\n"
            .to_owned(),
    ));
    assert_eq!(files.len(), 14);
    files
}

pub fn assert_retained(
    metrics: [&MetricProjection; 3],
    public: [&(String, String); 3],
    empty: &str,
    metadata: &nf_nex_shadow::ShadowMetadata,
) {
    use super::{METRIC, OUTER, QUEUE, fixture};
    assert_eq!((METRIC.len(), OUTER.len(), QUEUE.len()), (613, 487, 163));
    for (metric, public) in metrics.into_iter().zip(public) {
        assert_eq!(metric.stable.as_bytes(), METRIC.as_bytes());
        assert_eq!(public.0.as_bytes(), OUTER.as_bytes());
        assert_eq!(public.1.as_bytes(), QUEUE.as_bytes());
        let values = metric
            .values
            .map(|value| value.expect("one sample percentile required"));
        assert!(values.iter().all(|value| *value == values[0]));
        assert!(
            u128::from(values[0]) <= metric.enclosing_ns,
            "real caller bracket exceeded"
        );
        for text in [
            metric.stable.as_str(),
            metric.actual.as_str(),
            &public.0,
            &public.1,
            empty,
        ] {
            fixture::assert_private_bindings_absent(text, metadata);
        }
    }
    assert_eq!(
        metrics[0].values, metrics[1].values,
        "observation changed numeric samples"
    );
    assert_eq!(
        metrics[1].values, metrics[2].values,
        "selection changed numeric samples"
    );
    assert_eq!(
        metrics[0], metrics[1],
        "observation changed real metric window"
    );
    assert_eq!(
        metrics[1], metrics[2],
        "selection reset or appended metric samples"
    );
    assert_eq!(public[0], public[1], "same owned queue observation drained");
    assert_eq!(
        public[1], public[2],
        "unselected queue changed later observation"
    );
}
