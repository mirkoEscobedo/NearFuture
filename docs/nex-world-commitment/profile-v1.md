# Private NF-NEX-WORLD-1 proposed profile

Status: approved version1 implemented in new owned modules, pending independent review. Source shape inspected: `crates/nf-nex-boundary/src/{model,lib,admission,validation,float}.rs`. This profile commits a complete currently declared Snapshot admitted as NexWorld. It does not serialize Debug output or participate in NF-CANON, IPC payload schemas or durable authoritative state.

## Primitives and framing

Digest is maintained SHA256 over these exact bytes. Header is ASCII `NF-NEX-WORLD-1` plus NUL, followed by u16LE profile version1, u8 authority1=ShadowOnly, u8 runtimeCertification1=Unobserved, then the complete Snapshot below. Admission must report precisely these derived assessment values and an empty findings list before hashing. They do not certify native observation.

Boolean is u8 0/1. u8/u16/u32/u64 use their stated unsigned widths; all multibyte integers are little endian. FloatBits and DoubleBits encode their exact finite u32/u64 raw bits, including signed zero and subnormals; no widening, decimal rendering, normalization or epsilon. Typed IDs are their raw16 bytes. Digests are raw32 bytes. Text is u32LE byte length plus exact UTF8, nonempty, at most256 bytes, without controls, admitted Unicode13 assigned-scalar/NFC predicate from nf-contract; reject rather than normalize. Opaque raw IDs are not text.

Option is one u8 presence marker, followed by the value only for marker1. None differs from Some; optional strings may not be empty. Every collection starts with its u32LE element count. A record is its exact field sequence below; no optional field omission, padding, trailing metadata, unknown field or implicit defaults. Empty ordered collections remain explicit count0. Variant payload follows its u8 selector immediately.

Closed selectors: Observation Synthetic1/CapturedUnverified2; Module Diplomatic1/Economic2/Military3/Executive4; ModifierKind Flat1/Percent2/Multiplier3; ActionStatus Starting1/InProgress2/Success3/Failure4/Cancelled5; EnemyFilter AdjustedGetter1 followed by its allow_pirates Boolean. Each Rust match is exhaustive with no wildcard. Unknown future enum values require a new reviewed profile.

## Complete record field inventory and order

Each row is exhaustive, including fields currently unused by war evaluation. Parenthesized names identify the record type, not additional wire labels.

| Record | Exact field sequence |
| --- | --- |
| Snapshot | provenance(Provenance), subject_faction(text), factions(list FactionView), relations(list RelationView), strengths(list StrengthView), weariness(WearinessView), priority_rules(PriorityRules), concern_config(ConcernConfig), action_configs(list ActionConfig), concerns(list ConcernView), timers(TimerView), extensions(ExtensionInventory) |
| Provenance | source_commit(text), source_digest(raw32), runtime_digest(raw32), ruleset_digest(raw32), merged_config_digest(raw32), universe(raw16), history(raw16), runtime_session(u64), frontier(u64), observation(u8 selector) |
| Definition | id(text), class_path(text), module(u8 selector) |
| FactionView | id(text), live(bool), traits(set text), diplomatic_alignment(raw f32), priority_multipliers(list pair text/raw f32) |
| RelationView | from(text), to(text), relationship(raw f32), disposition(option raw f32), hostile(bool) |
| MarketStrength | id(text), size(u8), strength(raw f32) |
| StrengthView | faction(text), markets(list MarketStrength), fleet_strength(raw f32) |
| WearinessView | manager_present(bool), map_entry_present(bool), raw(raw f32), adjusted(raw f32), enemies(list text), filter(EnemyFilter), minimum_for_peace(raw f32) |
| PriorityRules | max_alignment_modifier(raw f32), positive_trait_multiplier(raw f32), negative_trait_multiplier(raw f32) |
| ConcernConfig | definition(Definition), enabled(bool), no_auto_generate(bool), tags(list text), cooldown_multiplier(raw f32), anti_repetition_multiplier(raw f32) |
| ActionConfig | definition(Definition), enabled(bool), shim(bool), tags(list text), chance(raw f32), cooldown(raw f32), anti_repetition(raw f32), repetition_id(option text) |
| ActionView | instance(raw16), definition(Definition), status(u8 selector), ended(bool), meetings_since_ended(u32) |
| Modifier | id(text), kind(u8 selector), value(raw f32) |
| ConcernView | instance(raw16), definition(Definition), ended(bool), cooldown(raw f32), action(option ActionView), target_faction(option text), market_id(option text), priority_base(raw f32), priority(list Modifier) |
| IntervalView | id(text), elapsed(raw f32), minimum(raw f32), maximum(raw f32) |
| RandomDraw | purpose(text), ordinal(u32), value(raw f64) |
| TimerView | meeting(u64), advance_days(raw f32), intervals(list IntervalView), draws(list RandomDraw) |
| Registration | class_path(text), origin(text) |
| ExtensionInventory | complete(bool), listeners(list Registration), direct_calls(list Registration), other_definitions(list Definition) |

