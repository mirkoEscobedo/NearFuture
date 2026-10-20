use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{ConceptualSlot, PlacementEntry, PlacementRegistry, ReservationOutcome},
    system_presentation::{
        ClientPresentation, PresentationDomain, PresentationProfile, ProjectedPlacement,
    },
    system_publication::{
        PublicationHistory, PublicationKind, PublicationLimits, PublicationOutcome,
        PublicationProfile, PublicationRecord,
    },
    system_topology::{self, Kind, Limits, Topology},
};

const A1_SHA: &str = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
const A1_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440100000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const A2_SHA: &str = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
const A2_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440200000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const FIXTURES: [(&str, &str, u64); 2] = [(A1_HEX, A1_SHA, 1), (A2_HEX, A2_SHA, 2)];
const DOMAINS: [PresentationDomain; 2] = [
    PresentationDomain {
        origin_x: 1_099_511_627_776,
        origin_y: 1_099_511_627_776,
        width: 16_777_216,
        height: 16_777_216,
        half_extent: 32_767,
    },
    PresentationDomain {
        origin_x: -1_099_511_627_776,
        origin_y: -1_099_511_627_776,
        width: 16_777_216,
        height: 16_777_216,
        half_extent: 32_767,
    },
];
const HISTORY_LIMITS: PublicationLimits = PublicationLimits {
    keys: 2,
    records: 3,
    canonical_bytes: 597,
};
const ROWS: [(usize, u16, i64, i64); 2] = [
    (0, 65_535, 1_099_528_372_224, 1_099_528_372_224),
    (1, 0, -1_099_511_595_008, -1_099_511_595_008),
];
type Key = (HistoryId, AccountId, EntityId);
type OutputImage = (
    PresentationProfile,
    UniverseId,
    Key,
    u16,
    u64,
    [u8; 32],
    PresentationDomain,
    i64,
    i64,
);

