# Data-only kernel (NF-004)

`nf-kernel` is a `no_std` library with bounded owned values and immutable views. Its only production dependencies are `nf-contract` typed identities and pinned SHA-256. No clocks, threads, game objects, storage, network, SQL, dynamic plugins or ambient random generator enter the library.

The miniature world holds factions, directed relation records with scores from -10000 through 10000, unsigned integer market credits, aggregate revisions, provider draws and cooldown ticks. Entity IDs and aggregate IDs are globally unique; references resolve to existing factions. Faction identity is fixed after genesis. Two statically dispatched provider implementations are available: `SyntheticPeace`, which raises a relation by 1–10 synthetic points, and `Manual`, which accepts an explicit relation or market adjustment. These rules characterize this test kernel; they do not reproduce Nexerelin AI or economy.

`World::new(WorldSpec)` validates and sorts constructor input. `World::view()` borrows immutable data. `World::to_spec()` returns a clone for explicit construction, never a mutable reference to the live world. Registry declarations identify implementation hash/version, state schema, principals, capabilities, read/write domains and event budgets. Only state/version 1 and the closed static kinds are supported. Implementation hashes are explicit deployment metadata; the composition root must supply a trusted binary-to-manifest binding. The library cannot authenticate that claim.

`OwnershipRecord` grants one provider a particular aggregate at a generation, activation sequence and ruleset hash. Both the target aggregate and provider-state aggregate require ownership before a proposal can write. Factions are read-only. There is no live ownership swap API; checkpointed migration and leadership policy remain composition responsibilities.

The public transition is:

```rust,ignore
let frontier = admit(&world, intents, authority)?;
// Persist encode_frontier(&frontier) before dispatching its exact jobs.
let proposal = evaluate(&frontier, job_id); // immutable, deterministic
let result = settle(&world, &frontier, results, authority, MissingPolicy::Pause)?;
// Committed { world: Box<World>, batch: CommittedBatch } is prepared data.
// Atomically persist batch + post-world + outcomes before publishing/acknowledging.
```

Admission freezes the complete command/job frontier and snapshot. It rejects duplicate request, operation or job identities within that frontier and previously recorded operations/jobs. Jobs sort by `(provider ID, target entity ID, operation ID, job ID)` using raw ID bytes. Intents include principal, universe/history, provider, closed command and the exact expected read revisions. Invalid intent semantics remain jobs with recorded rejection outcomes when their frontier is settled.

Every job reads its target, provider module and referenced factions and writes exactly its target/module. Successful earlier jobs reserve their read/write sets. Overlapping read/write jobs lose with `Conflict` in stable order. Proposal validation checks manifest, capabilities, principal, ownership, exact revisions, input hash, implementation identity, session and the full deterministic static result. Static calculations are deliberately recomputed during validation in this minimal version; there is no measured offload benefit.

`MissingPolicy::Pause` returns the exact absent IDs and changes no world. `Recompute` evaluates only absent statically supported jobs locally. Unknown/duplicate results and changed world/authority context fail the whole transition. A failed provider result has `ProviderFailed`; deterministic provider errors keep their own reason. Invalid individual proposals produce persisted outcomes and no events. Reducers stage the target and module changes together; quantity/revision/draw overflow cannot leak a partial update. A final tick/event-sequence or retention-limit failure returns an error and publishes nothing.

RNG uses `SHA256("NF-RNG-1\0", seed, history, ruleset, provider, entity, world_tick, operation, draw_index)` with fixed widths; the first eight digest bytes are an unsigned little-endian number. Term, session, worker count, arrival order and wall clock are absent. Committed draws/cooldowns survive snapshots. This is a synthetic deterministic selection rule, with no claim of Java numerical fidelity or cryptographic unpredictability.

`encode_snapshot`/`decode_snapshot`, `encode_frontier`/`decode_frontier`, `encode_batch`/`decode_batch`, and `encode_intent`/`decode_intent` are closed kernel-local records described in [the schema](schema.md). `state_hash` and `intent_digest` use maintained SHA-256 over those bytes. `Frontier::input_hash`, `snapshot_world`, `authority`, `jobs`, and `Job::intent` expose read-only persistence inputs. `apply_batch` validates the before hash, exact sequence/tick, admitted identities, permissions, event shape and conflicts, then reduces committed events and checks the after hash. It does not execute providers or draw RNG. Authentication, durable ordering, leader fencing and choosing a recovery history belong to the driver.

The codec is an explicit **domain 7 extension of NF-CANON-1**, implemented only by this Rust kernel. The foundation `nf-contract` registry still rejects it. It is not a registered wire capability, operation or payload; the probe payload must never carry these bytes as an opaque substitute. A named kernel capability/payload schema and identical Java/Rust vectors are required before IPC or cross-language use. Storage may persist these local records behind its own atomic commit boundary.

Limits are 128 combined factions/relations/markets, 32 providers/registrations, 160 aggregate revisions/ownership entries, 64 exact commands/results per frontier, 64 principals per manifest, 4096 retained outcomes, 16384 total expanded frontier/snapshot entries, 1 MiB encoded record, and 256 KiB nested canonical byte field. Current providers emit exactly two events each. The constructor, decoders and admission check counts before expansion beyond these envelopes. Outcome compaction is absent: the kernel pauses with `Limit` instead of forgetting replay protection.

This implementation supplies no production strategic economy, reservation/native transfer semantics, journal durability, consensus, migration, or Starsector integration. [Verification](verification.md) records the executable synthetic acceptance evidence and remaining gates.
