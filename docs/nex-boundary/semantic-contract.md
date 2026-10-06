# Immutable semantic contract

The approved pure `nf-nex-boundary` crate depends only on local nf-contract. No NF-CANON/protobuf registration, game loading, fake Global/MarketAPI, decision algorithm or game mutation is introduced. Incoming snapshots are ordinary owned values; validated views expose immutable copies.

Java floats retain finite IEEE-754 single-precision bits through FloatBits(u32); supplied Java Math.random draws retain finite double-precision bits through DoubleBits(u64). There is no fixed-point conversion or source formula approximation. Non-finite inputs are explicit errors. Text/collection bounds and duplicate/reference checks are enforced before usability.

| View | Copied facts |
| --- | --- |
| Provenance | Pinned source identity, runtime/ruleset/post-merge config digest, universe/history/runtime session, observation frontier and synthetic/captured label |
| Faction | Stable semantic ID, live flags, traits, diplomatic alignment, concern-specific priority multipliers |
| Weariness | Raw and enemy-adjusted values, enemy IDs/filter policy, threshold, manager/map-entry presence |
| Relations/strength | Directed faction relationship/disposition/hostility facts; copied market/fleet strength where relevant |
| Configuration | Definition IDs, exact paths/module, enabled/auto-generation, tags, cooldown/anti-repetition and action settings |
| Lifecycle | Exact class identity, instance ID, ended/cooldown/current action/status/retention and existing priority entries |
| Time/RNG | Meeting/advance deltas, interval state, ordered double-width draw purposes/ordinals; a seed alone does not certify call ordering |
| Extensions | Definitions, listener/direct-call origins, writer coverage and explicit inventory completeness |

The initial handler accepts only the pinned warWeariness ID/class/module. The makePeace route has no available provider. Unknown/replaced definitions and unknown semantic extension tokens are errors, not dropped fields. Extension findings are retained explicitly. All successful views remain SHADOW_ONLY and capture/reference/suppression certification UNOBSERVED.

Tests use synthetic copied facts to establish representability, preservation and explicit rejection. Real captured identity/coherence awaits #11; source-oracle and lifecycle/algorithm equivalence await #14. No caller-supplied label can promote authority.
