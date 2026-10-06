mod support;
use nf_contract::identity::DeviceId;
use nf_identity::{keys::random_id, model::*, signing::device_digest};
#[test]
fn unknown_device_cannot_authorize_protected_operations() {
    let c = support::Community::new();
    let mut proof = DeviceProof {
        scope: c.state.scope,
        account: c.owner.account,
        device: DeviceId::from_bytes(random_id().unwrap()),
        frontier: 0,
        peer: vec![1, 2],
        challenge: [7; 32],
        signature: [0; 64],
    };
    proof.signature = c.owner_key.sign(&device_digest(&proof).unwrap());
    assert_eq!(
        c.state.authorize(
            &proof,
            &[1, 2],
            &[7; 32],
            0,
            ProtectedOperation::Administration
        ),
        Err(IdentityError::UnknownDevice)
    );
}
