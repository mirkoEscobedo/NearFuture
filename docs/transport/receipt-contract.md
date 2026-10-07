# Control2 retained receipt contract

Phase A normative proposal for independent design review. This document registers no running behavior. Source implementation begins only after this finite contract is accepted. Phase B uses the separately specified bounded notification broker; semantic history/snapshot synchronization (C/D) remains deferred until Store36 acceptance and separate design review. Native game responsiveness and issue23 full acceptance remain open.

## Namespace and authority

Keep every NF-PEER-1/control1/bulk1 byte, capability and behavior unchanged. New Rust-only protocol `/nearfuture/peer/control/2` is a separate closed codec and application authentication state machine. Its only protected operation is ReadRetainedReceipt. It cannot submit an Intent, Prepare/Commit, import membership, acknowledge an outbox, install a snapshot or authorize native effects. Successful profile1 outcomes remain Unsupported on control1.

Receipt profile u16=1 means the accepted profile1 `nf-store::Store` retained request metadata. Unknown profiles, including Store2, are not admitted by this registry. Profile2 support needs an explicit later typed owner integration. No foundation/Java SchemaPayload registry or kernel domain7 admission changes.

A CommittedReceipt means a retained `RequestStatus::Committed { rejection: None, .. }` produced by the source Store's durable transaction/replay. It certifies that metadata only, not receipt delivery of a strategic command, the content of an opaque kernel outcome, native projection, quorum, transferable authority or spendable remote state. Rejected is the corresponding retained committed rejection, with a generic rejection marker rather than a kernel payload.

Both endpoints authenticate actual Noise PeerId, then application DeviceProof using current admitted durable membership. The client additionally pins the actual server peer/account/device to the current membership owner and active PLAYER role; the server requires the client's current active PLAYER role and exact account/device mapping. Reuse the accepted Economic policy check as a conservative receipt-read gate; no incoming host bit, new role or Noise-only authority is introduced. Current device key/role/frontier are reloaded from the one actual Store owner at every protected stage. A membership revision change fences the session and requires a new handshake, including unchanged numeric status after revocation.

## Exact framing and fields

An outer u32 big-endian body length is checked before body allocation. All integers inside a body/transcript are unsigned little endian. Exactly one record occupies the body; trailing bytes, unknown kinds, invalid fixed widths/padding and another actual lane reject. There is no variable text or compressed/opaque payload. The actual selected protocol and Swarm event determine namespace/lane; an incoming header cannot choose them.

Header is fixed126 bytes: ASCII `NF-PEER-2` plus NUL (10), version u16=2, kind u8, actual lane u8=1, session16, universe16, history16, ruleset32, content32. Scope/policy must equal trusted local configuration before operation admission. Only Hello uses zero session; the server creates a fresh nonzero OS-random16 session, and all later records match the active session exactly.

PeerField is length u8 in1..128 followed by128 bytes, with zero unused padding and a canonical parse/round-trip PeerId in the used prefix. DeviceProof is297 bytes: universe16/history16/account16/device16/frontier u64/PeerField129/challenge32/signature64. It is data until cryptographic admission. Record scope must equal proof scope. Request/operation/account/device identifiers are nonzero16 bytes when present; all nonces are fresh OS-random32 bytes, never copied from a saved session. No fixed-width ID permits truncation.

Reuse the exact validated26-byte PeerLimits layout: control frame u32, bulk frame u32, chunk u32, control queue bytes u32, bulk queue bytes u32, control items u16, bulk items u16, pending challenges u16. Hard maxima remain4096/9216/8192/65536/131072/16/4/8. Before limits selection a receiver enforces its admitted local advertised control cap; after selection it enforces the exact selected cap. Both frames are at least1024; other fields positive, queues admit one maximum frame, chunk<=bulk frame-146. Selected limits are componentwise minima of two admitted sets. Bulk fields are transcript-bound compatibility fields only and grant no control2 bulk operation. Required capability is exactly bit1 ReadRetainedReceipt, server available and selected are exactly1. Optional u32 may contain ignored unknown bits; all offered bits are transcript-bound and cannot authorize another body. There is no silent fallback to control1 or another negotiated capability.

All following widths exclude header/framing. Fields are listed in exact order.

