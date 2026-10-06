# Rust wire implementation evidence

NF-003 integration acceptance remains with the coordinator; these checks cover the Rust wire slice, not authentication, durable operations, transport or licensed-game integration.

`crates/nf-wire` generates maintained Protobuf Rust contracts and descriptors at build time using exact prost/prost-types/prost-build 0.13.5 and protoc-bin-vendored 3.2.0. Build configuration uses `Config::protoc_executable`, with no unsafe environment mutation or hand-written generated wire classes. `nf-contract` remains a separate pure `no_std` canonical/domain crate.

The strict boundary borrows bounded input before generated decoding, traverses trusted generated descriptors, checks shortest varints/types, duplicate singular/oneof/packed occurrences, unknown semantic fields, UTF-8 and the shared Unicode 13 scalar/NFC predicate, fixed IDs/digests, counts/depth/cumulative budgets and mandatory presence. Only unknown observation fields inside TransportMetadata may be discarded. Converted domain records use the canonical crate's closed registry, so Intent payload digests and Snapshot state hashes are validated without hashing protobuf reserialization. Unsorted/duplicate revisions and ID-keyed lists, Required mismatch/unknown IDs, contradictory status and unknown provider/payload versions reject.

TDD evidence at public decode seams:

- Duplicate-singular case was RED when the scaffold returned a default generated value, then GREEN after descriptor preflight.
- Empty-envelope case was RED before mandatory semantic presence, then GREEN.
- Shared raw corpus was RED on unknown-required capability 999 being admitted, then GREEN after semantic/canonical conversion. An unrelated transient missing-source module during concurrent canonical work was not counted as RED evidence.
- A 65-byte signature was admitted in RED; the closed baseline was tightened to empty/absent or 64-byte Ed25519 shape in GREEN. Signature shape grants no authenticated authority.

- Independent review round 1 found optional capability selector zero bypassing the positive-ID rule. The shared raw corpus case was RED with an admitted Handshake, then GREEN after a nonzero check; this was the first semantic repair round.

Verification commands:

```powershell
cargo test --offline --locked -p nf-wire
cargo clippy --offline --locked -p nf-wire --all-targets -- -D warnings
cargo fmt -p nf-wire -- --check
node protocol/vectors/generate-fixtures.cjs --check
node protocol/vectors/generate-wire-fixtures.cjs --check
```

The current Rust suite has 12 tests across seven behavior modules, covering four independent raw positives, 33 malformed raw cases, three framed failures, bounded resource lowering, independent Snapshot hash integrity, chunk quotas, revision ordering/duplicates, pending/frontier contradictions and a deterministic 10,000-mutation raw-wire campaign. The campaign permits valid semantic mutations and rejects malformed ones; it is finite mutation testing, not proof against all possible inputs or a long-running coverage-guided fuzz service.

All test files are below the 500-line warning budget; handwritten production files are below the 800-line warning budget. Strict Clippy passed. Exact final command results should be refreshed by integration when shared canonical or schema files change.

Residual scope: no socket/session authentication, key authorization, durable deduplication, cancellation handler, cross-chunk assembler, persistent mutation or game launch exists in this crate. Schema/generated values cannot certify those later runtime/identity/storage gates. A valid signature primitive and a valid wire record are separate from permission to execute a request.


Shared Java verification evidence reported by its author (independent review remains a separate gate):

```powershell
.\java\gradlew.bat -p java syntheticCheck --offline --no-daemon
.\java\gradlew.bat -p java syntheticCheck "-PtestJavaExecutable=$env:STARSECTOR_HOME/jre/bin/java.exe" --offline --no-daemon
npm run check:types
npm run check:boundaries
npm run check:schemas
```

Both Java test-runtime runs passed before independent review repair. The compiler is pinned JDK 21.0.8 and bytecode targets Java 17; the second command executes tests on the installed game's Azul 17 runtime. Maintained generated Java Protobuf contracts use exact compiler/runtime 4.31.1, with verified Windows/Linux compiler artifact checksums. Findings from independent review require fresh focused and full-suite verification after repair; these earlier green suites are not acceptance evidence for those uncovered cases.