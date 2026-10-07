use nf_contract::identity::*;
use nf_identity::model::Scope;
use std::{collections::BTreeMap, path::PathBuf};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandError {
    Unsupported,
    Missing,
    Invalid,
    Limit,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenOptions {
    pub database: PathBuf,
    pub scope: Scope,
    pub minimum_event: EventSeq,
    pub minimum_store: u64,
    pub minimum_membership: u64,
    pub minimum_term: AuthorityTerm,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerOptions {
    pub vault: PathBuf,
    pub game_save_root: PathBuf,
    pub account: AccountId,
    pub device: DeviceId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunOptions {
    pub open: OpenOptions,
    pub signer: SignerOptions,
    pub active: bool,
    pub period_ms: u64,
    pub duration_seconds: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionSelection {
    Colony {
        site: u8,
        faction: u8,
    },
    Build {
        market: u8,
        kind: nf_world::IndustryKind,
    },
    Relationship {
        faction: u8,
        other: u8,
        score: i32,
    },
    Travel {
        faction: u8,
        destination: u8,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PendingPolicy {
    Pause,
    Resume,
    Cancel,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepOptions {
    pub open: OpenOptions,
    pub signer: SignerOptions,
    pub active: bool,
    pub request: RequestId,
    pub operation: OperationId,
    pub job: JobId,
    pub action: ActionSelection,
    pub pending_policy: PendingPolicy,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOptions {
    pub database: PathBuf,
    pub vault: PathBuf,
    pub game_save_root: PathBuf,
    pub membership_file: PathBuf,
    pub genesis: nf_store::miniature::MiniatureGenesisSpec,
    pub policy: nf_store::miniature::BootstrapPolicy,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Status(OpenOptions),
    Run(RunOptions),
    Step(StepOptions),
    Create(Box<CreateOptions>),
}
/// Parse bounded arguments only; no file/key reads, clocks, role changes or authority claims.
pub fn parse_command(args: &[String]) -> Result<Command, CommandError> {
    if args.len() > 80
        || args.iter().any(|s| s.len() > 4096)
        || args.iter().map(String::len).sum::<usize>() > 16384
    {
        return Err(CommandError::Limit);
    }
    if args.iter().any(|s| s.is_empty() || s.contains('\0')) {
        return Err(CommandError::Invalid);
    }
    let (verb, rest) = args.split_first().ok_or(CommandError::Missing)?;
    let (pairs, trailing) = rest.as_chunks::<2>();
    if !trailing.is_empty() {
        return Err(CommandError::Missing);
    }
    let mut values = BTreeMap::new();
    for [key, value] in pairs {
        if !key.starts_with("--") || values.insert(key.as_str(), value.as_str()).is_some() {
            return Err(CommandError::Invalid);
        }
    }
    let result = match verb.as_str() {
        "status" => Command::Status(open_options(&mut values)?),
        "run" => Command::Run(run_options(&mut values)?),
        "step" => Command::Step(step_options(&mut values)?),
        "create" => Command::Create(Box::new(create_options(&mut values)?)),
        _ => return Err(CommandError::Unsupported),
    };
    if !values.is_empty() {
        return Err(CommandError::Invalid);
    }
    Ok(result)
}
type Values<'a> = BTreeMap<&'a str, &'a str>;
fn take<'a>(values: &mut Values<'a>, name: &str) -> Result<&'a str, CommandError> {
    values.remove(name).ok_or(CommandError::Missing)
}
fn number(value: &str) -> Result<u64, CommandError> {
    if !value.bytes().all(|b| b.is_ascii_digit()) || (value.len() > 1 && value.starts_with('0')) {
        return Err(CommandError::Invalid);
    }
    value.parse().map_err(|_| CommandError::Invalid)
}
fn fixed<const N: usize>(value: &str) -> Result<[u8; N], CommandError> {
    if value.len() != N * 2 {
        return Err(CommandError::Invalid);
    }
    let mut result = [0; N];
    for (slot, pair) in result.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
        let digit = |c: u8| match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(CommandError::Invalid),
        };
        *slot = (digit(pair[0])? << 4) | digit(pair[1])?;
    }
    Ok(result)
}
fn open_options(values: &mut Values<'_>) -> Result<OpenOptions, CommandError> {
    Ok(OpenOptions {
        database: PathBuf::from(take(values, "--database")?),
        scope: Scope {
            universe: UniverseId::from_bytes(fixed(take(values, "--universe")?)?),
            history: HistoryId::from_bytes(fixed(take(values, "--history")?)?),
        },
        minimum_event: EventSeq(number(take(values, "--min-event")?)?),
        minimum_store: number(take(values, "--min-store")?)?,
        minimum_membership: number(take(values, "--min-membership")?)?,
        minimum_term: AuthorityTerm(number(take(values, "--min-term")?)?),
    })
}
fn boolean(value: &str) -> Result<bool, CommandError> {
    match value {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(CommandError::Invalid),
    }
}
fn signer_options(values: &mut Values<'_>) -> Result<SignerOptions, CommandError> {
    if !boolean(take(values, "--claim-authority")?)? {
        return Err(CommandError::Invalid);
    }
    Ok(SignerOptions {
        vault: PathBuf::from(take(values, "--vault")?),
        game_save_root: PathBuf::from(take(values, "--game-save-root")?),
        account: AccountId::from_bytes(fixed(take(values, "--account")?)?),
        device: DeviceId::from_bytes(fixed(take(values, "--device")?)?),
    })
}
fn run_options(values: &mut Values<'_>) -> Result<RunOptions, CommandError> {
    let open = open_options(values)?;
    let signer = signer_options(values)?;
    let active = boolean(take(values, "--active")?)?;
    let period_ms = number(take(values, "--period-ms")?)?;
    let duration_seconds = number(take(values, "--duration-seconds")?)?;
    if !(100..=60000).contains(&period_ms) || !(1..=60).contains(&duration_seconds) {
        return Err(CommandError::Invalid);
    }
    Ok(RunOptions {
        open,
        signer,
        active,
        period_ms,
        duration_seconds,
    })
}
fn ordinal(value: &str, max: u8) -> Result<u8, CommandError> {
    let n = number(value)?;
    if n > u64::from(max) {
        Err(CommandError::Invalid)
    } else {
        Ok(n as u8)
    }
}
fn action(value: &str) -> Result<ActionSelection, CommandError> {
    let parts: Vec<_> = value.split(':').take(6).collect();
    match parts.as_slice() {
        ["colony", site, faction] => Ok(ActionSelection::Colony {
            site: ordinal(site, 5)?,
            faction: ordinal(faction, 2)?,
        }),
        ["build", market, kind] => Ok(ActionSelection::Build {
            market: ordinal(market, 5)?,
            kind: match *kind {
                "1" => nf_world::IndustryKind::Farming,
                "2" => nf_world::IndustryKind::Workshop,
                _ => return Err(CommandError::Invalid),
            },
        }),
        ["relationship", faction, other, score] => {
            let score = score.parse::<i32>().map_err(|_| CommandError::Invalid)?;
            if !(-10000..=10000).contains(&score) {
                return Err(CommandError::Invalid);
            }
            Ok(ActionSelection::Relationship {
                faction: ordinal(faction, 2)?,
                other: ordinal(other, 2)?,
                score,
            })
        }
        ["travel", faction, destination] => Ok(ActionSelection::Travel {
            faction: ordinal(faction, 2)?,
            destination: ordinal(destination, 2)?,
        }),
        _ => Err(CommandError::Invalid),
    }
}
fn step_options(values: &mut Values<'_>) -> Result<StepOptions, CommandError> {
    Ok(StepOptions {
        open: open_options(values)?,
        signer: signer_options(values)?,
        active: boolean(take(values, "--active")?)?,
        request: RequestId::from_bytes(fixed(take(values, "--request")?)?),
        operation: OperationId::from_bytes(fixed(take(values, "--operation")?)?),
        job: JobId::from_bytes(fixed(take(values, "--job")?)?),
        action: action(take(values, "--action")?)?,
        pending_policy: match values.remove("--pending-policy").unwrap_or("pause") {
            "pause" => PendingPolicy::Pause,
            "resume" => PendingPolicy::Resume,
            "cancel" => PendingPolicy::Cancel,
            _ => return Err(CommandError::Invalid),
        },
    })
}
fn peer_hex(value: &str) -> Result<Vec<u8>, CommandError> {
    if value.is_empty() || value.len() > 256 || !value.len().is_multiple_of(2) {
        return Err(CommandError::Invalid);
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| CommandError::Invalid)?;
            Ok(fixed::<1>(text)?[0])
        })
        .collect()
}
fn create_options(values: &mut Values<'_>) -> Result<CreateOptions, CommandError> {
    use nf_store::miniature::{AuthConfig, BootstrapPolicy, MiniatureGenesisSpec};
    let database = PathBuf::from(take(values, "--database")?);
    let vault = PathBuf::from(take(values, "--vault")?);
    let game_save_root = PathBuf::from(take(values, "--game-save-root")?);
    let membership_file = PathBuf::from(take(values, "--membership-file")?);
    let scope = Scope {
        universe: UniverseId::from_bytes(fixed(take(values, "--universe")?)?),
        history: HistoryId::from_bytes(fixed(take(values, "--history")?)?),
    };
    let list: Vec<_> = take(values, "--controllers")?.split(',').take(4).collect();
    let controllers = match list.as_slice() {
        [a, b, c] => [
            AccountId::from_bytes(fixed(a)?),
            AccountId::from_bytes(fixed(b)?),
            AccountId::from_bytes(fixed(c)?),
        ],
        _ => return Err(CommandError::Invalid),
    };
    let genesis = MiniatureGenesisSpec {
        genesis: nf_world::Genesis {
            universe: scope.universe,
            history: scope.history,
            seed: fixed(take(values, "--seed")?)?,
            accounts: controllers,
        },
        aggregate: AggregateId::from_bytes(fixed(take(values, "--aggregate")?)?),
        provider: ProviderId::from_bytes(fixed(take(values, "--provider")?)?),
        provider_aggregate: AggregateId::from_bytes(fixed(take(values, "--provider-aggregate")?)?),
    };
    if genesis.aggregate == genesis.provider_aggregate
        || nf_world::generate(genesis.genesis).is_err()
    {
        return Err(CommandError::Invalid);
    }
    let owner = nf_identity::model::PublicIdentity {
        account: AccountId::from_bytes(fixed(take(values, "--owner-account")?)?),
        account_key: fixed(take(values, "--owner-account-key")?)?,
        device: DeviceId::from_bytes(fixed(take(values, "--owner-device")?)?),
        device_key: fixed(take(values, "--owner-device-key")?)?,
        peer: peer_hex(take(values, "--owner-peer-hex")?)?,
    };
    if !controllers.contains(&owner.account) {
        return Err(CommandError::Invalid);
    }
    let policy = BootstrapPolicy {
        scope,
        membership_digest: fixed(take(values, "--membership-digest")?)?,
        owner,
        controllers,
        auth: AuthConfig::default(),
    };
    Ok(CreateOptions {
        database,
        vault,
        game_save_root,
        membership_file,
        genesis,
        policy,
    })
}
/// Fixed bounded foreground usage; never includes local keys, paths or generated identifiers.
pub fn command_help() -> &'static str {
    "nf-world-driver create|status|step|run\n\
status: --database PATH --universe HEX16 --history HEX16 --min-event N --min-store N --min-membership N --min-term N\n\
step/run: status options plus --vault PATH --game-save-root PATH --account HEX16 --device HEX16 --claim-authority yes --active yes|no\n\
step: --request HEX16 --operation HEX16 --job HEX16 --action colony:SITE:FACTION|build:MARKET:1|build:MARKET:2|relationship:FACTION:OTHER:SCORE|travel:FACTION:SYSTEM\n\
step: optional --pending-policy pause|resume|cancel (default pause); only an exact retained binding can resume or cancel. Resume requires every original actor's existing key.\n\
run: --period-ms 100..60000 --duration-seconds 1..60; retained work pauses. Inactive commands do not claim or advance. The deadline stops new attempts; admitted synchronous commits finish normally.\n\
create: --database PATH --vault PATH --game-save-root PATH --membership-file PATH --membership-digest HEX32 --universe HEX16 --history HEX16 --seed HEX32 --controllers HEX16,HEX16,HEX16 --aggregate HEX16 --provider HEX16 --provider-aggregate HEX16 --owner-account HEX16 --owner-account-key HEX32 --owner-device HEX16 --owner-device-key HEX32 --owner-peer-hex HEX\n\
Create imports explicitly pinned public membership and proves possession of an existing private key. It does not create keys or roles. Ordinals select immutable genesis entities; request/operation/job IDs are explicit.\n"
}
