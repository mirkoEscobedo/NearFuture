//! Test-only simulation of synchronous work after a real validated SQL read.
use std::{cell::RefCell, time::Duration};
thread_local! { static NEXT: RefCell<Option<usize>> = const { RefCell::new(None) }; }
pub(crate) struct Guard(Option<usize>);
pub(super) fn arm(skip: usize) -> Guard {
    Guard(NEXT.with(|next| next.replace(Some(skip))))
}
impl Drop for Guard {
    fn drop(&mut self) {
        NEXT.with(|next| {
            next.replace(self.0.take());
        });
    }
}
pub(super) fn after_current_read() {
    let delay = NEXT.with(|next| {
        let mut next = next.borrow_mut();
        match *next {
            Some(0) => {
                *next = None;
                true
            }
            Some(n) => {
                *next = Some(n - 1);
                false
            }
            None => false,
        }
    });
    if delay {
        std::thread::sleep(Duration::from_millis(5100));
    }
}
