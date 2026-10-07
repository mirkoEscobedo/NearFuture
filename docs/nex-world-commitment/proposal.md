# Complete admitted-world commitment proposal

Status: API/profile approved by coordinator; implementation complete in separate owned modules, pending independent review. The reviewed partial issue15 baseline and its NF-NEX-SHADOW-1 copied-facts bytes remain unchanged. This proposal adds an independent private diagnostic commitment for every field of the current `nf_nex_boundary::Snapshot`, including fields the selected war evaluator does not read. It grants ShadowOnly diagnostic comparison, never actuator permission, native capture certification or a source/JAR equivalence certificate.

The current world factory returns an inspectable `WorldWithoutCommitment` result and guarded acceptance returns `MissingFact`. That old public behavior remains unchanged even after the separate guard is introduced. Do not make the copied-facts primitive a publication bypass.

## Proposed ownership and dependencies

Implement only new `crates/nf-nex-shadow/src/world_commitment/{mod.rs,profile.rs,budget.rs,records.rs}` and `world_session.rs`, plus minimal additive exports in that crate's `lib.rs`. Own corresponding `tests/world_commitment.rs`, `tests/world_guard.rs`, support fixtures and an independent Node standard-crypto fixture generator under that crate. Existing selected evaluators, NF-NEX-SHADOW-1 encoder, Java reference, boundary admission, global manifests and wire registries stay frozen unless a separately reviewed integration requires an additive call.

Use existing no_std `alloc`, local nf-contract/nf-nex-boundary and exact sha2 0.10.9 without default features. No new dependency, Debug serialization, Serde oracle, custom cryptography, process runner or global NF-CANON/protobuf registration. Routine commitments use a bounded borrowed preflight pass and checked streaming SHA256, not a 4MiB allocation. Exhaustive struct destructuring without `..` and closed enum matches make future model growth a compile-time profile decision. The new profile is private NF-NEX-WORLD-1 version1; a separate private NF-NEX-WORLD-EVAL-1 binding joins its digest to the existing copied-facts digest. Independent fixtures describe these new profiles rather than modifying old goldens.

## Proposed public seam

```rust
// All fields private; callers can inspect digest/profile but cannot construct from a digest or bytes.
pub struct WorldCommitment { /* complete admitted-world digest */ }
pub fn commit_world(world: &NexWorld) -> Result<WorldCommitment, Unavailable>;

pub struct WorldShadowSession { /* original validated owner metadata and world digest; active */ }
pub struct WorldShadowEvaluation { /* private world commitment, method/target, copied evaluation */ }
impl WorldShadowSession {
    pub fn new(metadata: ShadowMetadata, world: &NexWorld) -> Result<Self, Unavailable>;
    pub fn evaluate_war(&mut self, world: &NexWorld, method: WarOperation,
                        target: Option<EntityId>) -> Result<WorldShadowEvaluation, Unavailable>;
    pub fn accept_war<'a>(&self, result: &'a WorldShadowEvaluation,
                         current_metadata: &ShadowMetadata, current_world: &NexWorld,
                         current_method: WarOperation, current_target: Option<EntityId>)
                         -> Result<&'a WarResult, Unavailable>;
    pub fn invalidate(&mut self);
}
```

The constructor accepts only an actually admitted immutable `NexWorld`. A naked Snapshot, untrusted byte string, externally supplied digest, copied WarFacts or public conversion from a copied evaluation cannot mint a world-bound result. Evaluation validates metadata provenance, subject and named instance against the exact world and extracts supported facts from it. It requires the original world digest and owner binding to match. A failure disables that session; refresh uses an explicitly new admitted world and new session, not an implicit reset.

Acceptance first validates the current metadata, current admitted-world quotas and selected method/target. It recomputes the full current-world digest, re-extracts current war facts from that same world, and computes the existing copied-facts digest. It compares original/result/current owner metadata exactly, original/result/current world digests, the original evaluation versus recomputed current copied-input digest, and method plus named target exactly. Generate requires target None; Update/IsValid require a named existing instance. Invalid Generate+Some is Unsupported; missing Update/IsValid target is MissingFact. A well-formed changed binding or world yields `Stale`; unsupported/missing facts remain their existing errors. Invalidation yields `Disabled`. No output is returned before all comparisons pass. The returned value is borrowed immutable diagnostic data, not an operation or apply certificate.

A successful comparison proves equality of the supplied immutable values under this private profile. The owner still must supply the actual current capture; a hash cannot observe a missed native writer or silently advance a runtime lifecycle. Session/load invalidation and G1 writer coverage remain separate unobserved obligations.

## Input completeness and promotion limits

The profile commits every currently declared Snapshot field listed in profile-v1.md, including extensions, alternate concern/action instances, raw weariness, unused timers, relations, strengths, market fields, flags and configuration. Preserve conservative observed sequence order where native order relevance is unproven. Only faction traits have declared membership-only semantics and canonical set ordering.

This is a whole-declared-Snapshot commitment, not an aggregate read-set and not a claim that Snapshot contains every native fact. The boundary still lacks selected peace predicates and precise draw purposes, so world-derived peace remains `MissingFact`. Unknown extensions/definitions remain admission failures; committing complete=true cannot manufacture verified extension coverage. Native G1/coherence/writers, MutableStat total ordering/aggregation, complete action selection, multi-target/player branches, source/JAR/runtime observation and live-effect success remain promotion blockers.
