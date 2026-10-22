# Copied-fact untargeted peace traversal

Qualification: LOCAL PARTIAL REFERENCE RED/GREEN PASSED. The unchanged public ordering assertion failed against the unfinished scaffold, then passed with the reviewed implementation. JDK21.0.8 compiled this increment for Java17 with strict diagnostics; domain smoke ran on the actual bundled Java17.0.10. No Java21 domain-smoke runtime, fresh JAR/full-workspace/hosted or live-game qualification is claimed.

This partial issue #14 increment isolates the body of pinned DiplomacyBrain.checkPeace after its outer guards, only when targetFactionId is null. It characterizes candidate ordering, eligibility skips, three reported null attempt outcomes, early reported non-null return, and an immutable request to updateEnemiesAndCeasefires(+0f). It never invokes tryMakePeace, getters, event creation, weariness writes, player delegates or enemy-cache mutation. Reported null/non-null outcomes are supplied facts and do not certify an event succeeded.

Source pin: Nexerelin a669f4d0740e95a4acbb6b894dde09ade67aa754; DiplomacyBrain blob8d4cafeebe579c7fc7f3906088f5a9fd71ed873d SHA256a8ec5be42031efaf2340bad199bdebbb435ed774ebfd7718e0b7ecbb50c04dcb, lines622-669. Its comment says descending, but the actual comparator calls Float.compare(first,second), which is ascending. Sorting uses the raw getWarWeariness(String) overload, not enemy-adjusted weariness. That overload can lazily insert a missing map entry: the reference receives copied raw bits and never calls it.

Input: copied candidate-list order; per occurrence ID, raw32 ranking bits, recent-war/can-ceasefire/commissioned-player/offensive predicates, and reported return when actually attempted. The caller supplies the Boolean conjunction for the commissioned-player guard and the existential offensive-blocking predicate. Actual loaded list order and predicate capture remain unverified. List occurrences are preserved without deduplication. Reported outcomes may be absent only on skipped/unreached entries.

All32-bit score encodings are allowed because original Float.compare orders signedzero, infinities and NaN. No arbitrary nonfinite rejection is added to this ordering seam; existing finite arithmetic/RNG guards in the selected-enemy evaluator are unchanged. Java stable sort retains supplied tie order, including equal NaNs. Reference envelope bounds of256 candidates and128 Java UTF16 units per ID are explicit copied-input limits, not original game validation or certification rules.

Independent expected cases: ascending order versus misleading comment; exactly three null attempts and no fourth; all four skip gates/precedence without attempt consumption; early non-null return plus one+0f cache-refresh request; signedzeros/infinities/twoNaNs; finite tie order; duplicate list occurrences; empty list. Additional input-completeness, copied-result immutability and envelope bounds are support checks. The first RED scaffold preserved incoming unsorted order and attempted none; the unchanged public assertion expected low/middle/high/fourth and produced exactly that ordering AssertionError. All ten traversal case groups passed against reviewed GREEN; only the ordering assertion has a separate genuine RED. Other groups are independent regression controls.

No Rust port, provider registration, world commitment profile, game capture, installed-JAR equivalence, proprietary MutableStat behavior, economic effect, writer suppression or authority handover is delivered. This is not whole issue #13/#14/#16 closure. Primary API references: https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/lang/Float.html#compare(float,float) and https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/Collections.html#sort(java.util.List,java.util.Comparator).

## Actual commands and evidence

From the author root with JDK21.0.8, the exact compile command was:

```text
java\gradlew.bat -p java :nex-reference:verifyReferenceBoundary :nex-reference:testClasses :nex-reference:exportDifferentialClasspath --no-daemon --offline --rerun-tasks --no-build-cache
```

