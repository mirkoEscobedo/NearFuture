use super::{super::*, support::*};
use nf_contract::identity::{AccountId, DeviceId};
pub fn auth(lane: SyncLane) -> SyncAuthTranscript {
    SyncAuthTranscript {
        lane,
        client_peer: peer(true),
        server_peer: peer(false),
        client_account: AccountId::from_bytes([1; 16]),
        client_device: DeviceId::from_bytes([2; 16]),
        server_account: AccountId::from_bytes([3; 16]),
        server_device: DeviceId::from_bytes([4; 16]),
        context: context(lane, false),
        client_nonce: [10; 32],
        server_nonce: [11; 32],
        required: 1,
        optional: 0,
        available: 1,
        selected_caps: 1,
        offered: SyncLimits::default(),
        server_limits: SyncLimits::default(),
        selected: SyncLimits::default(),
        membership: stamp(),
        pins: pins(),
    }
}
#[test]
fn eight_exact_750_byte_transcripts_and_domain_separation() {
    let mut count = 0;
    for r in rows()
        .into_iter()
        .filter(|r| r.category() == "transcript" && r.bytes().len() == 750)
    {
        let lane = if r.name().contains("lane2") {
            SyncLane::Transfer
        } else {
            SyncLane::Control
        };
        let stage = if r.name().starts_with("neutral") {
            SyncAuthStage::Neutral
        } else if r.name().starts_with("server") {
            SyncAuthStage::Server
        } else if r.name().starts_with("client") {
            SyncAuthStage::Client
        } else {
            SyncAuthStage::Finished
        };
        let a = auth(lane);
        assert_eq!(a.encode(stage, pins()).unwrap().as_slice(), r.bytes());
        assert_eq!(a.digest(stage, pins()).unwrap(), hash(&r.bytes()));
        count += 1;
    }
    assert_eq!(count, 8);
    assert_ne!(
        auth(SyncLane::Control)
            .digest(SyncAuthStage::Neutral, pins())
            .unwrap(),
        auth(SyncLane::Transfer)
            .digest(SyncAuthStage::Neutral, pins())
            .unwrap()
    );
}
