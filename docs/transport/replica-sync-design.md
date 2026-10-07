# Profile1 immutable export and read-only replica design

Documentation-only proposal for independent root review. No protocol, role, codec, storage port, listener, archive or dependency is registered. Initial support is accepted `nf-store::Store` profile1 only. Issue39 and issue23 remain open. Receipt/control2 and notification/notify1 effect acceptance and portal composition are separate dependencies; existing registries remain unchanged.

Inputs are [live GitHub issue39](https://github.com/mirkoEscobedo/NearFuture/issues/39) (NF-035), read via `gh issue view 39 --repo mirkoEscobedo/NearFuture --json number,title,body,state,updatedAt,url` on 2026-10-07 (OPEN, last update 2026-10-06T21:50:18Z), cached issue23 (NF-019), design issue2 section9, design issue4 sections18–19, [portable-sync-plan.md](portable-sync-plan.md), [Local SQLite profile1](../storage/profile-v1.md), and actual `nf-store/src/{store,model,codec,schema,persistence,recovery,identity,transitions,mirrors}.rs`. Root selects receiver policy current PLAYER **and** REPLICA, with maintained DeviceProof verification. PLAYER alone, ARCHIVE, Noise identity and downloaded bytes do not authorize replica reads.

## Starting facts and exact meaning

Store1 has one local writer, ten STRICT tables, DELETE journal/EXTRA sync and a private closed state codec. `Store::snapshot()` returns state bytes, SHA256 digest and StoreRevision, but omits journal head and separate membership. `known_frontiers()` returns scope, EventSeq, StoreRevision and optional MembershipRevision. It supplies no persisted current authority term/lease. No public history export or replica admission API exists. Private recovery validates a consecutive journal tail<=256, exact replay, final state and mirrors. Compaction can prune older snapshots/journal/events.

| Name | Meaning |
| --- | --- |
| WorldHash | `nf_kernel::state_hash` of committed World; canonical authority-state comparison for this profile |
| StoreStateDigest | SHA256 of exact `NF-STORE-1` state bytes, including pending frontier, requests, reservations and outbox |
| JournalHead | Existing `NF-STORE-JOURNAL-1` SQL-transition chain head |

Kind1 Prepare persists intents/bound devices/reservations, advances StoreRevision/head, and leaves committed World/EventSeq unchanged. Kind2 Commit requires that frontier and exactly replays a canonical CommittedBatch, atomically updates World/provider state/outcomes/releases holds/outbox, and advances EventSeq. Kind3 OutboxAck removes an existing outbox record and advances StoreRevision/head without changing World/EventSeq. A committed SQL Prepare is not a committed canonical batch. Incoming prepared jobs, reservations, outbox and Pending requests remain **inert source metadata**. They cannot execute intents, run providers, debit assets, deliver native effects, become local queues or authorize retries. Only embedded committed World becomes the public replica view.

Membership CAS advances MembershipRevision in a separate transaction, not StoreRevision/EventSeq/JournalHead. There is no signed membership-transition archive. Transfer membership digest/frontier for consistency and authorization binding, never as imported policy. Both peers must already possess separately admitted current membership with exact same canonical digest/revision. Disagreement fences sync until a trusted local policy update. Historical Store records do not prove historical device authorization from current policy alone.

Export/admission must also compare actual WorldSpec universe/history/ruleset_hash with the trusted scope/ruleset. Profile1 WorldSpec has no content-hash field: content32 is the separately trusted operator/repository context bound by this protocol, not a claim that WorldHash commits the installed game content. Unknown native state and source-JAR/capture equivalence remain outside this kernel profile.

A replica exposes committed World and privately retains read-only source metadata for exact continuity. It has no conversion to writable Store, MembershipRepository, job submission, DurableAck, quorum decision, authority grant or native projection. Report StoreStateDigest equality only after complete metadata replay; WorldHash alone does not establish whole-Store or policy equality. No profile1 AuthorityTerm/AuthoritySession minimum is fabricated. Profile2 and writer transfer are Unsupported. The configured trusted source attests stored state; this does not prove a historical authority lease or protect against a malicious trusted source.

## Ownership and additive ports

A new SyncOwner consumes the already-open ReceiptRepo, one replica installer and two private Swarms. The finite first CLI slice replaces the receipt portal; it neither opens another writable Store beside ReceiptRepo nor adds unaccounted lanes to its three-Swarm owner. Source and receiver each need their own already admitted local Store/membership/vault. Replica installation writes a separate cache database with a read-only strategic API, never the local authoritative Store. Current-policy reads use the sole ReceiptRepo.

Proposed new types/signatures, not existing APIs:

```rust
pub struct ImmutableExportPlan { /* private captured binding and retention token */ }
pub struct ExportProgress { /* bounded observations only */ }
impl Store {
    pub fn begin_replica_export(&mut self, request: ExportRequest)
        -> Result<ImmutableExportPlan>;
    pub fn export_step(&mut self, plan: &mut ImmutableExportPlan,
                       output: &mut OwnedExportSpool) -> Result<ExportProgress>;
    pub fn release_export(&mut self, plan: ImmutableExportPlan) -> Result<()>;
}
pub struct ReplicaStore { /* private active cache; read-only strategic API, not Store wrapper */ }
pub struct ReplicaInstaller { /* one owned staging generation */ }
impl ReplicaStore {
    pub fn open_existing(path: &Path, known: &ReplicaKnown) -> Result<Self>;
    pub fn view(&self) -> ReplicaView<'_>;
    pub fn anchor(&self) -> ReplicaAnchor;
}
impl ReplicaInstaller {
    pub fn begin(config: ReplicaInstallConfig, manifest: VerifiedSyncManifest,
                 active: Option<ReplicaStore>, known: &ReplicaKnown) -> Result<Self>;
    pub fn admit_document(&mut self, verified: VerifiedSyncDocument) -> Result<()>;
    pub fn previous_view(&self) -> Option<ReplicaView<'_>>;
    // No public activate/current-policy constructor on this type.
}
impl Store {
    pub fn issue_replica_activation(&mut self, installer: &mut ReplicaInstaller)
        -> Result<ReplicaActivationChallenge>;
    pub fn activate_replica(&mut self, installer: ReplicaInstaller,
                            evidence: ReplicaActivationEvidence)
        -> std::result::Result<ReplicaActivation, ReplicaActivationFailure>;
}
```

Repo delegates narrowly; handlers receive no mutable Store/connection/signer closure/private path/supplied MembershipState. Cross-crate activation is an explicit public **Store method**, not access to a private nf-store method/type from nf-transport. `CurrentReplicaAdmission` is constructed and consumed entirely inside nf-store. ReplicaActivationEvidence is untrusted typed data: receiver DeviceProof, pinned source final DeviceProof and exact closed source/receiver transcript fields. It has no authorization boolean or public conversion to the private admission. Store validates/reconstructs those preimages against the installer's private manifest/target/profile/pin/configuration; nf-store owns the additive pure evidence shapes/preimage writer, which nf-transport can use without a reverse dependency on transport/libp2p. Peer identifiers at this boundary are bounded exact bytes; canonical PeerId parsing and actual Noise tuple checks remain nf-transport responsibilities.

Store::issue_replica_activation checks its actual persisted membership/local receiver and creates one private five-second, OS-random challenge bound to this installer instance, manifest/target, source/receiver context and current exact membership revision/digest. The public challenge is read-only; its fields cannot replace the private expectation. ReceiptRepo signs its receiver DeviceProof using the actual loaded local device through its narrow owner delegation, without exporting keys or an arbitrary signer callback. The source's previously verified final proof is also supplied as data, not cached policy approval. The local activation preimage has its own closed purpose/domain and binds the source-final proof digest plus manifest/target/local challenge; its exact private layout is specified below and requires supplemental profile/vector review before code, distinct from the278-byte wire InstallReceipt transcript. No local commit challenge is sent to the source or silently added to the frozen wire layout.

The private local activation preimage is842 bytes, all numeric fields LE and fixed PeerFields padded canonically:

```text
NF-SYNC-ACTIVATE-1 NUL19, version u16=1, purpose u8=1
private installer instance16, newly issued local nonce32
universe16/history16, ruleset32/content32
profile u16=1, implementation32, schema32
source PeerField129/account16/device16
receiver PeerField129/account16/device16
current membership revision u64/digest32
manifest digest32, target Point112
SHA256(exact final verified DocumentEnd header+581-byte body)32
control neutral context digest32, transfer neutral context digest32
next activation generation u64
```

All fields except the privately issued nonce/instance are reconstructed from trusted installer configuration, admitted manifest/final-source evidence and actual SQL; response data cannot replace them. SHA256 of these842 bytes is the expected local receiver DeviceProof.challenge. Scope/account/device/peer/frontier also match the existing maintained device digest, so a signature from another role/device/context fails. The private registry retains instance, challenge, exact expected data, disable-only epoch fence and std::Instant expiry; there is no caller-supplied time. This local proof does not replace the source's wire-purpose5 verification or the wire-purpose6 postactivation observation.
Store::activate_replica consumes the issued challenge on the first attempt. After the fault hook, immediately before the cache commit, it reloads/validates actual SQL membership through its own existing connection, checks exact captured revision/digest and configured source owner/PLAYER, verifies both maintained typed DeviceProofs against privately reconstructed expected contexts, and requires receiver PLAYER AND REPLICA. It also checks all protected installed minima, instance/manifest/target and the live epoch fence. Only then does it internally construct CurrentReplicaAdmission and invoke crate-private installer activation. It does not claim to infer actual TCP/Noise ConnectionId from signed bytes.

The issuer creates an opaque disable-only ReplicaEpochInvalidator paired with the installer. ReceiptRepo retains it with exact actual control/transfer PeerId/ConnectionId/session and owner epoch; connection replacement/closure, pending-operation failure, shutdown or policy drift invalidates it. Public invalidation can only revoke, never set an epoch active, revive a poisoned handle or attach a newly constructed handle. Store retains the issued instance/generation internally and checks that original fence after the hook and at the cut. ReceiptRepo first rechecks both actual tuples and fresh source evidence, then delegates synchronously with no await/poll/reentrant callback. The Store verifies crypto/current policy; ReceiptRepo verifies actual transport ownership. No supplied epoch label is authorization.

Installer owns the prior read-only cache view throughout staging. The error variants are BeforeCommit { previous: Option<ReplicaStore>, error: BoundedReplicaError } and Uncertain { recovery: ReplicaCacheRecovery, error: BoundedReplicaError }. BeforeCommit returns the prior ReplicaStore by value (None only when no prior generation existed), after deleting only owned staging artifacts; the attempted challenge is still spent. Uncertain is a distinct quarantined cache recovery handle with no trusted view/receipt: reopen the exact cache and validate catalog head/anchor to recover old or fully committed new generation. The authoritative Store connection is not replaced or recreated by this cache recovery. A fresh attempt requires a new issued challenge and still-live original epoch; it cannot reset a closed transport session. Shared decoder extraction stays private/data-only and preserves accepted bytes/reducer behavior. Exact signatures/fault-hook ownership and the local activation preimage remain required frozen source-port review items.

`begin_replica_export` captures State bytes, WorldHash, StoreRevision/EventSeq, JournalHead, schema digest and freshly validated membership revision/digest in one read transaction on the existing connection. Quarantine/below-known minima refuse. Delta requires exact retained base and complete consecutive tail through captured target. Missing/pruned/wrong base returns authenticated GapRequiresSnapshot. A private retention token prevents all compaction/GC paths pruning its checkpoint/journal rows until release; later commits may append but never rewrite them. Policy change invalidates export. No SQL transaction survives a network await.

`export_step` reads one immutable row with checked SQL lengths (action/state each<=1MiB), retains at most that row, streams<=64KiB per owner step to one spool and hashes it. Cancellation releases only its exact owned references/artifacts. Live preparation can immediately return authenticated ExportPreparing and retain one private preparation job<=30s; retries require new proofs, at most3 within the foreground deadline. It must not hold a5s RR request while building a large export. An explicit offline local preparation mode may run before listeners. SQL/CPU step latency needs measurement; a bounded write is not hard scheduling evidence.

Root-controlled identity builders `build_sync_control_lane`/`build_sync_transfer_lane` clone the same loaded key internally, reuse exact maintained TCP/Noise/Yamux/RR dependencies, and set61s backend idle for a foreground run<=60s. Existing builders remain unchanged. Actual PeerId/ConnectionId are private state keys. Each physical lane has its own fresh handshake/session; a control session never migrates to transfer. Verified ExportId/manifest and exact admitted principals/pins link them. No signing seed/raw token leaves the vault.

## Proposed closed NF-SYNC-1 registry

Exact review proposal only. Namespaces `/nearfuture/peer/sync/1` and `/nearfuture/peer/sync-transfer/1` have no fallback to existing controls/bulk/foundation/Java or another profile. Actual namespace/owner slot select decoder. Integers inside records are unsigned LE; outer body length u32BE. No compression/text/extensions/unknown-required capability.

Header126: ASCII `NF-SYNC-1` NUL10, version u16=1, kind u8, actual lane u8(control1/transfer2), session16, universe16, history16, ruleset32, content32. Only Hello has zero session. Exact fixed IDs are nonzero when present; PeerField129/DeviceProof297 reuse canonical PeerId/padding and maintained signing format. SyncRequestId is fresh correlation data, not a strategic RequestId. Point112: StoreRevision u64, EventSeq u64, WorldHash32, StoreStateDigest32, JournalHead32. Genesis counters/head may be zero. Absent Point uses a separate presence byte plus all112 zero bytes. MembershipStamp40: revision u64/digest32.

Required/available/selected capability exactly1 ReadProfile1Replica; optional exactly0. Profile u16=1, implementation pin32 and Store1 schema digest32 must equal compiled closed values. A required new pin producer enumerates reviewed decoder/replay/profile/schema source closure with LF normalization and excludes only its generated literal module; independent --check verifies it. This is source consistency, not binary attestation or authentication. A sender hash cannot select an implementation.

Limits reuse26-byte PeerLimits field order, with protocol-specific maxima control frame1024/transfer9216/chunk8192/control queued bodies16384/transfer32768/control items4/transfer items2/pending challenges1. Control frame exactly1024; transfer>=1024; chunk>=256 and<=selected transfer frame-200; queues admit one selected frame and item/challenge counts positive. Componentwise minima must pass constraints. Advertised transfer frame is a negotiated resource ceiling up to9216; the closed codec has the stricter absolute8392 body ceiling because no registered record is larger. Declared transfer bodies must be<=min(selected transfer frame,8392) before allocation where selected context is available, otherwise<=8392 hard parser admission followed by selected-cap re-admission. A9216 declared body is never accepted merely because Limits advertises9216. Control parser hard<=1024 before allocation. Selected limits and exact queue reservations are rechecked before use; hard parser allocations are distinct from negotiated application quotas.

Widths exclude header/framing; fields are ordered.1–4 run on either actual lane;5–9/16/17 control only;10–15 transfer only.

|Kind|Fields|Body bytes|
|---|---|---:|
|1 Hello|receiver selector u8=1; client account16/device16; nonce32; required u32=1; optional u32=0; Limits26; profile u16=1; implementation32; schema32|165|
|2 ServerHello|nonce32; available u32=1; selected u32=1; server Limits26; selected Limits26; membership digest32; server DeviceProof297|421|
|3 ClientProof|client DeviceProof297|297|
|4 Finished|server DeviceProof297|297|
|5 BeginSync|mode u8(delta1/snapshot2); profile u16; implementation32; schema32; client operation nonce32; minimum event/store/member three u64; base-present u8; base Point112; expected membership digest32; SyncRequestId16; maximum documents u16; maximum total data-document bytes u64|294|
|6 SyncChallenge|SyncRequestId16; client nonce32; server nonce32; captured Point112; MembershipStamp40; challenge32|264|
|7 ProveSync|SyncRequestId16; client nonce32; client DeviceProof297|345|
|8 ManifestOffer|SyncRequestId16; ExportId16; target Point112; manifest digest32; length u32; document count u16; total data bytes u64; fresh current Point112; MembershipStamp40; server DeviceProof297|639|
|9 GapRequiresSnapshot|SyncRequestId16; reason u8; oldest-present u8; oldest Point112; current Point112; MembershipStamp40; server DeviceProof297|579|
|10 BeginDocument|TransferId16; ExportId16; document digest32; total u32; count u16; client nonce32|102|
|11 DocumentChallenge|TransferId16; ExportId16; document digest32; client nonce32; server nonce32; MembershipStamp40; challenge32|200|
|12 ProveDocument|TransferId16; client nonce32; client DeviceProof297|345|
|13 DocumentReady|TransferId16; ExportId16; document digest32; total u32; count u16; current Point112; MembershipStamp40; server DeviceProof297|519|
|14 SyncChunk|TransferId16; ExportId16; document digest32; index u16; count u16; total u32; data length u16; exact data|74+data|
|15 DocumentEnd|TransferId16; ExportId16; document digest32; received u32; assembled digest32; current Point112; MembershipStamp40; fresh completion nonce32; server DeviceProof297|581|
|16 ReplicaInstallReceipt|ExportId16; manifest digest32; target Point112; activation generation u64; receiver DeviceProof297|465|
|17 SyncRefused|SyncRequestId16; ExportId16(zero if absent); reason u8; current Point112; MembershipStamp40; server DeviceProof297|482|

BeginSync delta requires base-present1/exact admitted base; snapshot requires0/all-zero Point. Count1..257/total1..8388608 lower hard limits. Gap reasons1 PrefixPruned/2 BaseMismatch are closed; absent oldest has zero Point. Refusals1 ExportPreparing/2 UnsupportedBase/3 SourceBelowMinima/4 Capacity/5 ExportExpired are closed. Foreign history is not disclosed before authentication. Unknown profile/pin, malformed proof, policy failure/quarantine closes; it cannot fabricate a signed source stamp or advance minima.

Manifest is transferred first using signed ManifestOffer hash/length and is not listed in itself. SyncChunk is unsigned but requires exact active actual transfer tuple/current policy at every progress cut; complete bytes are committed by signed manifest/Ready/End. Index begins0, count=ceil(total/selected chunk)<=257; negotiated semantic maximum is min(2105344,257*selected_chunk), using checked multiplication. A worst-case two1MiB-blob Transition444 requires selected chunk8192; a lower selected chunk must yield authenticated Capacity or a supported smaller snapshot strategy before BeginDocument, never a larger count or hidden segmentation. Every nonfinal chunk exactly selected chunk, final exact positive remainder. Duplicate/reorder/empty/extra/wrong descriptor aborts. Partial bytes never activate.
## Exact cryptographic transcript proposal

Use maintained SHA256 and accepted DeviceProof/Ed25519 verification, including canonical nonidentity prime-subgroup point admission. DeviceProof signs the maintained device digest whose challenge is SHA256 of these exact bytes. Current PLAYER+REPLICA checks supplement signature verification. Source must equal configured peer/account/device and actual current membership owner with PLAYER. No candidate/lease/term inference for profile1; ARCHIVE serving needs a separate policy gate.

Handshake750:

```text
NF-SYNC-AUTH-1 NUL15, stage u8(0 neutral/1 server/2 client/3 finished)
version u16=1, actual lane u8
actual client PeerField129, actual server PeerField129
client account16/device16, server account16/device16
fresh session16, universe16/history16, ruleset32/content32
client nonce32, server nonce32
required u32, optional u32, available u32, selected u32
client Limits26, server Limits26, selected Limits26
proof membership frontier u64
profile u16=1, implementation32, schema32, membership digest32
SHA256(exact actual protocol ASCII without NUL)32, receiver selector u8=1
```

Arithmetic: accepted619-byte layout, equal15-byte replacement domain, plus2+32+32+32+32+1=131. Neutral context uses stage0/frontier0 and is not a DeviceProof grant. Exact tuple/pins/policy revision/digest/gates/limits agree before verification. Both pending endpoints consume the first attempt, including malformed/wrong context/invalid signature. Stages expire5s; raw session IDs cannot recreate grants.

Operation278:

```text
NF-SYNC-OP-1 NUL13, purpose u8
neutral handshake digest32, SyncRequestId16, ExportId16
expected document digest32, initiating-record prefix digest32
client operation nonce32, server operation nonce32
current membership revision u64, current membership digest32
exact response-prefix digest32
```

Purposes1 client Sync/2 source manifest/3 client Document/4 source Ready/5 source End/6 receiver Install/7 source Gap or Refused. Client stages use zero response digest. Export/document absent at Sync challenge are zeros. Document context derives from verified manifest and its exact entry, not an incoming expectation. Initiating prefix is SHA256(header+BeginSync294) or SHA256(header+BeginDocument102). Reply prefix is SHA256(exact header+all fields before DeviceProof297). No proof or outer framing participates in its own challenge. Gap/refusal keeps the exact initiating context; replies never replace trusted expectations. Every challenge registry entry is privately issued, bounded and consumed first attempt. Supplied DeviceProof.challenge is compared with the privately recomputed expected digest.

Source End creates a fresh completion nonce32. Only the final manifest entry's verified End enables InstallReceipt purpose6, using that final transfer's client nonce and the new completion nonce. For this control-lane observation, neutral handshake is the active control digest; expected document digest is manifest digest; initiating-record digest is the final verified End prefix digest. Source privately links the final transfer tuple/End to the control ExportId before enabling this expectation. Exact response-prefix is the Install header+168-byte fields preceding proof. Source keeps one expectation5s after End flush, consumes first attempt and verifies current PLAYER+REPLICA again. Receiver signs only after durable activation/fresh policy cut. A lost observation leaves source delivery uncertain, never undoing local activation or creating a strategic acknowledgement. Reconnect gets fresh sessions/proofs and reports externally anchored installed point.

## Semantic documents and composed limits

Every document begins common219:

```text
NF-SYNC-DOC-1 NUL14, version u16=1, kind u8
profile u16=1, implementation32, schema32
universe16, history16, ruleset32, content32
membership revision u64, membership digest32
```

Snapshot kind1 appends Point112, state length u32 and exact NF-STORE-1 bytes: fixed335+state<=1048576. Includes full source request/dedup/provider/reservation/pending/outbox metadata, no policy snapshot/seed/runtime grant. Decode/re-encode equality and typed invariants hold. Pending frontier remains inert; settlement on the follower is prohibited.

Transition kind2 appends result StoreRevision u64, journal kind u8(1..3), base/result EventSeq two u64, base/result WorldHash two32, base/result StoreStateDigest two32, previous/result JournalHead two32, action length u32, result-state length u32, action then state bytes. Fixed444+action/state each<=1048576. Recompute accepted head formula with revision **BE8** and journal kind **LE4**, despite this document's LE counters/u8 selectors. Validate trusted base digest/head and checked revision+1, replay accepted Prepare/Commit/OutboxAck, compare complete candidate state. Commit validates canonical before/after hashes, tick/sequence, authority tuple, exact admitted intents/outcomes/provider state against trusted-before replay; no provider/RNG rerun. Metadata-only transitions preserve WorldHash/EventSeq. Construct mirrors locally from typed state, never incoming SQL/mirror rows/database files.

Manifest kind3 appends ExportId16, mode u8, base-present u8, base Point112, target Point112, count u16, total data-document bytes u64 and exact ordered entries. Entry149 is kind u8(snapshot1/transition2), digest32, length u32, result Point112. Fixed471+149*count; max38764 at257, hard65536. Snapshot mode has one leading snapshot followed by optional consecutive transitions; delta has transitions only from the exact active base. Initially support captured-target snapshot with zero transitions, or complete delta to captured target. Hybrid stays unavailable until its producer/replay coverage is reviewed. No duplicate digest/result revision, surplus fields or contradictory totals. Checked lengths sum before allocation. Final entry point equals target. All documents share captured export/profile/source/policy. SHA256 addresses exact complete bytes.

These proposed caps intentionally supersede the earlier suggestion1MiB/128 chunks. Accepted state can be1MiB before335-byte envelope; a journal row can have two1MiB blobs before444-byte envelope. Per document **2105344=257*8192**, count<=257, one document active; whole export data<=8MiB plus manifest<=65536. Delta beyond that bound returns explicit GapRequiresSnapshot/Capacity and offers a consistent target snapshot, never a truncated synchronized tail. There is no promise all256 worst-case rows fit. An absolute30s export attempt remains; slow peers fail closed, with no lifetime widening or accumulated resume fragments.

A shared budget must reach every nested decoder before allocation: actual/declared document bytes, nesting<=32, combined logical entries<=32768, existing lower per-type count caps, checked arithmetic and conservative aggregate copied text/byte charge<=4MiB per decode. These limits are a proposal requiring independent boundary evidence, not established acceptance of every maximally sized existing state. Existing separate defaults cannot be called composed negotiated heap enforcement. Budget-carrying private Store/kernel decoder ports need review/public one-over cases. Validate lengths/selectors/pins/counts before temporary collection construction. Hash borrowed streaming bytes. Simultaneous trusted-before/candidate state has its own bounded live-memory accounting, not an RSS promise.

## Staging, activation and anchors

Use an existing owned private local root, preflight canonical directory/type/containment, reject links/reparse/nonregular entries/network/device paths, and create_new. No remote filename/hash becomes a path. Cooperative filesystem-race limitations remain explicit. One document file<=2105344; staging DB<=256MiB; active cache DB<=256MiB; archive budget<=256MiB. At most two logical cache generations, one document and bounded catalog exist. One transfer/export active, document idle5s and export/activation absolute30s. Disconnect/policy change/failure removes only owned incomplete artifacts and leaves prior view intact.

Separate replica schema has a versioned application ID/closed object inventory, validated source descriptor/private metadata-state blob, derived committed World/hash/frontiers, bounded transition ledger and archive references. Exact CREATE statements/schema digest are required frozen deliverables before implementation. It is not MembershipRepository and does not import remote policy grants. ReplicaStore has a read-only public strategic API. Existing cache opens without CREATE and refuses/preserves unknown or corrupt files. A single private cache connection is retained for validated activation transactions; it is never exposed as a writable Store or arbitrary SQL port. Installer consumes the prior ReplicaStore, can still expose its immutable previous_view during staging, and alone owns the separate staging writer. Both use pinned DELETE/EXTRA settings/readback. Derived rows come from typed decoding, never sender SQL.

Before activation require all entries/hashes/continuity/replay/target equality. After any fault hook, obtain a fresh synchronous current local-policy cut immediately before commit: source pin/scope/ruleset/content/profile/pin, exact shared membership revision/digest and receiver PLAYER+REPLICA. Manifest/End source proof was verified with the pinned source's current key; same policy must still hold. No await/callback separates cut and commit. A policy change invalidates its private permit. Remote target may lag writer's fresh current point but must meet externally protected **installed** event/store minima and separately admitted policy floor. Live observations show lag; they never label target equal to latest state. Source observed frontiers and installed minima are separate types/records. Trusted operator may explicitly raise installed floors; it never lowers them.

Use one existing cache database for immutable logical generation rows and the active-head catalog. After the staging database/document has been completely typed-validated, copy only admitted typed state and derived rows into a new generation and select its active head in the SAME SQLite transaction. The fresh policy cut is immediately before that commit. No raw database attach/import, cross-file rename or open-file overwrite is required. The staging file is disposable and never itself active. Precommit failure/crash preserves old generation; postcommit reopen validates exact complete new generation. Checked generation MAX refuses. Uncertain commit quarantines the installer, reopens/queries the exact active head and emits no speculative receipt. Old logical generation is removed only after no view/recovery reference remains, using a separate bounded private cache-maintenance transaction. A caller cannot hold a borrowed previous_view across mutable activation. Exact schema derives immutable generation records and catalog hash without accepting sender rows. Real SQLite quota/EIO/crash gates must establish this protocol; settings alone are not evidence.

ReplicaAnchor externally retains source pin, scope/profile/pin/schema/policy digest, installed Point, membership floor, activation generation and catalog-head digest. Protect latest anchor outside replaced cache/catalog, as accepted backup minima require. A file beside catalog does not prevent rollback. Explicit trusted initial snapshot configuration is required; missing active state does not infer genesis or trust first source. Local authoritative KnownFrontiers and remote ReplicaKnown are distinct. ReplicaInstallReceipt means only local cache activation, not source durability/quorum/native save/writing permission.

## Archives and availability

A replication download does not satisfy issue39's retained-serving-archive requirement. This source/replica slice establishes formats/replay/activation. A subsequent reviewed archive owner uses the same documents under explicit PLAYER+ARCHIVE serving policy and receiver PLAYER+REPLICA. No auto role changes. Fresh archive proof and trusted original-source signed provenance are distinct: archive serves retained source-signed manifest/documents, cannot mint a new original-source target signature. Archive signature alone does not prove authority/current read policy. Offline/current-policy freshness rules require a frozen policy/source-port design before serving; initial direct-source route does not claim offline archive availability.

Archive catalog stores hash/exact kind/profile/scope/policy/source provenance/length and references from retained manifest closures, active/staging generations and exports. Hard256MiB,<=4096 objects,<=256 manifests. One job can require up to258 closure references (257 listed data documents plus the manifest itself); therefore the initial archive per-job limit is258 and total catalog reference edges are capped65536, charged before retention across manifests, jobs and generations. A configured lower reference quota refuses the complete plan before retention, never drops the manifest/last dependency. GC marks every retained recovery dependency and active/export/reader reference before deleting unreachable owned regular objects. Referenced recovery data cannot be sacrificed for quota: refuse admission/Capacity. Catalog/deletion crash recovery is explicit; absent/corrupt referenced objects refuse availability, never initialize empty history. No recursive delete of caller-derived paths.

Public observations include installed/source current event/store lag, target WorldHash, LocalSQLiteExtra durability, direct connection/offline, retained archive coverage/quota and signed GapRequiresSnapshot. Durable local source commit alone guarantees no future replica/archive availability. Offline producer is recoverable only while an admitted authorized archive actually retains the closure; otherwise explicit Unavailable. No automatic failover, host advertisement authority, secret disclosure, histories merge, anonymous read, DHT or infinite storage.
## Source ownership, dependencies and observable gates

After root approves exact ports/profile, proposed source is additive `nf-store/src/{replica_export,replica_codec,replica_cache}/**` and `nf-transport/src/sync/**`; foreground CLI remains portal-author owned. Root owns exports/manifests/lock/architecture/registered pins. Existing Store1, receipt/notification bytes remain unchanged. Exact maintained SQLite/rusqlite, libp2p/TCP/Noise/Yamux/RR, SHA256, Ed25519/DeviceProof, entropy and zeroizing dependencies suffice. No consensus/compression/gossip dependency is justified. If a feature/OS operation is unavailable, submit its exact additive requirement instead of inventing unsafe VFS/I/O.

Separately assigned test-only Node Buffer/standardcrypto fixtures must freeze all17 records,750/278 transcripts and prefix hashes, three documents with independent Store1/kernel goldens, and role/context/malformed boundaries. Explicit public RFC seeds are synthetic data; actual process grants use generated disposable keys. Producer never imports production codecs or gets expected output from a server. Shape/context vectors are not runtime authorization evidence.

Required meaningful compiled public RED/GREEN and independent review:

1. Prepare with reservations without Commit: installed committed WorldHash stays unchanged; pending state cannot execute. Full kind1→kind2→kind3 reproduces complete StoreStateDigest/head, with EventSeq advancing only for2. Omitting metadata transitions fails equality. Independent policy CAS changes export stamp while journal remains unchanged.
2. Real encrypted processes and current SQL: reject PLAYER without REPLICA, wrong pins/scope/profile, revoked devices, moved session/ConnectionId and first-attempt reuse. Revocation at preparation/chunk/activation cuts keeps prior view. Uncommitted SQL/tentative World, foreign history and corrupt data cannot activate; validated committed SQL pending metadata is permitted only as inert metadata.
3. Lag/reconnect delta matches committed authority WorldHash and complete metadata hash; compacted/missing/wrong base gives signed gap then consistent snapshot. Outbox-only revisions are distinct from EventSeq. Restart uses original externally anchored installed point/fresh proof and never repeats a spend/provider.
4. Atomic export stamp/state/head, immutable documents despite later commits, compaction/GC respects private references. Release/error cleans only owned artifacts. Measure control progress during bounded preparation/bulk saturation; structural caps do not prove native p99/RSS.
5. Owned process crashes at spool/staging/replay/activation before/after commit/lost receipt. Actual SQLITE_FULL and calibrated required Linux SQLite sync/commit EIO: no activation receipt before durable selection; reopen gives old valid or exact new view. Actual Windows/Linux path/ACL/reparse/cleanup gates, explicit hardware-power-loss residual.
6. Composed limits/finite fuzz/unknown SQL objects/schema/pin preserve bytes. Below-known installed/member/generation/head minima refuse even with unchanged WorldHash. Overflow/conflict never recreates empty history.
7. Archive follow-up: actual producer-offline authorized serving, reference-aware retention/GC crash cuts and disk failure; unavailable archive explicitly reports loss of availability. Issue39 remains open until that slice and gates pass.

Concrete source blockers: no atomic export/head port; no compiled replica decoder pin; no retention-aware export token; no whole-input Store/kernel decode budget; no replica schema/activation owner; current ProtectedOperation has only Economic/Chat/Administration/Worker and no ReadReplica policy; no archive provenance/serving/GC port; no frozen independent NF-SYNC corpus. These are implementable headless dependencies. The new caps, registry/transcripts, pin closure and exact cache schema need frozen review before implementation. No native game/IPC payload promotion or install occurs. Portable replica success does not close issue23's native responsiveness or issue39's archive/availability obligations.
