mod notification_support;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, OperationId, RequestId, UniverseId};
use nf_identity::model::Scope;
use nf_transport::notification::*;

fn handshake() -> NotifyHandshakeTranscript {
    let rows = notification_support::vectors();
    let peer = |name| {
        libp2p::PeerId::from_bytes(
            &rows
                .iter()
                .find(|row| row.category == "value" && row.name == name)
                .unwrap()
                .bytes,
        )
        .unwrap()
    };
    NotifyHandshakeTranscript {
        client_peer: peer("client-peer"),
        server_peer: peer("server-peer"),
        client_account: AccountId::from_bytes([1; 16]),
        client_device: DeviceId::from_bytes([2; 16]),
        server_account: AccountId::from_bytes([3; 16]),
        server_device: DeviceId::from_bytes([4; 16]),
        context: NotifyContext {
            session: [5; 16],
            scope: Scope {
                universe: UniverseId::from_bytes([6; 16]),
                history: HistoryId::from_bytes([7; 16]),
            },
            ruleset: [8; 32],
            content: [9; 32],
        },
        client_nonce: [10; 32],
        server_nonce: [11; 32],
        required: 1,
        optional: 0,
        server_available: 1,
        selected_caps: 1,
        offered: NotifyLimits::default(),
        server_limits: NotifyLimits::default(),
        selected: NotifyLimits::default(),
    }
}
#[test]
fn independent_handshake_bytes_bind_the_exact_notification_protocol_and_stage() {
    let input = handshake();
    for row in notification_support::vectors()
        .into_iter()
        .filter(|row| row.category == "transcript" && row.name.starts_with("handshake-"))
    {
        assert_eq!(row.layer, "value");
        assert_eq!(row.expectation, "617_BYTES");
        let stage: u8 = row.name.rsplit('-').next().unwrap().parse().unwrap();
        assert_eq!(
            input
                .preimage(stage, if stage == 0 { 0 } else { 12 }, PROTOCOL)
                .unwrap(),
            row.bytes,
            "{}",
            row.name
        );
    }
}

#[test]
fn independent_signed_prefixes_exclude_only_final_proof_and_outer_framing() {
    let rows = notification_support::vectors();
    let mut count = 0;
    for row in rows.iter().filter(|row| row.category == "prefix") {
        assert_eq!(row.layer, "value");
        let expected_length: usize = row.expectation.trim_end_matches("_BYTES").parse().unwrap();
        let body = rows
            .iter()
            .find(|body| body.category == "record" && body.name == row.name)
            .unwrap();
        let record = decode_body(&body.bytes, PROTOCOL, NotifyLimits::default()).unwrap();
        let actual = signed_prefix(&record, PROTOCOL, NotifyLimits::default()).unwrap();
        assert_eq!(actual.len(), expected_length);
        assert_eq!(actual, row.bytes, "{}", row.name);
        count += 1;
    }
    assert_eq!(count, 3);
}

fn selector() -> NotifySelector {
    let rows = notification_support::vectors();
    let value = rows
        .iter()
        .find(|row| row.category == "value" && row.name == "original-receipt-binding")
        .unwrap();
    NotifySelector {
        request: RequestId::from_bytes([12; 16]),
        operation: OperationId::from_bytes([13; 16]),
        binding: value.bytes.as_slice().try_into().unwrap(),
    }
}
fn context_digest() -> [u8; 32] {
    notification_support::vectors()
        .into_iter()
        .find(|row| row.category == "transcript" && row.name == "handshake-0")
        .unwrap()
        .digest
}
#[test]
fn independent_subscribe_purposes_bind_minima_lifetime_and_exact_reply_prefix() {
    let input = NotifySubscribeTranscript {
        context_digest: context_digest(),
        subscription: [21; 16],
        selector: selector(),
        client_nonce: [14; 32],
        server_nonce: [15; 32],
        frontier: 12,
        minimum_membership: 11,
        lifetime: 10,
    };
    let rows = notification_support::vectors();
    for row in rows.iter().filter(|row| {
        row.category == "transcript" && matches!(row.name, "subscribe-1" | "subscribed")
    }) {
        assert_eq!(row.layer, "value");
        assert_eq!(row.expectation, "244_BYTES");
        let (stage, prefix) = if row.name == "subscribe-1" {
            (1, [0; 32])
        } else {
            (
                2,
                rows.iter()
                    .find(|prefix| prefix.category == "prefix" && prefix.name == "subscribed")
                    .unwrap()
                    .digest,
            )
        };
        assert_eq!(
            input.preimage(stage, prefix).unwrap(),
            row.bytes,
            "{}",
            row.name
        );
    }
}

