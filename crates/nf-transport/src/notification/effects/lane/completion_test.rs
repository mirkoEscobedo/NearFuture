//! Test-only fixed scheduling delay at actual matched completion events.
use crate::{PeerError, receipt_effects::ReceiptRepo};
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::notification_effects) enum Cut {
    ClientSubscribed,
    ServerSubscribed,
    ServerAck,
    ServerOriginGap,
}
thread_local! { static ARMED: Cell<Option<Cut>> = const { Cell::new(None) }; static REACHED: Cell<bool> = const { Cell::new(false) };
static ORIGINAL: Cell<Option<Instant>> = const {Cell::new(None)};
static GAP: Cell<Option<(Instant,Instant)>> = const {Cell::new(None)};
static PARITY: Cell<(bool,bool)> = const {Cell::new((false,false))}; }
pub(in crate::notification_effects) struct Guard;
pub(in crate::notification_effects) fn arm(cut: Cut) -> Guard {
    ARMED.with(|p| {
        assert!(p.replace(Some(cut)).is_none());
    });
    REACHED.with(|p| p.set(false));
    PARITY.with(|p| p.set((false, false)));
    Guard
}
impl Drop for Guard {
    fn drop(&mut self) {
        ARMED.with(|p| p.set(None));
        ORIGINAL.with(|p| p.set(None));
        GAP.with(|p| p.set(None));
    }
}
pub(in crate::notification_effects) fn reached() -> bool {
    REACHED.with(Cell::get)
}
pub(super) fn at<T>(
    cut: Cut,
    skip: usize,
    action: impl FnOnce() -> Result<T, PeerError>,
) -> Result<T, PeerError> {
    let armed = ARMED.with(|p| {
        if p.get() == Some(Cut::ServerOriginGap) && cut == Cut::ServerSubscribed {
            p.set(None);
            return true;
        }
        if p.get() == Some(cut) {
            p.set(None);
            true
        } else {
            false
        }
    });
    if !armed {
        return action();
    }
    if GAP.with(|p| p.get().is_some()) {
        return action();
    }
    let _delay = ReceiptRepo::delay_current_read_for_test(skip);
    let started = Instant::now();
    let result = action();
    REACHED.with(|p| p.set(started.elapsed() >= Duration::from_millis(5100)));
    result
}

pub(super) fn before_prove(original: Instant) {
    if ARMED.with(Cell::get) == Some(Cut::ServerOriginGap) {
        ORIGINAL.with(|p| p.set(Some(original)));
        std::thread::sleep(Duration::from_millis(800));
    }
}
pub(super) fn owner_end(end: Instant) {
    if let Some(original) = ORIGINAL.with(Cell::get) {
        GAP.with(|p| p.set(Some((original, end))));
    }
}
pub(in crate::notification_effects) fn after_current_read() {
    if let Some((original, owner)) = GAP.with(Cell::take) {
        let remaining = original.saturating_duration_since(Instant::now());
        std::thread::sleep(remaining + Duration::from_millis(50));
        let now = Instant::now();
        PARITY.with(|p| p.set((now >= original, now < owner)));
        REACHED.with(|p| p.set(true));
    }
}
pub(in crate::notification_effects) fn origin_parity() -> (bool, bool) {
    PARITY.with(Cell::get)
}
