#[path = "support/child.rs"]
mod child;
#[path = "support/scratch.rs"]
mod scratch;
#[test]
fn foreground_cli_has_closed_args_and_no_implicit_network_start() {
    let tmp = scratch::Scratch::new();
    let mut c = child::OwnedChild::spawn(&[], &tmp.0, "closed-args");
    let status = c.wait(std::time::Duration::from_secs(3));
    assert!(!status.success());
    assert_eq!(c.output(), "");
    assert_eq!(c.error().trim(), "NF_PEER_ERROR Malformed");
}
