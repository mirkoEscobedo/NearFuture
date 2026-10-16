use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{ConceptualSlot, PlacementRegistry, ReservationOutcome},
    system_presentation::{
        ClientPresentation, PresentationDomain, PresentationProfile, ProjectedPlacement,
    },
    system_publication::{
        PublicationHistory, PublicationKind, PublicationLimits, PublicationOutcome,
        PublicationProfile, PublicationRecord,
    },
    system_topology::{self, Limits, Topology},
};

const A1_SHA: &str = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
const A2_SHA: &str = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
const A1_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440100000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const A2_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440200000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const CLIENT_A: PresentationDomain = PresentationDomain {
    origin_x: -1024,
    origin_y: -2048,
    width: 8192,
    height: 16384,
    half_extent: 7,
};
const CLIENT_B: PresentationDomain = PresentationDomain {
    origin_x: 4096,
    origin_y: -4096,
    width: 16384,
    height: 8192,
    half_extent: 7,
};
const HISTORY_LIMITS: PublicationLimits = PublicationLimits {
    keys: 2,
    records: 3,
    canonical_bytes: 597,
};

fn bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn universe() -> UniverseId {
    UniverseId::from_bytes([0x11; 16])
}

fn key() -> (HistoryId, AccountId, EntityId) {
    (
        HistoryId::from_bytes([0x22; 16]),
        AccountId::from_bytes([0x33; 16]),
        EntityId::from_bytes([0x44; 16]),
    )
}

fn limit_values(limits: Limits) -> [usize; 4] {
    [
        limits.entities,
        limits.dependencies,
        limits.label_bytes,
        limits.document_bytes,
    ]
}

fn assert_input(value: &Topology, original: &Topology, literal: &[u8], sha: &str) {
    assert_eq!(value, original);
    assert_eq!(system_topology::validate(value, Limits::default()), Ok(()));
    assert_eq!(
        system_topology::encode(value, Limits::default())
            .unwrap()
            .as_slice(),
        literal
    );
    assert_eq!(
        system_topology::digest(value, Limits::default())
            .unwrap()
            .as_slice(),
        bytes(sha).as_slice()
    );
}

fn assert_registry(registry: &PlacementRegistry) {
    let (history, owner, system) = key();
    assert_eq!(registry.universe(), universe());
    assert_eq!(registry.capacity(), 2);
    assert_eq!(registry.entries().len(), 1);
    let entry = &registry.entries()[0];
    assert_eq!(entry.history(), history);
    assert_eq!(entry.owner(), owner);
    assert_eq!(entry.system(), system);
    assert_eq!(entry.slot().value(), 7);
    assert_eq!(entry.revision(), 1);
    assert_eq!(entry.topology_sha256().as_slice(), bytes(A1_SHA).as_slice());
}

fn assert_history_counts(history: &PublicationHistory, records: usize, used_bytes: usize) {
    assert_eq!(history.universe(), universe());
    assert_eq!(history.limits(), HISTORY_LIMITS);
    assert_eq!(history.keys_len(), 1);
    assert_eq!(history.records().len(), records);
    assert_eq!(history.canonical_bytes_len(), used_bytes);
}

fn assert_history_record(record: &PublicationRecord, revision: u64, sha: &str, body: &[u8]) {
    let (history, owner, system) = key();
    assert_eq!(record.universe(), universe());
    assert_eq!(record.history(), history);
    assert_eq!(record.owner(), owner);
    assert_eq!(record.system(), system);
    assert_eq!(record.profile(), PublicationProfile::SelectedTopologyV1);
    assert_eq!(record.kind(), PublicationKind::PublishedVersion);
    assert_eq!(record.revision(), revision);
    assert_eq!(record.referenced_publication_revision(), revision);
    assert_eq!(
        record.referenced_topology_sha256().as_slice(),
        bytes(sha).as_slice()
    );
    assert_eq!(record.canonical_topology_bytes(), Some(body));
}

fn assert_held_history(history: &PublicationHistory, a1: &[u8], a2: &[u8]) {
    assert_history_counts(history, 2, 398);
    assert_history_record(&history.records()[0], 1, A1_SHA, a1);
    assert_history_record(&history.records()[1], 2, A2_SHA, a2);
    let (h, o, s) = key();
    assert_history_record(history.latest(h, o, s).unwrap(), 2, A2_SHA, a2);
}

fn assert_domain(actual: PresentationDomain, expected: PresentationDomain) {
    assert_eq!(actual.origin_x, expected.origin_x);
    assert_eq!(actual.origin_y, expected.origin_y);
    assert_eq!(actual.width, expected.width);
    assert_eq!(actual.height, expected.height);
    assert_eq!(actual.half_extent, expected.half_extent);
}

fn assert_layout(layout: &ClientPresentation, domain: PresentationDomain) {
    assert_eq!(layout.profile(), PresentationProfile::Grid256V1);
    assert_eq!(layout.universe(), universe());
    assert_domain(layout.domain(), domain);
    assert_eq!(
        format!("{layout:?}"),
        "ClientPresentation { profile: Grid256V1 }"
    );
}

