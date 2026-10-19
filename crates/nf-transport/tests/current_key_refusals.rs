#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/scratch.rs"]
mod scratch;

use nf_identity::{
    keys::SecretSeed,
    model::{MembershipRepository, Roles},
};
use nf_transport::{PeerError, node::PeerServer, records::PeerLimits, session::SessionPolicy};

#[test]
fn local_owner_selection_refuses_missing_keys_and_unauthorized_membership() {
    for case in 0..7 {
        let tmp = scratch::Scratch::new();
        let mut c = owned_community::OwnedCommunity::new(&tmp.0);
        let mut next = c.state.clone();
        next.revision += 1;
        let device = next.devices.get_mut(&c.server.public.device).unwrap();
        let expected = match case {
            0 => {
                // A durable key selection cannot regenerate a missing private key.
                next.accounts.get_mut(&c.server.public.account).unwrap().key =
                    SecretSeed::generate().unwrap().public_key();
                PeerError::Storage
            }
            1 => {
                device.revoked = true;
                PeerError::Unauthorized
            }
            2 => {
                device.peer = c.client.public.peer.clone();
                PeerError::Unauthorized
            }
            3 => {
                next.accounts
                    .get_mut(&c.server.public.account)
                    .unwrap()
                    .roles = Roles::WORKER;
                PeerError::Unauthorized
            }
            4 => {
                next.owner = c.client.public.account;
                PeerError::Unauthorized
            }
            5 => {
                // A second device bound to this Noise peer must not select an
                // arbitrary principal, even when the genesis device still exists.
                next.devices.get_mut(&c.client.public.device).unwrap().peer =
                    c.server.public.peer.clone();
                PeerError::Unauthorized
            }
            6 => PeerError::Unauthorized,
            _ => unreachable!(),
        };
        c.server_store
            .commit_membership(Some(c.state.revision), &next)
            .unwrap();
        let policy = SessionPolicy {
            scope: next.scope,
            ruleset: [4; 32],
            content: [5; 32],
            limits: PeerLimits::default(),
            minimum_membership: next.revision + u64::from(case == 6),
        };
        assert!(
            matches!(PeerServer::new(c.server_store, &c.server_vault, policy),
            Err(error) if error == expected),
            "refusal case {case}"
        );
    }
}
