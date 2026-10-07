use super::Scratch;
use libp2p::PeerId;
use nf_contract::{canonical::binding::RequestBinding, identity::*};
use nf_identity::{
    model::*,
    private_storage::{LocalIdentity, PrivateVault},
    signing::{admission_proof, sign_invitation},
};
use nf_kernel::*;
use nf_store::{PrincipalDevice, Store};
use nf_transport::{
    auth::ServerPin,
    identity::TransportIdentity,
    notification::NotifyLimits,
    notification_effects::NotifyPolicy,
    receipt::OriginalReceipt,
    receipt_effects::{ReceiptConfig, ReceiptRepo, TrustedPreparedCommit},
};
/// Real generated vaults and one owned server database; no public vector key grants authority.
pub struct RepoFixture {
    pub server: LocalIdentity,
    pub client: LocalIdentity,
    pub server_peer: PeerId,
    pub client_peer: PeerId,
    pub state: MembershipState,
    pub repo: ReceiptRepo,
    pub client_repo: ReceiptRepo,
    pub original: OriginalReceipt,
    pub commit: Option<TrustedPreparedCommit>,
    _scratch: Scratch,
    _gate: std::sync::MutexGuard<'static, ()>,
}
impl RepoFixture {
    pub fn new() -> Self {
        let gate = super::fixture_gate();
        let scratch = Scratch::new();
        let saves = scratch.original.join("saves");
        std::fs::create_dir(&saves).unwrap();
        let vault = PrivateVault::create(&scratch.original.join("server"), &saves).unwrap();
        let transport = TransportIdentity::create(&vault).unwrap();
        let server_peer = transport.peer_id();
        let server = vault.create_identity(server_peer.to_bytes()).unwrap();

        let client_vault = PrivateVault::create(&scratch.original.join("client"), &saves).unwrap();
        let client_transport = TransportIdentity::create(&client_vault).unwrap();
        let client_peer = client_transport.peer_id();
        let client = client_vault
            .create_identity(client_peer.to_bytes())
            .unwrap();
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let founder = MembershipState::bootstrap(scope, &server.public).unwrap();
        let invitation = Invitation {
            scope,
            id: [3; 16],
            issuer: server.public.account,
            recipient: client.public.clone(),
            roles: Roles::PLAYER,
            expires_at: 100,
            issued_revision: 0,
            reusable: false,
        };
        let admission =
            admission_proof(&invitation, &client.account_key, &client.device_key).unwrap();
        let signed = sign_invitation(invitation, &server.account_key).unwrap();
        let state = founder.redeem(&signed, &admission, 1).unwrap();
        let mut spec = WorldSpec::empty(scope.universe, scope.history, [3; 32], [4; 32]);
        spec.factions.push(Faction {
            id: EntityId::from_bytes([10; 16]),
            aggregate: AggregateId::from_bytes([10; 16]),
        });
        spec.markets.push(Market {
            id: EntityId::from_bytes([30; 16]),
            aggregate: AggregateId::from_bytes([30; 16]),
            faction: EntityId::from_bytes([10; 16]),
            credits: 100,
        });
        let provider = ProviderId::from_bytes([41; 16]);
        spec.providers.push(ProviderState {
            id: provider,
            aggregate: AggregateId::from_bytes([41; 16]),
            draws: 0,
            cooldown_until: WorldTick(0),
        });
        spec.registry.push(ProviderManifest {
            id: provider,
            implementation_hash: [41; 32],
            version: 1,
            state_schema: 1,
            kind: ProviderKind::Manual,
            read_domains: [Domain::Faction, Domain::Market, Domain::Provider].into(),
            write_domains: [Domain::Market, Domain::Provider].into(),
            capabilities: [Capability::AdjustMarket].into(),
            principals: [client.public.account].into(),
            max_events: 2,
        });
        for n in [30, 41] {
            spec.ownership.push(OwnershipRecord {
                aggregate: AggregateId::from_bytes([n; 16]),
                provider,
                generation: 1,
                activation_seq: EventSeq(0),
                ruleset_hash: [4; 32],
            });
        }
        for n in [10, 30, 41] {
            spec.revisions
                .insert(AggregateId::from_bytes([n; 16]), AggregateRevision(0));
        }
        let world = World::new(spec).unwrap();
        let intent = Intent {
            request: RequestId::from_bytes([9; 16]),
            operation: OperationId::from_bytes([9; 16]),
            job: JobId::from_bytes([9; 16]),
            actor: client.public.account,
            universe: scope.universe,
            history: scope.history,
            provider,
            expected: [10, 30, 41]
                .map(|n| (AggregateId::from_bytes([n; 16]), AggregateRevision(0)))
                .into(),
            command: Command::AdjustMarket {
                market: EntityId::from_bytes([30; 16]),
                delta: 1,
            },
        };
        let authority = AuthorityContext {
            term: AuthorityTerm(1),
            session: RuntimeSession(1),
        };
        let frontier = admit(&world, vec![intent.clone()], authority).unwrap();
        let mut store =
            Store::create(scratch.original.join("server/policy.sqlite"), &world).unwrap();
        store.commit_membership(None, &founder).unwrap();
        store.commit_membership(Some(0), &state).unwrap();
        store
            .prepare(
                &frontier,
                &[PrincipalDevice {
                    request: intent.request,
                    device: client.public.device,
                }],
                &[],
            )
            .unwrap();
        let pin = ServerPin {
            peer: server_peer,
            account: server.public.account,
            device: server.public.device,
            minimum_membership: 1,
        };
        let binding = RequestBinding {
            request_id: intent.request,
            account_id: client.public.account,
            device_id: client.public.device,
            universe_id: scope.universe,
            history_id: scope.history,
            operation_kind: 3,
            payload_digest: intent_digest(&intent).unwrap(),
        };
        let original = OriginalReceipt::new(intent.operation, binding, pin).unwrap();
        let Settlement::Committed { batch, .. } = settle(
            &world,
            &frontier,
            vec![],
            authority,
            MissingPolicy::Recompute,
        )
        .unwrap() else {
            panic!("real deterministic settlement")
        };
        let commit = TrustedPreparedCommit::seal(&store, original.clone(), batch).unwrap();
        let config = ReceiptConfig {
            scope,
            ruleset: [4; 32],
            content: [5; 32],
            local_account: server.public.account,
            local_device: server.public.device,
            server_pin: pin,
            minimum_membership: 1,
        };
        let repo = ReceiptRepo::open_owned(store, vault, config, &[]).unwrap();
        let client_world = World::new(WorldSpec::empty(
            scope.universe,
            scope.history,
            [3; 32],
            [4; 32],
        ))
        .unwrap();
        let mut client_store =
            Store::create(scratch.original.join("client/policy.sqlite"), &client_world).unwrap();
        client_store.commit_membership(None, &founder).unwrap();
        client_store.commit_membership(Some(0), &state).unwrap();
        let client_config = ReceiptConfig {
            local_account: client.public.account,
            local_device: client.public.device,
            ..config
        };
        let client_repo =
            ReceiptRepo::open_owned(client_store, client_vault, client_config, &[]).unwrap();
        Self {
            server,
            client,
            server_peer,
            client_peer,
            state,
            repo,
            client_repo,
            original,
            commit: Some(commit),
            _scratch: scratch,
            _gate: gate,
        }
    }
    pub fn policy(&self) -> NotifyPolicy {
        let c = self.repo.config();
        NotifyPolicy {
            scope: c.scope,
            ruleset: c.ruleset,
            content: c.content,
            limits: NotifyLimits::default(),
            minimum_membership: 1,
        }
    }
}
