#![allow(dead_code)]
use crate::process_support::read_bounded;
use nf_contract::identity::*;
use nf_store::{KnownFrontiers, miniature::*};
use std::path::Path;
pub fn write_before(store: &MiniatureStore, base: &Path) {
    std::fs::write(
        base.with_extension("before.envelope"),
        store.snapshot_bytes().unwrap(),
    )
    .unwrap();
    std::fs::write(
        base.with_extension("before.world"),
        nf_kernel::miniature::encode_miniature_snapshot(store.world()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        base.with_extension("before.pending"),
        store.pending().map_or_else(Vec::new, |f| {
            nf_kernel::miniature::encode_miniature_frontier(f).unwrap()
        }),
    )
    .unwrap();
    let known = store.known_frontiers().unwrap();
    let mut meta = Vec::with_capacity(65);
    meta.extend_from_slice(known.storage.scope.universe.as_bytes());
    meta.extend_from_slice(known.storage.scope.history.as_bytes());
    meta.extend_from_slice(&known.storage.event_sequence.0.to_le_bytes());
    meta.extend_from_slice(&known.storage.store_revision.to_le_bytes());
    meta.push(u8::from(known.storage.membership_revision.is_some()));
    meta.extend_from_slice(&known.storage.membership_revision.unwrap_or(0).to_le_bytes());
    meta.extend_from_slice(&known.minimum_authority_term.0.to_le_bytes());
    std::fs::write(base.with_extension("before.meta"), meta).unwrap();
    let mut authority = Vec::new();
    if let Some(a) = store.authority() {
        authority.extend_from_slice(&a.term.0.to_le_bytes());
        authority.extend_from_slice(&a.session.0.to_le_bytes());
        authority.extend_from_slice(a.account.as_bytes());
        authority.extend_from_slice(a.device.as_bytes());
        authority.extend_from_slice(&a.membership_revision.to_le_bytes());
    }
    std::fs::write(base.with_extension("before.authority"), authority).unwrap();
}
pub fn known(base: &Path) -> MiniatureKnownFrontiers {
    let b = read_bounded(&base.with_extension("before.meta"), 65).unwrap();
    assert_eq!(b.len(), 65);
    assert_eq!(b[48], 1);
    let count = |s: usize| u64::from_le_bytes(b[s..s + 8].try_into().unwrap());
    MiniatureKnownFrontiers {
        storage: KnownFrontiers {
            scope: nf_identity::model::Scope {
                universe: UniverseId::from_slice(&b[..16]).unwrap(),
                history: HistoryId::from_slice(&b[16..32]).unwrap(),
            },
            event_sequence: EventSeq(count(32)),
            store_revision: count(40),
            membership_revision: Some(count(49)),
        },
        minimum_authority_term: AuthorityTerm(count(57)),
    }
}
pub fn authority(base: &Path) -> Option<MiniatureAuthority> {
    let b = read_bounded(&base.with_extension("before.authority"), 56).unwrap();
    if b.is_empty() {
        return None;
    }
    assert_eq!(b.len(), 56);
    let count = |s: usize| u64::from_le_bytes(b[s..s + 8].try_into().unwrap());
    Some(MiniatureAuthority {
        term: AuthorityTerm(count(0)),
        session: RuntimeSession(count(8)),
        account: AccountId::from_slice(&b[16..32]).unwrap(),
        device: DeviceId::from_slice(&b[32..48]).unwrap(),
        membership_revision: count(48),
    })
}
