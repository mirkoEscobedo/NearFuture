use super::fixture_support::{self as f, Mode};
use crate::{model::IdentityError, private_acl_session::protocol::Completion};
#[test]
fn actual_wrong_ack_refuses_request() {
    let p = f::fixture(Mode::WrongAck, None).unwrap();
    let h = f::held(&p);
    assert_eq!(
        p.request(b"GO\n".to_vec(), 1),
        Err(IdentityError::PrivateStorage)
    );
    drop(p);
    f::reaped(h);
}
#[test]
fn actual_successful_early_exit_is_not_ack() {
    let p = f::fixture(Mode::EarlyExit, None).unwrap();
    let h = f::held(&p);
    assert_eq!(
        p.request(b"GO\n".to_vec(), 1),
        Err(IdentityError::PrivateStorage)
    );
    drop(p);
    f::reaped(h);
}
#[test]
fn actual_nonzero_exit_refuses() {
    let p = f::fixture(Mode::Nonzero, None).unwrap();
    let h = f::held(&p);
    assert_eq!(
        p.request(b"GO\n".to_vec(), 1),
        Err(IdentityError::PrivateStorage)
    );
    drop(p);
    f::reaped(h);
}
#[test]
fn actual_stderr_refuses_correct_stdout() {
    let p = f::fixture(Mode::Stderr, None).unwrap();
    let h = f::held(&p);
    let _ = p.request(b"GO\n".to_vec(), 1);
    assert_eq!(
        p.finish(Completion::ReadOnly, 2),
        Err(IdentityError::PrivateStorage)
    );
    f::reaped(h);
}
#[test]
fn actual_trailing_line_after_done_refuses() {
    let p = f::fixture(Mode::Trailing, None).unwrap();
    let h = f::held(&p);
    p.request(b"GO\n".to_vec(), 1).unwrap();
    assert_eq!(
        p.finish(Completion::ReadOnly, 2),
        Err(IdentityError::PrivateStorage)
    );
    f::reaped(h);
}
#[test]
fn actual_sixty_five_unterminated_stdout_bytes_refuse() {
    let p = f::fixture(Mode::Oversize, None).unwrap();
    let h = f::held(&p);
    assert_eq!(
        p.request(b"GO\n".to_vec(), 1),
        Err(IdentityError::PrivateStorage)
    );
    drop(p);
    f::reaped(h);
}
