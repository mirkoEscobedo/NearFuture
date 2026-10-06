# Kernel-local NF-CANON-1 domain 7, schema 1

Every standalone record starts with the same 17-byte fixed header: ASCII `NF-CANON-1` and NUL, then domain `u16=7`, type `u16`, schema `u16=1`, all little-endian. Unknown domain/type/schema, trailing input, duplicate keys/identities, noncanonical key order, excessive lengths and invalid closed enums reject. The foundation contract registry intentionally rejects all domain-7 records; wire capability/schema registration and Java vectors remain future gates.

IDs are exactly 16 raw bytes; digests/seeds are 32 bytes. Counters/credits are `u64 LE`. Relation scores/deltas and market deltas encode `i64 LE`; relation values must also fit i32, and live scores stay within [-10000,10000]. Enum tags and collection counts are `u32 LE`. Collections consist of the fixed field tuples specified below. Tuple entries have no extra record header; nested standalone snapshot/intent byte fields include their entire domain-7 header and a u32 byte length. No strings, floating point, arbitrary maps or opaque provider-state blobs are admitted.

| Type | Field order |
|---|---|
| 1 Snapshot | universe ID, history ID, seed, ruleset digest, tick, event sequence, factions, relations, markets, provider states, registry, ownership, revisions, outcomes |
| 2 CommittedBatch | before-state digest, after-state digest, resulting tick, resulting event sequence, authority term, runtime session, admitted intent byte records, events, outcomes |
| 3 ExactFrontier | snapshot byte record, input-state digest, authority term, runtime session, admitted intent byte records |
| 4 Intent | request ID, operation ID, job ID, actor account ID, universe ID, history ID, provider ID, expected-revision map, command |

Snapshot collection tuples:

| Collection | Entry fields and order |
|---|---|
| factions | entity ID, aggregate ID; sorted entity ID |
| relations | entity ID, aggregate ID, left faction ID, right faction ID, score; sorted entity ID |
| markets | entity ID, aggregate ID, faction ID, credits; sorted entity ID |
| provider states | provider ID, aggregate ID, draws, cooldown-until tick; sorted provider ID |
| registry | provider ID, implementation digest, version u32, state schema u32, kind u32, read-domain set, write-domain set, capability set, principal-ID list, maximum events u32; sorted provider ID |
| ownership | aggregate ID, provider ID, generation u64, activation event sequence, ruleset digest; sorted aggregate ID |
| revisions / intent expected map | aggregate ID, revision; sorted aggregate ID |
| outcomes | operation ID, job ID, rejection tag; semantic committed order, unique operation/job IDs across snapshot |

Sets are ascending numeric u32 tags with no duplicates. Principals sort raw account ID bytes. Registry kinds: 1 SyntheticPeace, 2 Manual. Domains: 1 Faction, 2 Relation, 3 Market, 4 Provider. Capabilities: 1 SeekPeace, 2 AdjustRelation, 3 AdjustMarket. Version and state schema must be 1.

Intent commands: tag 1 + relation ID (SeekPeace); tag 2 + relation ID + i64 delta (AdjustRelation); tag 3 + market ID + i64 delta (AdjustMarket). ExactFrontier and CommittedBatch admitted intents sort by `(provider, target entity, operation, job)`. Frontier decode reconstructs the immutable snapshot and validates the full declared frontier rather than trusting cached access sets.

Events are semantically ordered: tag 1 + relation ID + i64 delta; tag 2 + market ID + i64 delta; tag 3 + provider ID + draw count u64 + cooldown tick. Each accepted job contributes exactly its target event followed by its provider-state event. Rejected jobs contribute zero events. Replay checks that correspondence and ownership/conflicts. Synthetic peace replay checks the committed delta is 1–10; it consumes the authenticated committed choice instead of re-evaluating randomness. Provider audit is a separate operation.

Outcome rejection tags: 0 accepted; 1 InvalidReference; 2 DuplicateIdentity; 3 Limit; 4 InvalidValue; 5 UnsupportedProvider; 6 Unauthorized; 7 StaleRevision; 8 Conflict; 9 Overflow; 10 InvalidProposal; 11 ProviderFailed; 12 StaleSession; 13 FencedAuthority; 14 UnknownJob; 15 DuplicateJob; 16 InvalidFrontier; 17 StateMismatch. A reason is a closed numeric semantic value; no private error text enters canonical state.

Snapshot SHA-256 covers the complete type-1 record and contains no self hash. Authority term/session are excluded from snapshot state and scoped RNG, but retained in frontier/batch audit. Batch/frontier/intent bytes can themselves be hashed by the persistence driver. Snapshot normalization sorts semantic ID maps; decode also re-encodes and compares, preventing alternative byte representations.

Resource limits are in [README](README.md). All variable-size collections are count-checked before growing output values. Nested input is at most one snapshot plus a bounded list of closed intents; no recursive schema or decompression exists. The maximum representable snapshot under the model limits is below the nested 256 KiB byte-field cap. A provider declaration with max_events 1 is valid but cannot admit the current two-event result: it receives Limit.
