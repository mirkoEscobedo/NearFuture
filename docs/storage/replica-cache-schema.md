# Replica cache SQLite profile1

Documentation-only frozen proposal for root independent R1. This specifies new cache/staging schemas and feasible additive storage APIs; it implements none. Normative architecture is [replica-sync-design.md](../transport/replica-sync-design.md), reviewed architecture SHA256 `abf07e9720a22548c17eac17c31445f976d9032c7a33afaf122835e1d80159f7`. Architecture approval is not approval of this schema/source contract or a runtime grant. Initial supported source is accepted Store1/kernel domain7 only. No archive service, writable replica, role import, profile2, native effect or foundation/Java payload promotion.

## Identities, data and settings

Active cache application_id is `0x4e465243` (ASCII NFRC), user_version1. Staging application_id is `0x4e465253` (NFRS), user_version1. Both have separate exact schema digests. Every SQL counter is an exact8-byte big-endian BLOB for full-u64 lexical ordering, never a SQLite signed integer cast. Protocol/private proof counters remain LE as their profiles require. Hashes32, typed IDs16, strict presence/zero rules and canonical text/PeerId constraints are checked by typed decoders; SQL width checks alone are not semantic admission.

Source schema digest is SHA256 of accepted Store1 SCHEMA literal, not this cache schema. `replica_implementation32` is the future closed compiled NF-SYNC decoder/replay pin; source-selected hashes cannot choose code. Ruleset must equal actual decoded WorldSpec.ruleset_hash; universe/history must equal decoded World and every nested intent. WorldHash commits the World seed. No installed-content hash exists in WorldSpec: content32 is trusted repository/operator context, not proof of actual game content. Membership revision/digest is provenance, not a membership table or policy import.

Use existing exact rusqlite0.32.1 (`bundled`, `limits`, `backup`), bundled SQLite3.46.0/libsqlite3-sys0.30.1, sha2 and maintained identity proof/entropy APIs. No dependency or feature update is proposed. These are NEW connections with their own settings; accepted Store1 bytes/settings are unchanged.

