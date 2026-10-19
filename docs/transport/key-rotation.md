# Current membership signing keys

Refs #95, #22, #23. Foreground endpoints load their existing Noise identity,
check the durable membership against the configured scope, ruleset and minimum
revision, then select its unique device binding for that actual peer. Revoked
devices, non-PLAYER accounts and non-owner server identities are refused.
Ambiguous peer bindings are refused rather than selecting an arbitrary device.

The selected public identity supplies the current account and device signing
keys to `PrivateVault::load_membership_keys`. That loader binds the stable IDs
to the original local vault and loads only the exact selected private keys.
Missing keys are an error; opening an endpoint never generates a replacement.
Key staging alone grants no membership or runtime authority. Supported account
and device signing-key rotations keep their IDs and existing Noise PeerId.

`foreground_key_rotation` commits signed rotations for both local endpoints,
discards the original stores, vaults and signing seeds, reopens from disk, then
authenticates actual TCP/Noise status, bulk and paired requests. Its expected
keys come directly from independently generated replacements. The bulk cases
verify the exact 65,536-byte payload digest. `current_key_refusals` covers missing
private material, revocation, wrong or ambiguous peer binding, role, owner and
stale membership refusal alongside the original identity and transport tests.

The selected #76 candidate also carries the same endpoint fix and checks receipt
repository opening against current membership before loading signing keys.
Its receipt policy, stable configured IDs, owner pin, frontier and book checks
remain required. That separate candidate is preparation for receipt convergence;
it does not qualify the complete receipt capability.

This accepted development path is registered in `headless-foundation-v1`.
The profile still runs only build identity: peer, Chat, game and persistent
writer capabilities stay disabled. These tests do not certify remote discovery,
membership synchronization, live key replacement within an already-open owner,
native game integration or a complete Chat/replica/receipt release.
