use super::{bytes::{Encoder, Reader, hash, same}, catalog_command::Command,
    error::{ManagedError, Result}, foot::StorageFoot, proof, prospective::Prospective, challenge::Owned,
    receipt::Receipt, stamp::OwnerStamp, state::QuiescePolicy, summary::Summary};
use nf_identity::model::DeviceProof;

#[derive(Clone)]
pub(super) enum Payload {
    Owner { adopt: bool, transcript: Vec<u8>, proof: DeviceProof, stamp: OwnerStamp, foot: StorageFoot },
    Pending(Summary),
    Attempt(Summary),
    Outcome { summary: Summary, receipt: Receipt, disposition: u8 },
    Quiesce { foot: StorageFoot, policy: QuiescePolicy, cutoff: u64, transcript: Vec<u8>, proof: DeviceProof },
    Register { registry: [u8; 32], transcript: Vec<u8>, proof: DeviceProof },
    Select { retirement: [u8; 32], source: StorageFoot, target: StorageFoot,
        binary: [u8; 32], transcript: Vec<u8>, proof: DeviceProof },
    Abort { summary: Summary, receipt: Receipt, disposition: u8 },
    Checkpoint { revision: u64, state: [u8; 32], head: [u8; 32], rows: [u8; 32] },
}
impl Payload {
    pub fn kind(&self) -> u32 {
        match self {
            Self::Owner { .. } => 1, Self::Pending(_) => 2, Self::Attempt(_) => 3,
            Self::Outcome { .. } => 4, Self::Quiesce { .. } | Self::Register { .. } => 5,
            Self::Select { .. } => 6, Self::Abort { .. } => 7, Self::Checkpoint { .. } => 8,
        }
    }
    pub fn encode(&self, delta_sha: [u8; 32]) -> Result<Vec<u8>> {
        let mut w = Encoder::plain(131_072);
        match self {
            Self::Owner { adopt, transcript, proof, stamp, foot } => {
                let p = Prospective::decode(transcript)?;
                if p.purpose != if *adopt { 9 } else { 10 } || p.candidate != *stamp || p.foot != *foot {
                    return Err(ManagedError::Corrupt);
                }
                prospective_proof(&p, transcript, proof)?;
                w.byte(if *adopt { 1 } else { 2 })?; signed(&mut w, transcript, proof)?;
                w.blob(stamp.canonical_bytes(), 197)?; w.blob(foot.canonical_bytes(), 2048)?;
            },
            Self::Pending(summary) | Self::Attempt(summary) => {
                if summary.stage != if matches!(self, Self::Attempt(_)) { 2 } else { 1 } {
                    return Err(ManagedError::Corrupt);
                }
                w.blob(&summary.encode()?, 16_384)?;
            },
            Self::Outcome { summary, receipt, disposition } | Self::Abort { summary, receipt, disposition } => {
                let abort = matches!(self, Self::Abort { .. });
                if !matches!(disposition, 1 | 2) || receipt.sequence != summary.sequence
                    || receipt.operation != summary.operation || receipt.purpose != summary.purpose
                    || receipt.action != summary.action
                    || receipt.foot != if abort { summary.pre.clone() } else { summary.post.clone() }
                    || (abort && receipt.result != 2)
                    || (!abort && receipt.result != if summary.purpose == 8 { 3 } else { 1 })
                    || (abort && ((*disposition == 2) != (summary.purpose == 8)))
                    || (abort && summary.stage != 1)
                    || (!abort && summary.purpose == 8 && (summary.stage != 2 || *disposition != 2))
                { return Err(ManagedError::Corrupt); }
                w.blob(&summary.encode()?, 16_384)?; w.blob(&receipt.encode()?, 4096)?;
                w.byte(*disposition)?;
            },
            Self::Quiesce { foot, policy, cutoff, transcript, proof } => {
                let unsigned = Command::Quiesce { foot: foot.clone(), policy: *policy, cutoff: *cutoff, delta: delta_sha }.encode()?;
                owned_proof(transcript, proof, 11, &unsigned, foot)?;
                w.byte(1)?; w.blob(foot.canonical_bytes(), 2048)?; w.byte(policy.tag())?;
                w.u64(*cutoff)?; signed(&mut w, transcript, proof)?;
            },
            Self::Register { registry, transcript, proof } => {
                let unsigned = Command::Register { registry: *registry, delta: delta_sha }.encode()?;
                let auth = Owned::decode(transcript)?;
                if auth.purpose != 12 || auth.payload != hash(&unsigned) { return Err(ManagedError::Corrupt); }
                common_proof(transcript, proof, &auth.stamp)?;
                w.byte(2)?; w.raw(registry)?; signed(&mut w, transcript, proof)?;
            },
            Self::Select { retirement, source, target, binary, transcript, proof } => {
                let p = Prospective::decode(transcript)?;
                if p.purpose != 13 || p.foot != *source || source.profile != 5 || target.profile != 4 {
                    return Err(ManagedError::Corrupt);
                }
                prospective_proof(&p, transcript, proof)?;
                w.raw(retirement)?; w.blob(source.canonical_bytes(), 2048)?;
                w.blob(target.canonical_bytes(), 2048)?; w.raw(binary)?;
                signed(&mut w, transcript, proof)?;
            },
            Self::Checkpoint { revision, state, head, rows } => {
                if *revision == 0 { return Err(ManagedError::Corrupt); }
                w.u64(*revision)?; w.raw(state)?; w.raw(head)?; w.raw(rows)?;
            },
        }
        Ok(w.finish())
    }
    pub fn decode(kind: u32, bytes: &[u8], delta_sha: [u8; 32]) -> Result<Self> {
        let mut r = Reader::plain(bytes, 131_072)?;
        let value = match kind {
            1 => {
                let adopt = match r.byte()? { 1 => true, 2 => false, _ => return Err(ManagedError::Corrupt) };
                let (transcript, proof) = read_signed(&mut r)?;
                Self::Owner { adopt, transcript, proof, stamp: OwnerStamp::decode(r.blob(197)?)?,
                    foot: StorageFoot::decode(r.blob(2048)?)? }
            },
            2 => Self::Pending(Summary::decode(r.blob(16_384)?)?),
            3 => Self::Attempt(Summary::decode(r.blob(16_384)?)?),
            4 | 7 => {
                let summary = Summary::decode(r.blob(16_384)?)?;
                let receipt = Receipt::decode(r.blob(4096)?)?; let disposition = r.byte()?;
                if kind == 4 { Self::Outcome { summary, receipt, disposition } }
                else { Self::Abort { summary, receipt, disposition } }
            },
            5 => match r.byte()? {
                1 => {
                    let foot = StorageFoot::decode(r.blob(2048)?)?;
                    let policy = match r.byte()? { 1 => QuiescePolicy::Drain, 2 => QuiescePolicy::RequireAlreadyDrained,
                        _ => return Err(ManagedError::Corrupt) };
                    let cutoff = r.u64()?; let (transcript, proof) = read_signed(&mut r)?;
                    Self::Quiesce { foot, policy, cutoff, transcript, proof }
                },
                2 => { let registry = r.fixed()?; let (transcript, proof) = read_signed(&mut r)?;
                    Self::Register { registry, transcript, proof } },
                _ => return Err(ManagedError::Corrupt),
            },
            6 => {
                let retirement = r.fixed()?; let source = StorageFoot::decode(r.blob(2048)?)?;
                let target = StorageFoot::decode(r.blob(2048)?)?; let binary = r.fixed()?;
                let (transcript, proof) = read_signed(&mut r)?;
                Self::Select { retirement, source, target, binary, transcript, proof }
            },
            8 => Self::Checkpoint { revision: r.u64()?, state: r.fixed()?, head: r.fixed()?, rows: r.fixed()? },
            _ => return Err(ManagedError::Corrupt),
        };
        r.finish()?; same(bytes, &value.encode(delta_sha)?)?; Ok(value)
    }
}
fn signed(w: &mut Encoder, transcript: &[u8], value: &DeviceProof) -> Result<()> {
    w.blob(transcript, 4096)?; w.blob(&proof::encode(value)?, 300)
}
fn read_signed(r: &mut Reader<'_>) -> Result<(Vec<u8>, DeviceProof)> {
    Ok((r.blob(4096)?.to_vec(), proof::decode(r.blob(300)?)?))
}
fn common_proof(transcript: &[u8], proof: &DeviceProof, stamp: &OwnerStamp) -> Result<()> {
    if proof.challenge != hash(transcript) || proof.account != stamp.account || proof.device != stamp.device
        || Some(proof.frontier) != stamp.membership_revision { return Err(ManagedError::Corrupt); }
    Ok(())
}
fn prospective_proof(p: &Prospective, transcript: &[u8], proof: &DeviceProof) -> Result<()> {
    common_proof(transcript, proof, &p.candidate)?;
    if proof.scope != p.scope || proof.peer != p.peer { return Err(ManagedError::Corrupt); }
    Ok(())
}
fn owned_proof(transcript: &[u8], proof: &DeviceProof, purpose: u32, unsigned: &[u8], foot: &StorageFoot) -> Result<()> {
    let auth = Owned::decode(transcript)?;
    if auth.purpose != purpose || auth.payload != hash(unsigned) || proof.scope != foot.known.scope
        || auth.revision != foot.known.store_revision || auth.state != foot.state || auth.head != foot.head
    { return Err(ManagedError::Corrupt); }
    common_proof(transcript, proof, &auth.stamp)
}
