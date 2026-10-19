#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/rotated_community.rs"]
mod rotated_community;
#[path = "support/scratch.rs"]
mod scratch;

use nf_transport::{
    auth::ServerPin,
    receipt_effects::{ReceiptConfig, ReceiptRepo},
};
use rotated_community::{Rotation, reopen_rotated};

fn reopen_receipts(kind: Rotation) {
    let tmp = scratch::Scratch::new();
    let c = reopen_rotated(&tmp.0, kind);
    let pin = ServerPin {
        peer: libp2p::PeerId::from_bytes(&c.server.peer).unwrap(),
        account: c.server.account,
        device: c.server.device,
        minimum_membership: c.state.revision,
    };
    for (store, vault, expected) in [
        (c.server_store, c.server_vault, c.server),
        (c.client_store, c.client_vault, c.client),
    ] {
        let config = ReceiptConfig {
            scope: c.state.scope,
            ruleset: [4; 32],
            content: [5; 32],
            local_account: expected.account,
            local_device: expected.device,
            server_pin: pin,
            minimum_membership: c.state.revision,
        };
        let repo = ReceiptRepo::open_owned(store, vault, config, &[])
            .expect("receipt opening must load current membership signing keys");
        assert_eq!(repo.local_public(), &expected);
        assert!(repo.book_anchors().is_empty());
    }
}

#[test]
fn receipt_repositories_reopen_with_rotated_account_keys() {
    reopen_receipts(Rotation::Account);
}

#[test]
fn receipt_repositories_reopen_with_rotated_device_keys() {
    reopen_receipts(Rotation::Device);
}
