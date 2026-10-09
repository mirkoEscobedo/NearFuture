use crate::{
    PeerError,
    notification::{NotifyContext, NotifyLimits, NotifyRecord, PROTOCOL, encode_body},
};
use nf_identity::model::Scope;
#[derive(Clone, Copy, Debug)]
pub struct NotifyPolicy {
    pub scope: Scope,
    pub ruleset: [u8; 32],
    pub content: [u8; 32],
    pub limits: NotifyLimits,
    pub minimum_membership: u64,
}
impl NotifyPolicy {
    pub(super) fn context(self, session: [u8; 16]) -> NotifyContext {
        NotifyContext {
            session,
            scope: self.scope,
            ruleset: self.ruleset,
            content: self.content,
        }
    }
    pub(super) fn admit(self, r: &NotifyRecord) -> Result<(), PeerError> {
        encode_body(r, PROTOCOL, self.limits)?;
        if r.context.scope != self.scope {
            return Err(PeerError::Scope);
        }
        if r.context.ruleset != self.ruleset || r.context.content != self.content {
            return Err(PeerError::Policy);
        }
        Ok(())
    }
}
