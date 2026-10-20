use super::fixture::Fixture;
use nf_identity::model::MembershipRepository;
use nf_transport::{identity::TransportIdentity, receipt::*, receipt_effects::*};
use std::io::{Read, Write};
/// Disposable test credentials only. Existing protected destination files retain their ACL/mode.
/// This is adversarial fixture setup, never an operational vault import or grant.
pub fn alternate_client(f: &Fixture) -> ReceiptRepo {
    let vault = f.scratch.vault("alternate-private");
    let t = TransportIdentity::create(&vault).unwrap();
    vault.create_identity(t.peer_id().to_bytes()).unwrap();
    for name in ["identity-key-v1", "blob-transport-ed25519-v1"] {
        let src = f.scratch.root.join("client-private").join(name);
        let dst = f.scratch.root.join("alternate-private").join(name);
        let mut source = std::fs::File::open(src).unwrap();
        assert!(source.metadata().unwrap().len() <= 4096);
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        Read::by_ref(&mut source)
            .take(4097)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= 4096);
        let mut target = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(dst)
            .unwrap();
        target.write_all(&bytes).unwrap();
        target.sync_all().unwrap();
    }
    let mut config = f.client.config();
    config.ruleset[0] ^= 1;
    let world = super::fixture::alternate_world(&f.state, &f.client_public, config.ruleset);
    let mut store =
        nf_store::Store::create(f.scratch.root.join("alternate.sqlite"), &world).unwrap();
    let founder =
        nf_identity::model::MembershipState::bootstrap(f.state.scope, &f.server_public).unwrap();
    store.commit_membership(None, &founder).unwrap();
    store.commit_membership(Some(0), &f.state).unwrap();
    let mut repo = ReceiptRepo::open_owned(store, vault, config, &[]).unwrap();
    repo.initialize_originals(&[(
        0,
        f.original.clone(),
        SourceMinima {
            event: nf_contract::identity::EventSeq(0),
            store_revision: 0,
            membership_revision: f.state.revision,
        },
    )])
    .unwrap();
    repo
}
