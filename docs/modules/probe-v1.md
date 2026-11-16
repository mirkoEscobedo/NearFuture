# Neutral built-in module probe v1

Refs #139; initial port for #99 coordination. This is a new, opt-in executable,
`nf-sidecar serve-parent`. It never opens a vault, store, social account or game
world. The existing `nf-ipc-node`, protobuf, NF-IPC-AUTH-1 and NF-CANON-1 formats
retain their existing meaning and bytes. This is not a STRATCOM contract.

The trusted parent creates anonymous stdin/stdout pipes with ProcessBuilder on
a background owner thread. A non-terminal stream is necessary, not proof of
pipe provenance. Keep token/handshake bytes private; do not place credentials
in command arguments, environment variables, logs or discovery files.

First write exactly 48 bytes to stdin: ASCII `NFMPAR01`, a fresh nonzero random
32-byte token, lifetime milliseconds (u32, 100..60000), and enabled capability
bits (u32). All integers in this document are **big-endian**. Probe is bit1
(numeric value 1). No command-line token, persistent discovery or adoption of
an existing process is supported. Closing stdin, an extra malformed command,
or expiry revokes the process. A following 9-byte parent command consists of
byte1 and a strictly increasing positive campaign generation (u64). It
invalidates every connection. The child acknowledges application with32 bytes
on stdout: `NFMBND01`8, current serviceSession16 and applied generation8. Verify
session and expected generation. Await this acknowledgment before
another command or a new connection; commands with an unacknowledged predecessor
revoke the parent lease. Reconnect and renegotiate for the new binding.
This finite bootstrap allows at most8 generation commands per service lifetime;
the ninth revokes. Total stdout is bounded to290 bytes (READY plus8 ACKs).
A single bounded stdout writer owns at most one34-byte message; the main owner
waits only until EOF, its service lease, or a2-second output deadline. Process
exit terminates blocked pipe threads. Arbitrary redirected streams are outside
the trusted parent-pipe contract. A new service requires a fresh token,
fresh session and full negotiation, never adoption of retained credentials.
EOF during a command also revokes. Generation0 means service-neutral, with no
campaign loaded. A bound caller must carry the owner's current generation.

The child writes exactly 34 bytes on stdout, then flushes: ASCII `NFMRDY01`,
IPv4 loopback port (u16), fresh random nonzero 16-byte service session, current
campaign generation (u64, initially0). Use the actual owned process's stdout;
these bytes contain no credential but are trusted bootstrap metadata. Stderr
contains closed error labels only. The listener binds 127.0.0.1 with port0.

TCP frames retain existing nf-ipc framing: u32 length excluding the prefix,
then that many body bytes. Limit65536 before allocation, at most four
connections, one inbound and one outbound frame each, no request queue. Each
poll budgets4096 bytes per direction per connection. Handshake/idle expiry2s;
application receive/response expiry at most negotiated timeout (<=1000ms).
No sockets, child waits or retries belong on game callbacks.

Every HMAC below is HmacSHA256. Concatenation has no implicit lengths or text
encoding. Domain labels are ASCII including their final NUL. Tags are32 bytes.
Integer fixed fields avoid serialization ambiguity. All reserved bytes must0.

1. Client HELLO data H (80 bytes): `NFMHEL01`8, session16, port2,
   minimumVersion2, maximumVersion2, reserved2, random nonzero clientNonce16,
   requestedCapabilities4, requiredCapabilities4, maxRequestBody4,
   maxResponseBody4, maxTimeoutMs4, reserved12. Append
   HMAC(token, `NF-MODULE-HELLO-1\0` || H). Send this112-byte body.
