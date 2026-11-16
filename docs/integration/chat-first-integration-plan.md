# Chat-first integration plan

**Prepared, not dispatched or implemented — 16 November 2026.** Bounded plan for [NearFuture #25](https://github.com/mirkoEscobedo/NearFuture/issues/25), following the current [#92 selection](https://github.com/mirkoEscobedo/NearFuture/issues/92) and [#140 consumer proposal](stratcom-pandora-contract-proposal.md).

The first connected outcome is room selection, bounded text/history and truthful delivery/recovery state in NearFuture's own native Starsector presentation, through the real STRATCOM service. NearFuture owns its controls/panels, presentation/controller, input/focus, game adapter, scheduling and campaign/save lifecycle. This plan authorizes no duplicate social backend.

The [superseding ownership clarification](https://github.com/mirkoEscobedo/NearFuture/issues/140#issuecomment-6926175610) replaces the historical Pandora-native-port/view-framework interpretation. Pandora owns shared application references, tokens/assets and components for supported application renderers; portable reuse here is optional. No Pandora Starsector port/package/release, universal widget API or separate NF/Pandora view framework is a dependency. Immutable bounded models, lifetime fences, save exclusion and recovery remain engineering input for NearFuture's internal implementation. This correction neither dispatches new implementation nor changes separately authorized work.

The selected implementation path is incremental rename/evolution from scom to stratcom, coordinated by the dedicated GM coordinator "Coordinate SCOM to STRATCOM migration" through the existing SCOM PM, preserving unpublished work. The existing ProjectSynergy STRATCOM owner coordinates product, contract and reuse. Old scom pins remain source/reuse evidence. A repository rename does not establish compatible packages/APIs or authorize package, wire, storage or data migration; those concrete contracts remain proposed until agreed and implemented. NearFuture owns only the adapter/consumer and performs no backend work or unilateral API/package rename. Its PM relays exact component requirements to the dedicated coordinator; no PM replacement or duplicate dispatch.

## Current, next-ready and held

| State | Work |
| --- | --- |
| Current | #139 source is integrated through PR141; #99 owns supported native adapter/lifecycle work. #140 retains social-service proposals and internal native engineering notes under the corrected ownership. Concrete external service API/limits/packages remain proposed. |
| Next-ready | Record actual required STRATCOM capabilities/binding and the supported native route; NearFuture plans its own native presentation and internal scheduling/focus/lifetime behavior. Optional Pandora references do not block this planning. |
| Held | #25 actual integration needs the supported native route, real required STRATCOM capabilities and NearFuture-owned native presentation. No Pandora-native release, full marketplace, broad networking completion, later simulation or local-shadow alpha prerequisite. |

Keep #139's content-neutral probe independent. Contract discovery is not a prerequisite for its early delivery. Conversely, a working probe does not establish chat service availability, a native panel, ordinary-exit/save acceptance or AoTD coexistence.

STRATCOM's [selected first increment](https://github.com/mirkoEscobedo/NearFuture/issues/140#issuecomment-6925762591) establishes an internal read facade, not the required durable external service or Java binding. Its shell/headless equivalence must name the actual provider; memory fixtures and snapshot slicing do not prove persistence or bounded history. The neutral probe's 346-byte response ceiling and 60-second parent lifetime remain unchanged; a later social binding needs its own agreed profile.

## Preconditions for the first implementation slice

Record the answer/evidence beside each item before calling #25 ready. Missing components return to their existing owners through GM; this plan supplies no delivery dates.

| Requirement | Owner / sufficient evidence |
| --- | --- |
| Supported native host | #99: selected Java/game callback and attach route, component/runtime identity, campaign replacement/save exclusion, owned shutdown and relevant Starsector/AoTD Theory of Toolbox patched-API profile. Distinguish headless source preparation from live observations still missing. |
| Reusable client route | #139/#99: agreed module/capability/limit negotiation and authorized client connection. Reuse actual host composition; social capability does not activate game authority. |
| Actual service | STRATCOM: concrete component/package/revision, binding and launch/lease/auth API; supported initial operations, durable outbox and uncertain-response recovery. Generic Rust types or loopback grant alone are insufficient. |
| Actual presentation | NearFuture: its native controls/panels, controller, input/focus and game lifecycle implementation on the supported API. Optional portable Pandora references/assets create no native-package or release dependency. |
| Contract agreement | #140: owner answers for actual social delivery, stable IDs/retention/recovery, event overflow/replay, limits and service package/shutdown ownership. NearFuture chooses its internal view/controller boundaries. Record proposal, owner agreement and implementation separately; no shared view SDK is required. |
| Persistence compatibility | STRATCOM assesses preserved #79/#126/#106 source and any actual existing signed pending records. No automatic merge, live-store opening, migration or duplicate NearFuture outbox. If no affected data exists, record how that was established. |

## Bounded implementation sequence, once ready

The PM owns integration and overlapping-file assignments. Scope this sequence around available components; these are proposed slices, not additional dispatches.

| Slice | User-visible result and responsibility | Finish evidence |
| --- | --- | --- |
| 1. Compose service and panel | NearFuture implements its native controls/focus and binds the supported host/client route; STRATCOM supplies authorized session/association and social service. Show unavailable if a required service capability or supported native route is missing, keeping normal campaign play usable. | Actual service/native source and package profile; compatible/incompatible negotiation; owned child versus leased-host release; no game-thread I/O or second store. |
| 2. Rooms and history | Select authorized rooms; paginate bounded service history. Preserve stable message IDs, source ordering and cached/gap/empty distinctions. Connect actual supported mute/block semantics with truthful labels. | Pagination and event/page overlap do not duplicate display; membership/refusal and unsupported features are visible; bounds hold. |
| 3. Send and recovery | Enqueue once with a stable operation tuple (continuity, client/profile/persona, namespace and ID); display local queued, carrier accepted, recipient-scoped delivered or refused facts. Recover uncertain responses using the same ID and STRATCOM's durable operation list after restart. | Lost response and reconnect produce one admitted intent; dedup/retention and expired-namespace refusal and store-continuity behavior are evidenced. If those semantics are unavailable, the slice remains incomplete. |
| 4. Native lifecycle and failures | On panel close/campaign switch/reload, invalidate old generations, release focus and unsubscribe without erasing social state. Reattach from service snapshots and operation recovery. | Native detach/reopen, same-save reload, campaign replacement, paused campaign/suspension profile, service loss/restart, overflow/reset and ordinary game exit observed at declared scope. |

These slices stop at connected chat. Presence, attachments, broad discovery, richer moderation and marketplace surfaces wait for their actual service capability and separate useful scope. No economy writes, cross-save transfers, world simulation, renderer selection or provider/identity migration is included.

## Acceptance and evidence matrix

Use focused behavioral checks when implementation exists. This docs-only plan does not run them.

| Scenario | Required observation |
| --- | --- |
| Service absent / wrong version / denied association | Native unavailable/refusal reason; no invented queued/send state; campaign remains usable. No automatic install, alternate backend or raw credentials exposed to presentation. |
| Service present, remote peers absent | Service-owned durable local queued record and bounded cached history remain distinguishable from local service unavailability. No assumed permanent availability/retention. |
| Actual two-client text flow | In-game sender and recipient flow through the pinned real service and NearFuture-owned native presentation. Record client topology, carrier, supported network scope, recipient fact and reconnect behavior. Loopback echo, fixtures and LAN observations receive only their actual scope. |
| Response lost after admission | Query same ID; reconnect recovers actual record without a new automatic send. Caller crash requires service-owned recovery pages and namespace/continuity semantics; NOT_RECORDED is not proof that an in-flight original cannot commit. |
| ID conflict / expired recovery | Same ID with changed payload is refused; expired/unknown operation remains uncertain. Text equality is not deduplication. No automatic fresh-ID resend. |
| History/event replay / overflow | Duplicate IDs collapse; service ordering is preserved; cursor expiry/gaps/epoch reset mark stale and recover boundedly. No false global ordering for disconnected senders. |
| Refused/revoked/oversize/flood | Membership/identity/input limits and capacity/rate refusals are truthful; bounded parsing, queues and callback work avoid frame stalls. A grant revocation invalidates old actions. |
| Mute/block | Actual service filtering, scope, persistence and unsupported behavior are reflected accurately; local mute is not claimed as remote moderation. |
| Focus / detach / stale completion | NearFuture admits input only to its current valid view and implements Escape/focus return. Old view, campaign, association or room-query results cannot apply after invalidation, including reload of the same save. |
| Saves / campaign replacement | No UI controls, clients, subscriptions, futures, queues, sessions or runtime handles serialized in game saves. Social persona/history/accepted operations survive independently of campaign identity. |
| Shutdown / restart | Close only the ownership-verified child or this client's lease; unrelated service clients/data survive. Accepted sends remain recoverable per STRATCOM's retention contract. |
| Compatibility / authority | Record the tested Starsector/API/JVM/OS/mod profile, including the relevant AoTD patched API and absence of Nex/Market where applicable. No social message or receipt changes cargo/campaign/world authority. |

Headless adapter/contract checks establish only their asserted boundary. Full #25 acceptance needs actual in-game behavior with the real service and presentation, not a fixture or source review. Do not claim global communication from a LAN-only run. Missing native observations remain explicit.

Live game checks require a separately prepared complete scenario, saved plan/progress, serialized driver ownership, fresh foreground/focus and original image inspection. Read the current repository Starsector proxy/debugging instructions first and protect original saves and global autosave retention. This plan launches no process and sends no game input.

## Compatibility, preservation and rollback

- Preserve the historical #79/#126/#106 branches, failure evidence and signed-data semantics for STRATCOM's assessment. No branch is automatically merge-ready under the new ownership.
- Select one compatible service/presentation source closure and dependency profile; no unilateral portfolio ABI, global daemon/database/identity root or competing renderer. The whole marketplace is unnecessary.
- Stop accepting new UI intents before detach; fence old generations; close subscriptions and release the owned session. Cancellation does not retract an already admitted send.
- Missing/incompatible service or failed attach leaves the panel unavailable and campaign play intact. A runtime failure does not justify activating old NearFuture chat or silently rewriting service data.
- Disable/revert the adapter integration through the PM's ordinary reviewed source path if needed. Do not delete STRATCOM history/outbox, undo accepted remote messages or migrate existing stores as rollback.
- Keep evidence sanitized: source pins, scenarios, expected/observed outcomes and actual scope in the product record; private messages, local inventories, credentials, paths and session metadata stay in ignored local evidence.

## PM handoff and status updates

Before implementation, fill the #140 decision record with actual owner response links, chosen API/component revisions, supported subset and remaining gaps. After each slice, record returned result versus integrated commit/PR and native acceptance separately. No issue closure follows from this proposal.

For the current docs-only return, the next action is PM review/publication of the reconciled #140 consumer packet, then GM resolution of the remaining actual component/binding questions with the existing owners. #25 remains prepared/held. This document and the [contract proposal](stratcom-pandora-contract-proposal.md) are the only product-repository changes in this scope.