| Kind | Body | Width |
| --- | --- | ---: |
|1 Hello|client account16, device16, client nonce32, required u32, optional u32, offered Limits26|98|
|2 ServerHello|server nonce32, available u32, selected u32, server Limits26, selected Limits26, server DeviceProof297|389|
|3 ClientProof|client DeviceProof297|297|
|4 Finished|server DeviceProof297|297|
|5 BeginReceipt|request16, expected operation16, expected binding digest32, profile u16=1, client operation nonce32, minimum membership u64, minimum source EventSeq u64, minimum source StoreRevision u64|122|
|6 ReceiptChallenge|request16, expected operation16, expected binding digest32, profile u16=1, client operation nonce32, server operation nonce32, current membership u64, challenge32|170|
|7 ProveReceipt|request16, client operation nonce32, client DeviceProof297|345|
|8 ReceiptStatus|request16, original account16, original device16, operation16, binding digest32, phase u8, sequence-present u8, commit EventSeq u64, rejection u8, profile u16=1, source StoreRevision u64, source current EventSeq u64, source current MembershipRevision u64, server DeviceProof297|430|
|9 ReceiptUnsupported|request16, reason u8, profile u16=1, source StoreRevision u64, source current EventSeq u64, source current MembershipRevision u64, server DeviceProof297|340|

ReceiptStatus phases:1 Unknown,2 Pending,3 Rejected,4 CommittedReceipt. Unknown has zero operation/digest, sequence-present0/sequence0/rejection0; original account/device echo the authenticated querying principal. Pending has the exact expected nonzero operation and binding, no sequence and rejection0. Rejected has exact expected operation/binding, sequence-present1/nonzero commit sequence and rejection1. CommittedReceipt has the same sequence rules and rejection0. There is no other phase/rejection/presence value. Original account/device must equal the active client and retained owner. The client independently supplies expected operation/binding before the query; it never adopts an arbitrary returned digest as its expectation.

`binding_digest` means accepted `RequestBinding::digest()`. In accepted profile1 its payload_digest is `nf_kernel::intent_digest(intent)`, so the returned binding commits the full canonical intent hash as well as RequestId/account/device/scope/operation kind. The typed client descriptor retains the original operation kind and immutable payload_digest and independently derives the exact accepted133-byte RequestBinding digest before encoding or recovering a query. Operation kinds are1 SeekPeace,2 AdjustRelation,3 AdjustMarket; unknown kinds reject. A changed original payload therefore changes the independently expected digest and conflicts with the retained binding. Receipt control2 carries no Intent and cannot submit a retry; actual command owners retain `Store::query(&Intent, device)` and its explicit full-intent check. Unknown/Pending is never permission to build a different intent or repeat a network mutation. This slice exposes no mutating network route.

Source current EventSeq/StoreRevision are monotone current-source frontiers, not the historical commit EventSeq. An old valid receipt may have commit EventSeq below a protected current EventSeq. Compare supplied minima against current-source fields, and require current EventSeq>=commit sequence for terminal phases. Current MembershipRevision equals the fresh admitted membership and DeviceProof.frontier. Do not compare historical commit sequence to the client's current-source minimum.

ReceiptUnsupported reasons1 BindingConflict and2 SourceBelowKnownMinima are closed. Bind the original expected request/op/digest in its transcript, although the result prefix echoes only request. A retained request of another principal/device closes as authorization failure, without revealing its binding/status. A known principal's changed operation/digest returns BindingConflict only after a fresh admitted proof. SourceBelowKnownMinima is signed only after fresh current membership admission when the source's event/store frontier is below the supplied minimum. A source below the membership minimum cannot issue an admitted operation challenge and closes instead. Unknown profile/body/version fails shape admission locally; storage failure/quarantine or unavailable current policy cannot fabricate a signed current stamp. No Unsupported result is emitted before fresh operation proof. Neither Unsupported reason advances recovery minima or completes a request.

## Exact signed transcripts

Use maintained SHA256 and the existing maintained nf-identity DeviceProof signing/verification. No protobuf reserialization or implicit field normalization is hashed. Actual PeerIds come from Noise events. Actual ConnectionId is a private state key, not a serialized assertion; a fresh server session and both handshake nonces bind the authenticated transcript to that exact connection, and every method requires the matching actual connection event. Neither proof nor session migrates across a reconnect.

