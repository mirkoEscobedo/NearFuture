use nf_ipc::{BulkReceiver, IpcError, SessionFence};
use nf_wire::generated as g;
use prost::Message;
use sha2::{Digest, Sha256};
#[test]
fn ordered_digest_verification_is_bound_and_grants_no_semantic_admission() {
    let config = nf_ipc::SessionConfig {
        universe: [1; 16],
        history: [2; 16],
        runtime_session: 9,
        ruleset: [3; 32],
        content_policy: [4; 32],
        limits: nf_ipc::default_limits(),
    };
    let fence = SessionFence::new(9).unwrap();
    let mut receiver = BulkReceiver::new(
        config,
        nf_ipc::LocalPrincipal {
            account: nf_contract::identity::AccountId::from_bytes([5; 16]),
            device: nf_contract::identity::DeviceId::from_bytes([6; 16]),
        },
    )
    .unwrap();
    let digest = Sha256::digest(b"abc").to_vec();
    let chunk = |index, data: Vec<u8>| {
        g::SnapshotChunk {
            transfer_id: Some(g::OperationId { value: vec![8; 16] }),
            chunk_index: index,
            chunk_count: 2,
            total_bytes: 3,
            snapshot_digest: Some(g::Sha256Digest {
                value: digest.clone(),
            }),
            data,
        }
        .encode_to_vec()
    };
    assert!(
        receiver
            .accept(&chunk(0, vec![b'a']), &fence)
            .unwrap()
            .completion
            .is_none()
    );
    let complete = receiver
        .accept(&chunk(1, vec![b'b', b'c']), &fence)
        .unwrap();
    let certificate = complete.completion.unwrap();
    let verified = certificate.verify_bytes(b"abc", &fence).unwrap();
    assert_eq!(verified.bytes(), b"abc");
    assert_eq!(verified.config().runtime_session, 9);
    assert!(matches!(
        receiver.accept(&chunk(1, vec![b'b', b'c']), &fence),
        Err(IpcError::Malformed)
    ));
    let mut invalid = fence;
    invalid.invalidate();
    assert!(matches!(
        receiver.accept(&chunk(0, vec![b'a']), &invalid),
        Err(IpcError::ReadOnly)
    ));
}
#[test]
fn borrowed_verification_checks_digest_exact_size_and_fence_without_parsing_bytes() {
    let config = nf_ipc::SessionConfig {
        universe: [1; 16],
        history: [2; 16],
        runtime_session: 9,
        ruleset: [3; 32],
        content_policy: [4; 32],
        limits: nf_ipc::default_limits(),
    };
    let principal = nf_ipc::LocalPrincipal {
        account: nf_contract::identity::AccountId::from_bytes([5; 16]),
        device: nf_contract::identity::DeviceId::from_bytes([6; 16]),
    };
    let chunk = |total| {
        g::SnapshotChunk {
            transfer_id: Some(g::OperationId { value: vec![8; 16] }),
            chunk_index: 0,
            chunk_count: if total > 3 { 5 } else { 1 },
            total_bytes: total,
            snapshot_digest: Some(g::Sha256Digest {
                value: Sha256::digest(b"abc").to_vec(),
            }),
            data: b"abc".to_vec(),
        }
        .encode_to_vec()
    };
    let fence = SessionFence::new(9).unwrap();
    let mut receiver = BulkReceiver::new(config, principal).unwrap();
    assert!(matches!(
        receiver.accept(&chunk(1_048_577), &fence),
        Err(IpcError::Limit)
    ));
    for case in 0..3 {
        let certificate = receiver
            .accept(&chunk(3), &fence)
            .unwrap()
            .completion
            .unwrap();
        match case {
            0 => assert!(matches!(
                certificate.verify_bytes(b"abd", &fence),
                Err(IpcError::Malformed)
            )),
            1 => assert!(matches!(
                certificate.verify_bytes(b"abcd", &fence),
                Err(IpcError::Limit)
            )),
            _ => assert!(matches!(
                certificate.verify_bytes(b"abc", &SessionFence::new(10).unwrap()),
                Err(IpcError::SessionMismatch)
            )),
        }
    }
    let certificate = receiver
        .accept(&chunk(3), &fence)
        .unwrap()
        .completion
        .unwrap();
    let borrowed = certificate.verify_bytes(b"abc", &fence).unwrap();
    assert_eq!(borrowed.principal(), principal);
    assert_eq!(borrowed.bytes().as_ptr(), b"abc".as_ptr());
}
