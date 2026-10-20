# Local receiver receipt issuance

The local issuer is implemented and tested in the follow-on checkout at `.tmp/chat-receiver-issuance-worktree`, branched from local Chat commit d49f82e236dd3a910388abf864c04753c2e687a4. The separate forty-path Chat payload, original author HEAD and real index remain frozen. This slice does not complete issue #25.

## Public local API

```rust
pub struct LocalReceiptIssuer<'a> {
    pub policy: ChatPolicy,
    pub receiver: Author,
    pub peer: &'a [u8],
    pub device_key: &'a SecretSeed,
}
pub fn issue_delivery_receipt(
    &mut self,
    original_request: RequestId,
    issuer: LocalReceiptIssuer<'_>,
) -> Result<SignedChatReceipt>;
```

The trusted host selects its local receiver account/device, full peer identity and borrowed device seed. Every call verifies the existing bounded signed ledger and durable current membership in one read transaction. The receiver must be a current PLAYER with a nonrevoked device belonging to that account, matching the full 1..128-byte peer and seed-derived public key. Scope must match; policy currently contains only scope. The API accepts no remote key enrollment, caller membership snapshot, receipt fields or replacement message body. No private seed is copied into storage.

A nonzero request must identify the first verified durable original. Zero yields Malformed; missing or uncommitted originals and deduplicated aliases yield Conflict. Receipt fields derive from that original and the validated receiver selection. The production issuer reuses the existing canonical receipt encoder and signing domains: 259 body bytes plus 64 signature bytes, 323 total within the retained 420-byte storage bound. No schema or version change is needed.

## Authorization and effects

Fresh remote posts and permitted history still require their existing operation-bound one-shot challenge, current membership, real peer identity and signed device proof. Issuance creates and consumes no such ticket; its separate local authority comes from the matched local seed and current receiver membership.

A previously committed sender may later be revoked without erasing its authenticated original. The retained sender signature and immutable request/message/sequence/cursor relationships remain verified; fresh sender admission still rejects the revoked device. Device key rotation and archive key retention require a separate policy and are outside this slice.

Issuance writes no SQL rows and advances neither Chat nor membership frontier. It signs inside the verified read snapshot, then releases the receipt only after the precommit path check, successful read commit and postcommit path check. Uncertain commit/path failures retain the existing quarantine discipline. Repeating an original with the same valid local identity returns the same receipt.

## Accepted evidence

The original positive integration case first failed at the intended authenticated UnsupportedOperation versus independently expected receipt, then passed unchanged after the minimal issuer implementation. All 25 assertions and 5 fixture assertions remain unchanged. It uses genuine Alice/Bob admission and authenticated post/history, receiver close/reopen, independent canonical signature verification, durable outbox acknowledgement/reopen and unchanged receiver bytes/frontiers.

The separate controls target passed all 15 cases: 14 refusals plus retained delivery after genuine sender revocation. Its 35 physical assertions and 4 setup assertions cover wrong seed, authentic alias, absent/zero/uncommitted request, unknown device, wrong full peer/account, peer length, foreign universe/history, current receiver revocation and genuinely admitted RELAY membership. Public signed admission/revocation establishes these states. Read-only SQL observations occur after closing the exclusive owner; refusal cases preserve exact bytes, rows and frontiers through reopen. The scope-only policy does not offer an independent same-scope unequal-policy input.

Pinned Rust 1.98.1, locked offline exact-target compilation and owned formatting checks passed without warnings. Each native phase used managed Windows Jobs, a separate preservation postguard, a 240-second bound and exact restoration of seven environment variables. The positive test ran in 2.34 seconds and controls in 14.55 seconds. These are local issuance results, not a fresh run of the earlier 55-case Chat inventory. Evidence is retained under `.tmp/issue-completion-gates/chat-receiver-issuance`, including original RED, unchanged GREEN and control captures.

## Remaining scope

This local issuer is now composed by the [bounded wire profile](chat-wire-frames.md), [production Noise delivery and retry](chat-peer-delivery.md), and [permitted remote history](chat-peer-history.md). Their separate evidence does not come from the local issuance cases above. Presence, explicit expiry, retention, mute/block, moderator policy, game worker integration and Starsector UI/Intel remain pending. Save/reload, node restart, Nex/Market absence, frame performance and real-game acceptance require their own evidence. Receiver cursors remain local pagination positions; sender sequences do not establish a global order. Availability depends on an online peer or archive bridge. No financial authority, permanent availability or full issue closure is claimed.
