pub mod custody;
pub mod fixture;
pub mod projection;

// Independent selected contract literals; never produced by an application encoder.
pub const OUTER: &str = "{\"schema\":\"NF-ACTUAL-RUST-PRODUCER-OBSERVATION-1\",\"origin\":\"existing_owner_guard_observation\",\"scope\":\"portable_copied_facts_since_owner_start\",\"authority\":\"shadow_only\",\"native_origin\":\"unqualified\",\"evaluations\":{\"completed\":1,\"unavailable\":0,\"matched\":0,\"mismatched\":1},\"records\":[{\"kind\":\"nex_copied_facts_v1\",\"severity\":\"warning\",\"code\":\"nex_decision_mismatch\",\"difference\":\"decision\",\"count\":1}],\"dropped\":{\"count\":0,\"semantics\":\"exact\"},\"coalesced\":null,\"historical_records\":null}";
pub const QUEUE: &str = "{\"queued_records\":1,\"difference_counts\":[{\"difference\":\"decision\",\"count\":1}],\"dropped\":{\"count\":0,\"semantics\":\"exact\"},\"coalesced\":null,\"historical_records\":null}";
