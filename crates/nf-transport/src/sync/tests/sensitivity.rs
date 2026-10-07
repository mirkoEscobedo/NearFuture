use super::{super::*, support::*, transcripts::auth};
use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
#[test]
fn handshake_digest_binds_every_nonconstant_principal_and_context_field() {
    let original = auth(SyncLane::Control);
    let expected = original.digest(SyncAuthStage::Server, pins()).unwrap();
    macro_rules! changed {
        ($edit:expr) => {{
            let mut next = original.clone();
            ($edit)(&mut next);
            assert_ne!(
                next.digest(SyncAuthStage::Server, pins()).unwrap(),
                expected
            );
        }};
    }
    changed!(|v: &mut SyncAuthTranscript| v.client_peer = peer(false));
    changed!(|v: &mut SyncAuthTranscript| v.server_peer = peer(true));
    changed!(|v: &mut SyncAuthTranscript| v.client_account = AccountId::from_bytes([33; 16]));
    changed!(|v: &mut SyncAuthTranscript| v.server_account = AccountId::from_bytes([33; 16]));
    changed!(|v: &mut SyncAuthTranscript| v.client_device = DeviceId::from_bytes([33; 16]));
    changed!(|v: &mut SyncAuthTranscript| v.server_device = DeviceId::from_bytes([33; 16]));
    changed!(|v: &mut SyncAuthTranscript| v.context.session[0] ^= 1);
    changed!(
        |v: &mut SyncAuthTranscript| v.context.scope.universe = UniverseId::from_bytes([33; 16])
    );
    changed!(|v: &mut SyncAuthTranscript| v.context.scope.history = HistoryId::from_bytes([33; 16]));
    changed!(|v: &mut SyncAuthTranscript| v.context.ruleset[0] ^= 1);
    changed!(|v: &mut SyncAuthTranscript| v.context.content[0] ^= 1);
    changed!(|v: &mut SyncAuthTranscript| v.client_nonce[0] ^= 1);
    changed!(|v: &mut SyncAuthTranscript| v.server_nonce[0] ^= 1);
    changed!(|v: &mut SyncAuthTranscript| v.membership.digest[0] ^= 1);
    changed!(|v: &mut SyncAuthTranscript| v.membership.revision += 1);
    assert_ne!(
        original.digest(SyncAuthStage::Client, pins()).unwrap(),
        expected
    );
    let mut limits = original.clone();
    limits.offered.chunk_bytes = 256;
    limits.selected.chunk_bytes = 256;
    assert_ne!(
        limits.digest(SyncAuthStage::Server, pins()).unwrap(),
        expected
    );
    let mut pin = original.clone();
    pin.pins.implementation[0] ^= 1;
    assert!(pin.encode(SyncAuthStage::Server, pins()).is_err());
    let mut cap = original.clone();
    cap.optional = 1;
    assert!(cap.encode(SyncAuthStage::Server, pins()).is_err());
    let mut neutral = original.clone();
    neutral.membership.revision += 1;
    assert_eq!(
        neutral.digest(SyncAuthStage::Neutral, pins()).unwrap(),
        original.digest(SyncAuthStage::Neutral, pins()).unwrap()
    );
}
#[test]
fn install_transcript_binds_exact_final_document_values_and_original_correlation() {
    let original = SyncOperationTranscript {
        purpose: SyncPurpose::Install,
        neutral_digest: [1; 32],
        request: SyncRequestId([2; 16]),
        export: Some(ExportId([3; 16])),
        document_digest: Some([4; 32]),
        initiating_digest: [5; 32],
        client_operation_nonce: [30; 32],
        server_operation_nonce: [19; 32],
        membership: stamp(),
        response_digest: Some([6; 32]),
    };
    let expected = original.digest().unwrap();
    macro_rules! changed {
        ($edit:expr) => {{
            let mut next = original;
            ($edit)(&mut next);
            assert_ne!(next.digest().unwrap(), expected);
        }};
    }
    changed!(|v: &mut SyncOperationTranscript| v.neutral_digest[0] ^= 1);
    changed!(|v: &mut SyncOperationTranscript| v.request = SyncRequestId([33; 16]));
    changed!(|v: &mut SyncOperationTranscript| v.export = Some(ExportId([33; 16])));
    changed!(|v: &mut SyncOperationTranscript| v.document_digest = Some([33; 32]));
    changed!(|v: &mut SyncOperationTranscript| v.initiating_digest[0] ^= 1);
    changed!(|v: &mut SyncOperationTranscript| v.client_operation_nonce = [17; 32]);
    changed!(|v: &mut SyncOperationTranscript| v.server_operation_nonce = [31; 32]);
    changed!(|v: &mut SyncOperationTranscript| v.membership.revision += 1);
    changed!(|v: &mut SyncOperationTranscript| v.membership.digest[0] ^= 1);
    changed!(|v: &mut SyncOperationTranscript| v.response_digest = Some([33; 32]));
    let mut missing = original;
    missing.export = None;
    assert!(missing.encode().is_err());
    missing = original;
    missing.response_digest = None;
    assert!(missing.encode().is_err());
}
