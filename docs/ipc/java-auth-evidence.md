# Java local mutual authentication evidence

## Implemented public seam

`java/wire` generates both maintained protobuf schemas with pinned official protoc/runtime 4.31.1. `LocalAuthDecoder` admits only the separate `nearfuture.ipc.v1.LocalAuthEnvelope`: auth version 1, capability 2, schema 2, nonzero runtime session and one explicit auth body. Foundation `ControlEnvelope`, canonical profiles, payload schemas and `RequiredSemantics` retain their closed capability/schema 1 registry. Successful decoding establishes bounded shape only.

Auth admission caps the input and charged decoded budget at 4096 bytes, nesting at 8, total entries at 256 and repeated collections at 64. Required imported wrapper fields are explicit. Unknown fields, duplicate singular/oneof fields, wrong wire types, nonshortest varints and missing required fields fail before generated decoding. Nonce and proof declared lengths above 32 fail before reading their payload; present lengths below 32 fail semantic admission. All imported resource limits retain the foundation hard ceilings and relationships.

The immutable `AuthContext` binds role, listener port, runtime session, universe, history, ruleset/content hashes, account/device and descriptor limits. `AuthTranscript` writes the closed fixed 306-byte `NF-IPC-AUTH-1` transcript directly with specified little-endian widths; protobuf serialization is never hashed. It uses maintained JDK `Mac` HmacSHA256. Negotiation must equal the componentwise minimum of the exact offered and descriptor limits.

`MutualAuthClient` owns a cloned 32-byte secret and fresh `SecureRandom` 32-byte client nonce. It verifies the stage-1 server challenge before creating stage-2 client proof, then verifies stage-3 server finished before exposing its privately constructed active `Session`. Verification uses JDK `MessageDigest.isEqual`. Wrong stage, replay, changed bindings, limits or proofs close the client. Its owned token array is cleared on activation, failure and close; JDK provider internal copies and JVM memory reclamation have no additional zeroization guarantee. No raw token is sent in any auth message. Transport authentication alone grants no operation/role authority; callers still require the identity policy and current membership frontier.

These synchronized APIs are for the background connection owner. Campaign frame callbacks use only the bounded `FrameBridge`/`SessionFence` handoff; no authentication, file access, socket processing or cryptographic work is installed in game callbacks. No actual Starsector adapter lifecycle or G1 writer coverage is certified by this slice.

## Independent fixtures and observed checks

The runtime author owns `protocol/vectors/generate-local-auth-fixtures.cjs`, which uses Node standard `node:crypto.createHmac` and an independent raw protobuf fixture writer without importing production codecs. It reproduces `protocol/vectors/local-auth-v1.json`: all three exact transcript bytes and HMAC proofs, plus four positive and fourteen malformed raw envelopes. Its sequential byte key is explicitly public test input, not a credential.

`AuthClientTest` checks all three whole transcript/HMAC goldens, random connection nonce separation, ordered activation, selected-limits equality and authenticated-session limits. Generated disposable keys exercise changed secret, nonce, role, port, session, universe, history, ruleset, content and device, cross-connection challenge replay, reflection of stage-1 proof into stage 3, wrong message order, lower valid quotas, illegal quota relationships and unsigned out-of-range quotas. `AuthWireTest` consumes the same independent raw corpus with exact bounded error codes and additional missing/short identities, role/protocol, quota, duplicate, unknown-field and preallocation-length tests. Existing canonical and foundation wire corpus tests remain passing.

Observed on 2026-10-07:

```powershell
node protocol/vectors/generate-local-auth-fixtures.cjs --check
java/gradlew.bat -p java :wire:AuthWireTest :ipc:authClientTest --offline --no-daemon
java/gradlew.bat -p java syntheticCheck --offline --no-daemon
java/gradlew.bat -p java syntheticCheck --offline --no-daemon -PtestJavaExecutable=H:/Games/Starsector/jre/bin/java.exe
npm run check:types
npm run check:boundaries
npm run check:schemas
```

The full public Java gate passed on pinned compiler/JVM 21.0.8 and installed game JVM 17, including existing canonical eight-positive/twenty-four-malformed cases, strict signature fixtures, twenty thousand finite canonical mutations, foundation wire four-positive/thirty-three-malformed cases, ten thousand finite wire mutations, capture and reference suites. The added auth shared raw corpus passed after that broad gate and is included in `:wire:check` for subsequent runs.

## Owned headless interoperability harness

`AuthSocketClient` is a test-source main, not a production descriptor-discovery or access-control implementation. Its caller must supply the trusted, owner-private ephemeral blob created by the owned Rust daemon harness. It reads at most 265 bytes, requires the exact 264-byte `NF-BLOB-1` checksum wrapper around a fixed 218-byte `NF-IPC-R2` descriptor, and validates descriptor field widths, nonzero port/session and limits. This helper does not itself certify native ACL checks. It clears its owned record/token arrays on every exit.

It connects only to IPv4 loopback using the existing nonblocking `BackgroundChannel` with an auth frame maximum of 4096 and one three-second handshake deadline. It prints only `AUTH_SOCKET_OK` after verified server-finished proof or static `AUTH_SOCKET_FAILED` on failure, with no private path, token, peer bytes or exception text.

```powershell
java/gradlew.bat -p java :ipc:exportAuthTestClasspath --offline --no-daemon
# Owned daemon process harness supplies the private path; absent property is a hard failure.
java/gradlew.bat -p java :ipc:authSocketClient --offline --no-daemon -PipcDescriptor=<owned-private-blob>
# Optional same protocol against the distinct bulk listener:
java/gradlew.bat -p java :ipc:authSocketClient --offline --no-daemon -PipcDescriptor=<owned-private-blob> -PipcEndpointRole=bulk
```

Actual owned-loopback Java-to-Rust control and bulk mutual authentication was observed PASS on 2026-10-07 using the installed game Java 17. The runtime author ran the explicit integration fixture `crates/nf-ipc/tests/java_process.rs`: both clients emitted exactly `AUTH_SOCKET_OK`, the owned foreground Rust daemon shut down normally, and its exact private publication was removed. The single test passed in 13.91 seconds; owned child handles have bounded wait/kill/reap guards. This demonstrates the separate mutual-auth protocol over real local sockets, with no native game deployment or lifecycle claim.

```powershell
java/gradlew.bat -p java :ipc:exportAuthTestClasspath --offline --no-daemon
$env:NF_IPC_JAVA='H:/Games/Starsector/jre/bin/java.exe'
$env:NF_IPC_JAVA_CLASSPATH_FILE='E:/github/NearFuture/java/ipc/build/auth-test-classpath.txt'
cargo test -p nf-ipc --test java_process --locked --offline -- --ignored
```

The integration test is explicitly ignored unless invoked with its required environment, and absence of the executable/classpath is a failure when invoked. Root independently reviewed the Java auth source and fresh Java21/Java17 shared-fixture tests with a PASS verdict and no important findings. Root owns combined issue acceptance; native adapter G1 remains unobserved.