#[test]
fn notice_and_ack_purposes_bind_distinct_independent_prefixes() {
    let input = NotifyNoticeTranscript {
        context_digest: context_digest(),
        subscription: [21; 16],
        sequence: 1,
        selector: selector(),
        nonce: [22; 32],
        frontier: 12,
    };
    let rows = notification_support::vectors();
    for row in rows
        .iter()
        .filter(|row| row.category == "transcript" && matches!(row.name, "notice" | "notice-ack"))
    {
        assert_eq!(row.layer, "value");
        assert_eq!(row.expectation, "212_BYTES");
        let stage = if row.name == "notice" { 1 } else { 2 };
        let prefix = rows
            .iter()
            .find(|prefix| prefix.category == "prefix" && prefix.name == row.name)
            .unwrap()
            .digest;
        assert_eq!(
            input.preimage(stage, prefix).unwrap(),
            row.bytes,
            "{}",
            row.name
        );
    }
}

#[test]
fn public_challenges_and_prefix_hashes_match_independent_node_digests() {
    let rows = notification_support::vectors();
    let h = handshake();
    for row in rows
        .iter()
        .filter(|row| row.category == "transcript" && row.name.starts_with("handshake-"))
    {
        assert_eq!(row.layer, "value");
        assert_eq!(row.expectation, "617_BYTES");
        let stage: u8 = row.name.rsplit('-').next().unwrap().parse().unwrap();
        let digest = if stage == 0 {
            h.context_digest(PROTOCOL).unwrap()
        } else {
            h.challenge(stage, 12, PROTOCOL).unwrap()
        };
        assert_eq!(digest, row.digest, "{}", row.name);
    }
    let subscribe = NotifySubscribeTranscript {
        context_digest: context_digest(),
        subscription: [21; 16],
        selector: selector(),
        client_nonce: [14; 32],
        server_nonce: [15; 32],
        frontier: 12,
        minimum_membership: 11,
        lifetime: 10,
    };
    let notice = NotifyNoticeTranscript {
        context_digest: context_digest(),
        subscription: [21; 16],
        sequence: 1,
        selector: selector(),
        nonce: [22; 32],
        frontier: 12,
    };
    for row in rows.iter().filter(|row| row.category == "prefix") {
        let record_bytes = &rows
            .iter()
            .find(|record| record.category == "record" && record.name == row.name)
            .unwrap()
            .bytes;
        let record = decode_body(record_bytes, PROTOCOL, NotifyLimits::default()).unwrap();
        let prefix = signed_prefix_digest(&record, PROTOCOL, NotifyLimits::default()).unwrap();
        assert_eq!(prefix, row.digest, "{}", row.name);
        let expected = rows
            .iter()
            .find(|t| t.category == "transcript" && t.name == row.name)
            .unwrap()
            .digest;
        let actual = if row.name == "subscribed" {
            subscribe.challenge(2, prefix).unwrap()
        } else {
            notice
                .challenge(if row.name == "notice" { 1 } else { 2 }, prefix)
                .unwrap()
        };
        assert_eq!(actual, expected, "{}", row.name);
    }
    let expected = rows
        .iter()
        .find(|t| t.category == "transcript" && t.name == "subscribe-1")
        .unwrap()
        .digest;
    assert_eq!(subscribe.challenge(1, [0; 32]).unwrap(), expected);
}

