# Bounded private receipt inventory scan

Documentation-only effect supplement to [receipt-book-admission.md](receipt-book-admission.md), designR2 SHA256 `c300e34c07939b0d056b05050dcbbe3d69f3a79ddc6d43e85e3a62c16d8d4e30`. The frozen control2 contract,479-byte book records, vectors and strict optional single-read behavior remain unchanged. Root owns the additive identity API/sidecar and its source review; receipt owns the later book consumer. No Store/path/key API or dependency change is proposed.

## Measured need

The actual native Windows strict optional64-empty-name scan took25468ms in `.tmp/optional-blob-metric-native.json`, with real ACL helpers and no file-creation time. Repeating the root ACL process before/after each absence would block the foreground owner beyond a5s peer request. Do not skip reserved names, cache a policy grant or widen protocol timers to hide this result.

A batch shares root validation and ACL process startup across one finite inventory admission. Measure the actual new empty/full/error pass before owner responsiveness claims. This is a measured performance refinement, not a hard realtime5s guarantee or game-frame p99 evidence. Book readiness scans occur before network operation timers begin; later append/recovery must preserve independent lane scheduling and reject/cancel on finite lifetime rather than claim an unmeasured deadline.

## Additive owned API

```rust
impl PrivateVault {
    pub fn scan_optional_private_blobs(
        &self,
        names: &[&str],
        visit: impl for<'a> FnMut(usize, Option<&'a [u8]>)
            -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError>;
}
```

Admit1..64 distinct names under ASCII case-folding on every platform (reject Foo/foo before effects), each ASCII alphanumeric/underscore/hyphen of length1..64, using the accepted blob prefix/resolver. Reject invalid names, duplicate entries, count overflow or invalid root/path encoding before spawning an ACL effect. The receipt caller passes exactly its64 fixed reserved names, including unconfigured slots and every tail; generic shorter calls do not establish complete receipt admission. No raw path/root, directory iterator, arbitrary glob or caller-supplied ACL script is returned.

The trusted synchronous higher-ranked callback receives only index plus one bounded borrowed payload or strict None. It may decode the fixed book and retain fixed summaries/previous hashes/anchors; it must not await, mutate Store/vault, initialize a role/grant or publish partial recovery; the higher-ranked lifetime prevents borrowed data from escaping, while the other effect restrictions are trusted callback obligations. A callback can technically copy owned bytes; accepted receipt consumer review must verify that it retains fixed summaries rather than64 maximal payload buffers. Visitor Err aborts/refuses the pass. Earlier visitor effects cannot be rolled back by this API, so the consumer must treat all observations as provisional until final Ok and discard partial summaries on any failure.

## Root and present-entry checks

Validate that the configured root is an actual owner-private directory, not a file/link/reparse object. For all admitted names perform actual no-follow entry lookup. Only OS NotFound under the valid root becomes an absent candidate. Present links/reparse points/nonregular objects or metadata/access failures are errors. A present entry disappearing before open is an error, not None. Unrelated vault files remain allowed and outside this exact-name pass.

Share root checks across the whole pass rather than64 redundant per-name roots. Root access must be freshly checked at the successful end, including an all-empty inventory, and root directory/no-follow classification must remain valid before any absence can be accepted. Preserve the existing cooperative owner-local lifecycle contract: no concurrent same-owner remove/replace/reuse; these safe API steps do not establish an atomic filesystem snapshot or handle-relative ACL guarantee.

Windows checks root plus all present admitted regular-file ACLs in one bounded main PowerShell process. Invoke the existing unchanged `private-acl.ps1` script in-process using `&` for each literal validated path. Inspect and reset LASTEXITCODE around each invocation; a prior0/2/0 sequence must not mask the middle refusal. Root's actual probe `.tmp/acl-inprocess-native.json` observed that sequence with the maintained Windows PowerShell behavior; it is supporting feasibility, not batch implementation acceptance. A final fresh root ACL check after sequential reads may use a second bounded root-only process if needed; count that explicitly in measured evidence. Do not revert to per-file process spawning or claim one process for the whole pass if final validation creates another.

