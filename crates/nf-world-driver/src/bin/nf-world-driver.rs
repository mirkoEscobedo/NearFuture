use nf_world_driver::{Command, Driver, DriverError, parse_command};
use std::{io::Write, process::ExitCode};
fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("driver: {error:?}");
            ExitCode::FAILURE
        }
    }
}
fn execute() -> Result<(), DriverError> {
    let mut args = Vec::new();
    let mut bytes = 0usize;
    for argument in std::env::args_os().skip(1).take(81) {
        let argument = argument
            .into_string()
            .map_err(|_| DriverError::InvalidCommand)?;
        bytes = bytes
            .checked_add(argument.len())
            .ok_or(DriverError::Limit)?;
        if args.len() >= 80 || argument.len() > 4096 || bytes > 16384 {
            return Err(DriverError::Limit);
        }
        args.push(argument);
    }
    if args == ["--help"] {
        return std::io::stdout()
            .lock()
            .write_all(nf_world_driver::command_help().as_bytes())
            .map_err(|_| DriverError::Io);
    }
    let command = parse_command(&args).map_err(|_| DriverError::InvalidCommand)?;
    let snapshot = match command {
        Command::Create(options) => Driver::create(*options)?.status()?,
        Command::Status(options) => Driver::read_status(options)?,
        Command::Step(options) => {
            use nf_store::miniature::{MiniatureCancelCause, MiniatureRequestStatus};
            use nf_world_driver::{ActionRequest, PendingPolicy};
            let mut driver = Driver::open(options.open, options.signer)?;
            let action = ActionRequest {
                request: options.request,
                operation: options.operation,
                job: options.job,
                action: options.action,
            };
            let original = driver.query_action(&action)?;
            if options.active
                && !matches!(
                    original.as_ref().map(|o| o.status),
                    Some(MiniatureRequestStatus::Committed { .. })
                )
            {
                if original.is_some() {
                    match options.pending_policy {
                        PendingPolicy::Pause => (),
                        PendingPolicy::Resume => {
                            driver.claim_authority()?;
                            driver.resume_pending()?;
                            driver.advance_pending(true)?;
                        }
                        PendingPolicy::Cancel => {
                            driver.claim_authority()?;
                            driver.cancel_pending(MiniatureCancelCause::Cancelled)?;
                        }
                    }
                } else {
                    if driver.status()?.pending.is_some() {
                        return Err(nf_store::StoreError::InvalidTransition.into());
                    }
                    driver.claim_authority()?;
                    driver.prepare_action(&action)?;
                    driver.advance_pending(true)?;
                }
            }
            driver.status()?
        }
        Command::Run(options) => {
            let mut driver = Driver::open(options.open, options.signer)?;
            driver.run_as_owner(options.period_ms, options.duration_seconds, options.active)?;
            driver.status()?
        }
    };
    let g = snapshot.world.component().genesis();
    let m = snapshot.world.metadata();
    let hash = nf_kernel::miniature::miniature_state_hash(&snapshot.world)
        .map_err(|e| DriverError::Store(nf_store::miniature::MiniatureStoreError::Kernel(e)))?;
    let text = format!(
        "universe={}\nhistory={}\ntick={}\nevent_sequence={}\nstore_revision={}\nmembership_revision={}\nauthority_term={}\npending_jobs={}\npending={}\nstate_hash={}\n",
        hex(g.universe.as_bytes()),
        hex(g.history.as_bytes()),
        m.tick.0,
        m.event_sequence.0,
        snapshot.known.storage.store_revision,
        snapshot
            .known
            .storage
            .membership_revision
            .map_or("none".into(), |n| n.to_string()),
        snapshot.authority.map_or(0, |a| a.term.0),
        snapshot.pending.as_ref().map_or(0, |p| p.intents().len()),
        if snapshot.pending.is_some() {
            "yes"
        } else {
            "no"
        },
        hex(&hash)
    );
    std::io::stdout()
        .lock()
        .write_all(text.as_bytes())
        .map_err(|_| DriverError::Io)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|n| format!("{n:02x}")).collect()
}
