use super::{super::*, support::*, transcripts::auth};
#[test]
fn eight_exact_operation_transcripts_keep_control_and_transfer_nonces_distinct() {
    let begin = hash(&row("begin-sync-delta-lane1").bytes());
    let doc = hash(&row("begin-document-lane2").bytes());
    let end = hash(&row("document-end-lane2-prefix").bytes());
    let mut count = 0;
    for r in rows()
        .into_iter()
        .filter(|r| r.category() == "transcript" && r.bytes().len() == 278)
    {
        let name = r.name().strip_suffix("-transcript").unwrap();
        let record = row(name);
        let kind = record.number(6, 0);
        let purpose = match kind {
            7 => SyncPurpose::Sync,
            8 => SyncPurpose::Manifest,
            9 | 17 => SyncPurpose::GapOrRefused,
            12 => SyncPurpose::Document,
            13 => SyncPurpose::Ready,
            15 => SyncPurpose::End,
            16 => SyncPurpose::Install,
            _ => panic!("closed purpose"),
        };
        let document = matches!(kind, 12 | 13 | 15);
        let returned = matches!(kind, 8 | 12 | 13 | 15 | 16);
        let response = if matches!(kind, 7 | 12) {
            None
        } else {
            Some(hash(&row(&format!("{name}-prefix")).bytes()))
        };
        let op = SyncOperationTranscript {
            purpose,
            neutral_digest: auth(if document {
                SyncLane::Transfer
            } else {
                SyncLane::Control
            })
            .digest(SyncAuthStage::Neutral, pins())
            .unwrap(),
            request: SyncRequestId([14; 16]),
            export: returned.then_some(ExportId([15; 16])),
            document_digest: if document {
                Some(hash(&vec![0x28; 391]))
            } else if matches!(kind, 8 | 16) {
                Some([28; 32])
            } else {
                None
            },
            initiating_digest: if kind == 16 {
                end
            } else if document {
                doc
            } else {
                begin
            },
            client_operation_nonce: [if document || kind == 16 { 30 } else { 17 }; 32],
            server_operation_nonce: [if kind == 16 {
                19
            } else if document {
                31
            } else {
                18
            }; 32],
            membership: stamp(),
            response_digest: response,
        };
        assert_eq!(op.encode().unwrap().as_slice(), r.bytes(), "{}", r.name());
        assert_eq!(op.digest().unwrap(), hash(&r.bytes()));
        count += 1;
        if kind == 16 {
            let mut wrong = op;
            wrong.client_operation_nonce = [17; 32];
            assert_ne!(wrong.digest().unwrap(), op.digest().unwrap());
        }
    }
    assert_eq!(count, 8);
}