#[test]
fn transcript_domains_stages_and_exact_limit_selection_fail_closed() {
    let input = handshake();
    assert!(input.challenge(0, 0, PROTOCOL).is_err());
    assert!(input.preimage(0, 12, PROTOCOL).is_err());
    assert!(input.preimage(4, 12, PROTOCOL).is_err());
    assert!(input.preimage(1, 12, "/nearfuture/peer/control/2").is_err());
    let mut changed = input.clone();
    changed.selected.queue_items -= 1;
    assert!(changed.preimage(1, 12, PROTOCOL).is_err());
    changed = input.clone();
    changed.offered.frame = 1025;
    assert!(changed.preimage(1, 12, PROTOCOL).is_err());
    changed = input.clone();
    changed.client_nonce = [0; 32];
    assert!(changed.preimage(1, 12, PROTOCOL).is_err());
    let mut subscribe = NotifySubscribeTranscript {
        context_digest: context_digest(),
        subscription: [21; 16],
        selector: selector(),
        client_nonce: [14; 32],
        server_nonce: [15; 32],
        frontier: 12,
        minimum_membership: 11,
        lifetime: 10,
    };
    assert!(subscribe.preimage(0, [0; 32]).is_err());
    assert!(subscribe.preimage(1, [1; 32]).is_err());
    subscribe.minimum_membership = 13;
    assert!(subscribe.preimage(1, [0; 32]).is_err());
    subscribe.minimum_membership = 11;
    subscribe.lifetime = 31;
    assert!(subscribe.preimage(1, [0; 32]).is_err());
    let notice = NotifyNoticeTranscript {
        context_digest: context_digest(),
        subscription: [21; 16],
        sequence: 0,
        selector: selector(),
        nonce: [22; 32],
        frontier: 12,
    };
    assert!(notice.preimage(1, [1; 32]).is_err());
}

#[test]
fn public_challenges_change_with_each_supplied_binding_without_authorization_claim() {
    let h = handshake();
    let original = h.challenge(1, 12, PROTOCOL).unwrap();
    for field in 0..12 {
        let mut changed = h.clone();
        match field {
            0 => changed.client_peer = h.server_peer,
            1 => changed.server_peer = h.client_peer,
            2 => changed.client_account = AccountId::from_bytes([23; 16]),
            3 => changed.client_device = DeviceId::from_bytes([24; 16]),
            4 => changed.server_account = AccountId::from_bytes([25; 16]),
            5 => changed.server_device = DeviceId::from_bytes([26; 16]),
            6 => changed.context.session[0] ^= 1,
            7 => changed.context.scope.history = HistoryId::from_bytes([27; 16]),
            8 => changed.context.ruleset[0] ^= 1,
            9 => changed.context.content[0] ^= 1,
            10 => changed.client_nonce[0] ^= 1,
            11 => changed.server_nonce[0] ^= 1,
            _ => unreachable!(),
        }
        assert_ne!(changed.challenge(1, 12, PROTOCOL).unwrap(), original);
    }
    assert_ne!(h.challenge(1, 13, PROTOCOL).unwrap(), original);
    assert_ne!(h.challenge(2, 12, PROTOCOL).unwrap(), original);
    let notice = NotifyNoticeTranscript {
        context_digest: context_digest(),
        subscription: [21; 16],
        sequence: 1,
        selector: selector(),
        nonce: [22; 32],
        frontier: 12,
    };
    let digest = notice.challenge(1, [31; 32]).unwrap();
    for field in 0..9 {
        let mut changed = notice.clone();
        match field {
            0 => changed.context_digest[0] ^= 1,
            1 => changed.subscription[0] ^= 1,
            2 => changed.sequence += 1,
            3 => changed.selector.request = RequestId::from_bytes([32; 16]),
            4 => changed.selector.operation = OperationId::from_bytes([33; 16]),
            5 => changed.selector.binding[0] ^= 1,
            6 => changed.nonce[0] ^= 1,
            7 => changed.frontier += 1,
            8 => {
                assert_ne!(changed.challenge(1, [34; 32]).unwrap(), digest);
                continue;
            }
            _ => unreachable!(),
        }
        assert_ne!(changed.challenge(1, [31; 32]).unwrap(), digest);
    }
    assert_ne!(notice.challenge(2, [31; 32]).unwrap(), digest);
}
