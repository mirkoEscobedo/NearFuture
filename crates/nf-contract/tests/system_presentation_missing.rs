use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::PlacementRegistry,
    system_presentation::{
        ClientPresentation, PresentationDomain, PresentationError, PresentationProfile,
    },
    system_publication::{
        PublicationHistory, PublicationKind, PublicationLimits, PublicationOutcome,
        PublicationProfile, PublicationRecord,
    },
    system_topology::{self, Kind, Limits, Topology},
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

fn assert_input(value: &Topology, original: &Topology, literal: &[u8], sha: &str, revision: u64) {
    assert_eq!(value, original);
    let (history, owner, system) = key();
    assert_eq!(value.universe, universe());
    assert_eq!(value.history, history);
    assert_eq!(value.owner, owner);
    assert_eq!(value.system, system);
    assert_eq!(value.revision, revision);
    assert_eq!(value.campaign_day_millis, 2);
    assert_eq!(value.dependencies.len(), 1);
    assert_eq!(value.dependencies[0].semantic_key, [0x77; 32]);
    assert!(value.dependencies[0].required);
    assert_eq!(value.entities.len(), 2);
    let star = &value.entities[0];
    assert_eq!(star.id, EntityId::from_bytes([0x01; 16]));
    assert_eq!(star.kind, Kind::Star);
    assert_eq!(star.parent, None);
    assert_eq!(star.dependency, None);
    assert_eq!(star.label.as_deref(), Some("Sun"));
    let planet = &value.entities[1];
    assert_eq!(planet.id, EntityId::from_bytes([0x02; 16]));
    assert_eq!(planet.kind, Kind::Planet);
    assert_eq!(planet.parent, Some(EntityId::from_bytes([0x01; 16])));
    assert_eq!(planet.dependency, Some(0));
    assert_eq!(planet.label.as_deref(), Some("World"));
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

fn assert_empty_registry(registry: &PlacementRegistry) {
    assert_eq!(registry.universe(), universe());
    assert_eq!(registry.capacity(), 2);
    assert!(registry.entries().is_empty());
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

fn assert_sources(
    a1: &Topology,
    a2: &Topology,
    originals: &[Topology; 2],
    literals: [&[u8]; 2],
    history: &PublicationHistory,
    original_history: &PublicationHistory,
) {
    assert_input(a1, &originals[0], literals[0], A1_SHA, 1);
    assert_input(a2, &originals[1], literals[1], A2_SHA, 2);
    assert_eq!(history, original_history);
    assert_held_history(history, literals[0], literals[1]);
}

#[test]
fn empty_real_registry_refuses_projection_without_mutating_either_client() {
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
    assert_input(&a1, &originals[0], &a1_bytes, A1_SHA, 1);
    assert_input(&a2, &originals[1], &a2_bytes, A2_SHA, 2);

    // This History is an independent nonmutation guard, never projection admission.
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
    let literals = [a1_bytes.as_slice(), a2_bytes.as_slice()];

    let registry = PlacementRegistry::new(universe(), 2).unwrap();
    assert_empty_registry(&registry);
    let registry_before = registry.clone();
    let query = key();
    let query_before = query;
    let client_a = ClientPresentation::new(universe(), CLIENT_A).unwrap();
    let client_b = ClientPresentation::new(universe(), CLIENT_B).unwrap();
    assert_layout(&client_a, CLIENT_A);
    assert_layout(&client_b, CLIENT_B);
    let layout_a_before = client_a.clone();
    let layout_b_before = client_b.clone();

    assert_eq!(
        client_a.project(&registry, query.0, query.1, query.2),
        Err(PresentationError::MissingPlacement)
    );
    assert_eq!(registry, registry_before);
    assert_empty_registry(&registry);
    assert_eq!(client_a, layout_a_before);
    assert_eq!(client_b, layout_b_before);
    assert_layout(&client_a, CLIENT_A);
    assert_layout(&client_b, CLIENT_B);
    assert_eq!(query, query_before);
    assert_eq!(query, key());
    assert_eq!(limit_values(topology_limits), original_limits);
    assert_sources(
        &a1,
        &a2,
        &originals,
        literals,
        &held_history,
        &history_before,
    );

    assert_eq!(
        client_b.project(&registry, query.0, query.1, query.2),
        Err(PresentationError::MissingPlacement)
    );
    assert_eq!(registry, registry_before);
    assert_empty_registry(&registry);
    assert_eq!(client_a, layout_a_before);
    assert_eq!(client_b, layout_b_before);
    assert_layout(&client_a, CLIENT_A);
    assert_layout(&client_b, CLIENT_B);
    assert_eq!(query, query_before);
    assert_eq!(query, key());
    assert_eq!(limit_values(topology_limits), original_limits);
    assert_sources(
        &a1,
        &a2,
        &originals,
        literals,
        &held_history,
        &history_before,
    );
}
