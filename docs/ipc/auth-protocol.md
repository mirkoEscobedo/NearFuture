# NF-IPC-AUTH-1

Authentication lives in `protocol/near_future/ipc/v1/local_auth.proto` as a separate `LocalAuthEnvelope`; foundation `ControlEnvelope`, RequiredSemantics [1]/[1], probe schema 1 and the closed canonical registry remain unchanged. The separate header is auth version 1, capability 2, schema 2 and a present nonzero RuntimeSession. Capability/schema 2 authorizes only these authentication messages; it grants no SchemaPayload 2 or kernel-domain admission. No legacy raw-token fallback is accepted by these listeners.

Every message requires exactly one body: hello (field 10), challenge (11), proof (12), accepted (13). Unknown/duplicate fields, multiple bodies, missing required fields, invalid enum/range and noncanonical wire encodings reject. Each envelope is <=4096 bytes and can be further lowered by a Rust caller; preflight limits depth to 8, total fields to 256 and collection items to 64. Nonce and proof bytes are exactly 32. Declared 33-byte values reject before reading/allocating payload even if truncated. Short values are semantic errors. Java baseline decoder uses the same fixed authentication ceilings; it has no negotiated lower-limit overload. Production Java control/bulk activation beyond this headless client remains future work.

Hello contains a fresh CSPRNG client nonce, role CONTROL=1 or BULK=2, validated offered ResourceLimits, exact trusted account/device and ProtocolRange 1..1. Challenge contains a fresh server nonce, componentwise minimum of offered and the preregistered descriptor's local limits, and server proof. The client checks that exact minimum and verifies the server proof before emitting client proof. The server verifies client proof before activation and returns a finished proof; the client activates only after verifying it. Failure poisons/consumes the handshake and owned token material is zeroized. Each reconnect starts with a new client nonce; each server challenge uses a new nonce. Server restart also rotates runtime session and private token.

The proof transcript is fixed at 306 bytes, not Protobuf reserialization:

| Order | Encoding |
| --- | --- |
| Domain | ASCII `NF-IPC-AUTH-1` followed by NUL (14 bytes) |
| Stage | u8: server challenge 1, client proof 2, server finished 3 |
| Versions | u32 LE each: authVersion=1, capability=2, schema=2, selectedProtocol=1 |
| Endpoint | u8 role, u16 LE listener port |
| Session | u64 LE runtime session |
| Scope | universe 16 bytes, history 16 bytes |
| Policy | ruleset 32 bytes, content policy 32 bytes |
| Principal | account 16 bytes, device 16 bytes |
| Freshness | client nonce 32 bytes, server nonce 32 bytes |
| Offered limits | seven u32 LE values in ResourceLimits field order, followed by transfer u64 LE |
| Selected limits | same 36-byte layout |

The seven fields are control_frame_bytes, chunk_bytes, inflight_bytes, inflight_items, decoded_bytes, collection_items and nesting_depth. HMAC-SHA256 uses the private 32-byte token as key. Rust uses exact maintained hmac 0.12.1 and sha2 0.10.9 with verification through the maintained MAC API; Java uses JDK `Mac` and constant-time digest equality. Token bytes do not appear in protocol fields or logs. The deterministic vector key is public synthetic fixture material and is never used by the running node.

`protocol/vectors/generate-local-auth-fixtures.cjs` uses independent Node Buffer/manual wire encoding and Node crypto. `node protocol/vectors/generate-local-auth-fixtures.cjs --check` reproduces the shared JSON without any production codec/proof imports. Rust and Java consume the same three stage transcripts/proofs, four raw valid envelopes and fourteen malformed envelopes. The existing foundation wire admission deliberately rejects all of these separate auth messages.
