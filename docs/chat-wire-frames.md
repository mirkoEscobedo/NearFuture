# Bounded Chat wire values

This profile encodes canonical values for `/nearfuture/chat/1`, now mounted by [production Noise delivery](chat-peer-delivery.md). The value codec itself does not build a Noise swarm, authenticate a sender, dispatch a store operation or mark an outbox Delivered. Shape decoding carries signatures; the application still verifies current membership, operation-bound proof, observed Noise peer and retained receiver pins.

A four-byte big-endian length admits a body of 65..4096 bytes before allocating it. The body has the exact 15-byte `NF-CHAT-WIRE-1` domain with trailing NUL, one tag, policy digest32 and nonzero correlation request16. Integers are big-endian; byte lengths and exact EOF prevent alternate encodings. Unknown domains, tags or refusal codes fail closed.

| Tag | Value | Body bytes |
| --- | --- | --- |
| 1 | PostChallenge with one canonical signed message | 240..2287 |
| 2 | PostProof with the message, ticket and DeviceProof | 427..2601 |
| 3 | Issued ticket/challenge/membership revision | 120 |
| 4 | Existing canonical signed receiver receipt | 387 |
| 5 | Closed refusal code | 65 |

The largest PostProof is common64 + message length2 + canonical message2157 + author signature64 + ticket16 + DeviceProof298 =2601 bytes, 2605 with prefix. DeviceProof is scope32/account16/device16/frontier8/peer length2/peer1..128/challenge32/signature64. Its peer is a signature claim; the production adapter compares it against the observed Noise peer. Frontier0 remains structurally valid for a founder. Request tags1/2 and response tags3/4/5 are fenced by the libp2p codec direction methods.

Shared message and receipt value codecs delegate existing canonical storage codecs without changing domains, signature meanings or schema. Delivered correlation may differ from the retained original request, allowing a later authenticated alias exchange to carry the original receipt. ClientOutbox acknowledgement still verifies that receipt against its retained profile. Decoding a valid-shaped bad signature alone must not yield application authority.

The first public RED reached Unsupported versus the independently expected authentic maximum PostProof at line51 after five genuine fixture assertions and five preceding case assertions. The fixture uses actual signed PLAYER admission, a genuine maximum2048-byte Unicode post, receiver close/reopen, production Bob receipt with independent signature verification, and a fresh current sender proof. Compilation passed on pinned Rust1.98.1 with locked offline dependencies and no warnings. All later frame/framing/storage assertions were unreached at that RED.

Returning the already parsed, canonical-reencoded frame made the original positive case pass unchanged in 1.08 seconds: ten physical assertions expand to42 across all five frame families, plus five fixture assertions. Seven independent controls then passed in 4.34 seconds, covering all six closed refusal codes, all five tags, exact EOF/truncation, shape and binding fences, prefix-only body-read traps, directions, and valid-shaped forged signatures refused by actual store authority. These are separate codec runs, not a fresh run of the earlier Chat inventory or a real Noise measurement.

Later slices qualify [production two-peer delivery/reconnect](chat-peer-delivery.md), [permitted history](chat-peer-history.md), and [CLI/Java status](chat-cli-java-status.md) separately. Presence, expiration/retention, mute/block/moderation, game/Intel integration and real-game save/reload/performance remain separate work. Source sequences and receiver cursors retain their existing local meanings. No financial authority, permanent availability or full issue25 completion is claimed.
