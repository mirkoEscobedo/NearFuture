# Private NF-CANON-1 domain7 / schema2 / component5

This standalone record is owned by nf-world. It is fully decoded typed miniature state, not an opaque provider extension. Accepted nf-kernel schema1/Foundation/Java/wire codecs are unchanged. Future schema2 integration must explicitly call this decoder and compare its scope/seed/ruleset with the enclosing World. Supplied committed WorldTick is trusted context; it is not encoded as a second clock in the component.

Header is17 bytes: `NF-CANON-1` plus NUL (11), then domain u16=7, type u16=5, schema u16=2, all little endian. Unknown values reject. IDs are16 raw bytes; seeds/digests32. All resource/counter/tick values are u64 LE; relation score is i64 LE constrained to i32 and [-10000,10000]. Closed phase/kind/ordinal tags use u8. Each collection starts with u32 LE count. There is no text, float, recursion, decompression or arbitrary map.

Field order: universe16, history16, seed32, ruleset digest32, three controlling account IDs in ordinal order (48), aggregate revision u64; then systems, factions, markets, fleets, relations, schedules.

| Collection | Exact tuple order | Ordering/count |
| --- | --- | --- |
| Systems | entity16, ordinal u8 | Raw entity ID ascending; exactly3 |
| Factions | entity16, ordinal u8, account16, available credits u64, available supplies u64, spent credits u64, spent supplies u64 | Raw entity ID ascending; exactly3 |
| Markets | entity16, ordinal u8, system16, owner-present u8, owner16 only if present; Farming slot then Workshop slot | Raw entity ID ascending; exactly6 |
| Fleets | entity16, faction16, location tag u8, location ID16 | Raw entity ID ascending; exactly3 |
| Relations | entity16, lower faction16, higher faction16, score i64 | Raw entity ID ascending; exactly3 unordered faction pairs |
| Schedules | schedule16, creating operation16, due tick u64, kind u8, closed payload below | Ascending `(due tick,schedule ID)`; at most32 |

Owner-present is0 (no bytes) or1 (owner16). Industry slot is0 Absent,1 Constructing plus schedule16, or2 Complete. Fleet location is1 Docked plus system16, or2 InTransit plus schedule16. Schedule kind1 is industry completion plus market16 and industry u8 (Farming1/Workshop2); kind2 is arrival plus fleet16/origin16/destination16. All other tags reject.

Markets ordinal0..5 map system ordinal floor(ordinal/2). Even ordinals are immutable starting colonies; odd ordinals may be unclaimed or owned by that system's starting faction. Unclaimed sites have no industry. One fixed fleet belongs to each faction; every distinct known system pair is a direct NF triangle route. Static topology is derived from the committed ruleset; there is no unencoded arbitrary route graph.

Entity derivation is the first16 bytes of SHA256 of `NF-MINI-ID-1`+NUL, universe16, history16, seed32, kind u8, ordinal u32 LE. Kinds1 system,2 faction,3 market,4 fleet,5 relationship. Relation ordinals correspond faction ordinal pairs `(0,1),(0,2),(1,2)`; their encoded left/right IDs sort raw bytes. Schedule derivation is first16 SHA256 of `NF-MINI-SCHEDULE-1`+NUL, universe16/history16/seed32, operation16, kind u8 (industry1/arrival2), subject16. Detect collisions; never silently overwrite an identity. No hash is treated as a signature or entropy.

`RULESET_CONFIG` in validation.rs contains the exact fixed UTF8/ASCII configuration including one NUL. SHA256 covers those bytes only; pacing is excluded because wall-clock pacing changes no NF transition rule. A mismatched digest is Unsupported.

Byte ceiling128KiB is a hard constant. The decoder checks it before parsing/allocation; every collection count is checked before vector allocation. Fixed dimensions and at most32 schedules also bound total entries without a recursive budget. Unknown metadata/enums/versions, noncanonical ordering, duplicates, trailing bytes, truncation, invalid references, phase/schedule mismatch, nonconserving resources and due schedules at/before the enclosing committed tick or beyond the remaining kind-specific horizon reject. At committed N construction remaining duration must be1..2 and travel1..3 ticks. The decoder checks due-N after rejecting underflow, without an overflowing N+duration bound. These are snapshot consistency constraints, not proof of the schedule creation tick or authenticated history. Only journal replay establishes provenance. Encoder applies the same invariants; accepted decode re-encodes to exact byte equality. No optional caller limit can raise a hard ceiling.

Independent Node writer `crates/nf-world/fixtures/generate.cjs` explicitly emits the public test genesis without loading Rust. Golden component length1018, SHA256 `1ed2a52abcd828aacce3ae7efa86239b04323f0a253ea3652b3784953d78f93f`; all accounts/seeds there are public deterministic test inputs, not signing secrets. `--check` verifies fixture reproducibility. Rust compares every byte, not only a same-implementation roundtrip.
