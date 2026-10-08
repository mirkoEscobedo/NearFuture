# Private action V2 synthetic replay verification

Observed on 2026-10-08. The independently generated record is 434 bytes with SHA-256 `5910e944560a7655a232d48c53ea2934da74f38ee5f6b941a5fbd007d01df738`. The original first test produced genuine `Unsupported` RED before the decoder existed. The same complete test then passed after the minimum decoder, retaining original file SHA-256 `856bbac1290080ca5023058709ea7dbb0e6125e6030ed4689e21b844b02dac38`.

Five boundary controls passed, covering every truncated prefix, trailing/over-limit bytes, every invalid closed-tag and boolean octet, bounded UTF8/control/NFC text and pinned metadata rejection, 96 option/fact/observation combinations, and acceptance at 571 bytes with rejection at 572. CapturedUnverified metadata decodes with its supplied label but cannot be replayed. Synthetic reproduction remains CopiedFacts and ShadowOnly.

The first strict Clippy gate passed workspace formatting but rejected two fixture hex-reader `chunks_exact(2)` chains. The reviewed mechanical replacement uses `as_chunks::<2>().0.iter()`, retaining pair order and discarded remainder. Original tests and failure evidence are preserved; an exact inverse restores both complete originals. Every original assertion, literal, comment and import is retained. Current first file SHA-256 is `afe280e724d7a196abbf79e16be5f263a251c4abab78797bbac4e53eed25fd24`; it differs from the historical original.

Fresh checks after that repair passed:

- The current first test: 1 passed, 0 ignored or filtered; all five boundary controls: 5 passed, 0 ignored or filtered.
- Workspace format check; warning-denying all-target Clippy for nf-nex-shadow and nf-nex-boundary; TypeScript and domain boundaries.
- All 16 unchanged schema/fixture reproducibility checks.
- The affected Rust suites: 54 passed, 0 failed, 2 explicitly ignored configured JVM tests across 29 harness summaries.
- Locked offline optimized builds of nf-nex-shadow and nf-nex-boundary.

Each phase and its distinct postguard reported Windows Job Object ownership, status zero and no timeout. Six inherited Java/PATH environment values were restored exactly; the scoped process census was zero. The earlier Clippy failure remains recorded and no lint allowance was added.

These checks cover the portable private replay slice, rather than a full workspace or real-JVM gate. Hosted Linux/Windows validation for the eventual exact draft head remains pending, including the separately configured JVM tests. Native capture certification, an actual Java-authority/Rust-worker loop, live action selection/effects and disconnect/save/load validation remain unqualified. No native world or financial authority is granted; issue #15 remains open.