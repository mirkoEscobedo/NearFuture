use super::{ReceiptRepo, model::*};
use crate::{
    PeerError,
    receipt::{
        OriginalReceipt, ReceiptPhase, ReceiptStatus, ReceiptTarget, ReceiptUnsupportedReason,
        SourceMinima,
    },
};
use nf_contract::identity::OperationId;
use nf_store::RequestStatus;
pub(super) enum ObservedReply {
    Status(ReceiptStatus),
    Unsupported(ReceiptUnsupportedReason, SourceMinima),
}
impl ReceiptRepo {
    pub fn selector(&self, original: &OriginalReceipt) -> Result<ReceiptSelector, PeerError> {
        self.validate_original(original)?;
        Ok(ReceiptSelector {
            account: original.original().account_id,
            device: original.original().device_id,
            scope: self.config.scope,
            peer: self.transport.peer_id(),
            target: ReceiptTarget {
                request: original.request(),
                operation: original.operation(),
                binding: original.binding_digest(),
            },
        })
    }
    /// The caller obtains remote IDs only from its freshly checked active actual session.
    /// This independently checks current SQL and exact retained identity/binding again.
    pub(crate) fn admit_remote_selector(
        &mut self,
        remote: RemotePrincipal,
        target: ReceiptTarget,
        expected_revision: u64,
    ) -> Result<(ReceiptSelector, ProjectionRead), PeerError> {
        let selector = ReceiptSelector {
            account: remote.account,
            device: remote.device,
            scope: remote.scope,
            peer: remote.peer,
            target,
        };
        let projection = self.observe_selector(&selector, expected_revision)?;
        Ok((selector, projection))
    }
    pub(crate) fn observe_selector(
        &mut self,
        selector: &ReceiptSelector,
        expected_revision: u64,
    ) -> Result<ProjectionRead, PeerError> {
        match self.query_target(
            selector,
            expected_revision,
            SourceMinima {
                event: nf_contract::identity::EventSeq(0),
                store_revision: 0,
                membership_revision: expected_revision,
            },
        )? {
            ObservedReply::Status(status) if status.phase != ReceiptPhase::Unknown => {
                Ok(ProjectionRead {
                    selector: selector.clone(),
                    phase: status.phase,
                    source: status.current,
                })
            }
            _ => Err(PeerError::Unauthorized),
        }
    }
    pub(super) fn query_target(
        &mut self,
        selector: &ReceiptSelector,
        expected_revision: u64,
        minimum: SourceMinima,
    ) -> Result<ObservedReply, PeerError> {
        selector.target.validate()?;
        let current = self.with_current_read(|cut| {
            if selector.scope != cut.membership.scope
                || cut.membership.revision != expected_revision
            {
                return Err(PeerError::Policy);
            }
            role(
                cut.membership,
                selector.account,
                selector.device,
                selector.peer,
            )?;
            Ok(cut.source)
        })?;
        if current.membership_revision != expected_revision {
            return Err(PeerError::Policy);
        }
        if !current.admits(minimum) {
            return Ok(ObservedReply::Unsupported(
                ReceiptUnsupportedReason::SourceBelowKnownMinima,
                current,
            ));
        }
        let retained = self
            .store
            .query_bound(
                selector.target.request,
                selector.account,
                selector.device,
                selector.scope,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        let (operation, binding, phase) = match retained {
            None => (
                OperationId::from_bytes([0; 16]),
                [0; 32],
                ReceiptPhase::Unknown,
            ),
            Some(r) => {
                let (operation, phase) = match r.status {
                    RequestStatus::Pending { operation } => (operation, ReceiptPhase::Pending),
                    RequestStatus::Committed {
                        operation,
                        sequence,
                        rejection: Some(_),
                    } => (operation, ReceiptPhase::Rejected { sequence }),
                    RequestStatus::Committed {
                        operation,
                        sequence,
                        rejection: None,
                    } => (operation, ReceiptPhase::Committed { sequence }),
                };
                if operation != selector.target.operation
                    || r.binding_digest != selector.target.binding
                {
                    return Ok(ObservedReply::Unsupported(
                        ReceiptUnsupportedReason::BindingConflict,
                        current,
                    ));
                }
                (operation, r.binding_digest, phase)
            }
        };
        Ok(ObservedReply::Status(ReceiptStatus {
            request: selector.target.request,
            account: selector.account,
            device: selector.device,
            operation,
            binding,
            phase,
            current,
        }))
    }
}
