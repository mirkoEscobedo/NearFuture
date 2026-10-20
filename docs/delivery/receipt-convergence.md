# Receipt and Portal convergence

Refs #102, #92, #23. This candidate combines current dev
`bf281ecdd976059f86b00f4e5dfc269c98e683be` with the #76 vehicle
`8184441e178a3235c73d8a0b9e97132ecd121da7` through #114's current-key carry,
`e06cc218c6462a882d59741c4b3f81ee87d4dc35`. These are source identities;
the live issue and successor PR record the final reviewed head, required checks
and landing commit. Preparation alone does not meet the dev completion target.

The selected transport implementation remains the current-key vehicle. It opens
receipt repositories using current admitted account/device keys, retains the
actual subscription-response-sent signal and exposes finite foreground Watch
and Serve development commands. Dev's diagnosed private-vault APIs and Java
fixture diagnostics remain intact alongside the vehicle's separate detailed
diagnostic APIs. The protected blob callback engine remains in place.

The accepted headless executable still reports build identity only. The manifest
registers `receipt-portal-contracts` as a library regression path, with complete
transport and identity package coverage in the required CI groups. Peer runtime,
native writers and other incomplete capabilities stay disabled; profile writes
remain false. Development library/process tests do not qualify game settlement,
native cargo receipts, a production peer service or the whole of #23.

## Older source custody

The [per-file disposition map](receipt-source65-disposition.json) inventories all
251 paths changed by #65 at
`ee1db00aaef3a7698c109eea17af44b7f62b696a` from its common base with #76,
`921948739e43dabb7560d6206c10ccaa844019c9`. Each entry carries the immutable
source blob/reference, selected vehicle blob, composed blob where present,
declared Rust test functions and destination issues. Counts are 107 included
exactly, two composed, four split, 115 deferred and 23 historical evidence files.

| Preserved delta | Destination and limit |
| --- | --- |
| Pure canonical-peer, replica budget and replay foundation; broader sync/follower variants | #103 can extract its minimal independent foundation; #39 retains broader sync acceptance. Neither depends on completing the Portal lifecycle. |
| Extra identity/blob/scan and receipt Book storage-cause diagnostics with associated tests | Existing operator/support parent #46, with receipt acceptance under #23. This preserves source custody and does not expand #104's read-only slice or allocate another implementation/repair budget. |
| Older Portal/notification/transport API and test variants | #23 retains these explicitly unlanded variants. Current #102 invariants require their own current-candidate evidence; filename similarity is not assertion equivalence. |
| CI and package scheduling | Compose the protected dev aggregate with complete vehicle partitions; run both complete I/O packages serially in the local aggregate, retaining #65's transport scheduling and aligning identity with required CI. |
| Dependency/export metadata and binary-fixture rules | Retain current vehicle dependencies/exports and original-ACL byte policy. Defer canonical-peer/replica dependencies and sync-checkpoint byte rules with #103. |
| Historical documentation | Preserve exact pinned bodies as historical evidence, without current qualification credit. |

Deferred source is preserved for later acceptance, not represented as implemented,
equivalent or passing here. Keep #65 and the broader #23/#39/#46 trackers open.
Do not close #65 as superseded merely because this component lands. Preserve
its branch and immutable source references until every unique acceptance has
actually reached its own completion target.

## Current behavior and required evidence

| Required invariant | Current candidate coverage |
| --- | --- |
| Rotated account and device keys reopen receipt repositories | `tests/receipt_key_rotation.rs`, plus #95's admitted current-key/refusal coverage. |
| Authenticated receipt startup and correlated Watch status without resubmission | `tests/portal_internal/watch_status.rs`, `tests/portal_process.rs`; the latter runs actual production peer processes and observes subscription-response delivery. |
| Fresh reconnect fences stale valid device proofs | `tests/reconnect_fence.rs`. |
| Durable server/client revocation and live owner revocation refusal | `tests/durable_revocation.rs`, `tests/receipt_internal/repository.rs`, `tests/portal_internal/revocation.rs`. |
| Original lifetime and refusal when owned resources or SQL consume it | `tests/portal_internal/lifecycle.rs`, `bulk_listener.rs`, `sql_delay.rs`, notification completion/deadline modules and process lifetime bounds. |
| Retained persistence, framing, capacity and collision contracts | Receipt repository/unit modules and `tests/receipt_book_*.rs`, included in complete transport package coverage. |

The `portal_internal` modules are library unit tests wired through
`src/portal/owner/run.rs` and its lifecycle module. Notification and receipt
internal modules are likewise wired through their effect test modules; they are
not omitted merely because their support files are nested below `tests/`.

These are coverage locations, not a claim that every check has passed. The live
successor PR must record actual focused/package, complete local baseline,
cross-language/process, profile/release and all current hosted required results,
plus independent review against the exact head before serial dev landing.
Cancelled, missing or unreached required work cannot be credited. See
[the workload contract](ci-workload.md) and [integration policy](integration.md).
