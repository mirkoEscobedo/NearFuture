# Initial protocol artifact preparation checkpoint

This records the initial NF-003 (#7) specification-only checkpoint, before code integration. Subsequent Rust generation/decoding evidence is in [rust-wire-evidence.md](rust-wire-evidence.md); the coordinator records Java/Rust integration acceptance separately. The initial checks below remain historical artifact-preparation evidence, not the current complete implementation status.

Created artifacts:

- `protocol/near_future/v1/near_future.proto`: typed identity/counter wrappers and handshake, status, snapshot, proposal, event, error and chunk wire messages.
- `docs/protocol/nf-canon-1.md`: closed scalar/record registry, request binding, exact bytes/hash rules and limits.
- `docs/protocol/wire-contract.md`: generated decode boundary, required/unknown-field policy and lifecycle semantics.
- `protocol/vectors/nf-canon-1.json`: 8 conformance positive, 21 malformed, 7 binding, 6 semantic-envelope, 6 required-semantic and 6 invalid-domain-input cases.
- `protocol/vectors/generate-fixtures.cjs`: independent explicit Buffer assembly with standard Node SHA-256; imports no production code and contains no generic input validator.

Checks run on this artifact revision:

1. `node protocol/vectors/generate-fixtures.cjs --check` passed fixture reproducibility.
2. Existing `protoc-bin-vendored-win32` 3.2.0 binary (`libprotoc 31.1`) compiled `near_future/v1/near_future.proto` using `--proto_path=protocol --descriptor_set_out=.tmp/nf-protocol-v1.pb`; no schema errors.
3. Independent .NET `System.Security.Cryptography.SHA256.HashData` over the 21 checked-in canonical positive/binding/semantic byte vectors matched all expected hashes. Absent/present-empty hashes differed; seven principal/history/kind/payload binding variants had seven distinct digests. This verifies the hash fixtures, not domain decoding.
4. `git diff --check -- docs/protocol protocol` passed. The files were new/untracked at this inspection, so full tracked diff inspection belongs to integration before commit.

No new runtime behavior was implemented in this slice, so no TDD RED/GREEN claim is made. Golden bytes are deliberately external to later codecs. They must not be replaced with expected bytes computed by production Java/Rust encoders.

Acceptance work identified at that initial checkpoint:

- Pin maintained prost/prost-build and compatible Java protobuf runtime; generate contracts reproducibly for both languages and enforce freshness/schema compatibility in CI.
- Implement independent typed domain conversion and all supported registered canonical records, not protobuf reserialization hashes.
- Run both language implementations against positives, malformed cases, Required cases and request-conflict cases; cover nested semantic fields beyond the minimal empty semantic envelopes.
- Implement and fuzz bounded canonical/wire decoding. Verify duplicate singular/oneof/map handling, unknown-required semantics and limits before allocation/mutation; generated protobuf decode success alone is insufficient.
- Implement authenticated/durable request deduplication and demonstrate conflict-before-mutation; a digest fixture alone does not establish persistence/authorization correctness.
- Supply reviewed revision/commands/results before closing #7 or ticking the roadmap. Licensed-game and transport gates are outside these artifact checks.
