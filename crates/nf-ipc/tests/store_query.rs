use nf_contract::identity::*;
use nf_identity::{
    model::{MembershipRepository, MembershipState, Scope},
    private_storage::PrivateVault,
};
use nf_ipc::{IpcError, QueryPort, StoreQueryPort};
use nf_kernel::{AuthorityContext, Command, Intent, MissingPolicy, Settlement, World, WorldSpec};
use nf_store::{PrincipalDevice, Store};
use std::fs;
fn query(account: AccountId, device: DeviceId) -> nf_wire::generated::QueryOperation {
    use nf_wire::generated as g;
    g::QueryOperation {
        request_id: Some(g::RequestId { value: vec![8; 16] }),
        principal: Some(g::Principal {
            account_id: Some(g::AccountId {
                value: account.as_bytes().to_vec(),
            }),
            device_id: Some(g::DeviceId {
                value: device.as_bytes().to_vec(),
            }),
        }),
        universe_id: Some(g::UniverseId { value: vec![1; 16] }),
        history_id: Some(g::HistoryId { value: vec![2; 16] }),
    }
}
#[test]
fn durable_pending_and_rejected_queries_require_vault_identity_and_fresh_membership() {
    let root = std::env::temp_dir().join(format!("nf-ipc-store-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("vault"), &saves).unwrap();
    let local = vault.create_identity(vec![4, 5]).unwrap();
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let world = World::new(WorldSpec::empty(
        scope.universe,
        scope.history,
        [3; 32],
        [4; 32],
    ))
    .unwrap();
    let mut store = Store::create(root.join("history.sqlite"), &world).unwrap();
    let state = MembershipState::bootstrap(scope, &local.public).unwrap();
    store.commit_membership(None, &state).unwrap();
    let intent = Intent {
        request: RequestId::from_bytes([8; 16]),
        operation: OperationId::from_bytes([8; 16]),
        job: JobId::from_bytes([8; 16]),
        actor: local.public.account,
        universe: scope.universe,
        history: scope.history,
        provider: ProviderId::from_bytes([9; 16]),
        expected: Default::default(),
        command: Command::AdjustMarket {
            market: EntityId::from_bytes([7; 16]),
            delta: 1,
        },
    };
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(9),
    };
    let frontier = nf_kernel::admit(&world, vec![intent], authority).unwrap();
    store
        .prepare(
            &frontier,
            &[PrincipalDevice {
                request: RequestId::from_bytes([8; 16]),
                device: local.public.device,
            }],
            &[],
        )
        .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let store = Store::open_existing(root.join("history.sqlite"), known).unwrap();
    let mut port = StoreQueryPort::from_vault(store, &vault, vec![4, 5], 0).unwrap();
    let q = query(local.public.account, local.public.device);
    let pending = port.query(q.clone()).unwrap();
    assert_eq!(
        pending.phase,
        nf_wire::generated::OperationPhase::Pending as i32
    );
    assert!(pending.committed_event_seq.is_none());
    assert!(matches!(
        port.query(query(local.public.account, DeviceId::from_bytes([77; 16]))),
        Err(IpcError::Unauthorized)
    ));
    let mut store = port.into_store();
    let Settlement::Committed { batch, .. } = nf_kernel::settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("settle")
    };
    store.commit(&batch).unwrap();
    let mut port = StoreQueryPort::from_vault(store, &vault, vec![4, 5], 0).unwrap();
    let rejected = port.query(q.clone()).unwrap();
    assert_eq!(
        rejected.phase,
        nf_wire::generated::OperationPhase::Rejected as i32
    );
    assert_eq!(rejected.committed_event_seq.unwrap().value, 1);

    let mut next = state;
    next.revision = 1;
    next.devices.get_mut(&local.public.device).unwrap().revoked = true;
    port.owner_store_mut()
        .commit_membership(Some(0), &next)
        .unwrap();
    assert!(matches!(port.query(q), Err(IpcError::Unauthorized)));
    drop(port);
    fs::remove_dir_all(root).unwrap();
}
