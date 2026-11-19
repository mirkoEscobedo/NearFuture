//! Disabled diagnostic owner mode. The composition root must construct private pipes.
//! No game state, durable store, economic operation, detached service or process takeover.
use nf_contract::identity::{AccountId, DeviceId};
use nf_ipc::{
    EndpointRole, IpcError, LocalPrincipal, NoOperations, NodeServer, PublishedRendezvous,
};
use std::{
    io::{IsTerminal, Read},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

struct Parent {
    nonce: [u8; 32],
    gone: Arc<AtomicBool>,
}
impl Parent {
    fn admit() -> Result<Self, IpcError> {
        // Non-terminal is necessary, not proof of pipe provenance. The trusted Java
        // composition must create anonymous pipes directly; arbitrary redirections
        // are outside this explicit secret-handoff contract.
        if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
            return Err(IpcError::Unauthorized);
        }
        let gone = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&gone);
        let (send, receive) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("nf-parent-eof".into())
            .spawn(move || {
                let mut input = std::io::stdin().lock();
                let mut request = [0; 44];
                let admitted = input
                    .read_exact(&mut request)
                    .map_err(|_| IpcError::Malformed)
                    .and_then(|()| {
                        if &request[..12] != b"NF-PARENT-1\0"
                            || request[12..].iter().all(|b| *b == 0)
                        {
                            return Err(IpcError::Malformed);
                        }
                        let mut nonce = [0; 32];
                        nonce.copy_from_slice(&request[12..]);
                        Ok(nonce)
                    });
                let valid = admitted.is_ok();
                if send.send(admitted).is_ok() && valid {
                    // EOF, an unexpected byte, or an I/O error all revoke the owner.
                    // This reader is independent of socket polling and startup work.
                    let _ = input.read(&mut [0]);
                }
                signal.store(true, Ordering::Release);
            })
            .map_err(|_| IpcError::Io)?;
        let nonce = receive
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| IpcError::Disconnected)??;
        Ok(Self { nonce, gone })
    }
    fn gone(&self) -> bool {
        self.gone.load(Ordering::Acquire)
    }
}

pub(super) fn serve(args: &[String]) -> Result<(), IpcError> {
    let parent = Parent::admit()?;
    let duration = super::duration(&args[11])?;
    let vault = super::vault(args)?;
    let config = super::session_config(&args[5..9])?;
    let principal = LocalPrincipal {
        account: AccountId::from_bytes(super::hex(&args[9])?),
        device: DeviceId::from_bytes(super::hex(&args[10])?),
    };
    if parent.gone() {
        return Err(IpcError::Disconnected);
    }
    let mut node = NodeServer::bind(config.clone(), principal, NoOperations)?;
    let mut bulk = NodeServer::bind_bulk(config.clone(), principal, NoOperations)?;
    let publication = PublishedRendezvous::publish_with_bulk(
        &vault,
        &args[4],
        config,
        principal,
        node.address(),
        Some(bulk.address()),
    )?;
    node.install_authenticator(publication.record().authenticator(EndpointRole::Control)?)?;
    bulk.install_authenticator(publication.record().authenticator(EndpointRole::Bulk)?)?;
    if !parent.gone() {
        publication
            .record()
            .write_private_parent_handoff(&mut std::io::stdout().lock(), &parent.nonce)?;
    }
    let mut publication = Some(publication);
    let deadline = Instant::now() + Duration::from_millis(duration);
    while !parent.gone() && Instant::now() < deadline {
        node.poll(Instant::now())?;
        bulk.poll(Instant::now())?;
        if bulk.connection_count() != 0
            && let Some(discovery) = publication.take()
        {
            // Owned-start discovery is finished before bulk authentication can
            // complete. A subsequently killed authenticated child leaves no blob
            // to adopt or delete. Both authenticators retain their own token.
            discovery.close()?;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    node.invalidate();
    bulk.invalidate();
    drop(bulk);
    drop(node);
    if let Some(discovery) = publication {
        discovery.close()?;
    }
    // Returning from this finite binary invocation terminates its blocked stdin
    // reader too, including when the configured duration expires before EOF.
    Ok(())
}