Every Vec is conservatively an ordered sequence, including uniquely keyed faction/relation/strength/config/instance records, enemy/market/interval inventories, priority multipliers and modifiers, tag lists, draws and extension lists. Reordering any such sequence changes the commitment. Uniqueness and references are guaranteed by immutable boundary admission; the commitment rechecks its own quotas/text/provenance/assessment and does not discard observed order. Draw.ordinal must still match its observed index. Faction.traits alone is a membership-only set: validate bounded unique canonical text and the three recognized selectors, then encode in ascending unsigned UTF8-byte order. Reordered traits have the same commitment; duplicates fail before sorting. No automatic unordered-map conversion or sort of other arrays.

The admitted extension inventory is currently complete=true and all three lists empty, because NexWorld rejects uncharacterized findings. Those fields still have explicit encoding; they are not omitted. Nonempty or incomplete extension inventories cannot mint this public world token. Future admission expansion requires a reviewed profile decision and new fixtures; existing digest equality must not act as a source-coverage certificate.

## Bounded borrowed hashing

Hard ceilings: total encoded bytes 4,194,304; cumulative text bytes 1,048,576; cumulative collection elements 8,192, counting every element in every nested list/set/pair list once; text 256 bytes. These are stricter diagnostic hard ceilings, not authority or an expansion of admitted data. Counts and every numeric width use checked conversions/addition; overflow is Limit. No caller can raise these ceilings.

| Collection | Per-list cap |
| --- | --- |
| factions, strengths, each strength.markets, weariness.enemies, concerns | 256 |
| relations, timers.draws | 4096 |
| faction.traits, faction.priority_multipliers, concern tags, action tags, action_configs, each concern.priority, timers.intervals | 64 |
| extension listeners, direct_calls, other_definitions | 256 |

Do a first borrowed preflight/count pass over all fields, checking collection lengths before loops, text lengths before character scans/comparisons, cumulative quotas before nested work, closed enum selectors and exact final encoded size before any hashing or sorting copy. References and unique keys are guaranteed by the admitted immutable world; no second collection of copied reference maps is constructed. Reuse admitted structural semantics without reconstructing or cloning a Snapshot. The second pass streams checked small primitive/text slices directly to maintained SHA256; every write still checks byte quota. No routine 4MiB Vec allocation or text/record clone. Only bounded trait references (at most64 per faction) may be sorted after full preflight. No partial digest is returned after any error.

A diagnostic byte exporter is optional future surface, not routine commitment behavior. If needed, make it explicitly Synthetic-only, caller-requested and value-only, reuse the checked exact-size pass before allocation, and reject CapturedUnverified. Test-only sinks may inspect public synthetic fixture bytes without installing a filesystem export feature.

Implement field coverage with exhaustive destructuring of Snapshot and every nested struct without `..`, and exhaustive closed enum matches. A future model field or variant addition must break compilation until its profile treatment is deliberately reviewed. No reflection, inferred schema tags or automatic serializer fills new fields silently.

## Separate evaluation binding

Keep existing NF-NEX-SHADOW-1 encoding/digest byte-for-byte unchanged. The world evaluation binding is SHA256 of ASCII `NF-NEX-WORLD-EVAL-1` plus NUL, u16LE version1, raw32 world digest, raw32 existing copied-input digest, u8 method(Generate1/Update2/IsValid3), and option raw16 named concern target. The copied-input digest already includes full ShadowMetadata, including provider/campaign/branch, source/runtime/config/reference/corpus/implementation/capture-policy digests, subject and instance. Metadata provenance and subject must exactly equal the actual NexWorld, and metadata.concern_instance must equal the explicit target before binding. Binding duplication is intentional context separation, never a caller-supplied substitute for recomputing the actual current world and copied facts.