fn bytes(hex: &str) -> Vec<u8> {
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

fn key() -> Key {
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

fn assert_input(value: &Topology, literal: &[u8], index: usize, limits: Limits) {
    let (hex, sha, revision) = FIXTURES[index];
    let (history, owner, system) = key();
    assert_eq!(value.universe, universe());
    assert_eq!(
        (value.history, value.owner, value.system),
        (history, owner, system)
    );
    assert_eq!((value.revision, value.campaign_day_millis), (revision, 2));
    assert_eq!(value.dependencies.len(), 1);
    assert_eq!(value.dependencies[0].semantic_key, [0x77; 32]);
    assert!(value.dependencies[0].required);
    assert_eq!(value.entities.len(), 2);
    let star = &value.entities[0];
    let planet = &value.entities[1];
    assert_eq!(
        (
            star.id,
            star.kind,
            star.parent,
            star.dependency,
            star.label.as_deref()
        ),
        (
            EntityId::from_bytes([1; 16]),
            Kind::Star,
            None,
            None,
            Some("Sun")
        )
    );
    assert_eq!(
        (
            planet.id,
            planet.kind,
            planet.parent,
            planet.dependency,
            planet.label.as_deref()
        ),
        (
            EntityId::from_bytes([2; 16]),
            Kind::Planet,
            Some(EntityId::from_bytes([1; 16])),
            Some(0),
            Some("World"),
        )
    );
    assert_eq!(literal.len(), 199);
    assert_eq!(literal, bytes(hex).as_slice());
    assert_eq!(system_topology::validate(value, limits), Ok(()));
    assert_eq!(
        system_topology::encode(value, limits).unwrap().as_slice(),
        bytes(hex).as_slice()
    );
    assert_eq!(
        system_topology::digest(value, limits).unwrap().as_slice(),
        bytes(sha).as_slice()
    );
}

fn assert_record(record: &PublicationRecord, index: usize) {
    let (hex, sha, revision) = FIXTURES[index];
    let (h, o, s) = key();
    assert_eq!(record.universe(), universe());
    assert_eq!(
        (record.history(), record.owner(), record.system()),
        (h, o, s)
    );
    assert_eq!(record.profile(), PublicationProfile::SelectedTopologyV1);
    assert_eq!(record.kind(), PublicationKind::PublishedVersion);
    assert_eq!(record.revision(), revision);
    assert_eq!(record.referenced_publication_revision(), revision);
    assert_eq!(
        record.referenced_topology_sha256().as_slice(),
        bytes(sha).as_slice()
    );
    assert_eq!(
        record.canonical_topology_bytes(),
        Some(bytes(hex).as_slice())
    );
}

fn assert_history(history: &PublicationHistory, count: usize) {
    assert_eq!(history.universe(), universe());
    assert_eq!(history.limits(), HISTORY_LIMITS);
    assert_eq!(history.keys_len(), 1);
    assert_eq!(history.records().len(), count);
    assert_eq!(history.canonical_bytes_len(), [0, 199, 398][count]);
    for (index, record) in history.records().iter().enumerate() {
        assert_record(record, index);
    }
    let (h, o, s) = key();
    assert_record(history.latest(h, o, s).unwrap(), count - 1);
}

fn output_image(output: &ProjectedPlacement) -> OutputImage {
    (
        output.profile(),
        output.universe(),
        (output.history(), output.owner(), output.system()),
        output.slot().value(),
        output.revision(),
        output.topology_sha256(),
        output.domain(),
        output.x(),
        output.y(),
    )
}

fn assert_epoch(epoch: &ClientPresentation, fixture: usize) {
    assert_eq!(epoch.profile(), PresentationProfile::Grid256V1);
    assert_eq!(epoch.universe(), universe());
    assert_eq!(epoch.domain(), DOMAINS[fixture]);
    assert_eq!(
        format!("{epoch:?}"),
        "ClientPresentation { profile: Grid256V1 }"
    );
}

fn assert_registry(registry: &PlacementRegistry, slot: u16) {
    assert_eq!(registry.universe(), universe());
    assert_eq!(registry.capacity(), 1);
    assert_eq!(registry.entries().len(), 1);
    let entry = &registry.entries()[0];
    assert_eq!((entry.history(), entry.owner(), entry.system()), key());
    assert_eq!(entry.slot().value(), slot);
    assert_eq!(entry.revision(), 1);
    assert_eq!(entry.topology_sha256().as_slice(), bytes(A1_SHA).as_slice());
}

fn assert_output(output: &ProjectedPlacement, index: usize) {
    let (fixture, slot, x, y) = ROWS[index];
    assert_eq!(output.profile(), PresentationProfile::Grid256V1);
    assert_eq!(output.universe(), universe());
    assert_eq!((output.history(), output.owner(), output.system()), key());
    assert_eq!(output.slot().value(), slot);
    assert_eq!(output.revision(), 1);
    assert_eq!(
        output.topology_sha256().as_slice(),
        bytes(A1_SHA).as_slice()
    );
    assert_eq!(output.domain(), DOMAINS[fixture]);
    assert_eq!((output.x(), output.y()), (x, y));
    assert_eq!(
        format!("{output:?}"),
        "ProjectedPlacement { profile: Grid256V1 }"
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceImage {
    topologies: [Topology; 2],
    literals: [Vec<u8>; 2],
    limits: [usize; 4],
    query: Key,
    epochs: [ClientPresentation; 2],
    history: PublicationHistory,
    records: Vec<PublicationRecord>,
    latest: Option<PublicationRecord>,
}

struct Inputs {
    topologies: [Topology; 2],
    literals: [Vec<u8>; 2],
    limits: Limits,
    query: Key,
    epochs: [ClientPresentation; 2],
    history: PublicationHistory,
}

impl Inputs {
    fn new() -> Self {
        let limits = Limits::default();
        let literals = [bytes(A1_HEX), bytes(A2_HEX)];
        let topologies = [
            system_topology::decode(&literals[0], limits).unwrap(),
            system_topology::decode(&literals[1], limits).unwrap(),
        ];
        // Held History is independent evidence, never projection admission.
        let mut history = PublicationHistory::new(universe(), HISTORY_LIMITS).unwrap();
        assert_eq!(history.universe(), universe());
        assert_eq!(history.limits(), HISTORY_LIMITS);
        assert!(history.records().is_empty());
        assert_eq!(history.keys_len(), 0);
        assert_eq!(history.canonical_bytes_len(), 0);
        for (index, topology) in topologies.iter().enumerate() {
            assert_input(topology, &literals[index], index, limits);
            assert_eq!(
                history.append_publication(topology, limits),
                Ok(PublicationOutcome::PublicationAppended)
            );
            assert_history(&history, index + 1);
        }
        let value = Self {
            topologies,
            literals,
            limits,
            query: key(),
            epochs: [
                ClientPresentation::new(universe(), DOMAINS[0]).unwrap(),
                ClientPresentation::new(universe(), DOMAINS[1]).unwrap(),
            ],
            history,
        };
        value.assert_fixed();
        value
    }

    fn image(&self) -> SourceImage {
        let (h, o, s) = self.query;
        SourceImage {
            topologies: self.topologies.clone(),
            literals: self.literals.clone(),
            limits: limit_values(self.limits),
            query: self.query,
            epochs: self.epochs.clone(),
            history: self.history.clone(),
            records: self.history.records().to_vec(),
            latest: self.history.latest(h, o, s).cloned(),
        }
    }

    fn assert_fixed(&self) {
        assert_eq!(limit_values(self.limits), [128, 64, 128, 65_536]);
        assert_eq!(self.query, key());
        for index in 0..2 {
            assert_input(
                &self.topologies[index],
                &self.literals[index],
                index,
                self.limits,
            );
        }
        for (index, epoch) in self.epochs.iter().enumerate() {
            assert_epoch(epoch, index);
        }
        assert_history(&self.history, 2);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Retained {
    registry: PlacementRegistry,
    entries: Vec<PlacementEntry>,
    epoch: ClientPresentation,
    output: ProjectedPlacement,
    getters: OutputImage,
}

fn assert_retained(retained: &[Retained]) {
    assert!(retained.len() <= 2);
    for (index, item) in retained.iter().enumerate() {
        let (fixture, slot, _, _) = ROWS[index];
        assert_registry(&item.registry, slot);
        assert_eq!(item.registry.entries(), item.entries);
        assert_epoch(&item.epoch, fixture);
        assert_output(&item.output, index);
        assert_eq!(output_image(&item.output), item.getters);
    }
}

#[test]
fn origin_dimension_and_half_extent_boundaries_preserve_sources() {
    let inputs = Inputs::new();
    let initial = inputs.image();
    let mut retained: Vec<Retained> = Vec::new();
    let mut creation_images: Vec<Retained> = Vec::new();
    for (index, &(fixture, raw_slot, _, _)) in ROWS.iter().enumerate() {
        inputs.assert_fixed();
        assert_retained(&retained);
        assert_retained(&creation_images);
        assert_eq!(retained, creation_images);
        let source_before = inputs.image();
        let previous = retained.clone();
        let query_before = inputs.query;
        let slot = ConceptualSlot::new(raw_slot);
        let slot_before = slot;
        let epoch = inputs.epochs[fixture].clone();
        assert_epoch(&epoch, fixture);
        let epoch_before = epoch.clone();
        let mut registry = PlacementRegistry::new(universe(), 1).unwrap();
        assert_eq!(registry.universe(), universe());
        assert_eq!(registry.capacity(), 1);
        assert!(registry.entries().is_empty());
        assert_eq!(
            registry.reserve(&inputs.topologies[0], slot, inputs.limits),
            Ok(ReservationOutcome::Inserted)
        );
        inputs.assert_fixed();
        assert_eq!(inputs.image(), source_before);
        assert_eq!(retained, previous);
        assert_eq!(retained, creation_images);
        assert_retained(&retained);
        assert_registry(&registry, raw_slot);
        assert_eq!(slot, slot_before);
        assert_eq!(epoch, epoch_before);
        let registry_before = registry.clone();
        let entries_before = registry.entries().to_vec();
        let (h, o, s) = inputs.query;
        let output = epoch.project(&registry, h, o, s).unwrap();
        inputs.assert_fixed();
        assert_eq!(inputs.image(), source_before);
        assert_eq!(inputs.query, query_before);
        assert_eq!(slot, slot_before);
        assert_eq!(epoch, epoch_before);
        assert_epoch(&epoch, fixture);
        assert_eq!(registry, registry_before);
        assert_eq!(registry.entries(), entries_before);
        assert_registry(&registry, raw_slot);
        assert_eq!(retained, previous);
        assert_eq!(retained, creation_images);
        assert_retained(&retained);
        assert_output(&output, index);
        let getters = output_image(&output);
        let item = Retained {
            registry,
            entries: entries_before,
            epoch,
            output,
            getters,
        };
        // Capture this fixture once; older creation images are never refreshed.
        creation_images.push(item.clone());
        retained.push(item);
        assert_eq!(retained.len(), index + 1);
        assert_eq!(creation_images.len(), index + 1);
        assert_eq!(retained, creation_images);
        assert_retained(&retained);
    }
    assert_eq!(retained.len(), 2);
    assert_eq!(creation_images.len(), 2);
    assert_eq!(retained, creation_images);
    assert_retained(&creation_images);
    inputs.assert_fixed();
    assert_eq!(inputs.image(), initial);
    assert_retained(&retained);
}