The runtime vector was `H:/Games/Starsector/jre/bin/java.exe -cp <contents of java/nex-reference/build/differential-classpath.txt> nf.nex.reference.ReferenceSmoke`. The exported classpath was restricted to this module main/test classes and resources. The existing boundary requires an empty external/game compile classpath. Compilation used release17, `-Xlint:all -Werror`; seven Gradle tasks executed in each compile. No new JAR or Java21 domain smoke was run.

| Phase | Actual phase Jobs / separate postguard | Result / elapsed |
| --- | --- | --- |
| Scaffold compile | 5 / 1 | PASS;11.589s |
| Ordering RED | 2 before stop / 1 | Intended AssertionError;1.928s |
| Reviewed implementation compile | 5 / 1 | PASS;11.959s |
| Unchanged full smoke | 3 / 1 | PASS;2.182s |

The first failure reported actual `high,low,fourth,middle`, expected `low,middle,high,fourth`. GREEN printed `PASS: copied-fact peace traversal checks 10` and `PASS: isolated concern/peace source corpus 10/16`; original pre-call and lifecycle assertions also returned without failure. No skipped/filtered traversal cases or changed test assertions were introduced.

Each configured command ran under a package-owned Windows Job with concurrency1 and240s bounds. Distinct postguards preserved 931 author source pins, 959 held Main pins, 3 primary sources and 8 tools throughout, with 52 historical evidence leaves checked in scaffold/RED and 135 in GREEN; all earlier captures were retained. All six nullable process environment values were restored exactly; no timeout, warning, retry or live matching scoped process remained. These are local checks on the existing author base, not current-Main/hosted or real Nex effects qualification.

Production SHA-256: `1d2c7cf5ca2f928aef4e1d30913a0bf9a9769dc6208d58f9a8d06f2f29846345`. Characterization stayed `3c2afc6f795b557640aef36e7674504ba6db93aca6d1bf1fc4327e3bf6fe8906`; Smoke stayed `0b3bf6449beacd0d3deb64c14fe7f041e17ea6d60863f9d3f5f27436c3298f57` through RED/GREEN.

| Immutable local capture | SHA-256 |
| --- | --- |
| `traversal-first-scaffold2-firstRed.raw.json` | `83af330e9f176388dd7de55d9562b7535a5d2a23a050ecd6d2ab68c18ced6f0e` |
| `traversal-first-green-compile.raw.json` | `785df09d61150ab409248bbc5f1ce22ec381f7c8e4a68ecf3436fef436cb372d` |
| `traversal-first-green-greenSmoke.raw.json` | `72a092bf3d244177d91100391ec3a992895dfcfe4ebf9b3e60e812ea02d762b4` |
| `traversal-first-green-greenSmoke.postguard.raw.json` | `a22c8a10411692088665bcac5ed395385f9f6223a09a845c740170a7f2c96c29` |

Additional pinned getter source: DiplomacyManager blob `9fa8774d6b9ad809345eb7af13fd0456e618e11a`, SHA-256 `49bb9856a873ae6963839acb65ae994a6adf10d426301743314df556002f3435`, lines1309–1334. Adapted production retains the original complete MIT notice; the existing license resource is unchanged. No proprietary API implementation or excluded assets are copied.

This additional traversal covers the copied ordering/attempt omission in the previous [selected-enemy adaptation](source-adaptation.md) and [historical verification checkpoint](verification.md). Their original qualification remains historical; real multi-enemy capture/getter effects/cache mutation remain unavailable. Real returned events, player delegates, loaded merged inputs, installed source-JAR equivalence, MutableStat semantics, manager/alliance/economic effects, narrow writer suppression and authority handover remain unverified. This does not activate an optional feature or close issues #13/#14/#16.

[Pinned DiplomacyBrain](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/diplomacy/DiplomacyBrain.java#L622-L669); [pinned DiplomacyManager getter](https://github.com/Histidine91/Nexerelin/blob/a669f4d0740e95a4acbb6b894dde09ade67aa754/jars/sources/ExerelinCore/exerelin/campaign/DiplomacyManager.java#L1309-L1334).
