# Full-world implementation evidence

Status: independent semantic review R1 and isolated publication gate passed. No native capture or operation-authority claim. The frozen partial shadow baseline is commit6907179bd4f8b09366050f57bfa6522215271482; new work adds only owned world modules/tests/fixtures, additive lib.rs exports and these documents. A read-only diff over every frozen production source outside lib.rs was empty after implementation.

## Observed RED/GREEN

The first new public seam compiled closed and returned MissingFact when the behavior test required same-admitted-world diagnostic comparison. After the separate complete profile/guard implementation, that test passed and rejected four current-world changes invisible to selected WarFacts: raw weariness, timer meeting, concern cooldown signed-zero and unused concern anti-repetition multiplier. The copied primitive remains diagnostic-only and was not called a baseline defect.

A second meaningful public RED showed Generate with Some(existing concern) returning Ok(WorldShadowEvaluation). The new explicit method/target rule then produced GREEN: Generate requires None (Some is Unsupported); Update/IsValid require an exact named existing instance (None is MissingFact), and metadata.concern_instance must match. Named Update with no_auto_generate=true still works; explicit Generate policy failure disables the optional session. The old world factory and old ShadowSession guard remain MissingFact.

## Field coverage matrix

The profile inventory and source exhaustive destructures cover every declared Snapshot/nested field. The tests distinguish valid variations from admitted constants and unsupported origins; hashing a rejected Snapshot is never a substitute for admission.

| Field family | Observed evidence |
| --- | --- |
| Provenance/source/config/runtime | Valid digest, universe/history, SID/frontier and observation changes alter digest; source pin, zero identity and unknown source changes refuse admission/token. All eight context digests plus provider/campaign/branch, subject/instance and scope are checked at acceptance. |
| Subject/factions | Subject with valid enemy adjustment, live/traits/alignment/multiplier, faction ID with updated references and multiplier absence alter digest. Trait reorder preserves hash; duplicate traits reject. |
| Relations | Directed pair order, relationship, disposition absence/value and hostile flag alter digest; faction rename updates both directional references. |
| Strength/markets | Strength order, fleet strength, market ID/reference, size and strength alter digest. |
| Weariness | Both presence flags, raw/adjusted values, enemies absence, filter.allow_pirates and minimum alter digest. |
| Priority rules | All three raw float fields alter digest. |
| Concern config | Enabled/no-auto-generate, ordered tags, cooldown/anti-repetition alter digest. Exact definition ID/class/module constants occur in independent bytes; unknown selectors refuse admission. |
| Action config | Enabled/shim/tags/chance/cooldown/anti-repetition and repetition None/value alter digest. Absence with valid action removal changes hash. Unknown ID/module refuse admission. |
| Concerns/actions | Other concern instance, ended/cooldown, action None, action instance/status/ended/meetings, target/market presence/reference, priority_base and modifier ID/kind/value/order alter digest. Whole concern sequence/order is committed. Invalid action class refuses admission. |
| Timer/draws | Meeting/advance_days, interval ID/elapsed/min/max/presence/order, draw purpose/raw f64/presence/order with valid reordinaling alter digest. Invalid ordinal rejects admission. |
| Extensions/definitions/registrations | Exact complete=true and empty lists are encoded in fixtures. Incomplete inventory, uncharacterized listener/direct-call origin or other definition rejects admission; no origin/registration variation can mint an admitted-world token today. Every nested field still has exhaustive encoder coverage. |
| Float/option/collection representation | Independent fixtures distinguish signed zero, disposition absence, empty/populated lists, trait set reorder and concern sequence reorder. Raw finite bits are committed without numeric normalization. |

`world_fields.rs` observes66 valid scalar/presence mutations, nine ordered-array mutations, seven identifier/collection families, and closed-definition refusals. `world_guard.rs` rejects17 complete owner-binding changes with identical numeric facts, wrong method/target, invalid SID and cross-session old results; an owner evaluate_war error on observed drift disables the old session. Read-only accept_war rejects supplied changed context without poisoning or changing session state. CapturedUnverified comparison remains ShadowOnly and assessment remains Unobserved. `world_parallel.rs` verifies immutable diagnostics with1/2/4 actual workers and reversed completion order across repeats.

