# Independent notification fixtures

Test-only Node Buffer/standard crypto writer for frozen NF-NOTIFY-1. No production codec, Rust-generated output, configured identity, authority decision or live Store state is imported. The producer/data are independently owned; production authors must not edit them. The notification fixture's original request12/operation13/binding are the public positive inputs from the independent receipt corpus, with binding `2788a8743e8aecac5c03ca9402c34979f3f64667df33bf83100c4b5671030a93`. This is an immutable selector input, not a new receipt or kernel payload registration.

Raw contract reviewed SHA256 is `f3e818776bb3ee9fe0c660424bc420f938e852c3f7d3ba0eec51466440aa0dce`. Its existing two CR bytes normalize to UTF-8 LF identity `f7aebf226c0d9c03bf8c6c97945f371b333abf180bbc7aae498a70a4fbc4dc8d`; both identities are recorded. Generation pins the LF identity for Git checkout portability, admitting no semantic drift. The frozen contract is not edited by this producer.

`node tools/notification-vectors/generate.cjs` writes JSON and TSV. `--check` requires exact existing bytes; all other arguments reject. Output is UTF-8 LF. JSON carries scenario context and signer keys; TSV columns are category/name/layer/expectation/hex/SHA256. Local model UTF-8 JSON payloads explicitly have `wire_record:false` and are never passed to notification shape decoding. Only Node built-in Buffer, crypto, fs, path, test and assert are used. There is no executable production validator or custom cryptographic arithmetic.

The deterministic RFC8032 client/server seeds are explicitly public synthetic test data paired with their known public keys. They are not operational credentials. Their inline Ed25519 PeerIds are prefix `002408011220` plus32 public-key bytes. Real admission/process tests must use actual fresh private identities and admitted SQL policy. A mathematically valid fixture signature does not establish role, current membership, actual PeerId/ConnectionId, session, expiry or selector admission.

## Exact arithmetic

| Object | Byte arithmetic | Total |
| --- | --- | ---: |
| Header | magic12 + version2 + kind1 + lane1 + session16 + scope32 + policy64 |128|
| Limits | body2 + queue4 + items/rate/burst/challenges8 |14|
| Handshake | domain17 + stage1 + version2 + lane1 + protocol digest32 + peers258 + principals64 + session/scope48 + policy64 + nonces64 + capabilities16 + limits42 + frontier8 |617|
| Subscribe | domain16 + stage1 + context32 + subscription16 + topic1 + request16 + operation16 + binding32 + nonces64 + membership/minimum16 + lifetime2 + reply digest32 |244|
| Notice/Ack purpose | domain19 + stage1 + context32 + subscription16 + sequence8 + request16 + operation16 + binding32 + notice nonce32 + membership8 + prefix32 |212|
| DeviceProof | scope/principal64 + membership8 + fixed PeerField129 + challenge32 + signature64 |297|
| Subscribed fields/prefix | subscription16 + topic1 + request16 + operation16 + binding32 + firstSeq8 + lifetime2; add header128 |91 /219|
| Notice fields/prefix | subscription16 + sequence8 + request16 + operation16 + binding32 + nonce32; add header128 |120 /248|
| Ack fields/prefix | subscription16 + sequence8 + original Notice prefix SHA32 + admitted1; add header128 |57 /185|

Record kinds1..10 have body widths86/365/297/297/123/120/345/388/417/354 and total encoded body widths214/493/425/425/251/248/473/516/545/482 including header128. All positive lengths remain below the selected minimum768. Default body1024, queue16384, items16, rate8/s, burst8, challenge1; the minimum-limit variant is768/768/1/1/1/1. Subscription lifetime1 and30 and exact sequence u64::MAX are included. One-over cannot wrap the standard checked integer writer.

Handshake protocol digest at offset21 is SHA256 of exact ASCII `/nearfuture/peer/notify/1` without a terminal NUL. Namespace, lane3 and stages are distinct from control2/control1/bulk/foundation. Neutral stage0/frontier0 is context hashing only. Subscribe frontier/minimum/lifetime offsets are194/202/210; final reply hash starts212. Notice purpose sequence begins68, frontier172 and prefix180. Wire Notice sequence begins144, binding184 and nonce216; Ack sequence144, original Notice-prefix digest152 and admitted byte184.

Ed25519 signs SHA256 of the exact accepted identity canonical domain8/type5/schema1 preimage (163 bytes for these38-byte PeerIds), not the297-byte fixed wire proof. ServerNotice stage1 hashes header128+Notice fields120; SubscriberAck stage2 hashes header128+Ack fields57. The Ack itself echoes the original Notice prefix digest. These distinct digests bind both the Notice and Ack; no outer length or final proof bytes are hashed in their signed record prefixes.

## Corpus and limits of evidence

The frozen corpus includes16 positive records covering all10 kinds plus min/max lifetime, sequence and limits;8 transcripts;3 exact prefixes;12 primitive signatures;4 values;113 negatives;2 oversized missing-body framing declarations;12 local model scenarios and1 capacity scenario. Every positive body has truncation and trailing cases. Negatives cover unknown/zero tags, actual lane/header/session, limit ceilings/floors, capability bits, selector/topic/lifetime, padding/PeerId, namespace/stage, signature/result-prefix changes, wrong peer/connection, stale epoch and duplicate Notice/Ack. Seven context negatives deliberately retain valid primitive signatures; expected rejection depends on later actual protocol context, not signature shape.

Model scenarios cover burst/refill/rate refusal without Ack, owner-clock reversal,5s boundaries, client subscription start, checked sequence overflow, repeated terminal observation, dirty generation during a query, one awaited+one marker and rotating three-lane polling. They are finite expected inputs only. They do not establish an Instant fault, rate limiter, queue high water, real Store transition, fairness under encrypted flood, backend credit exhaustion or runtime capacity result. A legitimate immutable request still has at most one Pending-to-terminal transition; synthetic model observations are not asserted durable mutations.

Ten Node tests pass exact layouts/digests, field sensitivity, signatures and tamper, valid-signature context negatives, sequence boundaries and data-only classification. A valid compiled617-byte handshake assertion reached behavioral RED when the protocol digest incorrectly included a terminal NUL; removing that NUL produced GREEN. The initial normalized-contract pin mismatch was setup-only, caused by two CR bytes in the reviewed raw document; no source contract changed and no additional behavior RED is claimed.

Observed native command: `npm.cmd exec -- workspace-template verify .tmp/notification-vectors-native --scope module --module node:. --timeout 180000`. Structured steps run all10 tests, generation, exact `--check`. Every child had `windows-job-object`, status0 and no timeout. Historical RED result `.tmp/notification-vectors-red.json` SHA256 `b80a434140dadc38de97fc394f265745edbbaa1669ce0cef0573dfe9f8e1e72c`; final GREEN digest accompanies the frozen checkpoint. No Rust consumer, production auth/broker/process/native evidence, shared manifest/metadata or Git change belongs to this slice.
