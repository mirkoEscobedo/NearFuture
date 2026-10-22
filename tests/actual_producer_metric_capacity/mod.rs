#[path = "../actual_producer_metric_observation/custody.rs"]
pub mod custody;
#[path = "../actual_producer_metric_observation/fixture.rs"]
pub mod fixture;
pub mod projection;

use custody::{Evidence, Stage};
use near_future::application::ForegroundApplication;
use nf_nex_shadow::{MakePeaceEligibility, ShadowOutput};
use std::time::{Duration, Instant};

pub struct Brackets {
    values: [u128; 65],
    length: usize,
}
impl Brackets {
    pub fn new() -> Self {
        Self {
            values: [0; 65],
            length: 0,
        }
    }
    fn record(&mut self, elapsed: Option<Duration>) {
        assert!(self.length < 65, "bounded actual receipt rows");
        let nanos = elapsed.expect("real caller monotonic bracket").as_nanos();
        self.values[self.length] = nanos;
        self.length = self.length.checked_add(1).unwrap();
    }
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn maximum(&self) -> u128 {
        assert!(self.length > 0 && self.length <= 65);
        *self.values[..self.length].iter().max().unwrap()
    }
    pub fn capture(&self) -> String {
        assert_eq!(self.length, 65, "65 accepted actual calls required");
        let mut text = String::from("actual_accepted_public_call_ordinal enclosing_ns\n");
        for (index, value) in self.values[..self.length].iter().enumerate() {
            let ordinal = index.checked_add(1).unwrap();
            let row = format!("{ordinal} {value}\n");
            assert!(row.len() <= 128, "bounded bracket row");
            assert!(text.len().checked_add(row.len()).unwrap() <= 16_384);
            text.push_str(&row);
        }
        text
    }
}

pub fn evaluate(
    evidence: &mut Evidence,
    host: &mut ForegroundApplication,
    input: &MakePeaceEligibility,
    reference: &ShadowOutput,
    brackets: &mut Brackets,
) {
    let (receipt, elapsed) = evidence.run(Stage::EvaluateAction, || {
        let before = Instant::now();
        let receipt = host.evaluate_action_copied(input, reference);
        let after = Instant::now();
        (receipt, after.checked_duration_since(before))
    });
    let receipt = receipt.unwrap_or_else(|_| panic!("actual accepted evaluation required"));
    evidence.run(Stage::ReceiptAssertions, || {
        projection::assert_receipt(&receipt);
        brackets.record(elapsed);
    });
}

pub fn write(evidence: &mut Evidence, name: &str, bytes: &[u8]) {
    assert!(bytes.len() <= 16_384, "capacity capture admission");
    evidence.write(name, bytes);
}

// Independently authored stable predicates; no product encoder computes expectations.
pub const EXPECTED: &str = concat!(
    "{\"purpose\":\"PROPOSED_TEST_PRIVATE_STABLE_PREDICATES_NOT_PRODUCT_FORMAT\"",
    ",\"scope\":\"portable_copied_facts_since_owner_start\"",
    ",\"authority\":\"shadow_only\"",
    ",\"native_origin\":\"unqualified\"",
    ",\"metric\":\"rust_shadow_evaluation_duration\"",
    ",\"unit\":\"ns\"",
    ",\"clock\":\"local_owner_monotonic_elapsed_unaligned\"",
    ",\"window_capacity\":64",
    ",\"empty\":{\"started\":0",
    ",\"completed\":0",
    ",\"unavailable\":0",
    ",\"matched\":0",
    ",\"mismatched\":0",
    ",\"completed_samples\":0",
    ",\"sample_count\":0",
    ",\"evicted_samples\":0",
    ",\"percentiles\":\"all_none\"",
    ",\"queued_records\":0}",
    ",\"after64\":{\"started\":64",
    ",\"completed\":64",
    ",\"unavailable\":0",
    ",\"matched\":0",
    ",\"mismatched\":64",
    ",\"completed_samples\":64",
    ",\"sample_count\":64",
    ",\"evicted_samples\":0",
    ",\"percentiles\":\"all_present_ordered_within_actual_outer_bound\"",
    ",\"owner_decision_record_count\":64",
    ",\"queued_records\":1",
    ",\"queue_decision_count\":1",
    ",\"dropped_exact\":0}",
    ",\"after65\":{\"started\":65",
    ",\"completed\":65",
    ",\"unavailable\":0",
    ",\"matched\":0",
    ",\"mismatched\":65",
    ",\"completed_samples\":65",
    ",\"sample_count\":64",
    ",\"evicted_samples\":1",
    ",\"percentiles\":\"all_present_ordered_within_actual_outer_bound\"",
    ",\"owner_decision_record_count\":65",
    ",\"queued_records\":1",
    ",\"queue_decision_count\":1",
    ",\"dropped_exact\":0}",
    ",\"accepted_receipts\":65",
    ",\"receipt_codes\":\"nex_decision_mismatch\"",
    ",\"selection_sequence\":[[true,true],[true,true],[false,false],[true,true]]",
    ",\"omitted_metrics\":null",
    ",\"omitted_l1\":null",
    ",\"later65_metric_values_equal\":true",
    ",\"snapshot64_still_equals_original\":true",
    ",\"counts_and_queue_unchanged_by_observation\":true",
    ",\"coalesced_records\":null",
    ",\"historical_records\":null",
    ",\"stop\":\"stopped\"",
    ",\"owned_originals_and_public_fixture_unchanged\":true}\n",
);