## Independent fixtures and resource observations

`fixtures/generate-world.cjs` uses only Node standard crypto and an independently written fixed-field encoder. Six complete world cases, exact baseline bytes, one world-evaluation binding and its separately encoded copied-input digest match Rust. It imports no Rust code, Debug representation, serializer or Java expected-output columns. Existing generate-identity.cjs/identity-v1.tsv remain unchanged.

An actually admitted rich64-faction world with4032 directed relations and4070 draws reaches exactly8192 cumulative collection entries and hashes successfully. The next draw makes8193 and NexWorld admission returns Limit before any token: this is admission evidence, not an observed independent profile-budget refusal. Routine source has a borrowed Count preflight sink with no hashing or trait-reference allocation; only after the complete pass succeeds does the Hash pass sort at most64 borrowed trait references per faction. Every write still checks its byte ceiling and all counts/text use checked arithmetic. No routine profile Vec or captured byte exporter exists.

The4,194,304 encoded-byte and1,048,576 text-byte ceilings are implemented and source-inspected, but no admitted-world end-to-end fixture has independently exercised those exact ceilings. Do not report them as observed stress thresholds. There is no injected allocation/fsync/native fault claim in this pure scope.

## Fresh commands and outcome

```powershell
cargo fmt -p nf-nex-shadow -- --check
cargo test -p nf-nex-shadow --locked --offline
cargo clippy -p nf-nex-shadow --all-targets --locked --offline -- -D warnings
node crates/nf-nex-shadow/fixtures/generate-identity.cjs --check
node crates/nf-nex-shadow/fixtures/generate-world.cjs --check
npm.cmd run check:types
npm.cmd run check:boundaries
$env:NF_SHADOW_JAVA='H:/Games/Starsector/jre/bin/java.exe'
$env:NF_SHADOW_CLASSPATH_FILE='E:/github/NearFuture/java/nex-reference/build/differential-classpath.txt'
npm.cmd exec -- workspace-template verify .tmp/shadow-review --scope module --module node:. --timeout 180000
```

Fresh full crate checks passed29 public/unit tests; the optional differential test was intentionally separate and then actually passed all26 real Java17 raw-input cases. The native package-owned verifier reported both structured steps processOwnership=windows-job-object, status0, timedOut=false. Formatting, strict Clippy, both generators and repository type/boundary checks passed. A test-only single-element-loop Clippy failure was corrected before this strict GREEN; it was not a semantic result.

Snapshot equality is equality of supplied declared values, not evidence that the owner supplied the actual current native capture. Full aggregate read-set, G1 writer/coherence/thread/lifecycle, native source/JAR equivalence, MutableStat total priority/order, complete selection/player/multi-target policy and live side-effect success remain unavailable. The result has no actuator, IPC registry or promotion authority.

## Independent review and isolated publication

Reviewed implementation revision: `f60455a6319e0b4a6787a79d40be84bc374d3e14`. Coordinator R1 inspected the complete model/encoder inventory, private admitted-world and result construction, borrowed preflight, finite encodings, current-world/current-owner/method/target comparisons and independent goldens. Verdict PASS with no important semantic finding or semantic repair. Fresh native-supervised checks passed all29 public/unit tests plus the explicitly invoked Java17 differential test, both fixture generators, classpath export and strict all-target Clippy; all five steps returned status0, no timeout and windows-job-object ownership.

The isolated accepted-main-plus-commitment candidate then passed clean offline npm installation, full157 Rust/40 Node checks and Java checks, actual bundled Java17 control/bulk sockets13.77s and26-case differential0.20s, and locked offline workspace release. All six structured steps reported status0, timedOut=false and processOwnership=windows-job-object; verifier verdictPASS. These are headless test durations, not bridge latency. [Hosted Windows/Linux gates](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37554619689) passed, including mandatory Java21 socket/differential tests and locked release builds; [PR #57](https://github.com/mirkoEscobedo/NearFuture/pull/57) merged as ``19dfc4e85f1538a1388277577ac78a65f691015a``. No native writer/capture/authority gap is closed by this slice.
