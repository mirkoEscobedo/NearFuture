# NF-NEX-SHADOW-1 private diagnostic identity

This is a closed private commitment for the portable copied-facts shadow diagnostic evaluator. It does not encode a complete NexWorld Snapshot, unused boundary fields or other concern instance IDs. The privately marked world-derived result cannot pass guarded acceptance without a future reviewed full-world commitment; the scope marker is not a signed/protobuf field. It is not registered in NF-CANON-1 or protobuf; no generic wire payload, signature authority, durable canonical-world identity or interoperability extension is granted.

All integers are fixed-width little-endian. Boolean is one byte 0/1. Raw IEEE finite f32/f64 values retain u32/u64 bits including signed zero. Strings are u32 UTF8 byte length followed by Unicode13-assigned/NFC bytes, no controls; the general maximum is 256 bytes, prior modifier IDs at most 128. IDs are exactly 16 raw unnormalized bytes and digests exactly 32 bytes. Optional values have a one-byte presence followed by their value. Collections have u32 counts; ordered arrays preserve order, traits are unique and sorted lexicographically by UTF8 key. Duplicate prior modifier (ID,kind) pairs reject. Encoding checks quotas before appending: record at most 65536 bytes and at most 4096 cumulative entries; no arbitrary decoder exists.

Header field order:

1. ASCII `NF-NEX-SHADOW-1` plus NUL; schema u16=1.
2. Pinned source commit string.
3. Source, runtime, ruleset, merged-config, reference, corpus, implementation and capture-policy digests, in that order; each required nonzero.
4. Universe, history, provider, campaign and branch IDs, in that order.
5. Subject faction string (at most128 UTF8 bytes), optional concern-instance ID presence/raw16; runtime-session u64 (required nonzero), observation frontier/EventSeq u64.
6. Observation u8: synthetic=1 or captured-unverified=2. Authority u8=1 (ShadowOnly).
7. Input kind u8: war=1 or selected-peace=2.

War kind fields: operation u8 (generate=1,update=2,isValid=3); weariness/minimum f32; already-ended/current-action booleans; up to 256 existing concern entries (class u8 WarWeariness=1 or Other=2, ended boolean); priority alignment/max-alignment/positive-trait/negative-trait f32; sorted unique traits (at most64); optional faction-multiplier f32; ordered five exact supported tags; prior modifiers (at most64), each ID string, kind u8 flat=1/percent=2/multiplier=3, raw f32.

Selected-peace fields: faction/enemy strings (each at most128 UTF8 bytes); player/treaty-relation booleans; own/enemy weariness and own/enemy event-disposition f32; has-enemies/pirate/allow-pirate booleans; elapsed-days/minimum-war-interval f32; recent-war/can-ceasefire/commissioned/offensive-blocked/offensive-facts-provided booleans; minimum weariness/divisor/divisor-per-level f32; player-level u32; event-multiplier/treaty-chance/ceasefire-reduction/treaty-reduction f32; up to two ordered draws, each ordinal u32, purpose string, raw f64. The selected method still rejects player or absent offensive facts. Captured peace construction remains unavailable.

The commitment is standard SHA256 over these exact bytes. `fixtures/generate-identity.cjs` independently constructs a public synthetic war record with Node standard crypto and fixed fields, without importing Rust. `fixtures/identity-v1.tsv` contains full bytes plus expected SHA256; Rust compares both. The exact source/config/implementation inputs are supplied bindings, not proof of source-to-installed-JAR equivalence. A changed schema or semantic field requires a new profile version and new vector.
