# Pure replica foundation and replay delivery

Refs #103, #39, #92. The prerequisite extracts the dependency closure used by
#84's committed replay from #65 at
`ee1db00aaef3a7698c109eea17af44b7f62b696a`, including merged #66 at
`24f5cbfe439f6dc63ff131cdd4790efe3a1fda41` (merge `94566313`).
Current dev starts at `bf281ecdd976059f86b00f4e5dfc269c98e683be`.
The live issue/PR records reviewed heads, actual checks and dev landing commits;
source extraction alone does not meet the completion target.

The pure dependency graph is kernel replica admission/provider, world copying
and canonical snapshot hashing, backed by contract `replica_budget` scopes.
Imports require only the existing kernel model, `nf-world` and `nf-contract`.
No identity/transport API, staged sync module or receipt/Portal owner is needed.
These effectful source families remain excluded. The two shared module roots
are composed by adding exports while retaining dev's supplies/trade exports.
External package tests import the public modules, so validation must prove they
are exported and compiled rather than merely present on disk.

The contract scope keeps cumulative logical depth, entry and copied-byte charges
across nested callers. A budget failure remains sticky even if a caller catches
it; consumers cannot replace/reset counters. Limits remain depth1..32,
entries32768 and copied bytes4194304. Kernel callers preserve semantic admission,
authority/frontier/provider checks and charge owned copies before allocation.
Logical accounting is not a physical allocation, elapsed-time or performance
guarantee.

Existing independent budget/unwind tests, admission/ordering/exhaustion cases,
provider validation, world-copy and snapshot parity/boundary fixtures are kept
with the imported modules. Complete contract and kernel package tests, current
workspace baseline and profile/release checks remain required. The manifest
registers `replica-budget-contracts` under the existing required Rust gate.
The accepted executable still reports build identity only; writes and incomplete
runtime capabilities stay disabled.

## Separate replay successor

The refreshed #84 head is
`b3153d52059828893b6a7ba8a5febccaf2cd7347`. Its merge base with #65 is the exact
#65 head above, and its delta is five kernel files: replay helper visibility,
replica module export, scoped replay, world-copy reservation and its independent
replay/budget test. Apply those changes over this minimal foundation in a thin
successor, preserving #84's original branch and format-failure/correction history.
Land the prerequisite first, then retarget the thin successor to current dev and
qualify/land it serially. Keep #103 open until both accepted deltas have landed.

Scoped replay returns a candidate World and grants no installation/writer
authority. The default replay path remains separately available. This delivery
does not complete authenticated follower convergence, consistent gap snapshots,
archive authorization/retention/quotas, failover or disk-failure recovery.
Those acceptance criteria stay visible in open parent #39; broader #65 work is
preserved separately. No receipt/Portal/Chat completion becomes a prerequisite.
