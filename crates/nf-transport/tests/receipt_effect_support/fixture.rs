use super::scratch::Scratch;
use nf_contract::{canonical::binding::RequestBinding, identity::*};
use nf_identity::{
    keys::SecretSeed,
    model::*,
    signing::{admission_proof, sign_invitation},
};
use nf_kernel::*;
use nf_store::{PrincipalDevice, Store};
use nf_transport::{auth::ServerPin, identity::TransportIdentity, receipt::*, receipt_effects::*};
pub struct Fixture {
    pub server: ReceiptRepo,
    pub client: ReceiptRepo,
    pub owner_key: SecretSeed,
    pub server_public: PublicIdentity,
    pub client_public: PublicIdentity,
    pub original: OriginalReceipt,
    pub commit: Option<TrustedPreparedCommit>,
    pub state: MembershipState,
    pub scratch: Scratch,
}
fn random<const N: usize>() -> [u8; N] {
    let mut b = [0; N];
    getrandom::fill(&mut b).unwrap();
    b
}
impl Fixture {
    pub fn new() -> Self {
        Self::build(false)
    }
    pub fn historical() -> Self {
        Self::build(true)
    }
    fn build(historical: bool) -> Self {
        let scratch = Scratch::new();
        let sv = scratch.vault("server-private");
        let cv = scratch.vault("client-private");
        let st = TransportIdentity::create(&sv).unwrap();
        let ct = TransportIdentity::create(&cv).unwrap();
        let server = sv
            .create_identity_detailed(st.peer_id().to_bytes())
            .unwrap();
        let client = cv
            .create_identity_detailed(ct.peer_id().to_bytes())
            .unwrap();
        let scope = Scope {
            universe: UniverseId::from_bytes(random()),
            history: HistoryId::from_bytes(random()),
        };
        let founder = MembershipState::bootstrap(scope, &server.public).unwrap();
        let invitation = Invitation {
            scope,
            id: random(),
            issuer: server.public.account,
            recipient: client.public.clone(),
            roles: Roles::PLAYER,
            expires_at: 100,
            issued_revision: 0,
            reusable: false,
        };
        let proof = admission_proof(&invitation, &client.account_key, &client.device_key).unwrap();
        let state = founder
            .redeem(
                &sign_invitation(invitation, &server.account_key).unwrap(),
                &proof,
                1,
            )
            .unwrap();
        let (world, intent, frontier, batch) = prepared(&state, &client.public);
        let mut source = Store::create(scratch.root.join("server.sqlite"), &world).unwrap();
        source.commit_membership(None, &founder).unwrap();
        source.commit_membership(Some(0), &state).unwrap();
        source
            .prepare(
                &frontier,
                &[PrincipalDevice {
                    request: intent.request,
                    device: client.public.device,
                }],
                &[],
            )
            .unwrap();
        let mut local = Store::create(scratch.root.join("client.sqlite"), &world).unwrap();
        local.commit_membership(None, &founder).unwrap();
        local.commit_membership(Some(0), &state).unwrap();
        let pin = ServerPin {
            peer: st.peer_id(),
            account: server.public.account,
            device: server.public.device,
            minimum_membership: state.revision,
        };
        let original = OriginalReceipt::new(
            intent.operation,
            RequestBinding {
                request_id: intent.request,
                account_id: client.public.account,
                device_id: client.public.device,
                universe_id: scope.universe,
                history_id: scope.history,
                operation_kind: 3,
                payload_digest: intent_digest(&intent).unwrap(),
            },
            pin,
        )
        .unwrap();
        let commit = if historical {
            source.commit(&batch).unwrap();
            for _ in 0..2 {
                let authority = AuthorityContext {
                    term: AuthorityTerm(1),
                    session: RuntimeSession(1),
                };
                let frontier = admit(source.world(), vec![], authority).unwrap();
                let Settlement::Committed { batch, .. } = settle(
                    source.world(),
                    &frontier,
                    vec![],
                    authority,
                    MissingPolicy::Recompute,
                )
                .unwrap() else {
                    panic!("unrelated empty history");
                };
                source.prepare(&frontier, &[], &[]).unwrap();
                source.commit(&batch).unwrap();
            }
            assert_eq!(source.world().view().event_seq(), EventSeq(3));
            None
        } else {
            Some(TrustedPreparedCommit::seal(&source, original.clone(), batch).unwrap())
        };
        let config = |public: &PublicIdentity| ReceiptConfig {
            scope,
            ruleset: [4; 32],
            content: [3; 32],
            local_account: public.account,
            local_device: public.device,
            server_pin: pin,
            minimum_membership: state.revision,
        };
        let server_repo = ReceiptRepo::open_owned(source, sv, config(&server.public), &[]).unwrap();
        let mut client_repo =
            ReceiptRepo::open_owned(local, cv, config(&client.public), &[]).unwrap();
        client_repo
            .initialize_originals_for_test(&[(
                0,
                original.clone(),
                SourceMinima {
                    event: EventSeq(0),
                    store_revision: 0,
                    membership_revision: state.revision,
                },
            )])
            .unwrap();
        Self {
            scratch,
            server: server_repo,
            client: client_repo,
            owner_key: server.account_key,
            server_public: server.public,
            client_public: client.public,
            original,
            commit,
            state,
        }
    }
    pub fn sessions(
        &mut self,
        id: libp2p::swarm::ConnectionId,
    ) -> (ReceiptSession, ReceiptSession) {
        let sp = libp2p::PeerId::from_bytes(&self.server_public.peer).unwrap();
        let cp = libp2p::PeerId::from_bytes(&self.client_public.peer).unwrap();
        let mut ch = ReceiptClientHandshake::new();
        let mut sh = ReceiptServerHandshake::new();
        let hello = ch.hello(&mut self.client).unwrap();
        let challenge = sh.begin(hello, cp, id, &mut self.server).unwrap();
        let proof = ch.challenge(challenge, sp, id, &mut self.client).unwrap();
        let (finished, server) = sh.proof(proof, cp, id, &mut self.server).unwrap();
        let client = ch
            .finished(finished, sp, id, &mut self.client)
            .unwrap()
            .into_session();
        (client, server)
    }
}
fn prepared(
    state: &MembershipState,
    client: &PublicIdentity,
) -> (World, Intent, Frontier, CommittedBatch) {
    let mut spec = WorldSpec::empty(state.scope.universe, state.scope.history, [3; 32], [4; 32]);
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
        principals: [client.account].into(),
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
        request: RequestId::from_bytes(random()),
        operation: OperationId::from_bytes(random()),
        job: JobId::from_bytes(random()),
        actor: client.account,
        universe: state.scope.universe,
        history: state.scope.history,
        provider,
        expected: [10, 30, 41]
            .map(|n| (AggregateId::from_bytes([n; 16]), AggregateRevision(0)))
            .into(),
        command: Command::AdjustMarket {
            market: EntityId::from_bytes([30; 16]),
            delta: -5,
        },
    };
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(&world, vec![intent.clone()], authority).unwrap();
    let Settlement::Committed { batch, .. } = settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("prepared frontier")
    };
    (world, intent, frontier, batch)
}

pub fn alternate_world(
    state: &MembershipState,
    client: &PublicIdentity,
    ruleset: [u8; 32],
) -> World {
    let (world, ..) = prepared(state, client);
    let mut spec = world.to_spec();
    spec.ruleset_hash = ruleset;
    for owner in &mut spec.ownership {
        owner.ruleset_hash = ruleset;
    }
    World::new(spec).unwrap()
}
