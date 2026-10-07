# Reference verification checkpoint

2026-10-07, ready for independent read-only review. This is a partial issue #14 delivery, with no actual game or full Nex equivalence claim.

Observed public RED/GREEN: below-threshold end/abort initially missing then adapted; active exact-class duplicate incorrectly generated then suppressed before update; nonfinite public isValid silently returned a boolean then constructor validation made it unavailable; eligible equal-chance selected peace initially returned NONE then emitted one ordered immutable ceasefire call trace. Ordinary output-level NaN rejection alone was insufficient; the public isValid seam supplied the meaningful RED.

Fresh exact commands:

- `java\gradlew.bat -p java :nex-reference:check :nex-reference:jar --no-daemon --offline`: pass on pinned compiler/runtime21.0.8, release17.
- `java\gradlew.bat -p java :nex-reference:check :nex-reference:jar --no-daemon --offline --rerun-tasks --no-build-cache -PtestJavaExecutable=H:/Games/Starsector/jre/bin/java.exe`: all9tasks executed and passed, exercising the actual bundled Java17 on synthetic inputs.
- JAR listing contains only nf.nex.reference classes, manifest and retained MIT license; javap reports class major61.

The independently authored corpus has10 concern cases and16 peace cases. It covers adjacent f32 threshold inputs, absent/active/ended/other-class concerns, all three baseline traits plus faction modifier, increased/decreased config threshold, persistent ended object and explicit new-instance restart. Peace cases preserve adjacent doubles around a float chance, strict treaty chance equality, conditional second-draw consumption and selected eligibility failures. Additional public scenarios cover direct update/end/restart, exact canUse override, missing second draw, unsupported player branch, unavailable offensive coverage and nonfinite/out-of-range Math.random values.

The module-owned boundary gate requires an empty compile classpath and rejects game/ambient I/O/random APIs in production source before compilation. It is architectural lint, not a hostile-code sandbox. All module dependency-lock configurations are empty; no libraries or game binaries were added. Source/reference inputs and outputs are immutable copied records; no live effect is possible within the implemented methods.

Unverified: real merged inputs/capture, installed bytecode/source equivalence, proprietary MutableStat aggregate/iteration semantics, full action priority/selection, actual tag iteration order, multi-enemy traversal, player prompt/delegate behavior, live offensive enumeration, actual event success and manager/alliance effects, shadow equivalence, narrow writer suppression, game application and authority/recovery. Exact raw-bit comparisons apply to supported fields; no broad epsilon hides divergence. Those omissions block complete issue #14 acceptance and are not mock-certified.
