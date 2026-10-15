use near_future::application::{
    PortableCompletionV1, PortableEvaluationReceipt, ProducerNativeOriginV1,
    ProducerObservationOriginV1, ProducerObservationV1, ProducerScopeV1,
    RegisteredProducerCodeV1, RegisteredProducerKindV1, RegisteredProducerSeverityV1,
};
use nf_nex_shadow::{
    DiagnosticDroppedCountV1, DiagnosticObservationOriginV1, DiagnosticObservationScopeV1,
    DiagnosticObservationUnavailableV1, Difference, ShadowAuthority,
};

pub fn assert_receipt(value: &PortableEvaluationReceipt) {
    assert!(matches!(value.authority(), ShadowAuthority::ShadowOnly));
    assert!(matches!(value.scope(), ProducerScopeV1::PortableCopiedFactsSinceOwnerStart));
    assert!(matches!(value.completion(), PortableCompletionV1::Mismatched));
    assert!(matches!(value.codes(), [RegisteredProducerCodeV1::NexDecisionMismatch]));
}

// Test-private getters only. Both expected documents are independent literal constants.
pub fn project(value: &ProducerObservationV1) -> (String, String) {
    assert!(matches!(value.origin(), ProducerObservationOriginV1::ExistingOwnerGuardObservation));
    assert!(matches!(value.scope(), ProducerScopeV1::PortableCopiedFactsSinceOwnerStart));
    assert!(matches!(value.authority(), ShadowAuthority::ShadowOnly));
    assert!(matches!(value.native_origin(), ProducerNativeOriginV1::Unqualified));
    let counts = value.evaluations();
    assert_eq!(counts.started(), 1);
    assert_eq!((counts.completed(), counts.unavailable(), counts.matched(), counts.mismatched()), (1, 0, 0, 1));
    let records = value.records();
    assert_eq!(records.len(), 1, "one actual accepted comparison facet");
    let record = &records[0];
    assert!(matches!(record.kind(), RegisteredProducerKindV1::NexCopiedFactsV1));
    assert!(matches!(record.severity(), RegisteredProducerSeverityV1::Warning));
    assert!(matches!(record.code(), RegisteredProducerCodeV1::NexDecisionMismatch));
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
    assert!(matches!(queue.origin(), DiagnosticObservationOriginV1::ExternallySuppliedUnverified));
    assert!(matches!(queue.scope(), DiagnosticObservationScopeV1::QueuedStreamContentsOnly));
    assert!(queue.complete_within_scope());
    assert_eq!(queue.queued_records(), 1);
    let differences = queue.difference_counts();
    assert_eq!(differences.len(), 1, "bounded first queue differences");
    assert!(matches!(differences[0].difference(), Difference::Decision));
    assert_eq!(differences[0].count(), 1);
    assert!(matches!(queue.dropped(), DiagnosticDroppedCountV1::Exact(0)));
    assert!(queue.coalesced_records().is_none());
    assert!(queue.historical_records().is_none());
    assert!(matches!(queue.unavailable(), [
        DiagnosticObservationUnavailableV1::ProducerBinding,
        DiagnosticObservationUnavailableV1::ClockAlignment,
        DiagnosticObservationUnavailableV1::Metrics,
        DiagnosticObservationUnavailableV1::HistoricalRecords,
        DiagnosticObservationUnavailableV1::CoalescedRecords,
    ]));
    let outer = format!(
        concat!("{{\"schema\":\"NF-ACTUAL-RUST-PRODUCER-OBSERVATION-1\",",
            "\"origin\":\"existing_owner_guard_observation\",",
            "\"scope\":\"portable_copied_facts_since_owner_start\",",
            "\"authority\":\"shadow_only\",\"native_origin\":\"unqualified\",",
            "\"evaluations\":{{\"completed\":{},\"unavailable\":{},\"matched\":{},\"mismatched\":{}}},",
            "\"records\":[{{\"kind\":\"nex_copied_facts_v1\",\"severity\":\"warning\",",
            "\"code\":\"nex_decision_mismatch\",\"difference\":\"decision\",\"count\":{}}}],",
            "\"dropped\":{{\"count\":{},\"semantics\":\"exact\"}},",
            "\"coalesced\":null,\"historical_records\":null}}"),
        counts.completed(), counts.unavailable(), counts.matched(), counts.mismatched(), record.count(), dropped,
    );
    let retained = format!(
        concat!("{{\"queued_records\":{},\"difference_counts\":[{{\"difference\":\"decision\",\"count\":{}}}],",
            "\"dropped\":{{\"count\":{},\"semantics\":\"exact\"}},\"coalesced\":null,\"historical_records\":null}}"),
        queue.queued_records(), differences[0].count(), dropped,
    );
    assert!(outer.is_ascii() && outer.len() <= 2048);
    assert!(retained.is_ascii() && retained.len() <= 2048);
    (outer, retained)
}
