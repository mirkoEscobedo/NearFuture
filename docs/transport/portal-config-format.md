# Foreground portal trusted configuration format

Design proposal for independent review before parser/CLI source dispatch. This is local operator input, never a wire decoder, credential, permission or semantic synchronization format. It refines portal-owner-design.md without changing receipt/notification contracts or existing nf-peer configuration.

The input is one regular file of at most16384 bytes, read through one opened handle with a16385-byte limit; reject excess bytes, invalid UTF8 or changed/invalid opened-file length. The file is ASCII, LF-only, ends with exactly one LF, and contains no BOM, CR, NUL, tabs, blank lines or trailing spaces. Each line has exactly one ASCII space between tokens. Fields occur once in the fixed order below. Unknown, missing, repeated, reordered or trailing fields refuse; errors are static and never echo paths or input.

First line is exactly `NF-PORTAL-CONFIG-1`. Second is `mode serve`, `mode watch` or `mode init-book`, matching the CLI command. Then the following common key/value lines occur in this exact order:

| Key | Value |
| --- | --- |
| universe | Nonzero16-byte identity as32 lowercase hex digits |
| history | Same identity shape |
| ruleset |32-byte hash as64 lowercase hex digits; must equal the actual opened Store World ruleset |
| content |64 lowercase hex digits; trusted configured content context, not a claim of native/content capture |
| local_account | Nonzero16-byte identity |
| local_device | Nonzero16-byte identity |
| local_event_min | Canonical u64 decimal |
| local_store_min | Canonical u64 decimal |
| local_membership_min | Canonical u64 decimal |
| server_peer | Canonical base58 PeerId, decoded1..128 bytes and round-trip equal |
| server_account | Nonzero16-byte identity |
| server_device | Nonzero16-byte identity |
| server_membership_min | Canonical u64 decimal |

Canonical decimal is `0` or a nonzero digit followed by digits, without leading zeros, sign or whitespace; checked parsing rejects overflow. Hash zeroes are not silently replaced. Identity zeroes refuse. Original kind is exactly1,2 or3, matching reviewed OriginalReceipt::new. No record supplies a caller override for the derived133-byte RequestBinding digest.

For serve, append in order `receipt_listen ADDRESS`, `bulk_listen ADDRESS`, `notification_listen ADDRESS`, `anchor_count 0`, `END`. Listener ADDRESS is exactly `/ip4/LOOPBACK/tcp/PORT`, with IPv4 canonical dotted decimal and127.0.0.0/8 loopback; PORT is canonical decimal0..65535. Nonzero addresses must differ. Each tcp/0 listener requires a distinct actual NewListenAddr before Ready. No peer suffix, wildcard, DNS, relay, discovery or adjacent-port inference is accepted. Actual readiness appends the loaded peer, which must match configured server pin/current membership owner.

For watch, append in order `receipt_address ADDRESS`, `bulk_address ADDRESS`, `notification_address ADDRESS`, `anchor_count N`, N anchor rows, `END`. N is1..8. ADDRESS is exactly `/ip4/LOOPBACK/tcp/PORT/p2p/PEER`, PORT1..65535, and canonical PEER equals server_peer. All three addresses must differ.

Each anchor row is exactly:

`anchor SLOT GENERATION HEAD_SHA256 REQUEST OPERATION KIND PAYLOAD_SHA256 REMOTE_EVENT_MIN REMOTE_STORE_MIN REMOTE_MEMBERSHIP_MIN`

SLOT and GENERATION are canonical0..7; rows have strictly increasing distinct slots. HEAD_SHA256 is nonzero64-digit lowercase hex. REQUEST/OPERATION are nonzero32-digit lowercase hex identities, KIND1..3, PAYLOAD_SHA25664 lowercase hex. Remote minima are canonical u64; remote membership must be at least server_membership_min. All requests are distinct. The original principal/scope/pin/ruleset/content derive from common fields. The CLI-selected slot must occur, and every occupied reserved book slot must have its independent trusted anchor; recovery rejects unknown roots/tails/gaps. An older externally anchored prefix can admit a validated later chain only as observational recovery, requiring returned latest anchors to be retained independently for later rollback protection.

For init-book, append `original_count N`, N original rows, `END`; no network addresses occur. N is1..8. Each row is exactly:

`original SLOT REQUEST OPERATION KIND PAYLOAD_SHA256 REMOTE_EVENT_MIN REMOTE_STORE_MIN REMOTE_MEMBERSHIP_MIN`

Shapes/order/principal derivation/minimum rules match watch rows. Every reserved name must be absent; generation0 is created only from these explicit typed originals. No random request/op ID, role/key creation, strategic Store commit, automatic repair or network startup is permitted. Failure retains any partial new files and reports no readiness. Output is bounded public exact head anchors for independent trusted retention. A config or stdout beside the rollbackable book does not itself protect backup freshness.

Local minima become KnownFrontiers for opening the existing local Store; membership is Some(local_membership_min), never inferred None or zero from a missing policy. ReceiptConfig's local protected membership minimum is max(local_membership_min,server_membership_min). Remote event/store minima remain separate per-original SourceMinima and never become local Store-open frontiers. Current actual SQL membership/frontier consistency and local/server identities are checked again at every protected stage. Trusted hashes/anchor input cannot create an active session or authenticated receipt.

CLI accepts exactly `serve VAULT SAVE_ROOT DB CONFIG DURATION_MS`, `watch VAULT SAVE_ROOT DB CONFIG SLOT DURATION_MS`, or `init-book VAULT SAVE_ROOT DB CONFIG`. Duration is canonical500..60000ms and slot0..7. Surplus/missing args/options refuse before opening any state. Paths must be UTF8, nonempty, at most4096 UTF8 bytes, with no NUL/CR/LF; never print them. All roots, DB, policy and identities must already exist. Config parsing, vault/Store admission and full journal recovery happen before the network-run timer; the500..60000ms timer starts before listen/dial and is never restarted. It does not promise bounded OS admission/sync latency. Offline initialization has no network timer and stays bounded by fixed input/inventory and owned helper deadlines.

Required parser evidence includes valid each-mode fixtures; every missing/reordered/unknown/duplicate/trailing key, whitespace/UTF8/line-ending error; maximum plus one input/path; numeric overflow/noncanonical decimals; invalid/zero identities; kind boundaries; PeerId/address round-trip and mismatched pin; distinct endpoint/slot/request checks; count/row mismatch; absent selected watch slot; and actual open admission/recovery failures. Tests must establish input refusal before persistent/network effects, not merely mirror parser branches. Owned CLI process tests prove finite readiness/output/cleanup separately. This design does not close issue23 or establish native Starsector responsiveness.

Independent design R1 found no Important issue against initial grammar SHA2566c7a8668c274a6c400212e2a18d23ca11d4f5b879ed026c92305c32137752eb6. The following source preflight clarifications resolve its two minor observations without changing the grammar.

Before admission, require existing VAULT and SAVE_ROOT directories and existing regular DB/config files. Preserve accepted canonical vault/save containment (vault cannot lie beneath save root), actual opened-file bounds, and the cooperative owner lifecycle; this is not atomic protection against concurrent alias replacement. Accepted PrivateVault::open canonicalizes save paths but does not alone prove SAVE_ROOT is a directory, so CLI must check that precondition explicitly.

Preserve Store's local-disk precondition and existing Windows non-disk-prefix refusal. This does not claim mapped-drive classification; a network-mapped location remains outside the admitted host precondition. Store::open_existing uses no SQLITE_OPEN_CREATE: absent/invalid data refuses without creating an empty fallback. These checks do not confer membership or trust on input descriptors.
