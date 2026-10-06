# NF-CANON-1

Status: specified baseline for NF-003 (#7); implementations and cross-language/fuzz evidence are separate acceptance work. This profile is closed: a record is accepted only when its exact `(domain, type, schema_version)` and any payload schema are registered locally. Wire bytes are never canonical bytes.

## Bytes and scalar values

Every top-level and nested record starts with these exact bytes:

`4e 46 2d 43 41 4e 4f 4e 2d 31 00`, then domain `u16le`, type `u16le`, schema version `u16le`. The first eleven bytes are ASCII `NF-CANON-1` and a NUL. Registry values below use schema version 1. No record length, padding, alignment, field tags or trailing bytes are added. A nested record is not implicitly length-prefixed; its registered fields determine its extent.

| Notation | Bytes / meaning |
| --- | --- |
| `u8`, `u16`, `u32`, `u64` | Exactly 1, 2, 4, 8 bytes; unsigned little endian. |
| `i64` | Exactly 8 little-endian bytes, two's complement; -2^63 through 2^63-1. |
| `bool` | Exactly one byte, 0 or 1. Other values reject. |
| `id` | Exactly 16 uninterpreted bytes. The field's registered type determines which ID it is. |
| `digest` | Exactly 32 SHA-256 bytes. |
| `bytes` | `u32` byte count followed by exactly that many bytes. |
| `text` | `bytes` containing valid shortest-form UTF-8 Unicode 13 assigned scalars, excluding noncharacters, in NFC. |
| `optional<T>` | One presence byte: 0 means no following value; 1 means exactly one T. Other values reject. |
| `list<T>` | `u32` element count followed by T values in semantic order. |
| `map<K,V>` | `u32` entry count followed by K,V pairs in strict canonical-key order. |
| `set<u32>` | `list<u32>` with strictly increasing numeric values. |

No native integers, platform endianness, floating point, varints, JSON number conversions or locale ordering appear in this profile. Java represents u64 by the raw bits of a long and uses unsigned comparison/checked operations; JSON vectors represent all i64/u64 values as decimal strings. Quantities use checked i64 units; domain policies impose tighter ranges. Counter increment at u64 maximum fails instead of wrapping. Unspecified fixed-point units cannot enter authoritative payload schemas.

All `text` fields admit only scalars assigned in Unicode 13.0.0, excluding U+D800..DFFF, U+FDD0..FDEF and each plane's U+FFFE/U+FFFF noncharacters. Unassigned or later-assigned scalars reject; this avoids newer Rust/Node normalization data admitting forms unsupported by baseline JVM normalization data. Within that repertoire, all text requires standard NFC already at the input boundary. Reject non-NFC input instead of silently normalizing a signed value. Reject unpaired UTF-16 surrogates in Java before UTF-8 encoding; reject invalid/overlong UTF-8, encoded surrogates and values above U+10FFFF when decoding. NFC is Unicode normalization stability, not locale/case folding. The assigned repertoire is generated from `protocol/text/DerivedAge-13.0.0.txt`, official Unicode UCD source (https://www.unicode.org/Public/13.0.0/ucd/DerivedAge.txt), pinned SHA-256 `e779a443d3aa2a3166a15becaa2b737c922480e32c0453d5956093633555078f`; `protocol/text/generate-ranges.cjs --check` verifies source identity and both generated tables. `protocol/text/Unicode-LICENSE.txt` retains the source permission notice. Rust uses `canonical::text::is_canonical_text`; Java uses the same table plus standard NFC. Empty text is valid where the field policy permits it. IDs, peer multihashes, bytes, digests and content keys' opaque byte components are never normalized. A schema needing differently treated human text must register a new version explicitly.

Map order for an `id` key is unsigned lexicographic order of its 16 bytes. Map order for a `text` key is unsigned lexicographic order of the raw UTF-8 bytes, excluding the length prefix; a shorter prefix sorts first. For instance `z` sorts before `é` regardless of UTF-16/locale order. There are no duplicate keys. A domain constructor may sort unordered entries before encoding; a canonical decoder must reject unsorted input rather than repair it. Sequence order is significant; set order is numeric, not little-endian byte order. Every ID-keyed record list identified below is strictly sorted by the named ID and has no duplicates.

Optional absent differs from present-empty; an empty list differs from absent optional list. Protobuf absent scalar defaults must not erase presence: schema fields use messages or `optional` when absence is meaningful. Required messages must be present even when their counters are zero. Zero is a valid raw ID/counter value unless a later operation policy explicitly forbids it; distinct identity types cannot substitute for one another.

SHA-256 hashes the entire canonical record, including its header and nested headers. Signature inputs use the exact canonical Intent digest under a separately negotiated standard signature/key policy. Signatures themselves are excluded from Intent's canonical record to avoid recursion. The foundation signature primitive is standard Ed25519 over that exact 32-byte canonical digest, using maintained libraries. The public key A and signature point R must each use canonical compressed encoding and represent a nonidentity point in the prime-order subgroup; malformed, low-order, mixed-torsion and noncanonical points reject. The signature scalar S must be reduced below the subgroup order, and the maintained Ed25519 equation verifier must succeed. Rust uses maintained Dalek point validation plus strict verification; Java uses maintained full-point validation plus the JDK verifier. The RFC 8032 public test key in the corpus is only a known-answer fixture, never an operational identity. Key generation, account/device binding, revocation, authorization and history/session admission belong to identity/runtime policy; primitive signature validity alone grants no operation authority.

## Closed record registry

Fields are encoded in exactly the order listed. `Payload` below is the helper record, not its protobuf representation. `Required` is part of every semantic record so unknown required meaning cannot be stripped. `Revisions` means a map from raw AggregateId to u64 AggregateRevision. `Modules` means a list of Module records sorted by ProviderId. Other lists retain semantic order unless an ordering is stated.

| Domain/type | Record | Ordered fields |
| --- | --- | --- |
| 1/1 | RequestBinding | RequestId:id, AccountId:id, DeviceId:id, UniverseId:id, HistoryId:id, operation_kind:u32, payload_digest:digest |
| 1/2 | Intent | binding:RequestBinding, expected_revisions:Revisions, payload:Payload, expiry_tick:optional<u64>, required:Required |
| 2/1 | WorldSnapshot | UniverseId:id, HistoryId:id, event_seq:u64, world_tick:u64, ruleset_hash:digest, aggregate_revisions:Revisions, entities:list<Entity> sorted EntityId, module_state:Modules, scheduled_events:list<Schedule> sorted schedule_id, reservations:list<Reservation> sorted OperationId, unresolved_operations:list<Status> sorted OperationId, deduplication_state:list<Dedup> sorted RequestId, required:Required |
| 3/1 | Proposal | JobId:id, ProviderId:id, provider_version:u32, ruleset_hash:digest, input_hash:digest, runtime_session:u64, read_set:Revisions, write_set:Revisions, proposed_events:list<Payload>, module_state_delta:Modules, required:Required |
| 4/1 | EventBatch | HistoryId:id, event_seq:u64, previous_hash:digest, authority_term:u64, world_tick:u64, ruleset_hash:digest, request_outcomes:list<Status> sorted OperationId, events:list<Payload>, module_state_changes:Modules, state_hash:digest, required:Required |
| 6/1 | Payload | schema_id:u32, canonical_body:bytes |
| 6/2 | Required | capability_ids:set<u32>, schema_ids:set<u32> |
| 6/3 | AggregateVersion | AggregateId:id, revision:u64 |
| 6/4 | Module | ProviderId:id, state_version:u32, state:Payload |
| 6/5 | Entity | EntityId:id, aggregate:AggregateVersion, state:Payload |
| 6/6 | Schedule | schedule_id:EntityId, due_tick:u64, event:Payload |
| 6/7 | Reservation | OperationId:id, state:Payload |
| 6/8 | Status | RequestId:id, OperationId:id, HistoryId:id, binding_digest:digest, phase:u32, outcome_tag:u8, outcome:<see below>, committed_event_seq:optional<u64> |
| 6/9 | Dedup | RequestId:id, binding_digest:digest, status:Status |
| 6/10 | Error | code:u32, reason:text, unsupported_capabilities:set<u32>, unsupported_schemas:set<u32>, retryable:bool |
| 255/1 | ConformanceFixture | optional_bytes:optional<bytes>, optional_text:optional<text>, integers:list<i64>, counters:map<text,u64> |

Status `phase` is 1 accepted, 2 pending, 3 succeeded, 4 rejected, 5 cancelled. Outcome tag is 0 absent, 1 Payload, 2 Error; no other tags are valid. Accepted/pending have absent outcome and absent committed event sequence. Succeeded has Payload and present committed event sequence; rejected/cancelled have Error, with committed sequence present only when their terminal outcome was recorded in the event log. The protobuf Status's EventSequence message is therefore optional presence, never an implied scalar zero. Error codes 1..10 correspond exactly to the protobuf ErrorCode enum. UNSPECIFIED/zero enum values reject for semantic records.

Snapshot `state_hash` is SHA-256 of WorldSnapshot canonical bytes and is omitted from its own canonical fields. The digest covers scheduled work, reservations, unresolved operations, deduplication and all state-affecting provider data. EventBatch's digest is SHA-256 of its canonical record; `previous_hash` is the previous EventBatch digest (all-zero only for the explicitly defined genesis predecessor). Its `state_hash` is the resulting WorldSnapshot digest. Proposal and payload digests use their own registered records. Event/list order is preserved because execution order can change state.

Capability and schema selector IDs are strictly positive, including optional handshake and unsupported diagnostic sets; selector zero is invalid. Optional unknown positive capabilities may be ignored and cannot alter canonical semantics. Capability ID 1 identifies the non-mutating foundation contract probe. The capability registry contains no other IDs. Payload schema registry v1 contains only schema ID 1, the ConformanceFixture record, for contract probes without state mutation. Its canonical_body must be one complete valid 255/1/1 record. Schema ID 0 and all unregistered IDs reject. Production operation kinds/payload schemas must be added with named field semantics and tests before use; an arbitrary bytes blob is never an authorized economic operation. Operation kind 1 is a non-mutating contract probe; unknown operation kinds reject. The closed envelope profiles can carry additional registered payload schemas in later revisions without treating protobuf unknown fields as semantics.

## Request identity and replay binding

Compute `payload_digest = SHA256(canonical Payload)` after validating its recognized schema and body. A request's binding digest is SHA-256 of RequestBinding. Store the durable dedup key `(HistoryId, RequestId)`, binding digest, full Intent digest and final/pending outcome together. A duplicate returns the existing operation only when both digests match and the authenticated principal matches. Any change to principal, universe/history, kind, payload, expected revisions, expiry or required semantics is a REQUEST_CONFLICT, before mutation. A RequestId previously seen in another history is never used to infer an outcome in the current history. Authorization is rechecked on query/retry; knowing an ID grants no permission.

Transport correlation numbers, arrival time, optional diagnostic metadata and protobuf field order cannot influence validation/authorization/expiry/execution or the canonical digest. Semantically relevant additions require a recognized schema/capability, signed Required entries and a registered canonical field. Stripping Required on either the envelope or nested record is invalid; the envelope's required sets must equal the body's required sets where that body declares them.

## Limits and failure rules

Engineering defaults are maximums, not performance measurements. Peers negotiate component-wise minima and cannot raise these implementation hard caps through handshake input.

| Resource | Hard cap |
| --- | --- |
| Control frame, before allocation | 1,048,576 bytes |
| Snapshot chunk data | 262,144 bytes |
| Per-direction queued bytes / items | 16,777,216 bytes / 256 items |
| Single decoded/canonical document | 1,048,576 bytes |
| One collection / all entries per document | 4,096 / 16,384 |
| Nested records / protobuf message depth | 32 |
| One text / Error reason | 4,096 / 512 UTF-8 bytes |
| One arbitrary bytes field | 262,144 bytes, except documented fixed widths |
| Required/optional capability or schema list | 64 entries each |
| Signature / PeerId multihash | 128 / 128 bytes |
| One snapshot transfer | 67,108,864 bytes, at most 256 chunks |

Length/count/depth checks occur before allocating or iterating declared lengths; additions/multiplications and cumulative decoded budget use checked arithmetic. Parser work is bounded by input bytes and entry count; decompression is disabled in v1, so no expansion-ratio exception exists. A wall-time deadline/cancellation belongs to the runtime shell and cannot change canonical identity. Bulk assembly has its own fixed transfer quota; a state larger than the document cap requires a registered segmented state representation rather than bypassing limits. This deliberately limits the foundation profile's world size.

Truncation, trailing bytes, invalid presence/tag/enum, unknown registry values, invalid UTF-8/non-NFC text, duplicates, noncanonical key order, size/count/depth overflow and digest mismatch reject before domain construction or mutation. Return a bounded public error without tokens, filesystem paths, private state or stack traces. Failure to produce even that error closes the session safely. Useful unsupported IDs are bounded to 64, numeric sorted unique values. Decoder fuzzing must assert bounded work/allocation and no mutation on failure, independently of generated protobuf decode success.

## Evidence and fixtures

`protocol/vectors/nf-canon-1.json` is the independent fixture corpus. Expected canonical bytes were assembled by `protocol/vectors/generate-fixtures.cjs` using Node Buffer primitives and SHA-256 from Node's maintained standard crypto implementation; this generator is not a production codec or an acceptance oracle for malformed inputs. Decimal integer strings preserve all 64 bits. Checked-in positive bytes/hashes, malformed encoded bytes and rejection classifications are the cross-language input.

Run `node protocol/vectors/generate-fixtures.cjs --check` to detect accidental fixture edits. This checks reproducibility, not Java/Rust agreement. Implementations must decode/reject the malformed corpus and produce positive bytes/hashes independently; neither implementation may generate its expected values using its own codec. The corpus also includes independent Ed25519 signatures over two Intent digests, generated and verified using standard Node crypto and an explicitly public RFC 8032 test seed/key. The separate negative_signatures corpus includes fixed malformed points/lengths, transparent mutations and two mixed-torsion valid-equation cases generated by an isolated maintained curve25519-dalek 4.1.3 review probe; its policy is rejection, independent of any permissive library verifier. NF-003 is still unaccepted until independent Java/Rust contracts, generation, bounded decoders and fuzz coverage pass.
