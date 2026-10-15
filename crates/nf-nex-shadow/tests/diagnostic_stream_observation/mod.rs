pub mod custody;

use nf_nex_shadow::{
    Diagnostic, DiagnosticDroppedCountV1, DiagnosticObservationOriginV1,
    DiagnosticObservationScopeV1, DiagnosticObservationUnavailableV1,
    DiagnosticStream, DiagnosticStreamObservationV1, Difference,
};
use sha2::{Digest, Sha256};

pub const GOLDEN: &str = concat!(
    "{\"format\":\"NF-DIAGNOSTIC-STREAM-OBSERVATION-1\",",
    "\"origin\":\"externally_supplied_unverified\",",
    "\"scope\":\"queued_stream_contents_only\",",
    "\"complete_within_scope\":true,\"queued_records\":3,",
    "\"difference_counts\":[{\"difference\":\"lifecycle\",\"count\":2},",
    "{\"difference\":\"priority\",\"count\":1}],",
    "\"dropped\":{\"classification\":\"exact\",\"count\":1},",
    "\"coalesced_records\":null,\"historical_records\":null,",
    "\"unavailable\":[\"producer_binding\",\"clock_alignment\",\"metrics\",",
    "\"historical_records\",\"coalesced_records\"]}"
);

pub fn records(mask: u8) -> [Diagnostic; 4] {
    let a = Diagnostic {
        source_digest: [1; 32], config_digest: [2; 32], input_digest: [3; 32],
        corpus_digest: [4; 32], implementation_digest: [5; 32],
        difference: Difference::Lifecycle,
    };
    let mut b = a.clone();
    b.input_digest = [6; 32];
    let c = Diagnostic {
        source_digest: [7; 32], config_digest: [8; 32], input_digest: [9; 32],
        corpus_digest: [10; 32], implementation_digest: [11; 32],
        difference: Difference::Priority,
    };
    let mut d = c.clone();
    d.input_digest = [12; 32];
    d.difference = Difference::Effects;
    let mut result = [a, b, c, d];
    for record in &mut result {
        for digest in [
            &mut record.source_digest, &mut record.config_digest,
            &mut record.input_digest, &mut record.corpus_digest,
            &mut record.implementation_digest,
        ] {
            for byte in digest { *byte ^= mask; }
        }
    }
    result
}

pub fn stream(records: &[Diagnostic; 4]) -> DiagnosticStream {
    let mut result = DiagnosticStream::new(3, 483).unwrap();
    for index in [0, 1, 0, 2, 3] { result.push(records[index].clone()); }
    result
}

fn difference(value: Difference) -> &'static str {
    match value {
        Difference::Kind => "kind", Difference::Lifecycle => "lifecycle",
        Difference::Priority => "priority", Difference::Decision => "decision",
        Difference::Draws => "draws", Difference::Effects => "effects",
    }
}

fn unavailable(value: &DiagnosticObservationUnavailableV1) -> &'static str {
    match value {
        DiagnosticObservationUnavailableV1::ProducerBinding => "producer_binding",
        DiagnosticObservationUnavailableV1::ClockAlignment => "clock_alignment",
        DiagnosticObservationUnavailableV1::Metrics => "metrics",
        DiagnosticObservationUnavailableV1::HistoricalRecords => "historical_records",
        DiagnosticObservationUnavailableV1::CoalescedRecords => "coalesced_records",
    }
}

fn optional(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |count| count.to_string())
}

// Test-only closed getter projection. GOLDEN is independently literal, never derived here.
pub fn project(value: &DiagnosticStreamObservationV1) -> String {
    let origin = match value.origin() {
        DiagnosticObservationOriginV1::ExternallySuppliedUnverified => "externally_supplied_unverified",
    };
    let scope = match value.scope() {
        DiagnosticObservationScopeV1::QueuedStreamContentsOnly => "queued_stream_contents_only",
    };
    let rows = value.difference_counts();
    let unavailable_values = value.unavailable();
    assert!(rows.len() <= 6, "closed difference cardinality");
    assert!(unavailable_values.len() <= 5, "closed unavailable cardinality");
    let counts = rows.iter().map(|row| {
        format!("{{\"difference\":\"{}\",\"count\":{}}}", difference(row.difference()), row.count())
    }).collect::<Vec<_>>().join(",");
    let (classification, dropped) = match value.dropped() {
        DiagnosticDroppedCountV1::Exact(count) => ("exact", count),
        DiagnosticDroppedCountV1::AtLeast(count) => ("at_least", count),
    };
    let missing = unavailable_values.iter().map(|item| {
        format!("\"{}\"", unavailable(item))
    }).collect::<Vec<_>>().join(",");
    let result = format!(
        concat!("{{\"format\":\"NF-DIAGNOSTIC-STREAM-OBSERVATION-1\",",
            "\"origin\":\"{}\",\"scope\":\"{}\",",
            "\"complete_within_scope\":{},\"queued_records\":{},",
            "\"difference_counts\":[{}],",
            "\"dropped\":{{\"classification\":\"{}\",\"count\":{}}},",
            "\"coalesced_records\":{},\"historical_records\":{},",
            "\"unavailable\":[{}]}}"),
        origin, scope, value.complete_within_scope(), value.queued_records(), counts,
        classification, dropped, optional(value.coalesced_records()),
        optional(value.historical_records()), missing,
    );
    assert!(result.is_ascii() && result.len() <= 2048, "bounded ASCII projection required");
    result
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn assert_private_values_absent(text: &str, originals: &[Diagnostic; 4], control: &[Diagnostic; 4]) {
    for label in ["source_digest", "config_digest", "input_digest", "corpus_digest", "implementation_digest"] {
        assert!(!text.contains(label), "private digest field exposed");
    }
    for record in originals.iter().chain(control) {
        for digest in [record.source_digest, record.config_digest, record.input_digest,
            record.corpus_digest, record.implementation_digest] {
            let hex = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
            assert!(!text.contains(&hex), "private digest hex exposed");
            assert!(!text.contains(&format!("{digest:?}")), "private digest bytes exposed");
        }
    }
}

fn record_bytes(record: &Diagnostic, bytes: &mut Vec<u8>) {
    for digest in [record.source_digest, record.config_digest, record.input_digest,
        record.corpus_digest, record.implementation_digest] { bytes.extend_from_slice(&digest); }
    bytes.push(match record.difference {
        Difference::Kind => 0, Difference::Lifecycle => 1, Difference::Priority => 2,
        Difference::Decision => 3, Difference::Draws => 4, Difference::Effects => 5,
    });
}

pub fn fixture_bytes(records: &[Diagnostic; 4]) -> Vec<u8> {
    let mut bytes = b"PUBLIC-SYNTHETIC-DIAGNOSTICS-1\n".to_vec();
    for record in records { record_bytes(record, &mut bytes); }
    bytes
}

pub fn popped_bytes(records: &[Option<Diagnostic>; 4]) -> Vec<u8> {
    let mut bytes = b"PUBLIC-SYNTHETIC-POPPED-1\n".to_vec();
    for record in records {
        bytes.push(u8::from(record.is_some()));
        if let Some(record) = record { record_bytes(record, &mut bytes); }
    }
    bytes
}
