# Headless Chat integration candidate

Refs #106, #25, #24, #92. This candidate preserves source #79 against the
current dev identity, IPC and read-only operator baseline. It is not qualified
for landing: the new actual-process scenario fails before its first sidecar
becomes ready because the test constructs its Tokio listener outside the
runtime context. No successful two-client process flow is claimed. Source #78
remains a separate, unapplied loopback diagnostic delta on its preserved PR.
Game Chat UI, presence, moderation, remote routing and full parent acceptance
remain open. The accepted executable still reports build identity only;
`chat-runtime`, peer runtime and persistent writers remain disabled.

## Current evidence and limits

The mechanical composition preserves current root dependencies, the versioned
curve25519 test optimization with debug and overflow checks, strategic-reader
exports and the six required CI groups. Chat's seven caller-owned evidence
inputs are prepared in the full Rust job that consumes them. Its original
command and 20-minute bound remain. The seven existing Java Chat process cases
are explicitly registered in the existing Linux and Windows interoperability
lanes; that registration has not yet been qualified on this candidate.

The first local affected-package run passed identity (38 tests) and wire (15),
then failed the store library: six passed, fourteen failed, one child dispatcher
ignored. Eleven children timed out, two identity creations returned opaque
PrivateStorage errors, and the nonempty-v2 case exhausted its active budget.
One independently admitted serial diagnostic passed twenty cases in 31.66s.
The supported fixture scheduling change then passed the normal parallel library
command: twenty cases in 32.13s, with only the child dispatcher ignored.
Only thirteen crash parents and the nonempty-v2 parent share the test-only
mutex. Queue waiting is recorded separately in their public evidence, before
each unchanged twenty-second operation timer. Child and reap limits remain ten
and three seconds. This establishes a scheduling remedy for this comparison;
it does not establish an ACL/helper implementation repair.

`chat_retained_latency` measures the unchanged receiver using genuinely
admitted signed originals and fresh aliases, production Noise history and an
independent authenticated status query on the same single-thread executor.
The fixture advances its trusted receiver clock between admissions while
preserving the default ten-token/two-second policy. These are local synthetic
qualification samples, not full-capacity, game, remote or throughput results.

| Originals / aliases | Snapshot verification | History | Concurrent control |
| --- | --- | --- | --- |
| 0 / 0 | 627 us | 1,702,900 us | 1,727,586 us |
| 32 / 32 | 5,685 us | 1,640,485 us | 1,658,196 us |
| 128 / 128 | 20,945 us | 1,853,325 us | 1,857,554 us |

The measured case passed in 65.57s. Both public API budgets remain five seconds.
Repeated retained-signature verification remains enabled; no receiver algorithm,
signature check, replay check or capacity was changed. A passing measurement
does not replace the failed process scenario or the unreached full baseline.

## Exhaustion and retention follow-up

The receiver retains at most 4,096 signed originals and 16,384 request bindings;
an additional original or alias returns Limit without evicting earlier replay
evidence. Exact retained retries retain their original result. The sender outbox
also refuses new entries at its 4,096-entry bound. The account admission bucket
returns transient RateLimited with its bounded retry interval, rather than
deleting originals or persisting a terminal rejection. These bounds remain
unchanged and do not promise permanent availability.

Retention requires a separately reviewed, versioned persisted transition before
any deletion: retain durable request-to-original deduplication and source
signature/provenance, preserve membership and receiver/source frontiers, define
the proof and cursor semantics of removed presentation history, and prove
restart, stale-backup, rollback and interrupted-transition refusal. An operator
must explicitly authorize a compatible transition on its own data root. Raising
caps, silently pruning aliases, resetting an epoch or copying a database cannot
substitute for that policy. Until then, exhaustion stays an explicit refusal;
UI presentation must distinguish unavailable history from never-delivered data.
