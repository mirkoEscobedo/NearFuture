//! Explicit child-only composition. Private pipes own the lease; no persistent discovery.
use nf_sidecar::{
    protocol::Token,
    server::{Server, fresh_session},
};
use std::{
    io::{IsTerminal, Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

/// The process main/lease owner never blocks in stdout, including broken parents.
/// Returning from this binary terminates a blocked pipe writer as well as the reader.
struct ParentOutput(mpsc::SyncSender<(Vec<u8>, mpsc::SyncSender<bool>)>);
impl ParentOutput {
    fn new() -> Result<Self, &'static str> {
        let (tx, rx) = mpsc::sync_channel::<(Vec<u8>, mpsc::SyncSender<bool>)>(1);
        std::thread::Builder::new()
            .name("nf-module-output".into())
            .spawn(move || {
                let mut output = std::io::stdout().lock();
                while let Ok((bytes, done)) = rx.recv() {
                    let success = output
                        .write_all(&bytes)
                        .and_then(|()| output.flush())
                        .is_ok();
                    let _ = done.try_send(success);
                    if !success {
                        break;
                    }
                }
            })
            .map_err(|_| "parent-output-unavailable")?;
        Ok(Self(tx))
    }
    fn send(
        &self,
        bytes: Vec<u8>,
        deadline: Instant,
        gone: &AtomicBool,
    ) -> Result<(), &'static str> {
        if bytes.len() > 34 {
            return Err("parent-output-limit");
        }
        let (tx, rx) = mpsc::sync_channel(1);
        self.0
            .try_send((bytes, tx))
            .map_err(|_| "parent-output-unavailable")?;
        loop {
            if gone.load(Ordering::Acquire) || Instant::now() >= deadline {
                return Err("parent-output-unavailable");
            }
            match rx.recv_timeout(Duration::from_millis(2)) {
                Ok(true) => return Ok(()),
                Ok(false) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("parent-output-unavailable");
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

struct Parent {
    gone: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    applied: Arc<AtomicU64>,
    token: Token,
    lifetime: u32,
    capabilities: u32,
}
impl Parent {
    fn admit() -> Result<Self, &'static str> {
        if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
            return Err("private-pipes-required");
        }
        let gone = Arc::new(AtomicBool::new(false));
        let generation = Arc::new(AtomicU64::new(0));
        let applied = Arc::new(AtomicU64::new(0));
        let signal = Arc::clone(&gone);
        let updates = Arc::clone(&generation);
        let confirmed = Arc::clone(&applied);
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("nf-module-parent".into())
            .spawn(move || {
                let mut input = std::io::stdin().lock();
                let mut b = Zeroizing::new([0; 48]);
                if input.read_exact(b.as_mut()).is_err() || &b[..8] != b"NFMPAR01" {
                    signal.store(true, Ordering::Release);
                    return;
                }
                let token =
                    Zeroizing::new(<[u8; 32]>::try_from(&b[8..40]).expect("fixed bootstrap"));
                let lifetime = u32::from_be_bytes(b[40..44].try_into().expect("fixed bootstrap"));
                let capabilities =
                    u32::from_be_bytes(b[44..48].try_into().expect("fixed bootstrap"));
                if *token == [0; 32]
                    || !(100..=60000).contains(&lifetime)
                    || capabilities & !nf_modules::COMPILED_CAPABILITIES != 0
                {
                    signal.store(true, Ordering::Release);
                    return;
                }
                if tx.send((token, lifetime, capabilities)).is_ok() {
                    let mut command = [0; 9];
                    let mut bindings = 0;
                    while input.read_exact(&mut command).is_ok() {
                        let next =
                            u64::from_be_bytes(command[1..].try_into().expect("fixed command"));
                        if command[0] != 1
                            || bindings == 8
                            || next <= updates.load(Ordering::Acquire)
                            || confirmed.load(Ordering::Acquire) != updates.load(Ordering::Acquire)
                        {
                            break;
                        }
                        bindings += 1;
                        updates.store(next, Ordering::Release);
                    }
                }
                signal.store(true, Ordering::Release);
            })
            .map_err(|_| "parent-reader-unavailable")?;
        let (token, lifetime, capabilities) = rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "bootstrap-unavailable")?;
        Ok(Self {
            gone,
            generation,
            applied,
            token,
            lifetime,
            capabilities,
        })
    }
}
fn run() -> Result<(), &'static str> {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["serve-parent"] {
        return Err("usage-serve-parent");
    }
    let parent = Parent::admit()?;
    let host = nf_modules::Host::new(
        fresh_session().map_err(|_| "random-unavailable")?,
        parent.capabilities & 1 != 0,
    )
    .and_then(|host| host.with_snapshot(parent.capabilities & nf_modules::SNAPSHOT_CAPABILITY != 0))
    .map_err(|_| "host-unavailable")?;
    let mut server = Server::bind(parent.token, host).map_err(|_| "listener-unavailable")?;
    if parent.gone.load(Ordering::Acquire) {
        return Err("parent-unavailable");
    }
    let mut ready = Vec::with_capacity(34);
    ready.extend_from_slice(b"NFMRDY01");
    ready.extend_from_slice(&server.address().port().to_be_bytes());
    ready.extend_from_slice(&server.session());
    ready.extend_from_slice(&0u64.to_be_bytes());
    let end = Instant::now() + Duration::from_millis(u64::from(parent.lifetime));
    let output = ParentOutput::new()?;
    output.send(
        ready,
        end.min(Instant::now() + Duration::from_secs(2)),
        &parent.gone,
    )?;
    let mut generation = 0;
    while !parent.gone.load(Ordering::Acquire) && Instant::now() < end {
        let current = parent.generation.load(Ordering::Acquire);
        if current != generation {
            server
                .bind_campaign(current)
                .map_err(|_| "binding-unavailable")?;
            generation = current;
            parent.applied.store(current, Ordering::Release);
            let mut ack = [0; 32];
            ack[..8].copy_from_slice(b"NFMBND01");
            ack[8..24].copy_from_slice(&server.session());
            ack[24..].copy_from_slice(&current.to_be_bytes());
            output.send(
                ack.to_vec(),
                end.min(Instant::now() + Duration::from_secs(2)),
                &parent.gone,
            )?;
        }
        server
            .poll(Instant::now(), &parent.gone, &parent.generation)
            .map_err(|_| "transport-unavailable")?;
        std::thread::sleep(Duration::from_millis(1));
    }
    server.stop();
    // Process exit also closes the single blocked stdin reader on finite expiry.
    Ok(())
}
fn main() {
    if let Err(reason) = run() {
        eprintln!("nf-sidecar: {reason}");
        std::process::exit(1);
    }
}
