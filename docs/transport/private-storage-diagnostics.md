# Detailed private storage failures

PrivateVault offers create_identity_detailed, create_private_blob_detailed and scan_optional_private_blobs_detailed. Each performs the existing operation once and returns PrivateFailure, which retains the original IdentityError and optional local diagnostic.

The diagnostic contains a closed operation/stage, refusal or I/O kind and optional numeric OS code, or a closed Windows helper kind/step/exit stage. It contains no path, SID, private identity bytes, error-string payload or helper output. It is local evidence from this call; it is not an authenticated wire record or proof of a historical cause.

Existing callers keep their original IdentityError results through the compatibility wrappers. Receipt book recovery, initialization and append use private BookFailure propagation and retain public PeerError results. A semantic callback error takes precedence and has no fabricated storage cause. Repository initialization updates observational anchors only after success.

Windows single and batch helpers retain the original five-second deadlines, bounded output and owned-child cleanup. This change adds no retry, ACL relaxation, tail deletion, key regeneration or deadline extension.

The actual receipt fixture retains detailed identity-creation and initialization failures. Six existing public book-fixture create calls use the detailed twin while retaining their original input bytes, unwraps and assertions. ReceiptRepo::open_owned and other legacy-only operations are not fully instrumented by this slice.

Verification on the merged945 dependency graph passed50 identity cases, all84 transport unit cases (including existing receipts/notifications and pure sync), and7 public book cases. All9 supervised steps passed: source checks, tests, whole formatting, strict all-target workspace Clippy, architecture boundaries and reference-corpus drift checks. The initial harness module-discovery failure ran zero steps and is preserved separately.

Earlier f094 focused author evidence passed59 current Windows cases after13 compiled regression cycles. The combined graph gate above is fresh integration evidence. Linux execution of the new diagnostic change is pending hosted checks.

Historical hosted f094 Windows errors and the merged945 Windows failures remain unexplained. At945 the PR transport unit suite passed80 cases before a public book CreateNew call returned PrivateStorage; the push suite passed79/80 and failed during ReceiptRepo::open_owned with Storage. The same tracked tree passed all four24f5 hosted jobs. None of those generic errors establish an ACL, filesystem, helper or timeout cause.

Exact reviewed source identities and bounded evidence are recorded in private-storage-diagnostics.json. Native Starsector, complete peer-process behavior and semantic replica activation are separate acceptance gates.
