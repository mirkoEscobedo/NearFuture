# Private vault lifecycle diagnostics

PrivateVault::create_detailed and open_detailed retain the original IdentityError together with closed, local details from the failed operation. The legacy create and open APIs delegate once and preserve their error contract.

Creation preserves create-directory, private-access initialization and opening in their original order. Opening preserves the private-access check, both canonicalizations and game-save-root exclusion. Details retain only an operation/stage, closed refusal or I/O kind and optional numeric OS code, or the existing bounded helper classification. Paths, SIDs, private bytes and helper output are excluded.

The receipt test fixture now calls create_detailed at the same existing unwrap. Its arguments and body remain unchanged. An actual hosted failure can consequently retain this call’s creation, access or canonicalization stage. This adds no retries, replacement identities, ACL relaxation or deadline extension.

The original regression compiled and failed because an existing-directory creation returned PrivateStorage without its expected diagnostic. The minimum implementation then passed two public lifecycle tests. Independent reviews confirmed legacy effect order and the exact fixture-call inverse. A preserved harness-path setup failure ran no formatting or Cargo step; a metadata-only path correction allowed the bounded formatting gate to pass. Formatting affected only the two new files, including optional trailing commas.

Fresh Windows integration passed 52 identity, 84 transport unit and 7 public receipt-book cases: 143 total. All nine supervised gates passed, including whole formatting, strict workspace all-target Clippy, architecture boundaries, reference-corpus checks and source guards. The six owned source files and 976 unchanged raw parent blobs are bound by the companion checkpoint; two evidence files are appended after the source-bound run.

Hosted checks for the new publication are pending. At parent bb5a89f7d3c32888546756807e40218f428a4157, both Linux jobs and the Windows push job passed. The Windows PR job passed 83 of 84 transport unit tests and failed during PrivateVault::create in fixture provisioning; that legacy call supplied no typed cause. No historical storage root cause or production repair is established.

ReceiptRepo::open_owned and legacy-only load, read and removal effects remain outside this lifecycle slice. Actual two-process portal behavior, semantic replica activation and native Starsector acceptance remain separate requirements; issues #23 and #39 remain open.

private-storage-diagnostics.json retains the earlier source and integration checkpoint. This companion checkpoint records only the additive lifecycle change and its exact validation inputs.
