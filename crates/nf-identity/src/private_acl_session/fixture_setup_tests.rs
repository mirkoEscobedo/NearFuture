use super::fixture_support::{self as f, Mode, Setup, Stage};
use crate::model::IdentityError;
use std::sync::{Arc, Mutex};
#[test]
fn actual_owned_child_reaped_at_every_closed_setup_fault() {
    for stage in [
        Stage::Monitor,
        Stage::Pipes,
        Stage::Writer,
        Stage::Stdout,
        Stage::Stderr,
    ] {
        let observed = Arc::new(Mutex::new(None));
        let result = f::fixture(
            Mode::Silent,
            Some(Setup {
                stage,
                observed: Arc::clone(&observed),
            }),
        );
        assert!(matches!(result, Err(IdentityError::PrivateStorage)));
        let held = observed
            .lock()
            .unwrap()
            .take()
            .expect("actual owned child observed before fault");
        f::reaped(held);
    }
}
