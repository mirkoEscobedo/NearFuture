# Private parent-pipe checkpoint (#99)

This is an unqualified, default-off read-only lifecycle composition. The original
adapter staging and Rust formatting failures remain recorded in #99 and PR119.
Under the separately authorized 20 October recovery, the adapter repository and
generated lock preserve protobuf4.31.1/bcprov1.81 pins; licensed staging now passes
offline with and without lock regeneration. Licensed APIs remain local compileOnly
inputs and are absent from the six staged runtime jars. Eight JVM-host controls
and the four L0 checks pass; these are not native game acceptance.

All nine ignored owned-parent process cases are explicitly registered in the
required Linux and Windows Java/classpath lanes with the maintained focused
invocation. Historical semantic2/2 is retained separately from the recovery's
two additional rounds, now2/2 used for repository/formatting and coverage/status
corrections. The subsequent continuation authority preserves those counts and
authorizes completing the original outstanding process and native work. Full
candidate checks, native lifecycle evidence and protected dev landing remain
outstanding. Offload, a standalone production launcher and runtime
activation remain unqualified. The accepted runtime profile is unchanged.

`nf-ipc-node serve-parent` accepts the same trusted vault, saves, publication name,
scope, policy, principal and finite duration arguments as `serve`. It uses the
maintained private-vault validation, publication, authenticated control/bulk
listeners and `NoOperations` ports. It never opens a durable store or applies a
game result. Duration remains 100–60,000 ms.

The trusted background owner must directly construct private anonymous stdin and
stdout pipes for this exact child. Non-terminal checks alone cannot prove pipe
provenance. Arbitrary redirection is outside this owner-scoped secret-handoff
contract. The game experiment must retain a qualified Windows Job boundary from
launch; ordinary process creation alone does not supply that qualification.

Within two seconds, the parent sends exactly `NF-PARENT-1\0` (12 ASCII bytes)
and a fresh nonzero 32-byte random challenge. The child's sole stdout response is
263 binary bytes:

| Offset | Bytes | Value |
| --- | --- | --- |
| 0 | 13 | `NF-HANDOFF-1\0` |
| 13 | 32 | Exact parent challenge |
| 45 | 218 | Existing `NF-IPC-R2\0` private discovery payload |

The discovery payload contains two distinct loopback ports, a fresh nonzero node
runtime session, exact universe/history and policy hashes, the trusted local
principal, this session's 32-byte authentication token and bounded resource
limits. Vault/root keys are never exported. This response is a private channel,
not a diagnostic stream: never persist it, send it over a socket, put it in a
command argument or expose it in errors or logs. There are no READY/STOPPED lines
in this mode. The parent must reject extra or incomplete output and erase its raw
input after decoding.

`ParentHandoff` admits exactly 263 bytes against the expected parent challenge
and separately supplied trusted scope, policy and principal. It validates both
endpoints and all resource limits through the maintained authentication context.
It can construct the existing `MutualAuthClient` for either role; it exposes no
token getter. Closing the handoff erases its token and prevents new clients.
Clients retain their own independently owned, zeroized authentication material.
The handoff belongs exclusively to the background owner, never a game callback
or persistent campaign data. Parsing a descriptor does not establish that a
remote endpoint authenticated or that the game lifecycle is safe.

After the challenge, stdin EOF, an error or any extra input byte revokes this
child's owner. A reader watches concurrently with startup and socket polling.
The foreground owner invalidates both endpoints, drops connections and closes
only its own guarded publication. It also exits on the existing finite duration.
For the owned-start route, discovery is retired with that exact guard when bulk
first connects, before bulk authentication can complete. The two authenticators
and private parent retain their own in-memory token. Thus an authenticated child
killed externally leaves no discovery publication that a successor must adopt
or delete. A child killed during incomplete startup may leave a publication;
that collision is refused. Attachment and stale-publication takeover are unsupported.
No unrelated process is controlled, and no stale publication is adopted,
replaced or unlinked as a recovery shortcut. Publication names retain their
existing cooperative exclusive-owner contract; comparison and removal do not
become an atomic adversarial takeover primitive.

The original public process test proves a valid private handoff followed by EOF-driven
exit/reaping and publication removal before a configured 60-second lifetime.
It ran under Windows Job ownership. Java host tests cover the literal admission
format and refusal boundaries. The original five opt-in Java/Rust process cases pass on
the host JVM and the installed game JVM: authentication of both roles, wrong-pin
refusal, cancellation before and after startup, and cleanup after an owner error.
They verify owned-child reaping and publication removal; running the game JVM
headlessly does not establish game lifecycle acceptance.

Six additional ordinary Rust process cases now pass: malformed magic, zero nonce,
partial challenge with EOF, partial challenge held open past its admission bound,
finite session expiry with the parent pipe still open, and extra-input revocation.
Refused challenges produce no private stdout or publication; admitted sessions
remove their owned publication before bounded exit. Handoff bytes remain only in
zeroizing memory. The complete nine-case explicit Java target also passes on both
the host JVM and installed game JVM, including the two lifecycle cases below and
the newly specified stalled-child and abrupt-JVM-death cases. The stalled fixture
is a test-only executable compiled from a benign Rust source and pinned through
the real launch admission. It ignores EOF, exercising the existing 15-second
startup deadline, eight-second cleanup grace and forced reap. The abrupt-death
case authenticates both real Rust endpoints, captures the exact owned child
handle, forcibly terminates only its directly owned JVM, and requires the Rust
child to exit on pipe closure within five seconds without stale discovery.
These cases ran under Windows Job supervision, with no timeout or ignored case
credited as execution. Required Linux and Windows CI invokes the complete
explicit Java target; ordinary Rust discovery includes all six negatives.

The production game composition still requires native evidence. A source-level JDK inheritance
reference does not prove the bundled game JVM's runtime behavior. Actual
disposable-campaign responsiveness, save/load fencing, restart and exit evidence
remain required before #99 can complete.

`LifecycleOwner` starts one daemon at application startup. Callback activation
and suspension synchronously revoke the old value-only mailbox, with no process,
socket or file work. The worker serializes children, allows three finite child
lifetimes per campaign binding and reauthenticates both roles every three seconds
under the existing five-second idle timeout. Each planned reconnect and every
connection exit advances the mailbox epoch before disconnect or cleanup. A
cleanup failure stops the owner rather than launching another child.

Each mailbox reserves 16 items and 131,072 bytes independently for control and
bulk. It removes at most one item per role per callback. Consumers capture the
epoch before polling and recheck that captured epoch before using a value.
Revoked generations cannot restart. Headless real-process scenarios cover
renewal beyond idle expiry, campaign replacement, suspension, three killed child
lifetimes and rejection of queued authentication after the final death. They
do not substitute for native callback, serialization or responsiveness evidence.
