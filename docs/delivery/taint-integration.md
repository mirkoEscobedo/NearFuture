# Authenticated campaign-taint self-report component

Refs #107, #80, #26, #92. The headless development component records the first
authenticated prohibited-manifest self-report for an explicitly configured
campaign lineage. It preserves the source component's current registration,
lease and membership checks. Exact retries return the first receipt across
reopen; conflicting request content or another cause for the same lineage is
refused. A retained older lease-profile copy cannot satisfy a known taint
frontier or erase that evidence.

`TaintStore::open_existing` verifies the selected committed histories, complete
schema, policies and retained frontiers. Opening does not activate or migrate
the taint profile. Only a fully prepared, fresh device proof and valid lease
self-report can activate profile 3 and append the first cause, in one Immediate
transaction. Existing registration/admission metadata and hashes remain. An
uncertain commit quarantines the owner. Tickets are bound to the owner runtime,
request and current membership, consumed once, limited to 64, and expire after
five seconds. A receipt records self-report evidence; it grants no economic
capability or clean attestation.

Six fixed negative provenance codes distinguish missing evidence from recorded
network taint. Their messages take no caller-controlled paths, identifiers or
command text. A trusted receiver owner must establish the selected reason;
choosing a diagnostic code proves neither a detector nor campaign ancestry.

The component is registered in the headless profile's development acceptance
metadata and required workspace Rust gate. Its tests include authenticated
durability/retry/reopen, unauthorized and changed proofs, policy/request/ticket
refusals, first-cause monotonicity, stale profile refusal, and the unchanged
registration/lease/revocation regressions. The source's test-only exclusive-owner
inspection correction and historical cancelled Windows gate remain evidence;
current candidate checks must qualify this composition before landing.

The accepted root executable still reports build identity only. Persistent
writers and incomplete runtime capabilities remain disabled. The explicit
library owner is a development surface, not a profile service and not automatic
release-data migration. Disabling a future caller or reverting source cannot
undo an activated profile, a recorded cause or an advanced retained frontier;
ordinary older owners must refuse unsupported profile 3, and stale backups must
be refused against known evidence.

The unchanged revocation regressions exercise the older owner profiles.
`TaintStore` has no membership-update API for profile 3; post-taint membership
lifecycle and composition with an economic service require separate qualification.

Native prohibited-mod detection and console hooks, copied-save and descendant
campaign attestation, save lineage recovery, game integration and economic
enforcement remain outside this component. Parent #26 retains its full open
acceptance checklist. This component does not certify an honest native save.