On Unix use the same maintained actual owner/mode checks with no ACL CLI. Present file ACL/mode admission is never waived by shared root checks. Both platforms require the accepted byte wrapper validation on each opened file and a fresh final valid root before Ok.

## Sidecar and input bounds

Root owns a fixed sidecar that only performs these private access checks; it cannot execute caller code, read a blob payload, change ACLs, write files or return paths/credential text. Script/helper paths are package-owned literals resolved by identity, and process arguments are an argument vector rather than composed shell source. Every configured root and resolved literal path must be UTF8-encodable and at most4096 bytes with no NUL/CR/LF before creating its protocol input. Encoding failures reject without a lossy path conversion.

Sidecar stdin is a finite count-prefixed list: ASCII decimal path count1..65 followed by LF, then exactly that many UTF8 path lines terminated by LF. Index0 is the configured root; remaining lines are admitted present regular files. Each path line is1..4096 bytes excluding LF; duplicate file paths reject. Overall stdin is<=266KiB (272384 bytes), with checked accumulation before allocation. The root can be checked at both ends internally without adding another supplied path. No trailing lines, arbitrary commands, wildcard strings or environment-selected helper are admitted. If a separate final-root process is used it receives its own1-path bounded list with the same parser/policy.

The parent owns stdin/stdout/stderr, child identity, absolute deadline, bounded static output and exact kill/reap on error/cancel. Main ACL process lifetime is bounded5s using the accepted private-process ownership conventions; any extra root process has the same finite bound. Unexpected output/exit code or timeout is refusal, never None. Output must use finite status codes and contains no path/blob/key text. Do not infer that two bounded5s effects guarantee a5s network response. Native test children are additionally supervised by the package Windows Job Object; no detached worker/service is created.

## Sequential bounded records

After present entry/type/ACL admission, open each exact present file once. Enforce opened-handle size46..4142 and read at most4143 bytes; require exact accepted wrapper magic/count<=4096/length/SHA256. Growth, truncation, malformed/checksum mismatch or I/O failure is Err, never None. Release its private temporary bytes after visit and before the next record. Absent entries call visit(index,None) only within this valid whole-pass root context. Present corruption after an apparent end still refuses; the caller never stops after its first None.

Strict optional single-read remains unchanged and separately accepted; batch code cannot translate its MissingLocalState errors into absence. Batch may share private bounded reader internals if root scopes that source refactor and proves unchanged old behavior, or keep a small additive exact-format read implementation. It must not call a reader that respawns ACL validation for every file while claiming the startup refinement.

Only after all requested names, all visitor calls and final root validation succeed does Ok establish a complete observational pass. Receipt consumer verifies all64 names/anchors and then atomically publishes its in-memory admitted inventory. A later append uses a new full pass plus create_new; no cached empty-tail result or saved policy/session permission is reused. Failed append behavior and explicit repair remain those of the reviewed book supplement.

## Evidence and dispatch

Root behaviorTDD first uses a compiled actual empty/valid/corrupt inventory fixture: middle ACL refusal cannot be hidden by later success, a corrupt tail cannot become None, callback errors/refused root produce no accepted inventory, duplicate/invalid inputs do not spawn effects, and one opened-handle max+1 bound rejects growth. Include actual directory/nonregular/reparse tests, absent configured root and all64 callback coverage. Pure wrapper byte tests alone do not establish Windows/Linux filesystem policy.

Record real empty64 and present64 pass latency, process counts, payload high-water/reservation and failure cleanup using disposable owner-private vaults. Compare against the observed25468ms baseline without claiming an unobserved RSS or hard timing class. Preserve accepted identity single-reader/full-suite regressions and collect actual hosted Linux mode behavior. Independent review must inspect new sidecar stdin/output/deadline ownership, no-follow/ACL/root and sequential read/visitor semantics, not only green data tests.

Book effects stay held until root's batch implementation and guide's independent source review pass. Pure receipt and notification protocol work can proceed independently. The shared one-repository foreground owner must then demonstrate actual three-lane progress under batch/book work before any responsiveness claim; issue23 native/semantic synchronization gates remain open.