Handshake transcript is619 bytes, the same field order as accepted control1 but with domain `NF-PEER-AUTH-2` plus NUL (15), protocol version2 and actual lane1:

```text
prefix15, stage u8 (serverHello1/clientProof2/serverFinished3), version u16=2, lane u8=1
actual client PeerField129, actual server PeerField129
client account16/device16, server account16/device16
fresh session16, universe16/history16, ruleset32/content32
client nonce32, server nonce32
required u32, optional u32, server available u32, selected u32
offered Limits26, server Limits26, selected Limits26
proof membership frontier u64
```

Each cryptographic stage consumes its pending state on the first attempt, including malformed, wrong-peer, stale-policy and failed-signature attempts. The neutral context digest is SHA256 of that exact transcript using stage0/frontier0; it is not a DeviceProof authorization stage. Version/domain/capability binding forbids control1 reflection or transport-namespace-only authentication. Server Finished confirms the exact admitted context before either endpoint admits a receipt operation.

Receipt operation challenge is SHA256 of fixed245 bytes:

```text
NF-PEER-RECEIPT-2 NUL (18)
stage u8 (clientOperation1/serverReply2)
neutral handshake digest32
request16, expected operation16, expected binding digest32, profile u16=1
client operation nonce32, server operation nonce32
current membership u64, client minimum membership u64
client minimum source EventSeq u64, client minimum source StoreRevision u64
reply-prefix digest32
```

Stage1 uses zero reply digest. Stage2 uses SHA256 of exact header126 plus fields preceding the server DeviceProof: ReceiptStatus prefix133 (259 hashed bytes), ReceiptUnsupported prefix43 (169 hashed bytes). Proof and outer u32 framing are excluded. Header kind/lane/version/session/scope/policy and every status/frontier byte are therefore signed. The challenge echoes and proofs use the exact received minimum fields. Trusted owner minima are combined componentwise before BeginReceipt is encoded; never normalize incoming minima after signing. Both stage DeviceProofs bind their actual device/principal and current membership through existing device_digest.

Server state: active -> one pending BeginReceipt descriptor with both nonces/exact wire minima/actual peer+ConnectionId/session -> first-attempt proof consumes pending -> fresh membership authorization -> bound data lookup -> signed status. Pending expires after actual5s; there is at most one outstanding operation per connection, below the selected challenge cap. No retry restarts a consumed challenge. Reconnect creates a new session/challenge. Client likewise consumes AwaitReply before validating exact transcript/current server policy; duplicate, old-session, wrong-stage, contradictory or reordered status cannot be installed into recovery metadata.

## Sole storage owner and consistent observation

The foreground repository owns one accepted Store and actual transport/device identity. No codec, asynchronous handler or query queue can open another Store or invoke prepare/commit/CAS/outbox acknowledgement. Trusted server setup can create/commit an operation through the existing owner APIs for tests; this is distinct from receipt transport.

The proposed typed client descriptor is `OriginalReceipt { request, operation, original: RequestBinding, source_pin, profile: 1 }`; original.account_id/device_id/universe_id/history_id must match the actual local identity and configured scope, original.request_id must match request, and expected binding is computed rather than caller-overridden. The foreground `ReceiptOwner` holds the Store, borrowed vault, current identity/pin and externally protected slot anchors. Its begin/query/recover paths accept that data descriptor; crypto/actual Swarm events and private one-use state remain owner-derived. A returned `AdmittedReceipt` is observable receipt metadata only, with no conversion to Store DurableAck or kernel/native authorization.

Existing `Store::query_bound(request, account, device, scope)` returns binding_digest and `Pending { operation }` or `Committed { operation, sequence, rejection }`. `known_frontiers()` returns current event/store/member frontiers; `MembershipRepository::load_membership(&mut self, scope)` supplies admitted current public policy. These accepted read-only seams suffice: no additional Store API or SQL schema change is needed for PhaseA. Capture query result, known frontiers and fresh membership in one synchronous owner step, with no await, mutating owner callback or shared writable alias between observations. A missing/inconsistent membership revision rejects. Once captured, queued response admission retains its checked policy epoch; an owner membership change before send fences/discards it. The receiver independently checks its own current durable policy again.

