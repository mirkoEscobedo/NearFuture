# Verification checkpoint

2026-10-07: local pure source/shape implementation ready for independent read-only review. Whole issue #13 remains blocked on #11 capture/hook evidence and #14 source-reference characterization.

Observed vertical public RED/GREEN cycles: replaced class-path definition incorrectly accepted then rejected; non-finite float accepted then rejected while finite bits remain identical; missing subject faction accepted then refused; unknown merged priority tag silently admitted then refused; missing extension outcomes then all findings enumerated with admission refusal; duplicate faction admitted then refused; oversized extension text cloned then bounded; oversized raw definition diagnostic copied then bounded. Setup-only shared lock mismatches were coordinated and are not counted as behavior RED.

Fresh commands:

- `cargo test -p nf-nex-boundary --locked --offline`: 15 integration tests pass, including a finite 10,000 raw float mutation loop and a cumulative nested-entry quota fixture.
- `cargo clippy -p nf-nex-boundary --all-targets --locked --offline -- -D warnings`: pass.
- `cargo fmt -p nf-nex-boundary -- --check`: pass.
- `npm run check:boundaries`, `npm run check:types`, and `node --test tests/architecture/boundaries.test.mjs`: pass; independent scoped root policy review found no important finding.

The crate is no_std with alloc, only local nf-contract. Production files have at most 270 lines; tests/support remain below topology warning thresholds. There is no time, RNG, file, network, game API, key, binary input or mutation effect in production. No registry or protocol extension is introduced.

Hard input bounds: nonempty canonical Unicode13 text <=256 UTF-8 bytes, cumulative snapshot text <=1 MiB, cumulative snapshot collection entries <=8192; factions/strengths/concerns <=256, relations/draws <=4096, selected actions/traits/tags/modifiers/intervals <=64. The separately callable diagnostic assessment allows <=256 each of listeners/direct calls/other definitions and applies text limits before copying findings. Raw public definition diagnostics validate text before error cloning.

Source commit equality and nonzero digest fields validate the supplied provenance shape; they do not authenticate source bytes or prove a merged configuration agrees with those digests. Only synthetic copied fixtures were exercised. CapturedUnverified remains UNOBSERVED just like Synthetic. There is no output variant granting authority, behavioral certification or live mutation. Conservative baseline trait/tag/interval/draw/modifier tokens reject changed semantics until characterized. makePeace may be represented but remains an unavailable provider surface.

Unobserved: installed source/JAR equivalence, coherent real capture, loaded JAR/listener/direct-call inventory, source-oracle formulas/MutableStat equivalence, timer/RNG call-order replay, actual narrow writer suppression, shadow equivalence, game application and authority/recovery. Static hook requirements in the inventory describe what must be established; no hook coverage is claimed.

Independent R1 adopted one important precision gap: RandomDraw used f32 for Java Math.random f64. Repair 1 introduces finite raw DoubleBits(u64) and changes draw storage without any narrowing. The focused nonfinite guard was observed RED (positive infinity accepted), then GREEN for NaN and both infinities. A separate exact-precision fixture proves two adjacent doubles that collapse to the same f32 remain distinct after NexWorld admission. Full 15 tests, strict Clippy and formatting pass. Repair 1 is frozen for independent R2; every original game/reference/suppression residual remains UNOBSERVED.

Coordinator read-only review R2: PASS on implementation76ff038a64c09fae8df1572e5328ff2b6073861b. R1 identified narrowing of Java Math.random double draws to FloatBits; the author repaired it once with finite DoubleBits/u64 and a public adjacent-double preservation regression. Fresh focused15 tests, strict all-target Clippy and formatting passed. No unresolved important portable source finding remains. A separate agent reviewed the coordinator architecture rules; metadata-conditional scanning permits independently scoped IPC candidates without this crate.

A clean isolated checkout containing only accepted foundation/kernel/storage/identity plus this Nex boundary passed offline npm installation, the complete public check (106 Rust tests on Windows,39 Node tests and all Java assertion suites), and the locked offline workspace release build. Concurrent IPC/capture/reference implementation was excluded. Public Windows/Linux CI is still pending. This portable checkpoint does not close issue13: actual coherent capture, loaded extension inventory, runtime source equivalence and narrow native writer suppression remain unobserved. All admitted values remain SHADOW_ONLY/UNOBSERVED.
