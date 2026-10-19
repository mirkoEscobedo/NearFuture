use super::owned_community::OwnedCommunity;
use nf_identity::{
    keys::SecretSeed,
    model::*,
    private_storage::PrivateVault,
    rotation::{account_rotation_digest, rotation_digest},
};
use nf_store::Store;
use std::path::Path;

#[derive(Clone, Copy)]
pub enum Rotation {
    Account,
    Device,
}

pub struct ReopenedCommunity {
    pub server_vault: PrivateVault,
    pub client_vault: PrivateVault,
    pub server_store: Store,
    pub client_store: Store,
    pub server: PublicIdentity,
    pub client: PublicIdentity,
    pub state: MembershipState,
}

/// Commit signed rotations to both repositories, then discard every original
/// store, vault and signing seed before reopening from disk.
pub fn reopen_rotated(root: &Path, kind: Rotation) -> ReopenedCommunity {
    let mut c = OwnedCommunity::new(root);
    let mut expected = [c.server.public.clone(), c.client.public.clone()];
    for (index, (local, vault)) in [(&c.server, &c.server_vault), (&c.client, &c.client_vault)]
        .into_iter()
        .enumerate()
    {
        let replacement = SecretSeed::generate().unwrap();
        let original_roles = c.state.accounts[&local.public.account].roles;
        vault.stage_rotation_key(&replacement).unwrap();
        let next = match kind {
            Rotation::Account => {
                let change = AccountRotation {
                    scope: c.state.scope,
                    account: local.public.account,
                    frontier: c.state.revision,
                    new_key: replacement.public_key(),
                };
                let digest = account_rotation_digest(&change);
                expected[index].account_key = replacement.public_key();
                c.state
                    .rotate_account(
                        &change,
                        &local.account_key.sign(&digest),
                        &replacement.sign(&digest),
                    )
                    .unwrap()
            }
            Rotation::Device => {
                let change = DeviceRotation {
                    scope: c.state.scope,
                    account: local.public.account,
                    device: local.public.device,
                    frontier: c.state.revision,
                    new_key: replacement.public_key(),
                    new_peer: local.public.peer.clone(),
                };
                let digest = rotation_digest(&change).unwrap();
                expected[index].device_key = replacement.public_key();
                c.state
                    .rotate_device(
                        &change,
                        &local.account_key.sign(&digest),
                        &replacement.sign(&digest),
                    )
                    .unwrap()
            }
        };
        for store in [&mut c.server_store, &mut c.client_store] {
            store
                .commit_membership(Some(c.state.revision), &next)
                .unwrap();
        }
        c.state = next;
        assert_eq!(expected[index].account, local.public.account);
        assert_eq!(expected[index].device, local.public.device);
        assert_eq!(expected[index].peer, local.public.peer);
        assert_eq!(
            c.state.accounts[&local.public.account].roles,
            original_roles
        );
    }
    let server_known = c.server_store.known_frontiers().unwrap();
    let client_known = c.client_store.known_frontiers().unwrap();
    let state = c.state.clone();
    let [server, client] = expected;
    drop(c);
    let saves = root.join("saves");
    ReopenedCommunity {
        server_vault: PrivateVault::open(&root.join("server-private"), &saves).unwrap(),
        client_vault: PrivateVault::open(&root.join("client-private"), &saves).unwrap(),
        server_store: Store::open_existing(root.join("server.sqlite"), server_known).unwrap(),
        client_store: Store::open_existing(root.join("client.sqlite"), client_known).unwrap(),
        server,
        client,
        state,
    }
}