fn assert_projected(output: &ProjectedPlacement, domain: PresentationDomain, x: i64, y: i64) {
    let (history, owner, system) = key();
    assert_eq!(output.profile(), PresentationProfile::Grid256V1);
    assert_eq!(output.universe(), universe());
    assert_eq!(output.history(), history);
    assert_eq!(output.owner(), owner);
    assert_eq!(output.system(), system);
    assert_eq!(output.slot().value(), 7);
    assert_eq!(output.revision(), 1);
    assert_eq!(output.topology_sha256().as_slice(), bytes(A1_SHA).as_slice());
    assert_domain(output.domain(), domain);
    assert_eq!(output.x(), x);
    assert_eq!(output.y(), y);
    assert_eq!(
        format!("{output:?}"),
        "ProjectedPlacement { profile: Grid256V1 }"
    );
}

#[test]
fn real_reservation_projects_two_clients_with_owned_source_binding() {
    let topology_limits = Limits::default();
    let original_limits = limit_values(topology_limits);
    assert_eq!(original_limits, [128, 64, 128, 65_536]);
    let a1_bytes = bytes(A1_HEX);
    let a2_bytes = bytes(A2_HEX);
    assert_eq!(a1_bytes.len(), 199);
    assert_eq!(a2_bytes.len(), 199);
    let a1 = system_topology::decode(&a1_bytes, topology_limits).unwrap();
    let a2 = system_topology::decode(&a2_bytes, topology_limits).unwrap();
    let originals = [a1.clone(), a2.clone()];
    assert_input(&a1, &originals[0], &a1_bytes, A1_SHA);
    assert_input(&a2, &originals[1], &a2_bytes, A2_SHA);

    // This independent History is only a nonmutation guard, not projection admission.
    let mut held_history = PublicationHistory::new(universe(), HISTORY_LIMITS).unwrap();
    assert_eq!(
        held_history.append_publication(&a1, topology_limits),
        Ok(PublicationOutcome::PublicationAppended)
    );
    assert_history_counts(&held_history, 1, 199);
    assert_history_record(&held_history.records()[0], 1, A1_SHA, &a1_bytes);
    let (h, o, s) = key();
    assert_history_record(held_history.latest(h, o, s).unwrap(), 1, A1_SHA, &a1_bytes);
    assert_eq!(
        held_history.append_publication(&a2, topology_limits),
        Ok(PublicationOutcome::PublicationAppended)
    );
    assert_held_history(&held_history, &a1_bytes, &a2_bytes);
    let history_before = held_history.clone();

    let mut registry = PlacementRegistry::new(universe(), 2).unwrap();
    assert_eq!(
        registry.reserve(&a1, ConceptualSlot::new(7), topology_limits),
        Ok(ReservationOutcome::Inserted)
    );
    assert_registry(&registry);
    let registry_before = registry.clone();
    let query = key();
    let query_before = query;
    let client_a = ClientPresentation::new(universe(), CLIENT_A).unwrap();
    let client_b = ClientPresentation::new(universe(), CLIENT_B).unwrap();
    assert_layout(&client_a, CLIENT_A);
    assert_layout(&client_b, CLIENT_B);
    let layout_a_before = client_a.clone();
    let layout_b_before = client_b.clone();

    let first = client_a
        .project(&registry, query.0, query.1, query.2)
        .unwrap();
    // Fixed independent expectations: no test-side coordinate formula.
    assert_projected(&first, CLIENT_A, -784, -2016);
    let first_before = first.clone();
    let first_hash_before = first.topology_sha256();
    let first_domain_before = first.domain();
    assert_eq!(registry, registry_before);
    assert_eq!(held_history, history_before);
    assert_eq!(client_a, layout_a_before);
    assert_eq!(client_b, layout_b_before);
    assert_eq!(query, query_before);
    assert_eq!(limit_values(topology_limits), original_limits);
    assert_registry(&registry);
    assert_held_history(&held_history, &a1_bytes, &a2_bytes);
    assert_input(&a1, &originals[0], &a1_bytes, A1_SHA);
    assert_input(&a2, &originals[1], &a2_bytes, A2_SHA);

    let second = client_b
        .project(&registry, query.0, query.1, query.2)
        .unwrap();
    assert_projected(&second, CLIENT_B, 4576, -4080);
    let second_before = second.clone();
    let mut local_hash_copy = first.topology_sha256();
    local_hash_copy[0] ^= 0x80;
    assert_ne!(local_hash_copy, first_hash_before);
    assert_eq!(first.topology_sha256(), first_hash_before);
    assert_domain(first.domain(), first_domain_before);
    assert_eq!(first, first_before);
    assert_eq!(second, second_before);
    assert_projected(&first, CLIENT_A, -784, -2016);
    assert_projected(&second, CLIENT_B, 4576, -4080);
    assert_eq!(registry, registry_before);
    assert_eq!(held_history, history_before);
    assert_eq!(client_a, layout_a_before);
    assert_eq!(client_b, layout_b_before);
    assert_layout(&client_a, CLIENT_A);
    assert_layout(&client_b, CLIENT_B);
    assert_eq!(query, query_before);
    assert_eq!(limit_values(topology_limits), original_limits);
    assert_registry(&registry);
    assert_held_history(&held_history, &a1_bytes, &a2_bytes);
    assert_input(&a1, &originals[0], &a1_bytes, A1_SHA);
    assert_input(&a2, &originals[1], &a2_bytes, A2_SHA);
}
