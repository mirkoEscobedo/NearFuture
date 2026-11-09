# Opt-in provider timing for disposable seed creation

Control5 `DISPOSABLE_SEED_CREATION_INITIAL_FOCUS`, auth7 and the existing v3
history grant may pair this exact field in both the immutable original plan and
lease, and retain it unchanged in every phase:

```json
{"providerTiming":{"schemaVersion":1,"profile":"INITIAL_FOCUS_PROVIDER_480000","providerTimeoutMs":480000}}
```

Both documents must retain the original 1800000ms root watchdog and the same
closed four-counter baseline. With the field absent from both documents, the
provider budget remains 180000ms. Partial, duplicate, mismatched or alternative
timing selections are refused. Each opted-in provider needs its full 480000ms
interval inside the original effective root deadline minus cleanup; insufficient
room stops admission. Reading a phase does not renew either clock.

The provider timer, verifier argument, ledger clock, transfers and completed-seed
proof use the same admitted selection. The limits remain three providers,
32 calls each, eight wizard clicks and at most one initial activation. Native
calls, startup, mapping, closure and cleanup retain their existing limits.

`ROOT_CLOSED_PRE_CONTINUE_PROVIDER_BUDGET_FAILED_SEED_ATTEMPT_HISTORY` is a
separate historical-only case: known Play, New Game and Generate ACKs; five
original PNGs; B1 known PASS; E1 UNKNOWN with SIGTERM, null status, empty streams
and no child-bound terminal; and a Continue geometry refusal before its child or
input. The reader retains the original 180000ms historical clocks, unresolved
known Generate, all spent requests, original root cleanup and protection proofs.
Host handles being terminal/reaped does not turn UNKNOWN into success. A fresh
grant carries the spent dispatch context and counters without refunding them.

Historical mapping freshness is checked at the recorded mapper/point-check time.
Later exact foreground, HWND and bounds observations bind the three actual
clicks. This failed-history proof makes no claim of five-second geometry age at
native dispatch and cannot qualify a completed seed, Continue, ordinary exit or
campaign acceptance. Existing successful-seed and earlier history predicates are
unchanged.
