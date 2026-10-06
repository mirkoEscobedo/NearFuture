# NF-003 contract verification

Issue [#7](https://github.com/mirkoEscobedo/NearFuture/issues/7). Work date: 2026-10-07. This is the integration evidence for typed identities/counters, generated wire contracts, closed canonical codecs and signature primitives. The implementation revision and final review result are recorded after the checks below are refreshed on the published change.

The Rust domain crate is `no_std` with maintained SHA-256, NFC and Ed25519 libraries. The Java contract has the same closed registry; generated protobuf modules stay separate from the pure domain codecs. Canonical hashes use explicit NF-CANON-1 bytes, never protobuf reserialization. No decoder mutates state, authenticates a principal, opens a transport or records a durable operation.

Independent fixtures cover absence versus present-empty, integer extremes, ordered sequences and sorted maps, Unicode 13 text admission, request-binding changes, semantic records, required semantics, two digest signatures and fourteen adversarial signatures. Raw wire fixtures cover four positives, malformed/duplicate/oversized/unknown-required input and framed failures. Generators import neither production codec. Official Unicode data and license notices are checksum-pinned; both languages use the same admitted scalar ranges and reject input requiring normalization.

Fresh coordinator checks before Java's final repair gate:

| Command | Observed result |
| --- | --- |
| `cargo test --workspace --exclude nf-kernel --locked --offline` | All then-current contract, node and wire tests passed. The dependent kernel is under separate implementation/review. |
| `cargo clippy --workspace --exclude nf-kernel --all-targets --locked --offline -- -D warnings` | Passed before the final signature profile tightening; refreshed at final integration. |
| `cargo test -p nf-contract --test digest_signatures --locked --offline` | Four tests passed: independent positive signatures, tampering, mixed-torsion points and all shared adversarial signatures. |
| `npm run test:tools` | 36 passed, none skipped: architecture/build, runtime provenance and partial profiling tooling. |
| `npm run check:types`, `check:boundaries`, `check:schemas` | Passed, including all three artifact reproducibility checks. |
| `npm exec -- workspace-template verify .` | Passed its root Rust format/CLI-test plan with Windows Job Object ownership. This plan alone does not run the complete monorepo check. |

Meaningful public-seam RED/GREEN cycles include canonical absence/presence and ordering, malformed length/presence/header rejection, retry binding, status/dedup contradictions, wire singular duplication and mandatory presence, unknown required semantics and exact signature width. A finite 20,000-mutation profile campaign and 20,000-mutation semantic-record campaign preserve byte identity for accepted input; a 10,000-mutation wire campaign covers bounded raw decoding. These are finite adversarial tests, not a proof for every possible input or measured process-memory ceilings.

Independent review round 1 found hard-limit raising in Rust and Java gaps in zero selectors, cumulative embedded payload entry counts, exact registry pairs, weak-point signature admission, negotiated session binding, child status history binding and chunk/transfer limits. One bounded repair batch fixes those findings with regressions. Rust raised-limit regression first accepted raised ceilings in RED, then rejected them on both encode/decode in GREEN while preserving lower/zero limits. Maintained Dalek strict verification accepted the two mixed-torsion equation fixtures in RED; canonical nonidentity prime-subgroup checks on both key and R rejected them in GREEN. Java uses maintained full-point validation for that same profile. No handwritten curve arithmetic enters production or fixture generators.

Resource limits are explicit and may only decrease the registered hard caps. Wire preflight guards generated decoding; count/depth/field/document limits and logical allocation budgets bound work. Those budgets do not certify a measured whole-process heap size. Unknown semantic fields fail closed; only bounded observation-only transport metadata can be ignored. Unknown required protocol/capability/schema values reject with static useful reasons. Unsupported operation/provider/payload versions cannot reach a mutation handler.

Residual scope: account authorization, key storage, durable deduplication, sockets, transfer assembly, cancellation execution and game campaign integration belong to subsequent issues. Retry checks establish exact binding equality after the caller authenticates and looks up the original history/request; they do not supply persistence. The baseline payload is a nonmutating contract probe, not an invented economic schema. Dependent kernel types require an explicit versioned capability/payload registration before wire use. Native game capability gates and actual profiling baselines remain unmeasured.