2. Server ACCEPT data A (88 bytes): `NFMACK01`8, session16, version2 (=1),
   status2, availableCapabilities4, grantedCapabilities4, selectedRequest4,
   selectedResponse4, selectedTimeoutMs4, fresh nonzero serverNonce16,
   reserved24. Append HMAC(token, `NF-MODULE-ACCEPT-1\0` || fullHello || A).
   Body120 bytes. Verify before using any selected value. Status0 succeeds;
   otherwise close after this authenticated typed refusal. Required must be a
   subset of requested and available. Granted=requested & available. Supported
   version1 only. Offered request136..65536, response122..346, timeout1..1000ms;
   selected values are the componentwise minimum with those host ceilings.
3. Both derive K=HMAC(token, `NF-MODULE-KEY-1\0` || fullHello || A). This binds
   each connection to both nonces, service session, endpoint and negotiation.
4. REQUEST data: `NFMREQ01`8, session16, nonzero requestId16, generation8,
   sequence8, module2, operation2, apiVersion2, reserved2,
   requiredCapabilities4, timeoutMs4, payload (remaining bytes before tag).
   Append HMAC(K, `NF-MODULE-REQUEST-1\0` || data). Sequence starts1 and must
   increase exactly1; no duplicate or reconnect replay. Timeout1..selected.
5. REPLY data: `NFMRSP01`8, session16, requestId16, requestedGeneration8,
   sameSequence8, status2, payload. Append
   HMAC(K, `NF-MODULE-REPLY-1\0` || data). Check MAC, session, requestId,
   sequence, current campaign binding and caller deadline before delivery.

Probe: module1, operation1, API1, capability1. Supply exactly32 challenge
bytes; success returns the identical32 bytes. A server poll or authentication
alone is not readiness. `nf-modules::Host::dispatch` is the same headless entry
point. No successful reply grants game authority or proves a native lifecycle.

Status codes: 0success, 1malformed, 2unsupportedVersion,
3unsupportedCapability, 4limit, 5unsupportedModule, 6unsupportedOperation,
7disabled, 8staleBinding, 9deadline, 10cancelled, 11unavailable,
12invalidSnapshot. Error payloads are empty. Unauthenticated, invalid-MAC,
framing, replay and structural failures close the connection; no unauthenticated
error oracle. Disabled probe remains a known module and returns7 when invoked
without requiring its absent capability in the request. Negotiation requiring
an unavailable capability returns3.

Start/stop and generation belong to the composition owner; pure modules have no
fake persistent lifecycle. The parent EOF flag supplies cooperative cancellation
before/after each bounded handler. Closing a client connection abandons its
pending response; there is no queued work to cancel. Deadlines are monotonic,
start when a complete authenticated application frame is admitted, and apply
through the last response byte. Separately, receiving the next complete frame
is bounded by the negotiated timeout starting after the previous response is
flushed. Expired/stale queued replies are discarded. Trusted bounded
handlers run synchronously on the sidecar owner. Same-process modules are **not
a security sandbox, hard preemption or panic isolation boundary**. No files,
database, network dependency or game reference is injected into the probe.

The module host is separate from transport and from the Java/game adapter.
The accepted root headless profile remains build-identity only; this executable
is invoked explicitly for #99 and is not a profile/authority promotion.

Build and exercise without a game:

```powershell
cargo build -p nf-sidecar --locked --offline
node crates/nf-sidecar/examples/probe.mjs <absolute-path-to-nf-sidecar.exe>
```

The independent Node crypto caller owns and closes its two child processes. It
checks neutral/bound32-byte echoes, stale binding, unsupported module/version/
capability, a disabled probe, fresh restart sessions and parent-EOF exit. Its
stdout reports public correlation evidence, never a token or authentication key.
The pure entry point is `nf_modules::Host::dispatch(Invocation, granted, Context)`;
it needs only explicit request bytes, a monotonic deadline and cancellation flag.

Native placement remains #99's responsibility. This first port supports only a
trusted direct child owner. An external launcher and in-game attach require an
explicitly agreed safe bootstrap/handoff; listening on loopback does not establish
that route. No existing-process adoption, global configuration, loader bypass or
disabled bytecode verification is supplied by this artifact.
