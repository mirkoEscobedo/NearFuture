//! Unit-only final-capture boundary; no runtime setter, session, clock or generic callback.
use super::super::ReceiptRepo;
use crate::PeerError;
use nf_identity::model::DeviceRevocation;
use std::cell::RefCell;
enum Plan {
    Delay,
    Revoke(DeviceRevocation, [u8; 64]),
}
thread_local! {
    static PLAN: RefCell<Option<Plan>> = const { RefCell::new(None) };
    static REACHED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
pub(in crate::receipt_effects) struct Armed;
impl Drop for Armed {
    fn drop(&mut self) {
        PLAN.with(|p| {
            p.replace(None);
        });
    }
}
pub(in crate::receipt_effects) fn delay() -> Armed {
    arm(Plan::Delay)
}
pub(in crate::receipt_effects) fn revoke(change: DeviceRevocation, signature: [u8; 64]) -> Armed {
    arm(Plan::Revoke(change, signature))
}
fn arm(plan: Plan) -> Armed {
    PLAN.with(|p| {
        assert!(p.replace(Some(plan)).is_none());
    });
    REACHED.set(false);
    Armed
}
pub(in crate::receipt_effects) fn reached() -> bool {
    REACHED.get()
}
pub(super) fn before_capture(
    repo: &mut ReceiptRepo,
) -> Result<Option<super::super::read_delay::Guard>, PeerError> {
    match PLAN.with(|p| p.take()) {
        Some(Plan::Delay) => {
            REACHED.set(true);
            Ok(Some(ReceiptRepo::delay_current_read_for_test(0)))
        }
        Some(Plan::Revoke(change, signature)) => {
            REACHED.set(true);
            repo.revoke_trusted(change, signature)?;
            Ok(None)
        }
        None => Ok(None),
    }
}