Every begin, handshake proof, operation proof, response acceptance and queued response flush checks current policy/pin, actual peer/connection, exact scope/ruleset/content and selected limits. Store metadata can advance after BeginReceipt without a policy change; the response binds the actual current frontiers. Revocation/rotation/frontier change requires a new handshake even if request phase remains identical. No cached PLAYER snapshot or caller-claimed account initializes a grant.

## Finite persisted reconnect book

The foreground receipt repository contains the sole Store owner plus borrowed accepted PrivateVault, trusted server pin and externally protected recovery minima. Its immutable receipt book is separate observational recovery metadata, not a canonical World/SQLite transition. Use existing `create_private_blob`, `read_private_blob`, and explicitly authorized exact-name removal only. No second database, synthetic kernel commit, private identity key serialization, service or detached writer is introduced.

Hard capacity is8 request slots, each with8 immutable generations0..7, names `receipt-r<slot>-g<generation>` under that configured vault; no peer/caller path or arbitrary filename is accepted. Each canonical record is479 bytes and wrapper adds46 (maximum64 files/33600 wrapper bytes). Generation0 has zero previous digest; later generations contain SHA256 of the previous exact479-byte record. The owner exclusively controls these names; cooperating callers cannot replace/remove/reuse them during its lifecycle. No atomic cross-owner compare/unlink guarantee is claimed.

Exact479-byte record order:

```text
NF-RECEIPT-BOOK-1 NUL (18), book version u16=1
receipt profile u16=1, generation u8, previous-record SHA25632
universe16/history16, ruleset32/content32
local account16/device16, pinned source account16/device16, pinned source PeerField129
request16, expected operation16, original operation kind u32, immutable payload_digest32, expected binding digest32
protected source EventSeq u64, StoreRevision u64, MembershipRevision u64
last phase u8 (0 Unobserved or1..4 as above), sequence-present u8, commit EventSeq u64, rejection u8
```

Descriptor/profile/pin/principal/request/op/kind/payload_digest/binding remain identical across generations. Recompute the expected accepted RequestBinding digest from the retained original fields on every load; mismatch rejects before any query. Minima never decrease. An Unobserved record has absent sequence/rejection; terminal metadata follows ReceiptStatus rules. Unknown may progress to Pending/terminal; Pending cannot regress to Unknown and a retained terminal cannot change operation/sequence/rejection or regress. These checks detect contradictory observations, not new authorization. No saved proof, nonce, session, ConnectionId, active challenge, Activity/Claim or permission is persisted.

Before the first network query, write/sync generation0 containing the trusted original descriptor and combined minima. After a valid status, raise minima by componentwise maximum of prior protected/external/received current-source frontiers and append a generation if phase/sequence/minima change. Only report the newly admitted public receipt after the immutable write/sync succeeds. An identical valid status at identical protected minima needs no generation. At generation7 or exhausted request slots, a required append returns local Busy before public fresh receipt/minimum advancement; it never discards history to make room. A signed Unsupported is reported as unsupported and never appended as terminal or used to raise minima. The owner can explicitly retire a finished slot only after independently retaining its protected minima/head anchor; automatic offline retirement or request-ID replacement is excluded.

Each external slot anchor is an exact `(slot, generation, SHA256(record))` plus original descriptor and remote minima; a recovered chain must contain that exact anchored generation/digest and may extend it only with valid later generations. No caller-supplied wire anchor initializes local trust. Recovery loads only explicitly configured original slots and bounded names, verifies all present generations' fixed format/chain/unchanged descriptor/monotone metadata, and compares trusted external scope/pin/principal plus minimum generation/head digest and remote event/store/member minima. All files are bounded by accepted blob admission before decode. Gaps, unknown roots, unexpected later files, invalid tails, unreadable/corrupt records, collisions or a different principal/pin reject; never skip a bad tail, invent a new ID or regenerate missing state. A failed new-generation write leaves the previous valid prefix unchanged; a partial artifact is retained as a fail-closed recovery error until explicit owner repair of that exact name. Cleanup never extends outside the vault.

