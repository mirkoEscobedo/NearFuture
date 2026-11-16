# STRATCOM and Pandora consumer contract proposal

**Proposal NF-SOCIAL-UI/0.2, 16 November 2026.** For [#140](https://github.com/mirkoEscobedo/NearFuture/issues/140), under the current [#92 delivery selection](https://github.com/mirkoEscobedo/NearFuture/issues/92). The operation names and limits below are a concrete consumer proposal, not a published SDK, adopted portfolio standard, or permission to implement it.

## Decision and implementation status

| Kind | Current record |
| --- | --- |
| User-selected ownership | STRATCOM owns social state/services, identity/persona, rooms, membership, history, durable outbox, delivery and recovery. Pandora owns presentation, including native Starsector controls and focus/input. NearFuture owns Java/game adapter, scheduling, campaign/save invalidation and game authority. |
| Proposed | The minimum negotiated service and native host boundary below; exact encodings, packages and numeric limits await the owners. |
| Owner alignment / still proposed | [Pandora](https://github.com/mirkoEscobedo/NearFuture/issues/140#issuecomment-6924648921), [shared networking r1](https://github.com/mirkoEscobedo/NearFuture/issues/140#issuecomment-6924686994) and [STRATCOM](https://github.com/mirkoEscobedo/NearFuture/issues/140#issuecomment-6924722369) now endorse the value/intent boundary, separate lifetimes, truthful facts and service-owned recovery. Their concrete interfaces/limits remain proposals; NearFuture consumer acceptance and a frozen cross-owner contract are not recorded. |
| Cross-owner agreed | No frozen technical interface or NearFuture consumer acceptance is recorded. Limited owner endorsements above do not imply agreement on packages, wire binding or numeric limits. |
| Implemented / integrated | This document implements no product behavior. Actual compatible STRATCOM service and Pandora native components have not been identified for this contract. Existing source vocabulary is evidence only. |
| Current / next-ready / held | Reconciled proposal and [#25 plan](chat-first-integration-plan.md) ready for PM review; resolve the remaining component/binding questions next. #25 implementation waits for supported #99 native route and available agreed components. #139's content-neutral probe proceeds independently. |

GM coordinates the existing STRATCOM, shared-networking and Pandora owners. This proposal does not dispatch another owner, select a shared carrier, install services or create another social backend/renderer. Academy fiction grants no infrastructure authority.

The selected implementation path is incremental rename/evolution from scom to stratcom, coordinated by the dedicated GM coordinator "Coordinate SCOM to STRATCOM migration" through the existing SCOM PM, preserving unpublished work. The existing ProjectSynergy STRATCOM owner coordinates product, contract and reuse. Old scom pins remain source/reuse evidence. A repository rename does not establish compatible packages/APIs or authorize package, wire, storage or data migration; those concrete contracts remain proposed until agreed and implemented. NearFuture owns only the adapter/consumer and performs no backend work or unilateral API/package rename. Its PM relays exact component requirements to the dedicated coordinator; no PM replacement or duplicate dispatch.

## Minimum service contract

Use STRATCOM's proposed logical facade, working name stratcom.social/1; it is not a released SDK. Names/codec remain owner-selected. The consumer recommends the following semantics for PM acceptance, without selecting a portfolio wire format or carrier.

| Operation / boundary | Proposed minimum request and result | Authority and lifetime |
| --- | --- | --- |
| Negotiate and open | Negotiate API/framing range, implemented capabilities and finite limits; authenticate intended service/profile and client installation before social disclosure. Open/renew a scoped leased session with grant epoch, service build/boot identity and store continuity. Signed-record/canonical and provider versions are separate compatibility axes. | STRATCOM grants the local consumer; bind negotiation to authentication. An endpoint/loopback address is not authorization. No social keys, raw database, arbitrary network/filesystem ports or credentials exposed to Pandora. Same-JVM mods are not sandboxed by this boundary. |
| Associate persona | List/select pre-enrolled authorized personas; return association revision and permitted scope. Missing association requires explicit setup/consent. Disclose revocation. Caller-supplied IDs and visible rooms confer no send authority. | STRATCOM owns identity, key transitions and linking. No persona derived from campaign seed, content hash, save name or game-history identity. STRATCOM constructs/verifies signed social records; NearFuture preserves canonical bytes and never re-signs them with game keys. Association changes invalidate old views/actions. Revocation discards queued unauthorized event payloads; current grants govern recovery disclosure too. |
| List rooms | Scoped opaque continuation cursor and bounded page; return room references, labels and allowed actions plus next cursor/snapshot revision. | STRATCOM determines membership, visibility and moderation. No invented global discovery or create/join policy in the first slice. |
| Read history | Room reference, page cap and opaque cursor; return immutable message IDs, author references, text, service ordering metadata, source/cache completeness and next cursor. Pair the snapshot anchor with an event checkpoint from one consistent cut; explicitly continue/complete even filtered-empty pages. | Service cursor is scoped to association/room and has explicit expiry/gap semantics. Stable IDs deduplicate pages/events. Display order is a service projection, not a global causal clock. Cached history is identified as cached. |
| Enqueue text | Service-issued submission namespace, stable operation ID, room, bounded UTF-8 text and expected association revision; derive author from the authorized persona. Return operation ID plus durable disposition or explicit refusal. Reusing the same ID with changed content/scope is a conflict. | STRATCOM atomically persists operation record, message and outbox, with semantic-payload conflict detection. Local acceptance is reported only after its durability condition. No NearFuture outbox, shadow database or local "sent" guess. |
| Query / recover operations | Use operations.get by namespace and operation ID without requiring a first-response message ID; operations.list recovers admitted submissions in bounded pages after restart. Declare dedup/result/namespace retention and continuity. | Scope survives UI/session/campaign replacement in STRATCOM. An interrupted request is uncertain. Expired/closed namespaces refuse unknown IDs; lost journal continuity invalidates old namespaces. No new-ID automatic retry or silent profile/persona substitution. |
| Read / resume events | events.read pulls bounded batches after an opaque event cursor, with optional bounded wait on an I/O worker. Return stable IDs and checkpoint through the last included event. | History-page and event-replay cursors are separate. Duplicates allowed; gaps are explicit. Overflow, expiry or continuity reset requires consistent resnapshot/replay plus operation reconciliation. No cursor advancement past omitted events. |
| Mute / block | Use the existing service operation and return its actual scope, persistence and effect; expose only supported capabilities. | STRATCOM defines sender filtering/moderation. Do not relabel a receiver-local mute as remote blocking. Unsupported controls are unavailable. |
| Close / release | Idempotently release this subscription/session/lease; acknowledge closed or already closed. | Closing a panel does not delete social history, cancel accepted sends or terminate unrelated clients. Closing the game stops only its owned child or releases its own lease. |

Required initial capabilities are session/association, rooms, paginated history, durable enqueue, operation query/recovery, and bounded event/resync. Relevant mute/block capabilities follow the actual service. Presence, attachments, read receipts, discovery and richer moderation are later capabilities; no automatic read receipts.

### Uncertain outcome and recovery

For a concrete example, enqueue operation A for persona P/room R, then lose the response. Keep A while the caller is alive; query A and reconcile its returned record. Do not generate B as an automatic retry. A repeated A with identical payload returns its recorded operation; altered payload returns conflict.

After caller restart, NearFuture reconstructs admitted submissions from STRATCOM's operation pages rather than persisting a second pending queue. Scope the idempotency tuple to store continuity, authenticated client/profile/persona, submission namespace and operation ID. Exact retries preserve the same semantic payload and current disclosure grants. This is at most one local durable admission per tuple, not end-to-end exactly-once delivery.

STRATCOM's proposed durable namespace/retention mechanism supplies the recovery boundary. A missing record is still only an observation: an in-flight original request could commit later. Reconcile recovery pages with the consistent event checkpoint; query or retry only the same valid tuple. Expired/closed namespaces must reject unknown IDs. Lost journal continuity returns an explicit continuity-lost result rather than accepting an old tuple as new.

A timeout/cancel/deadline after dispatch may leave an accepted send. Query/recover and retain uncertainty until the service supplies evidence. Definitive pre-enqueue refusal is distinct. Unknown/expired outcomes never trigger automatic fresh-ID resubmission, even after a snapshot. A new user intent requires an explicit decision informed by uncertainty. Unadmitted transient drafts are not automatically sent after restart.
### Truthful state projection

Service availability and each operation's delivery facts are separate axes; losing connection must not erase a known queued/delivered record.

| Display fact | Evidence required | Does not establish |
| --- | --- | --- |
| Unavailable | No compatible authorized local session, service absent/disconnected, or required capability missing; include reason and reconnect availability. | No remote peers, or failure of an already accepted operation. |
| Local queued | STRATCOM confirms durable local intent/outbox record and operation ID. Remote peers may all be absent. | Carrier acceptance or recipient receipt. |
| Carrier accepted | Carrier-specific acceptance fact and scope from the service. | Peer/recipient delivery, read, or game commit. |
| Delivered | STRATCOM proposes a verified recipient-service acknowledgement that the exact message entered authorized durable social state. Advertise only when implemented; identify recipient scope and preserve partial/unknown coverage. | Every room member received, anyone read it, or any campaign effect committed. |
| Refused | Service supplies a refusal with stage, reason, retryability and whether any durable admission occurred. | Rollback of earlier accepted/delivered facts. |
| Outcome uncertain / expired | Response lost or recovery/receipt horizon unavailable; display last known facts and uncertainty. | Safe duplicate resubmission or a fabricated successful/failed outcome. |

STRATCOM owns ordering, terminal failure/expiry, retention and recipient semantics. Do not collapse these facts into one monotonically advancing status enum: a recipient receipt can arrive while the carrier/status event is missing. Historical facts remain attributed to their actual layer.

### Proposed bounds and errors

These are initial consumer ceilings for review, not measured capacities. Effective limits are the intersection of service advertisement and host caps; negotiate down, never silently truncate a signed record or message. Reject oversize/over-capacity work before admission where possible.

| Resource | Draft ceiling / behavior |
| --- | --- |
| Text | STRATCOM draft: 2,048 UTF-8 bytes; lower native editor cap applies. No attachments or silent truncation. |
| Request / response / event batch | STRATCOM draft: 16 KiB request, 256 KiB response, event batch at most 64 records / 64 KiB. Fixed bootstrap caps apply before negotiation. Bound decoded bytes, nesting, IDs and cursors before allocation. |
| Rooms / history / operation page | At most 50 records, also byte-bounded; opaque cursors and explicit exhausted/reset response. |
| Calls / display | At most eight service calls; Pandora proposes one history call per selected room, one unresolved submit per composer, one campaign panel and at most 200 visible history rows / 256 KiB text. Display limits are separate from social retention. |
| Buffered events | At most 256 records / 256 KiB per client. Durable replay retention and operation/outbox budgets are separately finite and advertised by STRATCOM. Saturation refuses before admission where applicable; no NF fallback queue. |
| Native mailbox / intents | Pandora draft: 32 pending typed intents, 64 update notifications, at most 16 notifications per permitted callback. Add a finite decoded-byte cap per model/notification and a sticky resync flag outside the lossy queue. Coalesce replaceable snapshots; recover operation facts separately. |
| Native time bound | NearFuture must set/measure the permitted callback budget with Pandora/#99; counts alone do not guarantee frame time. No forced preemption of one handler or unbounded layout work. |
| Cancellation / deadline | Finite call/long-poll/session budgets negotiated with owner; a proposed 5-second interactive call ceiling is not a remote-delivery promise. Use receiving-host monotonic duration accounting; no game-thread I/O, process waits or blocking exit flush. |

These reconcile the published owner starting profiles; they remain proposed, unmeasured limits. Use the lower service/native shared cap; reconnects cannot reset aggregate per-client/profile quotas. Missing required bounds do not mean unlimited.
Errors distinguish incompatible version, unsupported capability, unauthenticated/revoked association, forbidden room/action, invalid/oversize input, ID conflict, expired operation/namespace, continuity loss, storage failure, capacity/rate limit (with retry hint if supplied), cursor expired/overflow, stale generation, deadline/cancelled, service unavailable and outcome uncertain. Preserve the service stage and known-admission fact. Unknown required discriminants fail closed; optional fields follow the negotiated version policy. These names do not replace owners' existing wire errors.

## Pandora native view/action boundary

Pandora supplies the actual native Starsector presentation implementation and its API; NearFuture supplies host services and domain authorization. React, Flutter and GPUI widgets/native pointers are not assumed portable into Starsector. Packaging Pandora-owned source here does not transfer UI ownership.

| Host port | Proposed contract |
| --- | --- |
| Attach | On a supported game/UI callback, attach a Pandora-owned native panel to an explicit host surface; obtain an idempotent detach handle and panel generation. The exact Java/API/token/resource entry point remains owner-selected. |
| Publish immutable view | Data-only bounded snapshot: view revision; service/session/continuity epoch, panel/campaign/association generations and room-query revision; service availability; persona/rooms; one history page; operation facts; pagination/gap/loading/errors; capabilities. Optional authorized game context is separately labelled. No API objects, callbacks, sockets, futures or mutable game references in a snapshot. |
| Dispatch typed intent | Pandora emits select-room, fetch-page, enqueue-text, query/retry-same-operation, supported mute/block and close intents. Include intent/request ID, expected view/association/panel/campaign generations, room-query revision and deadline. Local intent admission is not durable service enqueue. Host validates capability, generations and bounds at dispatch and again before application. |
| Scheduling | Network/storage work runs off the game/UI callback. NearFuture marshals bounded completion data onto the specific permitted callback established by #99. Pandora creates/updates/disposes native controls only there. No arbitrary executor may touch game objects. Start with a campaign-only panel, with no combat overlay. Paused pumping must use a supported callback rather than simulation-time advancement. The host signals suspend/remove; hidden panels cannot dispatch. |
| Focus / input | Pandora owns key/text handling, focus capture/release, Escape/close, modal behavior and focus return. NearFuture grants attach/input only for the active host surface and reports loss of validity. Detach, game focus loss and replacement revoke panel input; no late callback or content update recaptures it. Consume only handled panel input; message text is plain text, not markup/commands. Native editor, hotkey, Escape and IME behavior require SDK evidence. |
| Detach / invalidate | Detach cancels view work and unsubscribes boundedly. Repeated detach is harmless. Closing/replacing a campaign increments the host generation before disposing the view; close/reopen also changes panel generation. Drop late results/actions for old generations, including same-save reload. Room selection also advances its query revision. Preserve bounded transient pending-operation references at client/persona scope across panel/room invalidation; they contain no old callback or game reference. Service accepted sends remain queryable. |
| Save boundary | Views, UI controls, subscriptions, clients, session leases, cancellation handles, action queues, focus state and runtime handles are transient and excluded from game saves. Domain save serialization remains NearFuture's responsibility. Social identity/history/outbox remain in STRATCOM's own lifecycle. |

Social lifetime is independent of campaign identity. Campaign replacement invalidates game context and its panel bindings, not persona/history or accepted social operations. A new panel rebinds from service snapshots. Pandora proposes a campaign-only first panel; menu/combat surfaces remain outside this slice. A value-only fence may revoke from another thread, while native disposal waits for the permitted callback without admitting new input. Save exclusion needs actual observation, not merely a transient field declaration.

The initial action allowlist has no cargo/economy/save/world writes. A message, transport receipt or arbitrary applet intent cannot become a game action. Future authorized gameplay actions need their own NearFuture validation and commit/recovery contract.

## Actual package and attachment gap

Recommend the smallest owner-supported STRATCOM service/client package and Pandora native source/package that meet the table. A bundled component need not include the marketplace. No global daemon, shared raw database, portfolio identity root or new deployment is implied.

The [STRATCOM owner's selected first increment](https://github.com/mirkoEscobedo/NearFuture/issues/140#issuecomment-6925762591) is an internal, UI-independent room/status read facade over an explicit retained provider, reused by shell and headless callers. It is not the external social service or Java binding. Memory fixtures do not establish durable headless storage, and slicing a full snapshot does not establish bounded persistent history. Atomic durable operation admission/recovery remains a separate STRATCOM behavior slice.

The neutral probe's response-body ceiling is 346 bytes and its parent lease is at most 60 seconds. These are intentionally separate from the proposed social pages and durable lifetime. A social binding requires its own explicitly agreed capability/version/limits and ownership profile; this proposal does not enlarge or repurpose the probe.

| Needed before #25 implementation | Known source / missing selection |
| --- | --- |
| STRATCOM service | Core vocabulary exists at the source pin below. Missing: actual runnable/embeddable component, revision/package/artifact, supported Windows/architecture/runtime, Java-to-service binding/encoding, minimal dependency closure, authorization and recovery support. A Rust core or generic loopback port alone is insufficient. |
| Pandora native component | Inspected package pins provide React and Flutter metadata/spec conventions. Pandora reports no native controls or Java token adapter in its inspected tree. Missing: actual Starsector-native implementation/spec, revision and exported attach/update/detach/action API; game/API/mod compatibility, tokens/resources and focus contract. Pandora proposes a tiny resolved, revision-pinned native token subset with game-unit/font mapping; no runtime Node/Dart dependency. |
| NearFuture host | #139 composition and #99 native adapter own this work. At inspected dev, the minimal plugin exposes build identity only. Missing accepted native attach route, lifecycle/save exclusion and relevant AoTD Theory of Toolbox patched-API evidence. |
| Attachment and shutdown | Owner must select game/launcher-owned child or authenticated lease on an existing host. Specify launch/endpoint discovery, startup readiness, component/version verification, per-consumer auth, lease expiry, crash recovery, and stop authority. Existing-host profile releases only this lease; child profile stops only an ownership-verified child. Neither deletes service data at game exit. |

Prefer owned-child packaging for an initial standalone game installation only if the service owner supports it; otherwise use its supported scoped-client lease. Neither profile is selected by this proposal. No ambient localhost scan, automatic service install, global process kill, identity creation/migration or update daemon is authorized.

## Focused remaining owner questions for #140

The responses above answer the conceptual ownership/lifetime questions; do not redispatch them. GM coordinates the existing owners. The PM can recommend the semantic direction while reserving acceptance of actual bindings/limits.

1. **STRATCOM:** Name the smallest actual headless service/storage composition and package/revision, its owner tracking item, supported Windows/runtime and actual implemented provider/receipt scope. The proposed facade is not implemented by the cited domain cores. Which parts require a new bounded component task?
2. **STRATCOM + shared networking + NearFuture:** Map the logical API to #139/#99's authenticated local binding and independent API/framing/record/provider compatibility axes. Select supported owned-child/module versus existing-host lease; specify protected credential handoff, intended endpoint/profile verification, startup/readiness/renewal/reconnect/shutdown and store continuity. No generic localhost/raw-secret port.
3. **STRATCOM:** Identify the actual pre-enrolled persona/existing-room setup path, mute/block availability, service ordering/snapshot-event cut, queue/journal/replay budgets, durable namespace expiry and recovery/continuity semantics. Which exact implementation supports verified recipient receipt and what carrier/retention scope is honestly available? Review actual persisted old records before proposing migration.
4. **NearFuture + Pandora:** Name the #99 native panel/control and callback/input surface for the actual game/JVM/AoTD profile; confirm paused pumping, suspend/combat/save transitions, room-query fencing and independent service lifetime. Agree finite model-byte/callback limits. Which native observations remain?
5. **Pandora + NearFuture:** Name the first native component/spec/package owner and revision when implemented, including resolved token mapping, attach/update/detach/visibility/focus and immutable data/typed intent API. Agree the published starting bounds or record concrete amendments. There is no current native component revision to claim.
## Source ledger and evidence limits

Inspected 16 November 2026. Public source links below support the distinctions, not an end-to-end integration claim.

| Source pin | Relevant observation |
| --- | --- |
| NearFuture dev [7e0ee0f](https://github.com/mirkoEscobedo/NearFuture/commit/7e0ee0f28622e71ef7c0bae85636d75beb6427bc), [minimal plugin](https://github.com/mirkoEscobedo/NearFuture/blob/7e0ee0f28622e71ef7c0bae85636d75beb6427bc/java/adapter/src/main/java/nf/adapter/NearFutureModPlugin.java) | Source inspected; plugin exposes build identity, not this native chat route. |
| SCOM [scom-sync README](https://github.com/mirkoEscobedo/scom/blob/331b9e3494c57f50aef5f74269d0088dffdb201e/crates/scom-sync/README.md) | Sync bookkeeping and carrier facts are separate from room/chat ownership and recipient receipt. |
| SCOM [host capability types](https://github.com/mirkoEscobedo/scom/blob/331b9e3494c57f50aef5f74269d0088dffdb201e/packages/host-capabilities/src/types.ts) | Generic grants, loopback and transport ports exist; this file does not supply the required chat service/session API. |
| SCOM [chat README](https://github.com/mirkoEscobedo/scom/blob/331b9e3494c57f50aef5f74269d0088dffdb201e/crates/scom-chat/README.md), [room README](https://github.com/mirkoEscobedo/scom/blob/331b9e3494c57f50aef5f74269d0088dffdb201e/crates/scom-room/README.md), same revision | Separate domain cores and explicit receipt facts; not proof of a native consumer SDK or deployed service. |
| Pandora [React package](https://github.com/mirkoEscobedo/pandora-ui-kit/blob/2dd4dce595ed256835e294a10b6f13ef727c4ff3/packages/ui-react/package.json), [Flutter package](https://github.com/mirkoEscobedo/pandora-ui-kit/blob/2dd4dce595ed256835e294a10b6f13ef727c4ff3/packages/ui-flutter/pubspec.yaml), [spec conventions](https://github.com/mirkoEscobedo/pandora-ui-kit/blob/2dd4dce595ed256835e294a10b6f13ef727c4ff3/packages/specs/api_contract_conventions.md) | Package metadata and specification conventions are source evidence; no Starsector-native component is established by these files. |
| [#92](https://github.com/mirkoEscobedo/NearFuture/issues/92), [#140](https://github.com/mirkoEscobedo/NearFuture/issues/140), [#25](https://github.com/mirkoEscobedo/NearFuture/issues/25) and their current comments | Current 16 November selection governs. Retained historical failures and partial results are not erased or promoted to acceptance. |

Checks for this docs-only change: path/link consistency, public source-pin reads, explicit proposal/agreement/implementation status and preservation of existing work. No behavior tests, component build, native check or network-delivery experiment is claimed.
