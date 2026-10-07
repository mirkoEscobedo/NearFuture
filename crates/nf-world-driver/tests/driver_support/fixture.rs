use nf_contract::identity::*;
use nf_identity::{
    model::*,
    private_storage::PrivateVault,
    signing::{admission_proof, sign_invitation},
};
use nf_store::miniature::{AuthConfig, BootstrapPolicy, MiniatureGenesisSpec};
use nf_world_driver::CreateOptions;
use sha2::{Digest, Sha256};
/// Disposable explicit provisioning only; this helper is never part of a production command.
#[allow(dead_code)]
pub struct Fixture {
    pub scratch: super::Scratch,
    pub options: CreateOptions,
}
#[allow(dead_code)]
impl Fixture {
    pub fn new() -> Self {
        let scratch = super::Scratch::new();
        let saves = scratch.0.join("saves");
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let vault = PrivateVault::create(&scratch.0.join("private0"), &saves).unwrap();
        let owner = vault.create_identity(vec![1]).unwrap();
        let mut membership = MembershipState::bootstrap(scope, &owner.public).unwrap();
        let mut controllers = vec![owner.public.account];
        for ordinal in 1..=2u8 {
            let private =
                PrivateVault::create(&scratch.0.join(format!("private{ordinal}")), &saves).unwrap();
            let local = private.create_identity(vec![ordinal + 1]).unwrap();
            let invitation = Invitation {
                scope,
                id: [ordinal; 16],
                issuer: owner.public.account,
                recipient: local.public.clone(),
                roles: Roles::PLAYER,
                expires_at: 100,
                issued_revision: membership.revision,
                reusable: false,
            };
            let proof =
                admission_proof(&invitation, &local.account_key, &local.device_key).unwrap();
            let signed = sign_invitation(invitation, &owner.account_key).unwrap();
            membership = membership.redeem(&signed, &proof, 1).unwrap();
            controllers.push(local.public.account);
        }
        let controllers: [AccountId; 3] = controllers.try_into().unwrap();
        let body = nf_identity::codec::encode_state(&membership).unwrap();
        let membership_file = scratch.0.join("public.membership");
        std::fs::write(&membership_file, &body).unwrap();
        let options = CreateOptions {
            database: scratch.0.join("world.sqlite"),
            vault: scratch.0.join("private0"),
            game_save_root: saves,
            membership_file,
            genesis: MiniatureGenesisSpec {
                genesis: nf_world::Genesis {
                    universe: scope.universe,
                    history: scope.history,
                    seed: [3; 32],
                    accounts: controllers,
                },
                aggregate: AggregateId::from_bytes([20; 16]),
                provider: ProviderId::from_bytes([21; 16]),
                provider_aggregate: AggregateId::from_bytes([22; 16]),
            },
            policy: BootstrapPolicy {
                scope,
                membership_digest: Sha256::digest(body).into(),
                owner: owner.public.clone(),
                controllers,
                auth: AuthConfig::default(),
            },
        };
        drop(owner);
        Self { scratch, options }
    }
}