SHA chaining and owner ACL/checksum do not prove backup freshness: deletion to an older valid prefix is undetectable without the external generation/head anchor. External remote minima alone do not prove which requests existed; externally anchored book inventory/head is required where rollback protection is claimed. Loading saved terminal metadata yields an observational last-known view only. Reconnect always reauthenticates and queries the same original descriptor; saved state never authorizes receipt acceptance or another command. Readiness and source identity load are explicit; missing transport/device identity fails, without regeneration.

## Bounds and scheduling

Receipt control2 replaces the control lane for its owner rather than silently adding another control connection; control1 remains available in separate explicit compositions. PhaseB's third physical lane has its own independent limits/accounting and no shared session/proof. No control2 bulk or notification body is admitted.

Use the accepted maintained TCP/Noise/Yamux dependencies and small bounded mux configuration, without new dependencies. Enforce selected control frame size before allocation and encode, one active operation, <=selected control items and <=selected control queue bytes reserved before enqueue. Account the framing overhead separately (+4 per queued record), fixed typed objects, challenge state and immutable book buffers. Largest new reply is556 body bytes; handshake maximum515, under the minimum1024 frame. Declared length above selected/hard cap rejects before body allocation. There is no nested variable semantic decoder on this route.

Preserve maintained Yamux's separate<=1MiB transient DATA parser allocation per physical connection before receive-credit validation; the stream window is not total heap/RSS. SQLite/ACL/cryptography runs in the foreground repository owner, outside game frames. One operation has5s challenge/request deadlines, bounded per-event polling and exact cleanup on close/cancel/deadline. No total RSS or game-frame p99 claim follows. Scope/history/role/minimum failures cannot reserve an unbounded queue; accepted queued responses are released only by their exact flushed/failed owner ticket, never silently dropped as a strategic transaction.

## Implementation and independent evidence slices

Proposed production ownership is new `nf-transport/src/receipt/**`, narrow explicit foreground-owner composition and public exports after root assigns them. Root owns shared protocol dispatch/manifests/architecture/CI; accepted control1 files/vectors are preserved. No Store source changes are proposed. Book metadata is implemented in the new receipt owner, borrowing the existing vault. Notification author owns the separate PhaseB contract/codec and references the receipt's exact immutable descriptor; its authentication namespace/session/challenge is independent and notice acknowledgement cannot acknowledge this book or Store outbox.

A separate test-only Node Buffer/crypto producer must freeze raw header/handshake245-operation/status/unsupported/book vectors before production codec tests. Compute SHA256/transcript bytes without importing production Rust, and verify real maintained Ed25519 signatures using disposable test identities; commit public keys/bytes only, not operational seeds. Corpus includes every width/truncation/trailing/padding case, domain/version/optional-bit/limit mutation, incorrect expected binding/op/principal, signed result-prefix tampering, old session/connection proof, and wire minima different from trusted minima. Keep Node fixtures/source independently owned and reproducible `--check`.

Meaningful public RED/GREEN first compiles against real APIs and fails an intended receipt behavior assertion. Run two actual encrypted owned foreground peer processes with output readiness, absolute deadlines, RAII kill/reap and Windows Job Object supervision; require Linux hosted equivalents. Through the source-owned Store create a real bound operation and commit with one economic debit/event, arrange loss before receipt delivery, restart the exact server identity/database, then query the same original RequestId/op/binding repeatedly. Assert metadata matches retained OperationId/commit EventSeq, no new economic debit/event/store mutation, and reconnect creates fresh session/nonces. This is lost-reply query recovery, not duplicated mutating network-command evidence.

Add actual current-client and current-server revocation during unchanged status, below-known current-source frontier versus valid older commit EventSeq, Pending/Unknown refusal to reexecute, changed binding/conflicting principal, first-attempt consume, expiry/old reply, book64-file/capacity refusal, failed append/no fresh public receipt, anchored valid-backup rollback and corrupt/gap/collision rejection. Real parser/queue saturation tests must show a bounded receipt while the independent bulk/notification lane is active; do not infer backend credit or total memory from a synthetic queue test. Verify all existing control1/bulk1 fixtures and tests unchanged.

Freeze source/inventory and run author-independent read-only review before publication. Whole issue23 remains open for native game responsiveness and deferred semantic history/snapshot sync; this finite receipt slice does not close it.