| Setting | Exact value/check |
| --- | --- |
| open existing | READ_WRITE + NO_MUTEX internally; no CREATE; public cache API read-only strategically |
| initial allocation | explicit create_new reservation, then open existing; never empty-history fallback |
| journal/sync/locking | DELETE / EXTRA(3) / EXCLUSIVE, verify readback |
| foreign_keys/trusted_schema | ON / OFF, verify readback |
| busy timeout |250ms|
| page size / maximum pages |4096 /65536, readback and file<=268435456|
| cache_size | -2048 KiB |
| SQLite LENGTH |4194304 bytes (new cache rows can exceed source Store's2MiB row limit)|
| SQL_LENGTH / COLUMN / VARIABLE_NUMBER |65536 /64 /128|
| EXPR_DEPTH / ATTACHED |64 /0|

Length4MiB bounds an encoded SQLite row/value, not semantic document acceptance. Snapshot document<=1048911, transition<=2097596, manifest<=38764 and protocol document hard<=2105344 remain independently enforced before copying. A row with two1MiB blobs plus proofs fits the new4MiB guard; oversized rows/capacity refuse. No WAL/network-filesystem mode. Existing local-disk/locking/sync host preconditions remain; mapped-drive locality is a host responsibility.

Admission requires existing private local directory, canonical containment, no symlink/reparse/foreign-access entry and regular SQLite/staging files. No filename comes from wire data; bounded trusted configuration selects a dedicated cache root. Paths must not alias the authoritative Store or vault key files. Cooperative filesystem-race limits are explicit. An additive identity-owned `validate_private_directory(&Path)->Result<(), IdentityError>` / regular-file counterpart is required to reuse maintained Unix permission/Windows ACL helper policy without exposing PrivateVault.root, keys or rewriting its sidecar. Those validation functions are effect checks called by nf-store, not caller-provided private-storage flags. Exact public helper source/child ownership needs separate root review before code. This profile grants no unsafe/VFS/raw-path bypass.

## Owned APIs and privacy boundary

All following types/functions are proposed additive public data/storage ports in nf-store, with nf-transport depending on them; nf-store never depends on libp2p/transport. Paths and source/receiver pin configuration are trusted local inputs, validated internally. Raw protocol evidence is untrusted.

```rust
pub enum ReplicaError {
    Io, Busy, Full, Corrupt, UnsupportedSchema, MissingCache, AlreadyExists,
    Scope, Context, Policy, Signature, StaleAnchor, AnchorGap, Conflict,
    Limit, Capacity, Expired, Invalidated, Replay, Quarantined, UncertainCommit,
    UninitializedCache, RecoveryRequired,
}
pub enum ReplicaCommitBoundary {
    BeforeTransaction, AfterGenerationWrites, BeforeCommit,
    AfterCommit, BeforeAcknowledgement,
}
pub struct ReplicaCacheConfig { /* private validated root/context/pins */ }
pub struct ReplicaKnown { /* private validated external anchor/minima */ }
pub struct ReplicaStore { /* private cache connection, no Store conversion */ }
pub struct ReplicaInstaller { /* owns previous ReplicaStore plus staging connection */ }
pub struct ReplicaActivationChallenge { /* read-only preimage842 */ }
pub struct ReplicaActivationEvidence { /* typed source and receiver data, no bool/grant */ }
pub struct ReplicaEpochInvalidator { /* disable-only; no reset/attach constructor */ }
pub struct ReplicaCacheRecovery { /* quarantined cache ownership, no trusted World */ }
pub struct ReplicaRecoveryChallenge { /* privately issued purpose2 preimage */ }
pub struct ReplicaRecoveryFailure { pub recovery: ReplicaCacheRecovery, pub error: ReplicaError }
pub enum ReplicaActivationFailure {
    BeforeCommit { previous: Option<ReplicaStore>, error: ReplicaError },
    Uncertain { recovery: ReplicaCacheRecovery, error: ReplicaError },
}
pub struct ReplicaActivation { /* private committed view + public external anchor */ }

impl ReplicaStore {
    pub fn open_existing(config: &ReplicaCacheConfig, known: &ReplicaKnown)
        -> Result<Self, ReplicaError>;
    pub fn view(&self) -> ReplicaView<'_>;
    pub fn anchor(&self) -> ReplicaAnchor;
}
impl ReplicaInstaller {
    pub fn begin(config: ReplicaCacheConfig, manifest: VerifiedSyncManifest,
                 previous: Option<ReplicaStore>, known: ReplicaKnown)
        -> Result<Self, ReplicaActivationFailure>;
    pub fn admit_document(&mut self, data: VerifiedSyncDocument)
        -> Result<(), ReplicaError>;
    pub fn previous_view(&self) -> Option<ReplicaView<'_>>;
    pub fn abort(self) -> Result<Option<ReplicaStore>, ReplicaActivationFailure>;
}
impl Store {
    pub fn bind_replica_epoch(&mut self, installer: &mut ReplicaInstaller)
        -> Result<ReplicaEpochInvalidator, ReplicaError>;
    pub fn issue_replica_activation(&mut self, installer: &mut ReplicaInstaller)
        -> Result<ReplicaActivationChallenge, ReplicaError>;
    pub fn issue_replica_recovery(&mut self, recovery: &mut ReplicaCacheRecovery)
        -> Result<ReplicaRecoveryChallenge, ReplicaError>;
    pub fn recover_replica_cache(&mut self, recovery: ReplicaCacheRecovery,
                                 proof: DeviceProof)
        -> Result<ReplicaStore, ReplicaRecoveryFailure>;
    pub fn activate_replica(&mut self, installer: ReplicaInstaller,
                            evidence: ReplicaActivationEvidence)
        -> Result<ReplicaActivation, ReplicaActivationFailure>;
    // Hidden fault-injection seam calls same reducer/cut/commit; not a signing callback.
    #[doc(hidden)]
    pub fn activate_replica_with_fault<F>(&mut self, installer: ReplicaInstaller,
        evidence: ReplicaActivationEvidence, fault: F)
        -> Result<ReplicaActivation, ReplicaActivationFailure>
        where F: FnMut(ReplicaCommitBoundary) -> Result<(), ReplicaError>;
}
```

Result here means std::result::Result, not the crate-private StoreError alias. Existing StoreError, Accepted, DurableAck and profile1 methods remain untouched. SQLite Busy/Locked->Busy; DiskFull->Full; Corrupt/NotADatabase->Corrupt; TooBig->Limit; other I/O->Io. Errors are static bounded classes, no private paths/keys/raw SQLite messages. ReplicaActivation is not DurableAck. No arbitrary public cache SQL callback, signer closure, supplied MembershipState or authorization flag exists.

VerifiedSyncManifest and VerifiedSyncDocument are proposed opaque nf-store data-admission types: their bounded raw-byte constructors validate the closed profile, exact hashes and document semantics internally. The names convey data validation only; there is no public field/boolean that claims transport or policy authorization, and nf-transport cannot construct a private CurrentReplicaAdmission. They require exact source API review before implementation.

Pure nf-store evidence types/preimage writers are data-only. The maintained signature verifier reconstructs expected bytes from the installer's private admitted manifest/config/target and actual persisted membership. `CurrentReplicaAdmission` exists only inside nf-store and has no public constructor. ReceiptRepo only delegates narrowly and signs its actual loaded local device. It checks actual Noise PeerId/ConnectionId/control and transfer session/owner epoch before synchronous delegation; Store does not infer sockets from peer strings. The disable-only handle is paired privately to the issued installer; a new handle cannot replace it. Shutdown/tuple drift/policy drift disables it; public invalidation can revoke but cannot authorize.

One private activation registry entry per authoritative Store instance, one installer owned by the SyncOwner. An existing unspent entry yields Capacity, never eviction of another installer's challenge. The private registry stores issuer Store instance, installer instance, random nonce, exact842-byte expectation/context, original disable-only fence, captured membership and std::Instant five-second deadline. OS entropy and checked monotonic time are mandatory; no public clock/expiry setter. After reopen, registry is empty, epoch disabled and all prior challenges are unavailable. No challenge, grant, role cache, live session, actual connection handle or deadline is persisted in SQLite. Completed proof bytes/nonces below are evidence only; their presence cannot recreate a registry entry.

For fresh-role verification use current actual SQL membership from Store's existing connection: exact scope/revision/digest, source pin/current owner with PLAYER, local receiver mapping with PLAYER AND REPLICA, unrevoked devices and exact key/peer mappings. Verify source wire-purpose5 and receiver private local-purpose1 DeviceProof signatures via maintained code; comparison against privately computed challenge precedes any commit. Cached generation evidence never replaces this cut. Existing nf_identity::signing::device_digest is public; nf_identity::signing::verify is crate-private and is not callable here. The feasible existing public route is device_digest plus nf_contract::signatures::verify_digest against the freshly loaded actual device key, retaining the maintained closed Ed25519 point/signature policy. PLAYER AND REPLICA is a reviewed explicit nf-store policy gate, not a fabricated ProtectedOperation or an authorization flag supplied by transport.
## Exact active-cache CREATE inventory

Twelve tables below, only their SQLite-maintained autoindexes, no trigger/view/user index. Cache schema digest is SHA256 of the exact UTF8 SQL block between these fences, LF endings, no blank leading/trailing line, exactly one LF after final semicolon. Before writes verify application/version, exact table SQL/object inventory, schema digest, integrity_check(1)=ok and foreign_key_check empty. The literal digest is recorded after the block; no SQL from network/config is executed.

```sql
CREATE TABLE cache_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), cache_id BLOB NOT NULL CHECK(length(cache_id)=16), profile INTEGER NOT NULL CHECK(profile=1), implementation BLOB NOT NULL CHECK(length(implementation)=32), source_schema BLOB NOT NULL CHECK(length(source_schema)=32), cache_schema BLOB NOT NULL CHECK(length(cache_schema)=32), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), ruleset BLOB NOT NULL CHECK(length(ruleset)=32), content BLOB NOT NULL CHECK(length(content)=32), source_account BLOB NOT NULL CHECK(length(source_account)=16), source_device BLOB NOT NULL CHECK(length(source_device)=16), source_peer BLOB NOT NULL CHECK(length(source_peer) BETWEEN 1 AND 128), receiver_account BLOB NOT NULL CHECK(length(receiver_account)=16), receiver_device BLOB NOT NULL CHECK(length(receiver_device)=16), receiver_peer BLOB NOT NULL CHECK(length(receiver_peer) BETWEEN 1 AND 128), active_generation BLOB CHECK(active_generation IS NULL OR length(active_generation)=8), active_head BLOB CHECK(active_head IS NULL OR length(active_head)=32), chain_base_generation BLOB NOT NULL CHECK(length(chain_base_generation)=8), chain_base_head BLOB NOT NULL CHECK(length(chain_base_head)=32), CHECK((active_generation IS NULL AND active_head IS NULL) OR (active_generation IS NOT NULL AND active_head IS NOT NULL)), FOREIGN KEY(active_generation) REFERENCES generations(generation) ON DELETE RESTRICT) STRICT;
CREATE TABLE generations (generation BLOB PRIMARY KEY CHECK(length(generation)=8 AND generation<>x'0000000000000000'), export_id BLOB NOT NULL CHECK(length(export_id)=16), manifest_digest BLOB NOT NULL CHECK(length(manifest_digest)=32), store_revision BLOB NOT NULL CHECK(length(store_revision)=8), event_sequence BLOB NOT NULL CHECK(length(event_sequence)=8), world_hash BLOB NOT NULL CHECK(length(world_hash)=32), state_digest BLOB NOT NULL CHECK(length(state_digest)=32), source_head BLOB NOT NULL CHECK(length(source_head)=32), membership_revision BLOB NOT NULL CHECK(length(membership_revision)=8), membership_digest BLOB NOT NULL CHECK(length(membership_digest)=32), state BLOB NOT NULL CHECK(length(state) BETWEEN 1 AND 1048576), world BLOB NOT NULL CHECK(length(world) BETWEEN 1 AND 1048576), descriptor_digest BLOB NOT NULL CHECK(length(descriptor_digest)=32), FOREIGN KEY(manifest_digest) REFERENCES documents(digest) ON DELETE RESTRICT) STRICT;
CREATE TABLE generation_evidence (generation BLOB PRIMARY KEY CHECK(length(generation)=8), source_key BLOB NOT NULL CHECK(length(source_key)=32), receiver_key BLOB NOT NULL CHECK(length(receiver_key)=32), control_neutral BLOB NOT NULL CHECK(length(control_neutral)=750), transfer_neutral BLOB NOT NULL CHECK(length(transfer_neutral)=750), begin_sync BLOB NOT NULL CHECK(length(begin_sync)=420), sync_challenge BLOB NOT NULL CHECK(length(sync_challenge)=390), manifest_offer BLOB NOT NULL CHECK(length(manifest_offer)=765), begin_document BLOB NOT NULL CHECK(length(begin_document)=228), document_challenge BLOB NOT NULL CHECK(length(document_challenge)=326), document_end BLOB NOT NULL CHECK(length(document_end)=707), activation_preimage BLOB NOT NULL CHECK(length(activation_preimage)=842), receiver_proof BLOB NOT NULL CHECK(length(receiver_proof)=297), FOREIGN KEY(generation) REFERENCES generations(generation) ON DELETE CASCADE) STRICT;
CREATE TABLE catalog_commits (generation BLOB PRIMARY KEY CHECK(length(generation)=8), previous_head BLOB NOT NULL CHECK(length(previous_head)=32), descriptor_digest BLOB NOT NULL CHECK(length(descriptor_digest)=32), head BLOB NOT NULL CHECK(length(head)=32)) STRICT;
CREATE TABLE documents (digest BLOB PRIMARY KEY CHECK(length(digest)=32), kind INTEGER NOT NULL CHECK(kind IN (1,2,3)), body BLOB NOT NULL, CHECK((kind=1 AND length(body) BETWEEN 335 AND 1048911) OR (kind=2 AND length(body) BETWEEN 444 AND 2097596) OR (kind=3 AND length(body) BETWEEN 620 AND 38764))) STRICT;
CREATE TABLE generation_documents (generation BLOB NOT NULL CHECK(length(generation)=8), ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 256), document_digest BLOB NOT NULL CHECK(length(document_digest)=32), PRIMARY KEY(generation,ordinal), UNIQUE(generation,document_digest), FOREIGN KEY(generation) REFERENCES generations(generation) ON DELETE CASCADE, FOREIGN KEY(document_digest) REFERENCES documents(digest) ON DELETE RESTRICT) STRICT;
CREATE TABLE aggregate_state (generation BLOB NOT NULL CHECK(length(generation)=8), aggregate_id BLOB NOT NULL CHECK(length(aggregate_id)=16), revision BLOB NOT NULL CHECK(length(revision)=8), PRIMARY KEY(generation,aggregate_id), FOREIGN KEY(generation) REFERENCES generations(generation) ON DELETE CASCADE) STRICT;
CREATE TABLE module_state (generation BLOB NOT NULL CHECK(length(generation)=8), provider_id BLOB NOT NULL CHECK(length(provider_id)=16), draws BLOB NOT NULL CHECK(length(draws)=8), cooldown BLOB NOT NULL CHECK(length(cooldown)=8), PRIMARY KEY(generation,provider_id), FOREIGN KEY(generation) REFERENCES generations(generation) ON DELETE CASCADE) STRICT;
CREATE TABLE request_outcomes (generation BLOB NOT NULL CHECK(length(generation)=8), request_id BLOB NOT NULL CHECK(length(request_id)=16), binding_digest BLOB NOT NULL CHECK(length(binding_digest)=32), intent_digest BLOB NOT NULL CHECK(length(intent_digest)=32), operation_id BLOB NOT NULL CHECK(length(operation_id)=16), phase INTEGER NOT NULL CHECK(phase IN (1,2)), event_sequence BLOB CHECK(event_sequence IS NULL OR length(event_sequence)=8), rejection INTEGER CHECK(rejection IS NULL OR rejection BETWEEN 0 AND 17), CHECK((phase=1 AND event_sequence IS NULL AND rejection IS NULL) OR (phase=2 AND event_sequence IS NOT NULL AND rejection IS NOT NULL)), PRIMARY KEY(generation,request_id), FOREIGN KEY(generation) REFERENCES generations(generation) ON DELETE CASCADE) STRICT;
CREATE TABLE reservations (generation BLOB NOT NULL CHECK(length(generation)=8), operation_id BLOB NOT NULL CHECK(length(operation_id)=16), market_id BLOB NOT NULL CHECK(length(market_id)=16), amount BLOB NOT NULL CHECK(length(amount)=8), PRIMARY KEY(generation,operation_id), FOREIGN KEY(generation) REFERENCES generations(generation) ON DELETE CASCADE) STRICT;
CREATE TABLE outbox (generation BLOB NOT NULL CHECK(length(generation)=8), operation_id BLOB NOT NULL CHECK(length(operation_id)=16), event_sequence BLOB NOT NULL CHECK(length(event_sequence)=8), batch_digest BLOB NOT NULL CHECK(length(batch_digest)=32), rejection INTEGER NOT NULL CHECK(rejection BETWEEN 0 AND 17), PRIMARY KEY(generation,operation_id), FOREIGN KEY(generation) REFERENCES generations(generation) ON DELETE CASCADE) STRICT;
CREATE TABLE retention_pins (reference_id BLOB PRIMARY KEY CHECK(length(reference_id)=16), generation BLOB NOT NULL CHECK(length(generation)=8), kind INTEGER NOT NULL CHECK(kind=1), anchor_head BLOB NOT NULL CHECK(length(anchor_head)=32), FOREIGN KEY(generation) REFERENCES generations(generation) ON DELETE RESTRICT) STRICT;
```

`cache_meta` is fixed trusted local configuration, not a sender-selected descriptor. cache_id is nonzero random16 and never a role/authority identity. It cannot change after explicit initialization. Exactly one row; any zero fixed principal/scope ID or noncanonical peer fails typed admission. Existing metadata mismatch refuses without migration. NULL active fields are permitted only while an explicitly created empty cache is being initialized, with no generations/evidence/documents/mirrors/commits/pins. Public open returns UninitializedCache, not an empty World or permission to recreate it.

Each completed generation has exactly one evidence row, its manifest object kind3, exact ordered data-document links, and all derived mirrors. There is no separately persisted membership table. No persisted `validated`, `active_role`, lease, activity or executable job flag. Generation State includes pending metadata but neither this schema nor its public APIs execute it. World bytes must equal exact `nf_kernel::encode_snapshot(decoded_state.world)` and its WorldHash. StateDigest is SHA256 exact State bytes. A canonical committed batch changes World/EventSeq; prepared frontier/outbox-only transitions do not. The current active row need not equal the live authority's current point; expose lag.

Manifest digest is an implicit document reference in generations; typed GC treats it exactly like generation_documents. Its entry count1..257 must equal the ordered0..count-1 links, each kind/hash/length/result point exactly the manifest entry. Kind3 is never an entry. No duplicate links/extra objects or unsolicited history. Snapshot mode starts kind1, delta starts from the exact prior validated source point; transition revision continuity and all replay steps are validated before any generation row is committed. Raw source SQLite files/mirror rows never install.
## Exact staging CREATE inventory

Five staging tables, their autoindexes only. Application NFRS/version1 and its own digest are required. Stage belongs to one private random installer instance under one owned scratch directory; it can never be selected by active catalog. Partial stage has no public World view. Initial snapshot/delta bytes are data, not a grant. Restart never resumes stage challenges or installs a discovered stage automatically; preserve unknown foreign files and remove only an exactly owned abandoned artifact after path admission.

```sql
CREATE TABLE stage_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), instance BLOB NOT NULL CHECK(length(instance)=16), profile INTEGER NOT NULL CHECK(profile=1), implementation BLOB NOT NULL CHECK(length(implementation)=32), source_schema BLOB NOT NULL CHECK(length(source_schema)=32), cache_schema BLOB NOT NULL CHECK(length(cache_schema)=32), stage_schema BLOB NOT NULL CHECK(length(stage_schema)=32), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), ruleset BLOB NOT NULL CHECK(length(ruleset)=32), content BLOB NOT NULL CHECK(length(content)=32), source_account BLOB NOT NULL CHECK(length(source_account)=16), source_device BLOB NOT NULL CHECK(length(source_device)=16), source_peer BLOB NOT NULL CHECK(length(source_peer) BETWEEN 1 AND 128), receiver_account BLOB NOT NULL CHECK(length(receiver_account)=16), receiver_device BLOB NOT NULL CHECK(length(receiver_device)=16), receiver_peer BLOB NOT NULL CHECK(length(receiver_peer) BETWEEN 1 AND 128), export_id BLOB NOT NULL CHECK(length(export_id)=16), manifest_digest BLOB NOT NULL CHECK(length(manifest_digest)=32), target BLOB NOT NULL CHECK(length(target)=112), membership_revision BLOB NOT NULL CHECK(length(membership_revision)=8), membership_digest BLOB NOT NULL CHECK(length(membership_digest)=32)) STRICT;
CREATE TABLE stage_documents (digest BLOB PRIMARY KEY CHECK(length(digest)=32), kind INTEGER NOT NULL CHECK(kind IN (1,2,3)), body BLOB NOT NULL, CHECK((kind=1 AND length(body) BETWEEN 335 AND 1048911) OR (kind=2 AND length(body) BETWEEN 444 AND 2097596) OR (kind=3 AND length(body) BETWEEN 620 AND 38764))) STRICT;
CREATE TABLE stage_links (ordinal INTEGER PRIMARY KEY CHECK(ordinal BETWEEN 0 AND 256), document_digest BLOB NOT NULL UNIQUE CHECK(length(document_digest)=32), FOREIGN KEY(document_digest) REFERENCES stage_documents(digest) ON DELETE RESTRICT) STRICT;
CREATE TABLE stage_state (singleton INTEGER PRIMARY KEY CHECK(singleton=1), state BLOB NOT NULL CHECK(length(state) BETWEEN 1 AND 1048576), world BLOB NOT NULL CHECK(length(world) BETWEEN 1 AND 1048576), target BLOB NOT NULL CHECK(length(target)=112)) STRICT;
CREATE TABLE stage_evidence (singleton INTEGER PRIMARY KEY CHECK(singleton=1), source_key BLOB NOT NULL CHECK(length(source_key)=32), control_neutral BLOB NOT NULL CHECK(length(control_neutral)=750), begin_sync BLOB NOT NULL CHECK(length(begin_sync)=420), sync_challenge BLOB NOT NULL CHECK(length(sync_challenge)=390), manifest_offer BLOB NOT NULL CHECK(length(manifest_offer)=765), transfer_neutral BLOB CHECK(transfer_neutral IS NULL OR length(transfer_neutral)=750), begin_document BLOB CHECK(begin_document IS NULL OR length(begin_document)=228), document_challenge BLOB CHECK(document_challenge IS NULL OR length(document_challenge)=326), document_end BLOB CHECK(document_end IS NULL OR length(document_end)=707), CHECK((transfer_neutral IS NULL AND begin_document IS NULL AND document_challenge IS NULL AND document_end IS NULL) OR (transfer_neutral IS NOT NULL AND begin_document IS NOT NULL AND document_challenge IS NOT NULL AND document_end IS NOT NULL))) STRICT;
```

Stage singleton rows are<=1 each; stage_links/documents are bounded by declared plan. stage_state is absent until complete typed replay. Final evidence quartet is absent until its complete last-document proof is admitted, inserted together and checked against actual final manifest entry. Stage has neither a receiver activation proof nor an active/authorized/validated flag. Installer keeps trusted parsed values privately and rechecks entire persisted stage before copy, not a saved phase label. No staged membership snapshot or executable provider work.

## Generation, descriptor and catalog hashes

Generation ordinal starts1, checked u64+1. Same source StoreRevision with a different Point rejects Conflict. Same canonical EventSeq with different WorldHash also rejects Conflict even if metadata StoreRevision is larger: metadata-only transitions cannot replace committed World. All protected event/store/member minima are nondecreasing; same MembershipRevision with a different digest rejects Conflict.

Exact manifest digest/Point/membership already installed is idempotent: privately choose Existing, revalidate current proof/policy/epoch and return that existing view/anchor, with no new SQL commit or fabricated durability acknowledgement. A distinct freshly authenticated manifest/provenance can select a new local cache generation even with the same source Point, for example actual policy CAS changed MembershipRevision while StoreRevision/EventSeq stayed unchanged. That is explicit provenance refresh, not canonical/source progress, and imports no policy. Both endpoints must already have the new exact policy; source proof and local receiver proof are fresh. Report unchanged source EventSeq/WorldHash and the new observed membership separately. The local842-byte preimage's generation field is the privately chosen result generation (current for Existing, checked current+1 for a new selection); caller cannot select this mode/counter. Snapshot is the bounded no-event/provenance-refresh route; the initially unsupported empty delta can return authenticated UnsupportedBase without claiming missing rows or canonical advancement.
Fixed public provenance is5539 bytes before any framing: source key32, receiver key32, neutral control750, neutral transfer750, BeginSync420, SyncChallenge390, ManifestOffer765, final BeginDocument228, final DocumentChallenge326, final DocumentEnd707, local activation preimage842, receiver DeviceProof297. The three prefix rows exclude outer u32 framing, include exact NF-SYNC header and ordered body. Field order is exactly the generation_evidence column order after generation. Verify primitive signatures using these exact reconstructed preimages; saved keys are data and do not initialize membership. Source/receiver keys at activation must equal freshly loaded actual membership keys. Recovery of a historical generation uses its protected catalog/source evidence, never calls cached DeviceProof.authorize as a live operation.

Descriptor digest is SHA256 of exact bytes:

```text
NF-REPLICA-GENERATION-1 NUL24
cache_id16, profile u16=1, replica implementation32, source schema32, cache schema32
universe16/history16, ruleset32/content32
source PeerField129/account16/device16, receiver PeerField129/account16/device16
generation u64LE, export_id16, manifest digest32
Point112 (LE counters), membership revision u64LE/digest32
SHA256(world canonical bytes)32, SHA256(provenance5539 bytes)32
```

Total828 bytes. Point already includes StateDigest; State bytes are compared separately against it. Profile/types/actual keys/provenance match generation and fixed cache_meta. WorldHash is the actual kernel hash, not an arbitrary alias for world-bytes digest. Every typed value is checked before descriptor hashing; no Debug/Serde/SQL row serialization. Pin/source consistency and cryptographic signature provenance are different assertions.

Catalog head is SHA256 of125 bytes: `NF-REPLICA-CATALOG-1` NUL21, cache schema32, previous catalog head32, generation u64LE8, descriptor digest32. First previous head is zero32 and generation1; catalog chain binds exactly consecutive local activation generations. It is distinct from source JournalHead. Store-replay source transitions may be many per one cache activation. catalog_commits has no FK to generations because old commit digests can survive pruning immutable generation data.

Retain<=256 catalog commits, ordered full-u64. chain_base_generation/head is the last discarded local commit or zero/zero before any pruning. Only drop the oldest consecutive prefix not needed by a retention pin/required external anchor, atomically updating that base. At least active catalog commit remains. An external anchor whose head has been pruned and cannot be linked returns AnchorGap, not an assumed chain match; the operator must supply a newer separately protected anchor or obtain a fresh explicitly trusted recovery proof. The latter needs the separate recovery port described below, not automatic first-peer trust.

## Current-policy cut, failure ownership and recovery

Installer consumes the previous ReplicaStore (one cache connection) and owns a separate staging connection. It can borrow previous_view while staging; no borrowed view crosses mutable activation. nf-store reads actual persisted membership through the authoritative Store's one existing connection, never opens another Store or imports remote policy. Public view/status methods return value-only data; no Prepare/Commit/ack_outbox/native mutation conversion.

At begin, privately pair original epoch fence and issuer Store instance before transfer progress. A separate `Store::bind_replica_epoch(&mut ReplicaInstaller)->Result<ReplicaEpochInvalidator,ReplicaError>` returns the disable-only handle exactly once; second bind/replacement returns Replay. At issue, require that original fence still live and same issuer/installer; issue_replica_activation returns only the read-only challenge, not a new fence. This refines the architecture's generic issuer route without adding a grant. ReceiptRepo associates that first handle with actual current control/transfer tuples and invalidates it on drift.

Local preimage842 remains the reviewed private NF-SYNC-ACTIVATE-1/purpose1 layout. OS-random nonce32, installer16 and the exact expected bytes are private registry state. First activate attempt takes registry entry before shape/signature verification and before hook; all failures spend it. Fault hook BeforeTransaction runs before opening cache transaction. AfterGenerationWrites runs after all candidate rows/references/mirrors/head are written but uncommitted. BeforeCommit runs **before** final SQL-policy/signature/fence/expiry/minima checks, never after the admissibility cut. Check deadline and original epoch again immediately before commit; no await/reentrant callback/transport poll intervenes. Source signature verification uses actual pinned current source key/PLAYER-owner mapping and exact source-purpose5 context; receiver proof must match private purpose1 and current PLAYER AND REPLICA. The new admission is constructed only inside nf-store.

The same IMMEDIATE cache transaction inserts complete immutable generation/evidence/documents/mirrors, its catalog commit and selected cache_meta pointer; it may prune only proven unreferenced oldest data in that transaction. Full typed validation, quotas and expected base/head must precede writes. BeforeCommit policy drift/invalid signature rolls back everything. AfterCommit/BeforeAcknowledgement hook errors or actual commit I/O uncertainty return Uncertain, no ReplicaActivation/InstallReceipt. A known hook refusal before writes or confirmed healthy rollback returns BeforeCommit with the original cache (None only explicit first initialization). Stage-only I/O error does not quarantine untouched authoritative Store/cache. Cache Full/Io/Corrupt writes/checkpoint errors conservatively quarantine affected cache connection unless rollback and old active state are actually revalidated; rollback failure is always Uncertain. No success from an in-memory guess. Postcommit cleanup failure preserves committed generation and recovery ownership; it cannot label the commit rolled back.

`ReplicaCacheRecovery` owns exact path/config/previous known anchor plus bounded candidate observations. It exposes no trusted World, active permission, signer or SQL connection. `observe(&self)->ReplicaRecoveryObservation` reports untrusted possible old/new generation only. Exact-anchor `ReplicaStore::open_existing` verifies closed schema/typed replay/mirrors/hashes/catalog/known minima and the supplied protected head. If actual selected head is newer than the supplied anchor, return RecoveryRequired until a fresh owner cut verifies it; do not silently trust an ahead counter or cached public key.

Proposed `Store::issue_replica_recovery(&mut ReplicaCacheRecovery)->Result<ReplicaRecoveryChallenge,ReplicaError>` and `Store::recover_replica_cache(ReplicaCacheRecovery, DeviceProof)->Result<ReplicaStore,ReplicaRecoveryFailure>` provide that cut. Recovery challenge uses the same842-byte private layout with purpose2, newly issued local nonce/instance/current membership, actual selected target/generation, stored source proof digest and recorded contexts. Registry is separate from activation but combined capacity1; first attempt consumed. Revalidate current actual source/receiver keys/roles, private expected proof, protected minima/old-anchor catalog continuity and complete actual cache state immediately before release. No cache generation is created or old anchor lowered by recovery. Key/policy incompatibility or missing continuity refuses; no downloaded policy fallback. Success returns a read-only cache plus externally retainable observed current anchor, not an operational session. Its disable-only epoch is new and initially invalid for network activity; network must authenticate fresh independently.

The purpose2 extension/evidence parser and precise cache root admission helpers require their own independent vectors/source-port review before code. They are data/signature validation of an existing SQL result, not authority/automatic policy import. No failure branch creates a replacement database, fabricates a request outcome, resumes a challenge or selects a source not in trusted configuration.
## Quotas, mirrors and reference-aware pruning

Committed active cache counts are bounded before allocation or write, using LIMIT(max+1) and exact typed expected counts; aggregate SQL count/Vec without a ceiling is prohibited.

| Inventory | Hard ceiling |
| --- | ---: |
| cache_meta |1|
| generations/evidence |2 each|
| catalog_commits |256|
| documents |516 (two closures of at most258 each, including manifests)|
| generation_documents |257 per generation,514 total|
| aggregate_state |160 per generation,320 total|
| module_state |32 per generation,64 total|
| request_outcomes |4096 per generation,8192 total|
| reservations |64 per generation,128 total|
| outbox |256 per generation,512 total|
| retention_pins |64|
| stage_meta/state/evidence |1 each|
| stage documents/links |258 /257|
| active/staging SQLite files |256MiB each|
| document spool |2105344 bytes|
| export semantic data/manifest |8MiB /65536 bytes|
| live activation+recovery challenges |1 combined, ephemeral|

Logical references across retained manifests/generations/jobs are charged against the architecture's65536 total edge cap; the two-generation cache's actual reachable object edges are tighter. Raw document length sum for each generation must meet its authenticated manifest<=8MiB, not merely SQLite disk limit. Negotiated maximum per document=min(2105344,257*selected_chunk); no implicit segmentation. Coupled document/store/kernel decode budget is one document-scoped guard (bytes, depth32, entries32768, copied bytes4MiB plus existing stricter type caps), passed before nested allocation; exact charged representation requires independently reviewed source and one-over fixtures. No SQL row ceiling or default decoder success is claimed as total RSS enforcement.

Mirrors are regenerated from complete decoded State exactly as accepted Store1: aggregate revisions, provider draws/cooldowns, full request/intent/binding digest, reservations and undelivered outbox. Verify exact expected rows/no extras across every retained generation. Pending uses phase1/NULL event/rejection; terminal uses phase2/recorded sequence/closed rejection0..17. Historical sequence cannot exceed World's current EventSeq, and a terminal record/outbox must match actual typed outcome/bound operation/job. Reservations match pending typed negative market intents and current market totals. Pending metadata stays inert; module rows are canonical committed provider state, not a worker cursor to execute. No profile2 codes18..21 or miniature authority row.

Retention pin kind1 is an explicitly retained local recovery anchor, not an auth role or network request. Only trusted local retention administration can register/release a pin after matching actual catalog/gen/head. A remote InstallReceipt/NoticeAck cannot unpin anything. Runtime reader/export pins are separate private bounded permits (<=8), lost/revoked on process exit and never persisted as live grants. They still block deletion while alive. A pin on the oldest generation can make a third-generation activation Capacity; never drop it or referenced objects to force progress. Caller must independently retain a newer anchor and explicitly update its local pin before old recovery data can be released; saying an anchor is externally saved is an operator retention responsibility, not cryptographic proof that storage exists elsewhere.

Prune in the same bounded IMMEDIATE cache maintenance/activation transaction: select at most one oldest unreferenced nonactive generation, confirm no durable/local runtime pins, remove that generation's evidence/links/mirrors, then remove at most258 now-unreferenced documents. Both explicit manifest FK and link FKs prevent dangling dependencies. catalog digests survive generation-data pruning until their separate<=256 chain policy permits prefix truncation. Never remove active cache_meta target or any referenced recovery object. If quota still cannot fit, rollback/Capacity. No recursive filesystem deletion or raw SQL selected by caller. Process failure cannot expose an active head pointing to half-deleted closure.

The cache retention feature is not an archive service. It cannot serve peers, use ARCHIVE as implicit permission, advertise offline producer availability or grow an unlimited journal. Any archive has a later separately reviewed owner/provenance/policy/storage profile; its full issue39 gates stay open.

## Exact reopen and comparison rules

Open existing checks root/file policy, size>=100 and<=256MiB, app/version/settings/object inventory/schema digests/integrity/FKs, exact fixed cache configuration, counts and active presence before reading variable blobs. Bounded ValueRef reads validate each column length before Vec allocation. Unknown schema/pin/extra trigger/view/index returns UnsupportedSchema without rewrite; corrupt/foreign scope preserves bytes, no CREATE or repair fallback.

Before first publication, always replay a delta from the exact admitted previous cache state or a consistent source snapshot and compare complete result State/World/head/mirrors. A completed generation then constitutes a locally validated immutable recovery checkpoint, analogous to accepted Store1 checkpointing: its complete State bytes, source-signed manifest target/End and receiver activation proof are bound by descriptor/catalog head and externally protected anchor. Reopen verifies that sealed checkpoint and all retained objects/evidence/mirrors; it does not rerun the original pre-checkpoint provider/job history. A saved State blob/hash or validated flag without this sealed provenance is insufficient. Future delta replay uses that admitted checkpoint as trusted-before state. Earlier base generation may be pruned only once this independent checkpoint is durably selected and no explicit recovery/reader/archive reference still needs the base. This does NOT manufacture an original-source SnapshotDocument or make the cache an archive: replay/serving an original historical delta closure requires its original base recovery data retained under a separate explicit pin, otherwise that historical service is unavailable. Neither checkpoint sealing nor GC grants remote serving or writing permission.

Reconstruct evidence exact750/278/private842 context, verify recorded signatures cryptographically as provenance using keys pinned by the protected anchor/admitted source evidence. Cached keys/signatures cannot initialize current membership or a transport session. Check descriptor828, catalog125, consecutive catalog revisions/base and exact meta selected head. Required externally protected minimum event/store/member/generation counters never lower; equal generation requires exact catalog head, equal source revision requires exact Point. Ahead generation needs fresh recovery verification, not trusting unauthenticated counters. Policy floor is read from separately admitted actual membership, not this cache. An old proof may be valid historical data while unusable as current authorization.

The first explicit cache create has no implicit anchor/genesis: requires trusted local source/scope/profile/pin configuration plus separately admitted current membership and a freshly signed complete source manifest/snapshot. It reserves a new path and never overwrites an existing file. Before first activation there is no World view; empty initial SQL is not an empty canonical history. A crash-created uninitialized file is preserved and requires explicit local recovery/removal of that exact owned artifact, never auto-open-create. Current profile1 source authority is configured/trusted, not a persisted term/lease/quorum certificate.

## Required independent evidence before implementation acceptance

This document records architecture/schema arithmetic only. No SQLite schema execution, typed codec, syscall failure, live policy or process test was run for this future cache. CREATE-block hashes and fixed-width arithmetic below were computed in-process only; they are not runtime/schema conformance evidence.

Independent fixtures must freeze CREATE digest/IDs, all descriptors/catalog/preimages/provenance shapes, mirrored typed genesis/pending/commit/outbox states and malformed/counter/pin cases without production encoder imports. Source ports/private recovery purpose2, composed budgets and exact helper behavior require frozen review before code. Actual compiled public cases: wrong schema/extra trigger/view/index/record, current role/signature/fence failure after hook, consumed retry, empty/invalid stage, policy-only provenance refresh at unchanged source Point, genuine SQL Full and calibrated Linux cache commit EIO, before/after commit/ack process crashes, matching-digest malformed state admission, delta-base retention/GC Capacity, externally known revocation/generation rollback, and actual platform ACL/reparse/cleanup tests. Before/after file/SQL hashes prove refusal preserves prior history.

Success requires a private committed ReplicaActivation only after actual durable cache transaction (or explicitly Existing, already validated committed view). No generic callback/flag, staged SQLite file, unsigned digest, opaque bulk verification or cached proof may mint that result. Hardware power-loss, total RSS, native responsiveness, actual content equivalence and offline archive availability remain distinct unobserved gates. Issue23/39 acceptance is not claimed.

## Frozen schema digests

Active CREATE block LF digest: `d8b76dd62dc4b4b16ffb81dd4f91adcddb92136914581f6a28cbd2bbf996ae26`.

Staging CREATE block LF digest: `2289346aa53020dd4e697509a1ad3df4f5553c444bfb5a2e3a52da630f4d5ec9`.
