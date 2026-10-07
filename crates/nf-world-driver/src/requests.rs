use crate::{ActionSelection, Driver, DriverError, resolve_action};
use nf_contract::identity::*;
use nf_kernel::miniature::MiniatureIntent;
use nf_store::{
    StoreError,
    miniature::{ChallengeRequest, MiniatureBoundRequest, MiniatureStoreError},
};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRequest {
    pub request: RequestId,
    pub operation: OperationId,
    pub job: JobId,
    pub action: ActionSelection,
}
impl Driver {
    /// Inspect and validate an original binding before any authority claim or fresh revision map.
    pub fn query_action(
        &self,
        request: &ActionRequest,
    ) -> Result<Option<MiniatureBoundRequest>, DriverError> {
        let public = self.signer.public();
        let original = self.store.query_bound(
            request.request,
            public.account,
            public.device,
            self.signer.scope(),
        )?;
        if let Some(original) = original {
            let action = resolve_action(self.store.world(), &request.action)
                .map_err(MiniatureStoreError::Kernel)?;
            let intent = &original.intent;
            let g = self.store.world().component().genesis();
            let m = self.store.world().metadata();
            if intent.request != request.request
                || intent.operation != request.operation
                || intent.job != request.job
                || intent.actor != public.account
                || intent.device != public.device
                || intent.universe != g.universe
                || intent.history != g.history
                || intent.provider != m.provider
                || intent.action != action
            {
                return Err(StoreError::RequestConflict.into());
            }
            Ok(Some(original))
        } else {
            Ok(None)
        }
    }
    /// Query the original binding before constructing any fresh expected revision map.
    pub fn prepare_action(
        &mut self,
        request: &ActionRequest,
    ) -> Result<MiniatureBoundRequest, DriverError> {
        if let Some(original) = self.query_action(request)? {
            return Ok(original);
        }
        let public = self.signer.public();
        let account = public.account;
        let device = public.device;
        let scope = self.signer.scope();
        let action = resolve_action(self.store.world(), &request.action)
            .map_err(MiniatureStoreError::Kernel)?;
        let world = self.store.world();
        let g = world.component().genesis();
        let m = world.metadata();
        let intent = MiniatureIntent {
            request: request.request,
            operation: request.operation,
            job: request.job,
            actor: account,
            device,
            universe: g.universe,
            history: g.history,
            provider: m.provider,
            expected: [
                (m.aggregate, world.component().revision()),
                (m.provider_aggregate, m.provider_revision),
            ]
            .into(),
            action,
        };
        let attempt = self.proof(ChallengeRequest::Prepare(&intent))?;
        self.check_budget()?;
        self.store.prepare_with_hook(
            vec![intent],
            vec![attempt],
            &mut crate::run_budget::deadline_hook(self.run_deadline),
        )?;
        self.store
            .query_bound(request.request, account, device, scope)?
            .ok_or_else(|| StoreError::Corrupt.into())
    }
}
