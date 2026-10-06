# NF-CANON-1 identity extension, domain 8 schema 1

This is a separately registered typed extension implemented by `nf-identity`, with maintained SHA-256/Ed25519. It uses the existing exact header `NF-CANON-1\0`, little-endian u16 domain=8, u16 kind, u16 schema=1. Baseline generic NF-CANON records and protobuf wire registration continue rejecting domain 8: enabling these records in a transport requires explicit capability/payload registration and cross-language admission work. Kernel extension domain 7 is independent.

All IDs are exactly 16 opaque bytes, public keys/digests exactly 32 bytes, signatures exactly 64 bytes. Counters/time/frontiers are raw u64 little-endian. Peer bytes are nonempty opaque bytes <=128, prefixed by u32 little-endian length; the transport must validate its actual PeerId separately. Booleans are exactly u8 0/1. Role masks are u8 nonzero subsets of 127. No strings, floating point, implicit defaults, normalization or unknown fields exist. Signatures are outside canonical records and sign the SHA-256 of the whole header and fixed-order body. Counts/lengths are checked before allocation.

`Scope` = UniverseId, HistoryId. `PublicIdentity` = AccountId, account public key, DeviceId, device public key, peer bytes.

| Kind | Fixed body field order |
| --- | --- |
| 1 Invitation | Scope, invitation ID16, issuer AccountId, recipient PublicIdentity, roles u8, expiry Unix seconds u64, issued membership revision u64, reusable bool |
| 2 DeviceRotation | Scope, AccountId, DeviceId, current membership frontier u64, replacement device public key, replacement peer bytes |
| 3 AccountRotation | Scope, AccountId, current membership frontier u64, replacement account public key |
| 4 DeviceRevocation | Scope, issuer AccountId, target DeviceId, current membership frontier u64 |
| 5 DeviceProof | Scope, AccountId, DeviceId, current membership frontier u64, authenticated peer bytes, challenge32 |
| 6 MembershipState | Scope, membership revision u64, owner AccountId, account map, device map, consumed invitation set |

Account map: u32 count<=128, then sorted unique AccountId, public key32, role mask u8. Device map: u32 count<=512, then sorted unique DeviceId, owning AccountId, public key32, peer bytes, revoked bool. Consumed set: u32 count<=1024, then sorted unique invitation IDs16. Ordering compares unsigned raw ID bytes. A snapshot must contain the owner account and every device must reference a contained account. Entire records are bounded to 256 KiB; overflow/unknown header/kind/schema, zero roles, malformed flags, duplicate/unordered keys, invalid references and trailing bytes reject. Used invitations are retained; reaching the cap refuses new grants instead of evicting replay protection.

Kinds 1/6 have bounded public byte decoders; kinds 2–5 are typed local policy/signature records without an incoming wire handler. Encoding a supported record or verifying its signature alone does not confer permission. Private key file formats are local storage formats and do not use this domain or enter authoritative membership.
