# NF-PEER-1 fixed record and proof contract

Normative candidate version1 for the partial Rust-only transport. This registers no public foundation/Java/kernel payload schema. Strategic writes, kernel-success results, semantic history/snapshot synchronization and Pub/Sub are Unsupported. The actual listener protocol determines lane; an incoming lane claim cannot select it.

Protocol names: `/nearfuture/peer/control/1`, `/nearfuture/peer/bulk-verify/1`. Each request-response body has a u32 big-endian length prefix checked before allocation. Within the body all integer fields are little endian. Exactly one record occupies the body and trailing bytes reject.

The fixed 126-byte header is `NF-PEER-1\0` (10 bytes), version u16=1, kind u8, lane u8 (control1/bulk2), app session16, universe16, history16, ruleset32, content32. App session is zero only for Hello; every later record requires a nonzero exact active session. Scope/policy must match trusted configuration before operation admission.

PeerField is a u8 length1..128 followed by128 bytes. Unused bytes must be zero; the used prefix must parse and round-trip as the actual canonical PeerId. DeviceProof is fixed297 bytes in existing semantic order: universe16/history16/account16/device16/frontier u64/PeerField129/challenge32/signature64. Shape decoding performs no cryptographic authorization. Proof scope must equal header scope.

Limits are fixed26 bytes: control frame u32, bulk frame u32, chunk bytes u32, control queue bytes u32, bulk queue bytes u32, control items u16, bulk items u16, pending challenges u16. Hard ceilings are4096/9216/8192/65536/131072/16/4/8 respectively. Frame minimum is1024; all other limits are positive. Queue bytes must admit one maximum frame; chunk bytes must be <=bulk frame-146. Negotiation is exact componentwise minimum of both valid sets. These limits cover the fixed codec/application queues, not total process RSS.

Required capability bit1 is retained status on control; bit2 is opaque digest verification on bulk. Unknown required bits reject. Known optional bits are intersected; selected capabilities include required bits. No registration or authority follows from capability negotiation.

| Kind | Lane | Fixed body fields in order (bytes unless stated) |
| --- | --- | --- |
|1 Hello|Either|client account16, device16, client nonce32, required caps u32, optional caps u32, offered Limits26 (98)|
|2 ServerHello|Either|server nonce32, server available caps u32, selected caps u32, server Limits26, selected Limits26, server DeviceProof297 (389)|
|3 ClientProof|Either|DeviceProof297|
|4 Finished|Either|server DeviceProof297|
|5 BeginQuery|Control|request16, client operation nonce32, known minimum membership u64 (56)|
|6 QueryChallenge|Control|request16, client operation nonce32, server operation nonce32, current membership u64, challenge32 (120)|
|7 ProveQuery|Control|request16, client operation nonce32, DeviceProof297 (345)|
|8 RetainedStatus|Control|request16, operation16, retained binding digest32, phase u8, sequence-present u8, sequence u64, rejection u8, server DeviceProof297 (372)|
|9 Unsupported|Either|request/transfer16, closed reason u8, server DeviceProof297 (314)|
|10 BeginBulk|Bulk|transfer16, total u64, count u16, whole bytes SHA25632, client operation nonce32, minimum known membership u64 (98)|
|11 BulkChallenge|Bulk|same fields as QueryChallenge, using transfer ID (120)|
|12 ProveBulk|Bulk|same fields as ProveQuery, using transfer ID (345)|
|13 BulkChunk|Bulk|transfer16, index u16, data length u16, exact nonempty data <=negotiated chunk (20+data)|
|14 BulkVerified|Bulk|transfer16, total u64, whole bytes digest32, server DeviceProof297 (353)|
|15 BulkProgress|Bulk|transfer16, next index u16, accepted total u64, server DeviceProof297 (323)|
|16 BulkReady|Bulk|transfer16, server DeviceProof297 (313)|

Retained phase1=UnknownRequest,2=Pending,3=Rejected; zero/other rejects. UnknownRequest requires operation/binding digest zero and no sequence/rejection. Pending has no sequence/rejection. Rejected requires sequence-present1/nonzero sequence and rejection1 meaning generic retained kernel rejection; this version does not transmit a kernel rejection payload. Absent sequence requires both presence0 and numeric zero. Unsupported reasons1=KernelOutcome,2=HistorySync,3=SnapshotInstall,4=PubSub,5=Unregistered,6=Offline,7=PolicyDenied,8=Backpressure. No arbitrary diagnostic text exists.

Bulk total is1..1048576 bytes, count1..128 and total<=count*negotiated chunk. Index is0..127 at shape admission and exact-next/count at transfer admission. Digest verifies exact ordered original transfer bytes. Opaque BulkVerified is not a canonical decoder, semantic installation or journal acknowledgement.

## Proof transcript

All hashes are maintained SHA256 of these exact bytes; no protobuf reserialization is hashed. Existing nf-identity device_digest then signs DeviceProof, including scope/account/device/frontier/actual peer/challenge. Actual peers come from Noise Swarm events, not record claims.

Handshake challenge is SHA256 of the fixed619-byte transcript:

```text
NF-PEER-AUTH-1 NUL (15)
stage u8 (serverHello1/clientProof2/serverFinished3)
version u16=1, actual lane u8
actual client PeerField129, actual server PeerField129
client account16/device16, server account16/device16
app session16, universe16/history16, ruleset32/content32
client nonce32, server nonce32
client required caps u32, optional caps u32, server available caps u32, selected caps u32
offered Limits26, server Limits26, selected Limits26
proof membership frontier u64
```

