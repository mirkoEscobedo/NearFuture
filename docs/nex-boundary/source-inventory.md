# Pinned source and writer inventory

All links pin Nexerelin commit `a669f4d0740e95a4acbb6b894dde09ade67aa754`. SHA-256 values describe exact public blob bytes inspected; no equivalence to the installed JAR is inferred.

| Source | SHA-256 | Boundary |
| --- | --- | --- |
| [WarWearinessConcern](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/ai/concern/WarWearinessConcern.java) | `dec8ddaea552f71669e8c8be6b661b6650a24c944833e1b26088a0cee5b42eca` | Exact-class duplicate guard, threshold/end, validity, priority |
| [BaseStrategicConcern](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/ai/concern/BaseStrategicConcern.java) | `f5750c1bc6503c41d391d225b04baa1f5d640134f9c937daafd11291862506ea` | Mutable-stat history, cooldown, action abort, inherited modifiers |
| [StrategicAIModule](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/ai/StrategicAIModule.java) | `7d7f71469bc869fc1f62e71bf1c9982e73721b016ec66dbe2cc8e6b65e38accf` | Generation, advancement, removal, ended-action retention |
| [StrategicDefManager](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/ai/StrategicDefManager.java) | `e446ec0f502c3f58c10451541a5dee3ea9dafe4126dfa10e8a2257c3dee9c15e` | Merged JSON, mutable maps, class-path instantiation |
| [SAIUtils](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/ai/SAIUtils.java) | `56bd7ce2c5603d24b33c5b2f307ebd566d372ffc5f01009b282e8810280e4ec3` | Alignment, traits, disposition, listener dispatch |
| [DiplomacyManager](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/DiplomacyManager.java) | `49bb9856a873ae6963839acb65ae994a6adf10d426301743314df556002f3435` | Weariness getter/storage, battle/capture/economy writers |
| [DiplomacyBrain](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/diplomacy/DiplomacyBrain.java) | `a8ec5be42031efaf2340bad199bdebbb435ed774ebfd7718e0b7ecbb50c04dcb` | Peace, RNG/timers, offensives, weariness reduction |
| [MakePeaceAction](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/ai/action/MakePeaceAction.java) | `d5d43828ea5391cc0bcc4079bc63594875aa77d0ec5ab903d95883fa7238e20d` | Mutating peace call; delegate may remain in progress |
| [strategicAIConfig](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/data/config/exerelin/strategicAIConfig.json) | `ea20d2a0a624172bd1c5e3884ba59bed4a6df1fe44974a88e5eb3f2d6c3ad783` | Baseline definition IDs, paths, tags/settings |

The first concern requires adjusted weariness and the current merged minimum-peace threshold. Duplicate detection considers active concerns of the exact Java class, not only the definition ID. Ending aborts a current action. Priority depends on inherited alignment/trait/config modifiers and existing mutable-stat entries. Lifecycle capture must retain those entries instead of assuming a fresh concern. The overridden update does not automatically execute every base-update listener callback.

`getWarWeariness(factionId, true)` lazily inserts a missing map entry and applies an enemy-count adjustment whose filter reads configuration. It is not a pure read. Background workers receive copied raw/adjusted facts and a capture policy, rather than invoking that getter.

| Original route | Narrow suppression requirement | Coverage |
| --- | --- | --- |
| `StrategicAIModule.findConcerns` | Gate selected ID plus exact mapped class; preserve unrelated definitions and listener dispatch | Hook UNOBSERVED |
| `updateConcerns` / `advance` | Fence only selected lifecycle at the same meeting/advance boundary, preserving unrelated concerns and ended-action retention | Hook UNOBSERVED |
| `BaseStrategicConcern.fireBestAction` and action generation | Gate selected action creation and characterize listener veto/priority effects | Handler/hook UNOBSERVED |
| `MakePeaceAction.generate` -> `DiplomacyBrain.checkPeace` | Replace only after complete selected peace decision characterization and fenced game application | Substitution unavailable |
| `DiplomacyBrain.update` fallback | It returns when `StrategicAI.getAI(factionId)` exists; removing the whole strategic AI would activate its `checkPeace(null)` fallback and disrupt unrelated modules | Guard identified; direct-call coverage UNOBSERVED |
| `modifyWarWeariness`, battle/capture/economy and peace reduction | Keep these world writers for read-only concern evaluation; capture a coherent frontier. Full weariness ownership requires separate coverage of every writer | Capture/frontier UNOBSERVED |
| Public mutable definition maps, `StrategicAIListener` callbacks and direct public peace calls | Enumerate loaded registrations/origins and direct callers; uncharacterized behavior prevents authority eligibility | Runtime enumeration UNOBSERVED |

Pinned mapping: concern `warWeariness` / `exerelin.campaign.ai.concern.WarWearinessConcern`, module DIPLOMATIC; action `makePeace` / `exerelin.campaign.ai.action.MakePeaceAction`. Modified tags/settings, unknown IDs and changed paths require explicit outcomes. Source JSON uses comments and game merge semantics; this slice does not invent a substitute JSON loader.

On 2026-10-07 a read-only installed text search found one `strategicAIConfig.json`, under Nexerelin-0.12.2c. Many mods are enabled. That search cannot see JAR-packaged definitions, executable registrations or actual merged settings. It does not establish a clean extension inventory. Capture must supply post-merge definitions, enabled contributors and current listeners, or explicitly report incomplete inventory.

RNG width: source Math.random calls in BaseStrategicConcern and DiplomacyBrain return Java double values. RandomDraw retains their raw u64 bits, purpose and ordinal; f32 narrowing is prohibited. WarWearinessConcern itself has no direct draw. Reproducing draw order still requires captured/reference evidence.
