use super::fixture::Fixture;
use nf_transport::receipt_effects::lane::activation_test;
use std::time::Duration;
#[path = "../receipt_effect_support/activation_cases.rs"]
mod cases;
#[tokio::test]
async fn final_actual_finished_capture_refuses_elapsed_original_handshake() {
    let mut f = Fixture::new();
    let _probe = activation_test::delay();
    tokio::time::timeout(Duration::from_secs(20), cases::refused(&mut f, true))
        .await
        .unwrap();
}
#[tokio::test]
async fn final_actual_finished_capture_refuses_new_signed_membership_epoch() {
    let mut f = Fixture::new();
    let change = nf_identity::model::DeviceRevocation {
        scope: f.state.scope,
        issuer: f.server_public.account,
        device: f.client_public.device,
        frontier: f.state.revision,
    };
    let signature = f
        .owner_key
        .sign(&nf_identity::rotation::revocation_digest(&change));
    let _probe = activation_test::revoke(change, signature);
    tokio::time::timeout(Duration::from_secs(20), cases::refused(&mut f, false))
        .await
        .unwrap();
    assert_eq!(
        f.server
            .with_current_read(|cut| Ok(cut.membership.revision))
            .unwrap(),
        2
    );
}
