use crate::{ActionRequest, Driver, DriverError, SignerOptions, VaultSigner, resolve_action};
use nf_contract::identity::*;
use nf_kernel::miniature::MiniatureIntent;
use nf_store::{
    StoreError,
    miniature::{ChallengeRequest, MiniatureStoreError},
};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorRequest {
    pub account: AccountId,
    pub device: DeviceId,
    pub action: ActionRequest,
}
impl Driver {
    /// Select an existing current controller key. A different device replaces that account's signer
    /// only after successful current-policy/key validation; pending old-device intents stay immutable.
    pub fn configure_signer(&mut self, options: SignerOptions) -> Result<(), DriverError> {
        if !self
            .store
            .world()
            .component()
            .genesis()
            .accounts
            .contains(&options.account)
        {
            return Err(MiniatureStoreError::Unauthorized.into());
        }
        let public = self
            .store
            .current_identity(options.account, options.device)?;
        if self
            .signer_for(options.account, options.device)
            .is_some_and(|s| s.public() == &public)
        {
            return Err(DriverError::InvalidCommand);
        }
        let slot = self
            .additional_signers
            .iter()
            .position(|s| s.public().account == options.account);
        if options.account != self.signer.public().account
            && slot.is_none()
            && self.additional_signers.len() >= 2
        {
            return Err(DriverError::Limit);
        }
        let public = self
            .store
            .current_identity(options.account, options.device)?;
        let signer = VaultSigner::open(
            &options.vault,
            &options.game_save_root,
            self.signer.scope(),
            &public,
        )?;
        if options.account == self.signer.public().account {
            self.signer = signer;
        } else if let Some(index) = slot {
            self.additional_signers[index] = signer;
        } else {
            self.additional_signers.push(signer);
        }
        Ok(())
    }
    pub(super) fn signer_for(&self, account: AccountId, device: DeviceId) -> Option<&VaultSigner> {
        std::iter::once(&self.signer)
            .chain(self.additional_signers.iter())
            .find(|s| s.public().account == account && s.public().device == device)
    }
    /// A new canonical frontier may not mix any already-bound request with new requests.
    /// Costs, order and the single legal winner are computed by the kernel after trusted proofs.
    pub fn prepare_frontier(
        &mut self,
        requests: &[ActorRequest],
    ) -> Result<nf_store::Accepted, DriverError> {
        if requests.is_empty() || requests.len() > 64 {
            return Err(DriverError::Limit);
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut intents = Vec::with_capacity(requests.len());
        for request in requests {
            if !ids.insert(request.action.request) {
                return Err(StoreError::RequestConflict.into());
            }
            let signer = self
                .signer_for(request.account, request.device)
                .ok_or(DriverError::MissingSigner)?;
            if self
                .store
                .query_bound(
                    request.action.request,
                    request.account,
                    request.device,
                    signer.scope(),
                )?
                .is_some()
            {
                return Err(StoreError::RequestConflict.into());
            }
            let world = self.store.world();
            let g = world.component().genesis();
            let m = world.metadata();
            let action = resolve_action(world, &request.action.action)
                .map_err(MiniatureStoreError::Kernel)?;
            intents.push(MiniatureIntent {
                request: request.action.request,
                operation: request.action.operation,
                job: request.action.job,
                actor: request.account,
                device: request.device,
                universe: g.universe,
                history: g.history,
                provider: m.provider,
                expected: [
                    (m.aggregate, world.component().revision()),
                    (m.provider_aggregate, m.provider_revision),
                ]
                .into(),
                action,
            });
        }
        let mut proofs = Vec::with_capacity(intents.len());
        for intent in &intents {
            proofs.push(self.proof(ChallengeRequest::Prepare(intent))?);
        }
        self.check_budget()?;
        Ok(self.store.prepare_with_hook(
            intents,
            proofs,
            &mut crate::run_budget::deadline_hook(self.run_deadline),
        )?)
    }
}
