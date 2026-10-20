# Managed admission preparation (#97)

This focused repair is stacked on #87 at `045f71d84cd54a8f1e84684ebcd265e6e90b4339`. Its completion artifact is a reviewed pushed commit with focused Windows/Linux evidence. #108 owns subsequent managed lifecycle qualification and integration into dev; this preparation does not qualify #87, close #43, or enable a managed runtime.

`Store::open_existing` performs integrity checking, which can leave SQLite's empty temporary schema registered. Adoption now explicitly binds the actual main database to the expected canonical local path. An optional empty-filename temp schema is permitted only when it contains no objects. Other attachments, temp objects, memory-only main databases and mismatched main paths are refused. Existing durable membership, owner and revoked-device authorization still apply.

Managed descriptor validation refuses support slices above 32 entries and cardinality mismatches before copying/sorting support entries or copying world state. Legacy `Composition::require_supported` also enforces the 32-entry input ceiling while preserving its existing valid subset/superset behavior. This addresses a local resource bound; it is not evidence of a network attack. Persisted formats, schema identities and signed proof layouts are unchanged.

Focused checks:

```text
cargo test -p nf-store --lib --test managed_source_admission --test managed_provider_bounds --locked --offline -- --test-threads=1
```

The focused workflow runs this command on Windows and Linux. Public tests create, commit, close and reopen an actual Store before signed adoption; cover 32 entries, 33 and 65,536 preallocated support entries, wrong paths, durable revocation and nonowner rejection; and assert unchanged source/catalog bytes and absent upgrade outputs on bounds refusal. Separate SQLite policy tests exercise real attachments and temp table/view/trigger refusal. Test binary pins contain copied real test executable bytes, explicitly labelled `MANAGED_RUNTIME_NOT_QUALIFIED`; they establish no runtime migration or game behavior.

Record `cargo test -p nf-store --locked --offline` and strict library Clippy results with the exact artifact in #108. The source stack's existing 30 strict Clippy errors are outside this production repair. Its separately ignored whole lifecycle test is not credited by focused checks or a default package test pass. No historical format or repair budget is reset, and no production or CI timeout is raised.
