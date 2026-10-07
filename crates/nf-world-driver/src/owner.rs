use crate::{CreateOptions, SignerError, VaultSigner};
use nf_store::miniature::{MiniatureSnapshot, MiniatureStore, MiniatureStoreError};
use std::{fs::File, io::Read, path::Path};
use zeroize::Zeroizing;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriverError {
    Unsupported,
    InvalidCommand,
    Inactive,
    WindowExpired,
    MissingSigner,
    Io,
    Limit,
    Signer(SignerError),
    Store(MiniatureStoreError),
}
impl From<SignerError> for DriverError {
    fn from(error: SignerError) -> Self {
        Self::Signer(error)
    }
}
impl From<MiniatureStoreError> for DriverError {
    fn from(error: MiniatureStoreError) -> Self {
        Self::Store(error)
    }
}
/// One sole local Store owner; no duplicate World, authority or strategic clock.
pub struct Driver {
    pub(super) store: MiniatureStore,
    pub(super) signer: VaultSigner,
    pub(super) additional_signers: Vec<VaultSigner>,
    pub(super) run_deadline: Option<std::time::Instant>,
}
impl Driver {
    pub fn claim_authority(&mut self) -> Result<nf_store::Accepted, DriverError> {
        use nf_store::miniature::ChallengeRequest;
        let public = self.signer.public();
        let request = ChallengeRequest::Claim {
            actor: public.account,
            device: public.device,
        };
        let attempt = self.proof(request)?;
        self.check_budget()?;
        Ok(self.store.claim_authority_with_hook(
            attempt,
            &mut crate::run_budget::deadline_hook(self.run_deadline),
        )?)
    }
    pub(super) fn proof(
        &mut self,
        request: nf_store::miniature::ChallengeRequest<'_>,
    ) -> Result<nf_store::miniature::ProofAttempt, DriverError> {
        self.check_budget()?;
        let issued = self.store.issue_challenge(request)?;
        let signer = self
            .signer_for(issued.template.account, issued.template.device)
            .ok_or(DriverError::MissingSigner)?;
        let proof = signer.sign(&issued.template)?;
        self.check_budget()?;
        Ok(nf_store::miniature::ProofAttempt {
            ticket: issued.ticket,
            proof,
        })
    }
    pub fn open(
        options: crate::OpenOptions,
        local: crate::SignerOptions,
    ) -> Result<Self, DriverError> {
        let store = open_store(&options)?;
        let identity = store.current_identity(local.account, local.device)?;
        let signer = VaultSigner::open(
            &local.vault,
            &local.game_save_root,
            options.scope,
            &identity,
        )?;
        Ok(Self {
            store,
            signer,
            additional_signers: Vec::new(),
            run_deadline: None,
        })
    }
    pub fn read_status(options: crate::OpenOptions) -> Result<MiniatureSnapshot, DriverError> {
        Ok(open_store(&options)?.snapshot()?)
    }
    pub fn create(options: CreateOptions) -> Result<Self, DriverError> {
        let membership = read_membership(&options.membership_file)?;
        let mut signer = VaultSigner::open(
            &options.vault,
            &options.game_save_root,
            options.policy.scope,
            &options.policy.owner,
        )?;
        let store = MiniatureStore::create(
            options.database,
            options.genesis,
            &membership,
            options.policy,
            &mut signer,
        )?;
        Ok(Self {
            store,
            signer,
            additional_signers: Vec::new(),
            run_deadline: None,
        })
    }
    /// Observing a guarded Store snapshot never claims authority or advances a tick.
    pub fn status(&self) -> Result<MiniatureSnapshot, DriverError> {
        Ok(self.store.snapshot()?)
    }
    pub fn local_identity(&self) -> &nf_identity::model::PublicIdentity {
        self.signer.public()
    }
}
fn read_membership(path: &Path) -> Result<Zeroizing<Vec<u8>>, DriverError> {
    const MAX: u64 = 262_144;
    let file = File::open(path).map_err(|_| DriverError::Io)?;
    if !file.metadata().map_err(|_| DriverError::Io)?.is_file() {
        return Err(DriverError::Io);
    }
    if file.metadata().map_err(|_| DriverError::Io)?.len() > MAX {
        return Err(DriverError::Limit);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DriverError::Io)?;
    if bytes.len() > MAX as usize {
        return Err(DriverError::Limit);
    }
    Ok(bytes)
}
fn open_store(options: &crate::OpenOptions) -> Result<MiniatureStore, DriverError> {
    use nf_store::{
        KnownFrontiers,
        miniature::{AuthConfig, MiniatureKnownFrontiers},
    };
    let known = MiniatureKnownFrontiers {
        storage: KnownFrontiers {
            scope: options.scope,
            event_sequence: options.minimum_event,
            store_revision: options.minimum_store,
            membership_revision: Some(options.minimum_membership),
        },
        minimum_authority_term: options.minimum_term,
    };
    Ok(MiniatureStore::open_existing(
        &options.database,
        known,
        AuthConfig::default(),
    )?)
}
impl From<nf_store::StoreError> for DriverError {
    fn from(error: nf_store::StoreError) -> Self {
        Self::Store(MiniatureStoreError::Storage(error))
    }
}
