//! Foreground retained observation; no strategic submission command is exposed.
use libp2p::Multiaddr;
use nf_identity::private_storage::PrivateVault;
use nf_store::Store;
use nf_transport::{
    PeerError,
    portal::{PortalClient, PortalEvent, PortalServer, WatchEvent},
    portal_config::{MAX_PATH_BYTES, PortalArguments, PortalConfig, PortalMode, read_config},
    receipt::{ReceiptPhase, ReceiptUnsupportedReason},
    receipt_effects::{AdmittedReceipt, ReceiptConfig, ReceiptRepo, book::BookAnchor},
};
use std::{
    ffi::OsString,
    fs::{File, Metadata},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const START_BYTES: usize = 16_384;
const PREPARATION_WAIT: Duration = Duration::from_secs(30);

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("NF_PORTAL_ERROR {error:?}");
        std::process::exit(1);
    }
}
fn arguments() -> Result<(PortalArguments, Option<PathBuf>), PeerError> {
    let mut args = Vec::with_capacity(8);
    for argument in std::env::args_os().skip(1) {
        if args.len() == 8 || argument.len() > MAX_PATH_BYTES {
            return Err(PeerError::Limit);
        }
        args.push(argument);
    }
    let staged = match args.first().and_then(|value| value.to_str()) {
        Some("watch-staged") => Some(("watch", 8)),
        Some("serve-staged") => Some(("serve", 7)),
        _ => None,
    };
    let start = if let Some((mode, length)) = staged {
        if args.len() != length {
            return Err(PeerError::Malformed);
        }
        let path = args.pop().ok_or(PeerError::Malformed)?;
        let text = path.to_str().ok_or(PeerError::Malformed)?;
        if text.is_empty()
            || text.len() > MAX_PATH_BYTES
            || text.bytes().any(|byte| matches!(byte, 0 | b'\r' | b'\n'))
        {
            return Err(PeerError::Malformed);
        }
        args[0] = OsString::from(mode);
        let path = PathBuf::from(path);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => return Err(PeerError::Backpressure),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(PeerError::Storage),
        }
        Some(path)
    } else {
        None
    };
    Ok((PortalArguments::parse(&args)?, start))
}
fn repository(args: &PortalArguments, config: &PortalConfig) -> Result<ReceiptRepo, PeerError> {
    let vault = PrivateVault::open(&args.vault, &args.save_root).map_err(|_| PeerError::Storage)?;
    let store =
        Store::open_existing(&args.database, config.local).map_err(|_| PeerError::Storage)?;
    let mut anchors = Vec::with_capacity(config.originals.len());
    if config.mode == PortalMode::Watch {
        for row in &config.originals {
            let (generation, digest) = row.head.ok_or(PeerError::Malformed)?;
            anchors.push(BookAnchor {
                slot: row.slot,
                generation,
                digest,
                original: row.original.clone(),
                ruleset: config.ruleset,
                content: config.content,
                minimum: row.minimum,
            });
        }
    }
    ReceiptRepo::open_owned(
        store,
        vault,
        ReceiptConfig {
            scope: config.scope(),
            ruleset: config.ruleset,
            content: config.content,
            local_account: config.local_account,
            local_device: config.local_device,
            server_pin: config.server,
            minimum_membership: config.protected_membership()?,
        },
        &anchors,
    )
}
fn flush() -> Result<(), PeerError> {
    std::io::stdout().flush().map_err(|_| PeerError::Storage)
}
async fn run() -> Result<(), PeerError> {
    let (args, start) = arguments()?;
    let config = read_config(&args.config, args.mode)?;
    let mut repo = repository(&args, &config)?;
    if args.mode == PortalMode::InitializeBook {
        let originals: Vec<_> = config
            .originals
            .iter()
            .map(|row| (row.slot, row.original.clone(), row.minimum))
            .collect();
        for anchor in repo.initialize_originals(&originals)? {
            let digest: String = anchor
                .digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            println!(
                "NF_PORTAL_BOOK_ANCHOR {} {} {digest}",
                anchor.slot, anchor.generation
            );
        }
        return flush();
    }
    let duration = args.duration.ok_or(PeerError::Malformed)?;
    if args.mode == PortalMode::Serve {
        if let Some(path) = start {
            println!("NF_PORTAL_PREPARED");
            flush()?;
            let addresses = wait_start(&path).await?;
            if config.addresses.as_ref() != Some(&addresses) {
                return Err(PeerError::Unauthorized);
            }
        }
        let mut owner = PortalServer::new(repo, config, duration)?;
        loop {
            for event in owner.next_round().await?.events.into_iter().flatten() {
                match event {
                    PortalEvent::Ready { addresses } => {
                        println!("NF_PORTAL_READY {} {} {}", addresses[0], addresses[1], addresses[2]);
                        flush()?;
                    }
                    PortalEvent::Notification(nf_transport::notification_effects::NotifyLaneEvent::SubscriptionResponseSent {
                        peer, connection, request,
                    }) => {
                        println!("NF_PORTAL_SUBSCRIPTION_RESPONSE_SENT {peer} {connection:?} {request:?}");
                        flush()?;
                    }
                    PortalEvent::Ended => {
                        println!("NF_PORTAL_ENDED");
                        return flush();
                    }
                    _ => {}
                }
            }
        }
    }
    let slot = args.slot.ok_or(PeerError::Malformed)?;
    let mut owner = if let Some(path) = start {
        let prepared = PortalClient::prepare(repo, config, slot)?;
        println!("NF_PORTAL_PREPARED");
        flush()?;
        // This bounded operator wait belongs to inert preparation; no owner exists yet.
        let addresses = wait_start(&path).await?;
        prepared.start(addresses, duration)?
    } else {
        PortalClient::new(repo, config, slot, duration)?
    };
    loop {
        for event in owner.next_round().await?.events.into_iter().flatten() {
            match event {
                WatchEvent::ReceiptAuthenticated { peer } => {
                    println!("NF_PORTAL_RECEIPT_AUTHENTICATED {peer}");
                    flush()?;
                }
                WatchEvent::Subscribed => {
                    println!("NF_PORTAL_SUBSCRIBED");
                    flush()?;
                }
                WatchEvent::Status(outcome) => {
                    match outcome {
                        AdmittedReceipt::Status(status) => {
                            let (phase, sequence) = match status.phase {
                                ReceiptPhase::Unknown => ("UNKNOWN", None),
                                ReceiptPhase::Pending => ("PENDING", None),
                                ReceiptPhase::Rejected { sequence } => {
                                    ("REJECTED", Some(sequence.0))
                                }
                                ReceiptPhase::Committed { sequence } => {
                                    ("COMMITTED", Some(sequence.0))
                                }
                            };
                            let hex = |bytes: &[u8]| -> String {
                                bytes.iter().map(|byte| format!("{byte:02x}")).collect()
                            };
                            let request = hex(status.request.as_bytes());
                            let operation = hex(status.operation.as_bytes());
                            let binding = hex(&status.binding);
                            let terminal = sequence
                                .map(|value| format!(" {value}"))
                                .unwrap_or_default();
                            println!(
                                "NF_PORTAL_STATUS {phase} {request} {operation} {binding} {} {} {}{terminal}",
                                status.current.event.0,
                                status.current.store_revision,
                                status.current.membership_revision
                            );
                        }
                        AdmittedReceipt::Unsupported(reason) => {
                            let reason = match reason {
                                ReceiptUnsupportedReason::BindingConflict => "BINDING_CONFLICT",
                                ReceiptUnsupportedReason::SourceBelowKnownMinima => {
                                    "SOURCE_BELOW_KNOWN_MINIMA"
                                }
                            };
                            println!("NF_PORTAL_UNSUPPORTED {reason}");
                        }
                    }
                    flush()?;
                }
                WatchEvent::Ended => {
                    println!("NF_PORTAL_ENDED");
                    return flush();
                }
            }
        }
    }
}
async fn wait_start(path: &Path) -> Result<[Multiaddr; 3], PeerError> {
    let until = Instant::now()
        .checked_add(PREPARATION_WAIT)
        .ok_or(PeerError::Limit)?;
    loop {
        if Instant::now() >= until {
            return Err(PeerError::Offline);
        }
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => {
                regular(&metadata)?;
                let result = read_start(path)?;
                if Instant::now() >= until {
                    return Err(PeerError::Offline);
                }
                return Ok(result);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(PeerError::Storage),
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
fn regular(metadata: &Metadata) -> Result<(), PeerError> {
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(PeerError::Storage);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(PeerError::Storage);
        }
    }
    Ok(())
}
fn read_start(path: &Path) -> Result<[Multiaddr; 3], PeerError> {
    let mut file = File::open(path).map_err(|_| PeerError::Storage)?;
    let before = file.metadata().map_err(|_| PeerError::Storage)?;
    regular(&before)?;
    if before.len() > START_BYTES as u64 {
        return Err(PeerError::Limit);
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    Read::by_ref(&mut file)
        .take(START_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PeerError::Storage)?;
    let after = file.metadata().map_err(|_| PeerError::Storage)?;
    regular(&after)?;
    if bytes.len() > START_BYTES {
        return Err(PeerError::Limit);
    }
    if before.len() != after.len() || after.len() != bytes.len() as u64 {
        return Err(PeerError::Storage);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| PeerError::Malformed)?;
    let mut lines = text.split_terminator('\n');
    if !text.ends_with('\n') || lines.next() != Some("NF-PORTAL-START-1") {
        return Err(PeerError::Malformed);
    }
    let addresses = [lines.next(), lines.next(), lines.next()].map(|line| {
        line.ok_or(PeerError::Malformed)?
            .parse()
            .map_err(|_| PeerError::Malformed)
    });
    let [first, second, third] = addresses;
    if lines.next() != Some("END") || lines.next().is_some() {
        return Err(PeerError::Malformed);
    }
    Ok([first?, second?, third?])
}
