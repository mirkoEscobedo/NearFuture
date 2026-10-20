use nf_contract::identity::{AccountId, DeviceId, EventSeq, HistoryId, RequestId, UniverseId};
use nf_identity::private_storage::PrivateVault;
use nf_ipc::{
    ChatQueryPort, DiscoveryRecord, FramePump, IpcError, LocalPrincipal, NoOperations, NodeServer,
    PublishedRendezvous, QueryPort, SessionConfig, StoreQueryPort, default_limits,
};
use nf_wire::generated as g;
use prost::Message;
use std::{
    net::TcpStream,
    path::Path,
    time::{Duration, Instant},
};
fn hex<const N: usize>(text: &str) -> Result<[u8; N], IpcError> {
    if text.len() != N * 2 || !text.is_ascii() {
        return Err(IpcError::Malformed);
    }
    let mut bytes = [0; N];
    for (i, value) in bytes.iter_mut().enumerate() {
        *value =
            u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(|_| IpcError::Malformed)?;
    }
    Ok(bytes)
}
fn vault(args: &[String]) -> Result<PrivateVault, IpcError> {
    PrivateVault::open(Path::new(&args[2]), Path::new(&args[3]))
        .map_err(|_| IpcError::PrivateStorage)
}
fn receive_frame(pump: &mut FramePump, deadline: Instant) -> Result<Vec<u8>, IpcError> {
    while Instant::now() < deadline {
        if let Some(bytes) = pump.poll(4096, 4096)? {
            return Ok(bytes);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Err(IpcError::Disconnected)
}
fn duration(text: &str) -> Result<u64, IpcError> {
    let value = text.parse().map_err(|_| IpcError::Malformed)?;
    if !(100..=60_000).contains(&value) {
        return Err(IpcError::Limit);
    }
    Ok(value)
}
fn session_config(fields: &[String]) -> Result<SessionConfig, IpcError> {
    let mut bytes = [0; 8];
    getrandom::fill(&mut bytes).map_err(|_| IpcError::Io)?;
    let config = SessionConfig {
        universe: hex(&fields[0])?,
        history: hex(&fields[1])?,
        runtime_session: u64::from_le_bytes(bytes),
        ruleset: hex(&fields[2])?,
        content_policy: hex(&fields[3])?,
        limits: default_limits(),
    };
    if config.runtime_session == 0 {
        return Err(IpcError::SessionMismatch);
    }
    Ok(config)
}
fn foreground<P: QueryPort>(
    vault: &PrivateVault,
    name: &str,
    config: SessionConfig,
    principal: LocalPrincipal,
    port: P,
    duration: u64,
) -> Result<(), IpcError> {
    let mut node = NodeServer::bind(config.clone(), principal, port)?;
    let mut bulk = NodeServer::bind_bulk(config.clone(), principal, NoOperations)?;
    let publication = PublishedRendezvous::publish_with_bulk(
        vault,
        name,
        config,
        principal,
        node.address(),
        Some(bulk.address()),
    )?;
    node.install_authenticator(
        publication
            .record()
            .authenticator(nf_ipc::EndpointRole::Control)?,
    )?;
    bulk.install_authenticator(
        publication
            .record()
            .authenticator(nf_ipc::EndpointRole::Bulk)?,
    )?;
    println!("READY");
    let deadline = Instant::now() + Duration::from_millis(duration);
    while Instant::now() < deadline {
        node.poll(Instant::now())?;
        bulk.poll(Instant::now())?;
        std::thread::sleep(Duration::from_millis(1));
    }
    node.invalidate();
    bulk.invalidate();
    drop(bulk);
    drop(node);
    publication.close()?;
    println!("STOPPED");
    Ok(())
}
fn peer(text: &str) -> Result<Vec<u8>, IpcError> {
    if text.is_empty() || text.len() > 256 || !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(IpcError::Malformed);
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| IpcError::Malformed))
        .collect()
}
fn run(args: &[String]) -> Result<(), IpcError> {
    match args.get(1).map(String::as_str) {
        Some("init") if args.len() == 4 => {
            PrivateVault::create(Path::new(&args[2]), Path::new(&args[3]))
                .map_err(|_| IpcError::PrivateStorage)?;
            println!("Owner-private IPC vault initialized.");
            Ok(())
        }
        Some("serve") if args.len() == 12 => {
            let vault = vault(args)?;
            let config = session_config(&args[5..9])?;
            let principal = LocalPrincipal {
                account: AccountId::from_bytes(hex(&args[9])?),
                device: DeviceId::from_bytes(hex(&args[10])?),
            };
            foreground(
                &vault,
                &args[4],
                config,
                principal,
                NoOperations,
                duration(&args[11])?,
            )
        }
        Some("serve-store") if args.len() == 15 => {
            let vault = vault(args)?;
            let config = session_config(&args[6..10])?;
            let member: u64 = args[13].parse().map_err(|_| IpcError::Malformed)?;
            let known = nf_store::KnownFrontiers {
                scope: nf_identity::model::Scope {
                    universe: UniverseId::from_bytes(config.universe),
                    history: HistoryId::from_bytes(config.history),
                },
                event_sequence: EventSeq(args[11].parse().map_err(|_| IpcError::Malformed)?),
                store_revision: args[12].parse().map_err(|_| IpcError::Malformed)?,
                membership_revision: Some(member),
            };
            let store =
                nf_store::Store::open_existing(&args[5], known).map_err(|_| IpcError::ReadOnly)?;
            if store.world().to_spec().ruleset_hash != config.ruleset {
                return Err(IpcError::PolicyMismatch);
            }
            let port = StoreQueryPort::from_vault(store, &vault, peer(&args[10])?, member)?;
            let principal = port.principal();
            foreground(
                &vault,
                &args[4],
                config,
                principal,
                port,
                duration(&args[14])?,
            )
        }
        Some("serve-chat" | "serve-chat-command") if args.len() == 18 => {
            let vault = vault(args)?;
            let config = session_config(&args[7..11])?;
            let membership = args[13].parse().map_err(|_| IpcError::Malformed)?;
            let scope = nf_identity::model::Scope {
                universe: UniverseId::from_bytes(config.universe),
                history: HistoryId::from_bytes(config.history),
            };
            let policy = nf_store::chat::ChatPolicy { scope };
            if nf_store::chat::codec::policy_digest(&policy) != config.content_policy {
                return Err(IpcError::PolicyMismatch);
            }
            let store = nf_store::chat::ChatStore::open_existing(
                &args[5],
                &policy,
                nf_store::chat::KnownChatFrontiers {
                    scope,
                    revision: args[12].parse().map_err(|_| IpcError::Malformed)?,
                    membership_revision: membership,
                },
            )
            .map_err(|_| IpcError::ReadOnly)?;
            let local_peer = peer(&args[11])?;
            let identity = vault
                .load_identity(local_peer.clone())
                .map_err(|_| IpcError::PrivateStorage)?
                .public;
            let current = store.current_membership().map_err(|_| IpcError::ReadOnly)?;
            // Trusted local selection; open_existing must match immutable stored lifetime pins.
            // Rotation/revocation does not re-enroll a key or recover an older profile.
            let profile = nf_store::chat::outbox::OutboxProfile::from_current(
                &policy,
                &current,
                nf_store::chat::Author {
                    account: identity.account,
                    device: identity.device,
                },
                nf_store::chat::Author {
                    account: AccountId::from_bytes(hex(&args[15])?),
                    device: DeviceId::from_bytes(hex(&args[16])?),
                },
            )
            .map_err(|_| IpcError::Unauthorized)?;
            let outbox = nf_store::chat::outbox::ClientOutbox::open_existing(
                &args[6],
                &profile,
                args[14].parse().map_err(|_| IpcError::Malformed)?,
            )
            .map_err(|_| IpcError::ReadOnly)?;
            if args[1] == "serve-chat-command" {
                let port = nf_ipc::ChatCommandPort::from_vault(
                    store, outbox, &vault, local_peer, membership,
                )?;
                let principal = port.principal();
                return foreground(
                    &vault,
                    &args[4],
                    config,
                    principal,
                    port,
                    duration(&args[17])?,
                );
            }
            let port = ChatQueryPort::from_vault(store, outbox, &vault, local_peer, membership)?;
            let principal = port.principal();
            foreground(
                &vault,
                &args[4],
                config,
                principal,
                port,
                duration(&args[17])?,
            )
        }
        Some("attach" | "query") if args.len() == 5 || args.len() == 6 => {
            let is_query = args[1] == "query";
            if args.len() != if is_query { 6 } else { 5 } {
                return Err(IpcError::Malformed);
            }
            let vault = vault(args)?;
            let record = DiscoveryRecord::read(&vault, &args[4])?;
            let stream = TcpStream::connect_timeout(&record.address(), Duration::from_secs(1))
                .map_err(|_| IpcError::Disconnected)?;
            let mut pump = FramePump::new(
                stream,
                4096.min(record.config().limits.control_frame_bytes as usize),
            )?;
            let client = record.client(nf_ipc::EndpointRole::Control)?;
            pump.send(client.hello())?;
            let challenge = receive_frame(&mut pump, Instant::now() + Duration::from_secs(2))?;
            let proof = client.respond(&challenge)?;
            pump.send(proof.proof())?;
            let accepted = receive_frame(&mut pump, Instant::now() + Duration::from_secs(2))?;
            let session = proof.finish(&accepted)?;
            pump.activate(&session)?;
            if !is_query {
                println!("ATTACHED protocol 1 (mutual local token scope)");
                return Ok(());
            }
            let config = record.config();
            let principal = record.principal();
            let query = g::QueryOperation {
                request_id: Some(g::RequestId {
                    value: hex::<16>(&args[5])?.to_vec(),
                }),
                principal: Some(g::Principal {
                    account_id: Some(g::AccountId {
                        value: principal.account.as_bytes().to_vec(),
                    }),
                    device_id: Some(g::DeviceId {
                        value: principal.device.as_bytes().to_vec(),
                    }),
                }),
                universe_id: Some(g::UniverseId {
                    value: config.universe.to_vec(),
                }),
                history_id: Some(g::HistoryId {
                    value: config.history.to_vec(),
                }),
            };
            pump.send(
                &g::ControlEnvelope {
                    protocol_version: 1,
                    runtime_session: Some(g::RuntimeSession {
                        value: config.runtime_session,
                    }),
                    required: Some(g::RequiredSemantics {
                        capability_ids: vec![1],
                        schema_ids: vec![1],
                    }),
                    body: Some(g::control_envelope::Body::QueryOperation(query)),
                    transport: None,
                }
                .encode_to_vec(),
            )?;
            let response = session.admit_response(
                &receive_frame(&mut pump, Instant::now() + Duration::from_secs(2))?,
                RequestId::from_bytes(hex(&args[5])?),
                &nf_ipc::SessionFence::new(config.runtime_session)?,
            )?;
            match response {
                nf_ipc::AdmittedServer::Status(status) => {
                    println!("RETAINED phase {}", status.phase);
                    Ok(())
                }
                nf_ipc::AdmittedServer::Error(_) => Err(IpcError::Unsupported),
            }
        }
        _ => Err(IpcError::Malformed),
    }
}
fn main() {
    if let Err(error) = run(&std::env::args().collect::<Vec<_>>()) {
        eprintln!("IPC unavailable: {error:?}");
        std::process::exit(1);
    }
}
