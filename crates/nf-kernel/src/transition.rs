use crate::Rejection;
use nf_contract::identity::*;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outcome {
    pub operation: OperationId,
    pub job: JobId,
    pub rejection: Option<Rejection>,
}