The connection owner consumes every pending handshake stage exactly once. Client verifies the pinned server actual Noise peer, exact trusted owner/account/device mapping and current admitted active PLAYER role before activation; server verifies current active PLAYER DeviceProof before admitting a client session. The third distinct server proof confirms accepted context. Membership frontiers are checked against current durable state and trusted minimum known frontier; no incoming claim initializes or advances membership.

The neutral handshake context digest is SHA256 of the same transcript with stage0 and frontier0. Query challenge is SHA256 of fixed177 bytes:

```text
NF-PEER-QUERY-1 NUL (16)
stage u8 (clientOperation1/serverReply2)
neutral handshake context digest32
request16, client operation nonce32, server operation nonce32
current membership frontier u64, client minimum known frontier u64 (explicit BeginQuery/BeginBulk field; current frontier must be >=minimum)
reply-body digest32 (zero for client proof; SHA256 of exact header126 + body prefix preceding server DeviceProof for reply)
```

For bulk, a distinct `NF-PEER-BULK-1`+NUL domain replaces the query domain, with transfer16 replacing request16 and descriptor digest32 included before reply digest. Descriptor is transfer16/total u64/count u16/whole bytes digest32 (58 bytes); the explicit minimum known frontier is separately bound as its own u64 in the208-byte proof transcript; its SHA256 is bound in the challenge. The exact transcript length is208 bytes: domain15/stage1/context32/transfer16/client nonce32/server nonce32/current frontier8/client minimum frontier8/descriptor digest32/reply prefix digest32. It grants no installation authority.

Pending operation challenges key actual per-lane ConnectionId/PeerId, app session, operation kind, request or whole transfer descriptor, both nonces and frontiers. This owner admits one operation per lane, below each selected pending-challenge limit, and at most two pending operation challenges globally (below the hard ceiling8); they expire5s and first proof attempt consumes them. Closing a connection, timeout or reconnect invalidates its session and all pending/queued values. A proof for another request/connection/session/lane cannot authorize this one. Every protected lookup reloads durable membership; each bulk progress step rechecks its admitted frontier/current active role and aborts after policy change. Server reply DeviceProof binds the exact retained/unsupported result, and the client verifies current server policy again before accepting it.

## Initial public API contract

Owned `records` module exposes typed Lane, PeerLimits, PeerContext, PeerRecord, PeerBody, RetainedPhase and UnsupportedReason plus `decode_body(input,actual_lane,limits)` / `encode_body(record,actual_lane,limits)`. Boundary errors are finite static enum values; no peer/credential/path text is copied into errors. `nf-identity::DeviceProof` is a data-only codec field and grants nothing until the actual Swarm owner verifies it.

Owned effects expose private TransportIdentity create/load/peer_id, actual two-lane Swarm composition, typed trusted ServerPin, and foreground PeerServer plus sequential and paired client helpers. Retained replies are admitted only after actual-peer/fresh-proof/durable-policy verification. The sole storage owner may query retained binding and outcome; no transport route can manufacture a committed success or mutate world/membership. Exact async event signatures settle through public vertical RED/GREEN, without changing this record contract silently.

Reply prefix hashes include the complete encoded126-byte header (including actual lane, kind, app session, scope and policy) plus precisely the fields before the final server DeviceProof: RetainedStatus75 bytes, Unsupported17 bytes, BulkVerified56 bytes, BulkProgress26 bytes, BulkReady16 bytes. No proof bytes or framing length prefix are included. The client recomputes this hash from the admitted exact record and verifies stage2 of the matching operation transcript. Unsupported may communicate an authenticated retained-operation result only after an active session, a fresh consumed operation proof and current policy checks. Pre-authentication failures close the connection with no authoritative result. Unknown record/lane/version at shape admission is a static local error, never an authenticated server result.

Maintained yamux0.14.1 has a distinct parser residual: frame/io.rs admits a body up to1MiB and allocates it before connection/stream-credit validation. Thus a bulk256KiB receive-credit window is not a256KiB parser or total-memory bound. Conservative transport accounting includes up to1MiB temporary frame body per physical connection plus connection/stream/application state; oversized frame declarations above1MiB fail the maintained parser. Whole process RSS remains unmeasured.

The initial retained-query owner admits one outstanding protected operation per active connection, below the advertised ceilings. Every Begin/challenge/proof/reply reloads actual SQLite membership. Any revision change fences that session and requires a fresh handshake; no automatic remote membership update exists. Queue byte quotas count exact encoded body bytes before enqueue; fixed framing prefixes add four bytes per queued item, and typed objects, parser temporaries and backend buffers are separate finite residuals. Response reservations bind an owner instance and local ticket, release only on the corresponding flushed ResponseSent event, and never silently drop accepted records.

BulkReady is accepted only in AwaitReady after the one-use client proof; it admits exactly the bound transfer and zero bytes. Chunks are strictly ordered, nonempty and bounded. BulkProgress is signed stage2 and accepted only for the exact next index and cumulative bytes of the most recently sent nonfinal chunk (index1..127, total>0); duplicates, old replies, reordered values and wrong stages consume/reject the pending stage. Final BulkVerified must match declared count, exact total and whole original-byte digest. Every server chunk and client Ready/progress/final reply uses the trusted owner's current durable membership, so a revision or revocation change fences both numeric-identical replies and transfers. No transfer retains a whole document or installs bytes: the server maintains an incremental digest and fixed counters only.

The foreground `query_and_verify_bulk` composition owns exactly one SQLite Store and two physical Swarms. It allows one awaited request per lane and polls bounded events from both. Fresh policy lookup shares the sole Store owner; it does not open a second authority owner or cache a remote grant. Individual foreground query and bulk helpers remain available for sequential operations. Async deadlines apply to these headless background effects; synchronous SQLite/ACL/cryptography work is outside a game frame and has no hard realtime timing claim.