pub mod custody;
pub mod fixture;
pub mod projection;

// Independently selected literals; no product encoder computes these expectations.
pub const METRIC: &str = concat!(
    "{\"schema\":\"NF-ACTUAL-RUST-PRODUCER-METRIC-ASSERTIONS-1\",",
    "\"origin\":\"existing_owner_guard_observation\",",
    "\"producer_scope\":\"portable_copied_facts_since_owner_start\",",
    "\"authority\":\"shadow_only\",",
    "\"native_origin\":\"unqualified\",",
    "\"metric\":\"rust_shadow_evaluation_duration\",",
    "\"unit\":\"ns\",",
    "\"metric_scope\":\"registered_owner_last_64_completed_evaluation_samples\",",
    "\"clock\":\"local_owner_monotonic_elapsed_unaligned\",",
    "\"window_capacity\":64,",
    "\"completed_samples\":1,",
    "\"sample_count\":1,",
    "\"evicted_samples\":0,",
    "\"all_percentiles_present\":true,",
    "\"one_sample_percentiles_equal\":true,",
    "\"completed_evaluations\":1,",
    "\"mismatched_evaluations\":1,",
    "\"l1_queued_records\":1}",
);

pub const OUTER: &str = concat!(
    "{\"schema\":\"NF-ACTUAL-RUST-PRODUCER-OBSERVATION-1\",",
    "\"origin\":\"existing_owner_guard_observation\",",
    "\"scope\":\"portable_copied_facts_since_owner_start\",",
    "\"authority\":\"shadow_only\",",
    "\"native_origin\":\"unqualified\",",
    "\"evaluations\":{\"completed\":1,",
    "\"unavailable\":0,",
    "\"matched\":0,",
    "\"mismatched\":1},",
    "\"records\":[{\"kind\":\"nex_copied_facts_v1\",",
    "\"severity\":\"warning\",",
    "\"code\":\"nex_decision_mismatch\",",
    "\"difference\":\"decision\",",
    "\"count\":1}],",
    "\"dropped\":{\"count\":0,",
    "\"semantics\":\"exact\"},",
    "\"coalesced\":null,",
    "\"historical_records\":null}",
);

pub const QUEUE: &str = concat!(
    "{\"queued_records\":1,",
    "\"difference_counts\":[{\"difference\":\"decision\",",
    "\"count\":1}],",
    "\"dropped\":{\"count\":0,",
    "\"semantics\":\"exact\"},",
    "\"coalesced\":null,",
    "\"historical_records\":null}",
);
