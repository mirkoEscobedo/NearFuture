# Starsector debugging notes

Reference: [AutoColonyManager operational playbook](https://github.com/mirkoEscobedo/AutoColonyManager/blob/fae8d11b317c96231cf56e85eb8e0abb2131fe6e/starsector-ui-tester.md), pinned at `fae8d11b317c96231cf56e85eb8e0abb2131fe6e`.
These observations guide experiments; they do not certify Near Future integration.

- Starsector's script classloader can reject reflection APIs even when ordinary JVM tests pass. Prefer public game APIs and test the actual game runtime before enabling a capability.
- Paused/idle LWJGL captures may show old frames. Refresh by defocusing/refocusing and verify a changed state before repeating a toggle. Check fresh log markers as separate evidence.
- Campaign map interaction may be ignored while paused. Record pause/speed state in a reproducer; avoid advancing a private campaign merely to collect diagnostics.
- Window identities change when the launcher hands off to the game. Rediscover the target. Coordinate/scaling behavior belongs to the UI tool in use; upstream's older MCP coordinates are not a portable contract.
- Identify actual game processes rather than matching any window title containing Starsector. Browser tabs can trigger a false launch guard; Gradle Java workers are not the game. Never terminate unrelated processes by executable name.
- Capture a log baseline before an experiment. Detect rotation/truncation before interpreting a byte-offset delta; old warnings in a heavily modded installation are not evidence of a new regression.
- When rebuilding custom panels, remove child panel regions to avoid invisible old controls consuming clicks. The rules engine adds numbering and shortcut hints; bare hyphenated rule arguments can be parsed as arithmetic.

Use `STARSECTOR_HOME` as a local configuration input. Logs are under `starsector-core/starsector.log` and screenshots under `screenshots`. Keep private logs, paths and saves out of public evidence. The Near Future build must not deploy to a game installation as a side effect of ordinary tests; AutoColonyManager's `build` and `buildModJar` do deploy and are not safe templates for that command contract.
