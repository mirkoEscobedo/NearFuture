use libp2p::identity;
use nf_transport::portal_config::{PortalConfig, PortalMode};
fn peer() -> String {
    identity::Keypair::ed25519_from_bytes([0x31; 32])
        .unwrap()
        .public()
        .to_peer_id()
        .to_string()
}
fn common(mode: &str) -> String {
    format!(
        "NF-PORTAL-CONFIG-1\nmode {mode}\nuniverse {}\nhistory {}\nruleset {}\ncontent {}\nlocal_account {}\nlocal_device {}\nlocal_event_min 7\nlocal_store_min 9\nlocal_membership_min 3\nserver_peer {}\nserver_account {}\nserver_device {}\nserver_membership_min 5\n",
        "01".repeat(16),
        "02".repeat(16),
        "03".repeat(32),
        "04".repeat(32),
        "05".repeat(16),
        "06".repeat(16),
        peer(),
        "07".repeat(16),
        "08".repeat(16)
    )
}
fn watch() -> String {
    format!(
        "{}receipt_address /ip4/127.0.0.1/tcp/12001/p2p/{}\nbulk_address /ip4/127.0.0.1/tcp/12002/p2p/{}\nnotification_address /ip4/127.0.0.1/tcp/12003/p2p/{}\nanchor_count 1\nanchor 2 4 {} {} {} 1 {} 100 120 6\nEND\n",
        common("watch"),
        peer(),
        peer(),
        peer(),
        "0a".repeat(32),
        "0b".repeat(16),
        "0c".repeat(16),
        "0d".repeat(32)
    )
}
#[test]
fn trusted_watch_input_preserves_original_and_separate_frontiers() {
    let parsed = PortalConfig::parse(watch().as_bytes(), PortalMode::Watch).unwrap();
    assert_eq!(parsed.local.event_sequence.0, 7);
    assert_eq!(parsed.local.store_revision, 9);
    assert_eq!(parsed.local.membership_revision, Some(3));
    assert_eq!(parsed.protected_membership().unwrap(), 5);
    let row = parsed.selected_original(2).unwrap();
    assert_eq!(row.minimum.event.0, 100);
    assert_eq!(row.minimum.membership_revision, 6);
    assert_eq!(row.original.original().account_id, parsed.local_account);
    assert_eq!(row.original.original().device_id, parsed.local_device);
    assert_eq!(row.original.source().peer, parsed.server.peer);
    assert_eq!(row.head, Some((4, [0x0a; 32])));
    assert!(parsed.selected_original(3).is_err());
}
fn serve() -> String {
    format!(
        "{}receipt_listen /ip4/127.0.0.1/tcp/0\nbulk_listen /ip4/127.0.0.1/tcp/0\nnotification_listen /ip4/127.0.0.1/tcp/0\nanchor_count 0\nEND\n",
        common("serve")
    )
}
fn init() -> String {
    format!(
        "{}original_count 1\noriginal 2 {} {} 3 {} 100 120 6\nEND\n",
        common("init-book"),
        "0b".repeat(16),
        "0c".repeat(16),
        "0d".repeat(32)
    )
}
#[test]
fn serve_ephemeral_endpoints_and_explicit_offline_originals_are_distinct_modes() {
    let server = PortalConfig::parse(serve().as_bytes(), PortalMode::Serve).unwrap();
    assert!(server.originals.is_empty());
    assert_eq!(server.addresses.unwrap().len(), 3);
    let original = PortalConfig::parse(init().as_bytes(), PortalMode::InitializeBook).unwrap();
    assert!(original.addresses.is_none());
    assert_eq!(original.originals[0].original.original().operation_kind, 3);
    assert!(original.originals[0].head.is_none());
    assert!(PortalConfig::parse(watch().as_bytes(), PortalMode::Serve).is_err());
    let mut incomplete = original;
    incomplete.local.membership_revision = None;
    assert!(incomplete.protected_membership().is_err());
}
#[test]
fn missing_reordered_duplicate_unknown_and_surplus_fields_refuse() {
    let valid = watch();
    let lines: Vec<_> = valid.lines().collect();
    for omit in 0..lines.len() {
        let candidate = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != omit)
            .map(|(_, line)| *line)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert!(
            PortalConfig::parse(candidate.as_bytes(), PortalMode::Watch).is_err(),
            "missing row {omit}"
        );
    }
    for swap in 0..lines.len() - 1 {
        let mut candidate = lines.clone();
        candidate.swap(swap, swap + 1);
        assert!(
            PortalConfig::parse((candidate.join("\n") + "\n").as_bytes(), PortalMode::Watch)
                .is_err(),
            "reordered row {swap}"
        );
    }
    for extra in ["unknown 1\n", "mode watch\n", "END\n"] {
        let candidate = valid.replace("END\n", &format!("{extra}END\n"));
        assert!(PortalConfig::parse(candidate.as_bytes(), PortalMode::Watch).is_err());
    }
}
#[test]
fn exact_ascii_line_and_input_bounds_refuse_noncanonical_text() {
    let valid = watch();
    let cases = [
        valid.trim_end().to_string(),
        valid.replace('\n', "\r\n"),
        format!("\u{feff}{valid}"),
        valid.replace("mode watch", "mode\twatch"),
        valid.replace("mode watch", "mode  watch"),
        valid.replace("END\n", "END \n"),
        format!("{valid}\n"),
        valid.replace("mode watch", "mode wátch"),
        valid.replace("END\n", "\0END\n"),
    ];
    for candidate in cases {
        assert!(PortalConfig::parse(candidate.as_bytes(), PortalMode::Watch).is_err());
    }
    for length in [16_384, 16_385] {
        assert!(PortalConfig::parse(&vec![b'x'; length], PortalMode::Watch).is_err());
    }
    for bytes in [&b""[..], &b"\xff\n"[..], &b"\n"[..]] {
        assert!(PortalConfig::parse(bytes, PortalMode::Watch).is_err());
    }
}
#[test]
fn canonical_counter_extremes_and_original_identity_rules_hold() {
    let valid = watch();
    let max = valid.replace("local_store_min 9", "local_store_min 18446744073709551615");
    assert_eq!(
        PortalConfig::parse(max.as_bytes(), PortalMode::Watch)
            .unwrap()
            .local
            .store_revision,
        u64::MAX
    );
    for counter in ["00", "09", "-1", "+1", "18446744073709551616", "1e3", ""] {
        assert!(
            PortalConfig::parse(
                valid
                    .replace("local_store_min 9", &format!("local_store_min {counter}"))
                    .as_bytes(),
                PortalMode::Watch
            )
            .is_err()
        );
    }
    for key in [
        "universe",
        "history",
        "local_account",
        "local_device",
        "server_account",
        "server_device",
    ] {
        let line = valid
            .lines()
            .find(|line| line.starts_with(&format!("{key} ")))
            .unwrap();
        for replacement in ["00".repeat(16), "A1".repeat(16), "01".repeat(15)] {
            assert!(
                PortalConfig::parse(
                    valid
                        .replace(line, &format!("{key} {replacement}"))
                        .as_bytes(),
                    PortalMode::Watch
                )
                .is_err()
            );
        }
    }
    for kind in ["0", "4", "01"] {
        let candidate = valid.replace(" 1 0d", &format!(" {kind} 0d"));
        assert!(PortalConfig::parse(candidate.as_bytes(), PortalMode::Watch).is_err());
    }
    let zero_hashes = valid
        .replace(&"03".repeat(32), &"00".repeat(32))
        .replace(&"04".repeat(32), &"00".repeat(32))
        .replace(&"0d".repeat(32), &"00".repeat(32));
    assert_eq!(
        PortalConfig::parse(zero_hashes.as_bytes(), PortalMode::Watch)
            .unwrap()
            .ruleset,
        [0; 32]
    );
    assert!(
        PortalConfig::parse(
            valid.replace(&"0a".repeat(32), &"00".repeat(32)).as_bytes(),
            PortalMode::Watch
        )
        .is_err()
    );
}
#[test]
fn remote_loopback_pin_and_distinct_addresses_are_required() {
    let valid = watch();
    for address in [
        format!("/ip4/127.0.0.1/tcp/0/p2p/{}", peer()),
        format!("/ip4/126.0.0.1/tcp/12001/p2p/{}", peer()),
        format!("/ip4/127.00.0.1/tcp/12001/p2p/{}", peer()),
        format!("/ip4/127.0.0.1/tcp/012001/p2p/{}", peer()),
        format!("/ip4/127.0.0.1/tcp/65536/p2p/{}", peer()),
        "/dns/localhost/tcp/12001".to_string(),
        "/ip4/127.0.0.1/tcp/12001".to_string(),
        format!("/ip4/127.0.0.1/tcp/12001/p2p/{}/p2p-circuit", peer()),
    ] {
        let candidate = valid.replace(
            &format!("/ip4/127.0.0.1/tcp/12001/p2p/{}", peer()),
            &address,
        );
        assert!(PortalConfig::parse(candidate.as_bytes(), PortalMode::Watch).is_err());
    }
    let other = identity::Keypair::ed25519_from_bytes([0x32; 32])
        .unwrap()
        .public()
        .to_peer_id()
        .to_string();
    assert!(
        PortalConfig::parse(
            valid
                .replace(
                    &format!("tcp/12001/p2p/{}", peer()),
                    &format!("tcp/12001/p2p/{other}")
                )
                .as_bytes(),
            PortalMode::Watch
        )
        .is_err()
    );
    assert!(
        PortalConfig::parse(
            valid.replace("tcp/12002", "tcp/12001").as_bytes(),
            PortalMode::Watch
        )
        .is_err()
    );
    let duplicate_listen = serve().replace("tcp/0", "tcp/12001");
    assert!(PortalConfig::parse(duplicate_listen.as_bytes(), PortalMode::Serve).is_err());
    let independent_listen = serve().replacen("tcp/0", "tcp/12001", 1);
    assert!(PortalConfig::parse(independent_listen.as_bytes(), PortalMode::Serve).is_ok());
}
#[test]
fn rows_are_sorted_unique_bounded_and_bound_to_the_pinned_member_floor() {
    let valid = watch();
    let row = valid
        .lines()
        .find(|line| line.starts_with("anchor "))
        .unwrap();
    for changed in [
        row.replace("anchor 2 4", "anchor 8 4"),
        row.replace("anchor 2 4", "anchor 2 8"),
        row.replace(" 100 120 6", " 100 120 4"),
        row.replace(&"0b".repeat(16), &"00".repeat(16)),
        row.replace(&"0c".repeat(16), &"00".repeat(16)),
        format!("{row} extra"),
    ] {
        assert!(
            PortalConfig::parse(valid.replace(row, &changed).as_bytes(), PortalMode::Watch)
                .is_err()
        );
    }
    for count in ["0", "2", "9", "01"] {
        assert!(
            PortalConfig::parse(
                valid
                    .replace("anchor_count 1", &format!("anchor_count {count}"))
                    .as_bytes(),
                PortalMode::Watch
            )
            .is_err()
        );
    }
    for second in [
        row.to_string(),
        row.replace("anchor 2 4", "anchor 1 4"),
        row.replace("anchor 2 4", "anchor 3 4"),
    ] {
        let candidate = valid
            .replace("anchor_count 1", "anchor_count 2")
            .replace("END\n", &format!("{second}\nEND\n"));
        assert!(PortalConfig::parse(candidate.as_bytes(), PortalMode::Watch).is_err());
    }
    let second = row
        .replace("anchor 2 4", "anchor 3 4")
        .replace(&"0b".repeat(16), &"0e".repeat(16));
    let candidate = valid
        .replace("anchor_count 1", "anchor_count 2")
        .replace("END\n", &format!("{second}\nEND\n"));
    assert_eq!(
        PortalConfig::parse(candidate.as_bytes(), PortalMode::Watch)
            .unwrap()
            .originals
            .len(),
        2
    );
}
