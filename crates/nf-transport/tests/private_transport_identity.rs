#[path = "support/scratch.rs"]
mod scratch;
use nf_identity::private_storage::PrivateVault;
use nf_transport::identity::TransportIdentity;
#[test]
fn private_noise_identity_reopens_same_actual_peer_and_never_regenerates_missing_state() {
    let s = scratch::Scratch::new();
    std::fs::create_dir(s.0.join("saves")).unwrap();
    let v = PrivateVault::create(&s.0.join("private"), &s.0.join("saves")).unwrap();
    let first = TransportIdentity::create(&v).unwrap();
    let peer = first.peer_id();
    drop(first);
    assert_eq!(TransportIdentity::load(&v).unwrap().peer_id(), peer);
    assert!(TransportIdentity::create(&v).is_err());
    v.remove_private_blob("transport-ed25519-v1").unwrap();
    assert!(TransportIdentity::load(&v).is_err());
}
