# Receipt control2 pure protocol evidence

This is an author-tested protocol and handshake slice, awaiting independent source review. It implements the frozen receipt contract without promoting kernel payloads, installing remote state, or creating a receipt repository. Existing control1 and bulk1 sources and bytes remain unchanged.

`receipt-pure-checkpoint.json` inventories 21 owned source/test files using SHA-256 of exact Windows working bytes. It excludes itself, this document, the root-owned additive module export and the independently owned fixture producer/data. Published Git text normalization requires a separately identified publication inventory; these historical author hashes must not be relabeled as Git-blob hashes.

## Compiled behavior changes

Each listed RED completed compilation and failed the intended assertion. Missing imports and unavailable symbols were setup failures and are excluded from behavioral evidence.

| Slice | Observed RED | Observed GREEN |
| --- | --- | --- |
| Committed receipt | Closed decoder returned Unsupported for the independently encoded committed receipt | Preserves historical commit EventSeq 7 separately from current source EventSeq 100 |
| Closed registry | Previously unhandled registered Hello rejected | All 14 positive records round-trip exactly; shape/limit negatives reject |
| Authentication namespace | Delegating to control1 produced a different challenge from the independent control2 transcript | Exact separate 619-byte handshake and 245-byte receipt transcripts |
| First attempt | Invalid signature rejected, but a subsequent valid proof reused the same stage | Invalid attempt consumes the stage; retry returns Replay |
| Frame declaration | A declared 557-byte body reached a payload read and returned UnexpectedEof | Declaration returns InvalidData before payload allocation/read |
| Immutable original binding | Original descriptor accepted zero history identity | Closed original identity validates; 133-byte binding commits the full intent digest |

Private native logs are `.tmp/receipt-native/{red,green}-committed.json`, `{red,green}-registry.json`, `{red,green}-domain.json`, `{red,green}-consumed.json`, `{red,green}-framing.json`, and `{red,green}-original.json`. They are local execution evidence, not committed operational data.

## Final author checks

The package-owned Windows Job Object supervisor ran focused strict Clippy and the complete `nf-transport` all-target test suite. Both steps returned status 0 with no timeout; the test suite passed 63 tests in 45 groups, including 12 new receipt tests and the existing encrypted two-peer/foreground process regressions. The receipt tests themselves do not create a receipt network owner. A moving notification test emitted one unused-field warning in this aggregate run; receipt-owned strict Clippy passed.

Result: `.tmp/receipt-native/pure-regression-full.json`, SHA-256 `1e787dc49280998d22676dabfc33dd82a1643d8b028999e5022f5e4fcadc3ca5`.

Reproduce the focused checks through the repository process supervisor, using cargo arguments:

```text
clippy --manifest-path E:/github/NearFuture/Cargo.toml -p nf-transport --lib --test receipt_committed --test receipt_records --test receipt_original --test receipt_framing --test receipt_transcript --test receipt_signatures --test receipt_session --locked --offline -- -D warnings
test --manifest-path E:/github/NearFuture/Cargo.toml -p nf-transport --all-targets --locked --offline
```

`rustfmt --check --edition 2024` passed for the checkpointed owned files. The immutable JSON and TSV corpus hashes match the independently reviewed fixture checkpoint. Primitive signature tests use declared public test keys solely to check Ed25519 bytes; session authentication uses fresh generated identities, genuine signed invitations and maintained membership authorization.

## Implemented boundaries and remaining work

The codec is borrowed and closed, with a 556-byte hard body cap applied before allocation. Every admitted negotiated control frame limit is at least 1,024 bytes, so this parser cap fits every admitted selected frame limit. Other selected codec limits and fixed shapes are checked before record use. Incoming typed records remain untrusted data.

`ReceiptSession` implements the three-stage mutually authenticated handshake, actual PeerId/ConnectionId/session fencing, current supplied membership policy, exact server pin, PLAYER role checks and first-attempt consumption. The trusted future repository owner must load current durable membership for every stage and active record cut. The active envelope guard does not verify a receipt operation proof or authorize a receipt result; the later operation state machine must do that separately.

No receipt book, private inventory reader, sole SQL repository, receipt query client/server, reconnect persistence or two-process lost-reply behavior is claimed by this slice. The strict batch vault API remains a separate reviewed dependency. Native game responsiveness, Java/kernel payload registration and semantic history/snapshot resync remain outside this portable protocol evidence; issue 23 remains open.
