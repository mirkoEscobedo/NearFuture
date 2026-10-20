use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{
        ConceptualSlot, PlacementEntry, PlacementError, PlacementRegistry, ReservationOutcome,
    },
    system_presentation::{
        ClientPresentation, PresentationDomain, PresentationError, PresentationProfile,
        ProjectedPlacement,
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
const B1_SHA: &str = "6ea8fd3113fc299431367483e01f808086cdc28062ac9daf9d03a761fdcde821";
const B1_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333555555555555555555555555555555550100000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const FIXTURES: [(&str, &str, u8, u64); 3] = [
    (A1_HEX, A1_SHA, 0x44, 1),
    (A2_HEX, A2_SHA, 0x44, 2),
    (B1_HEX, B1_SHA, 0x55, 1),
];
const CLIENTS: [PresentationDomain; 2] = [
    PresentationDomain {
        origin_x: -1024,
        origin_y: -2048,
        width: 8192,
        height: 16384,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: 4096,
        origin_y: -4096,
        width: 16384,
        height: 8192,
        half_extent: 7,
    },
];
const HISTORY_LIMITS: PublicationLimits = PublicationLimits {
    keys: 2,
    records: 3,
    canonical_bytes: 597,
};
const OUTPUTS: [(usize, u8, u16, u64, &str, i64, i64); 6] = [
    (0, 0x44, 7, 1, A1_SHA, -784, -2016),
    (1, 0x44, 7, 1, A1_SHA, 4576, -4080),
    (0, 0x44, 7, 2, A2_SHA, -784, -2016),
    (1, 0x44, 7, 2, A2_SHA, 4576, -4080),
    (0, 0x55, 8, 1, B1_SHA, -752, -2016),
    (1, 0x55, 8, 1, B1_SHA, 4640, -4080),
];
const OUTPUT_SEQUENCE: [usize; 7] = [0, 1, 2, 3, 4, 5, 2];
const REGISTRY_ROWS: [&[(u8, u16, u64, &str)]; 4] = [
    &[],
    &[(0x44, 7, 1, A1_SHA)],
    &[(0x44, 7, 2, A2_SHA)],
    &[(0x44, 7, 2, A2_SHA), (0x55, 8, 1, B1_SHA)],
];
const FOOTPRINTS: [([i64; 4], [i64; 4], i64, i64); 2] = [
    (
        [-791, -777, -2023, -2009],
        [-759, -745, -2023, -2009],
        18,
        32,
    ),
    (
        [4569, 4583, -4087, -4073],
        [4633, 4647, -4087, -4073],
        50,
        64,
    ),
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

fn key(system: u8) -> Key {
    (
        HistoryId::from_bytes([0x22; 16]),
        AccountId::from_bytes([0x33; 16]),
        EntityId::from_bytes([system; 16]),
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
    let (hex, sha, system, revision) = FIXTURES[index];
    let (history, owner, system) = key(system);
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
    let (hex, sha, _, revision) = FIXTURES[index];
    let (h, o, s) = key(0x44);
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
    let (h, o, s) = key(0x44);
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

fn assert_output(output: &ProjectedPlacement, index: usize) {
    let (client, system, slot, revision, sha, x, y) = OUTPUTS[OUTPUT_SEQUENCE[index]];
    assert_eq!(output.profile(), PresentationProfile::Grid256V1);
    assert_eq!(output.universe(), universe());
    assert_eq!(
        (output.history(), output.owner(), output.system()),
        key(system)
    );
    assert_eq!(output.slot().value(), slot);
    assert_eq!(output.revision(), revision);
    assert_eq!(output.topology_sha256().as_slice(), bytes(sha).as_slice());
    assert_eq!(output.domain(), CLIENTS[client]);
    assert_eq!((output.x(), output.y()), (x, y));
    assert_eq!(
        format!("{output:?}"),
        "ProjectedPlacement { profile: Grid256V1 }"
    );
}

fn footprint(output: &ProjectedPlacement) -> [i64; 4] {
    let radius = i64::from(output.domain().half_extent);
    [
        output.x() - radius,
        output.x() + radius,
        output.y() - radius,
        output.y() + radius,
    ]
}

fn assert_nonoverlap(outputs: &[ProjectedPlacement]) {
    for (client, &(expected_a, expected_b, gap, centers)) in FOOTPRINTS.iter().enumerate() {
        let a = &outputs[2 + client];
        let b = &outputs[4 + client];
        assert_eq!(a.domain(), CLIENTS[client]);
        assert_eq!(b.domain(), CLIENTS[client]);
        let actual_a = footprint(a);
        let actual_b = footprint(b);
        assert_eq!(actual_a, expected_a);
        assert_eq!(actual_b, expected_b);
        assert!(actual_a[1] < actual_b[0]);
        assert_eq!(actual_b[0] - actual_a[1], gap);
        assert_eq!(b.x() - a.x(), centers);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StableImage {
    topologies: [Topology; 3],
    literals: [Vec<u8>; 3],
    limits: [usize; 4],
    queries: [Key; 2],
    slots: [ConceptualSlot; 2],
    epochs: [ClientPresentation; 2],
    history: PublicationHistory,
    records: Vec<PublicationRecord>,
    latest: Option<PublicationRecord>,
    outputs: Vec<ProjectedPlacement>,
    getters: Vec<OutputImage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot {
    registry: PlacementRegistry,
    entries: Vec<PlacementEntry>,
    stable: StableImage,
}

struct Case {
    topologies: [Topology; 3],
    literals: [Vec<u8>; 3],
    limits: Limits,
    queries: [Key; 2],
    slots: [ConceptualSlot; 2],
    registry: PlacementRegistry,
    epochs: [ClientPresentation; 2],
    history: PublicationHistory,
    outputs: Vec<ProjectedPlacement>,
}

impl Case {
    fn new() -> Self {
        let limits = Limits::default();
        let literals = [bytes(A1_HEX), bytes(A2_HEX), bytes(B1_HEX)];
        let topologies = [
            system_topology::decode(&literals[0], limits).unwrap(),
            system_topology::decode(&literals[1], limits).unwrap(),
            system_topology::decode(&literals[2], limits).unwrap(),
        ];
        for index in 0..3 {
            assert_input(&topologies[index], &literals[index], index, limits);
        }
        // Independent held History is a nonmutation guard, never projection admission.
        let mut history = PublicationHistory::new(universe(), HISTORY_LIMITS).unwrap();
        assert!(history.records().is_empty());
        assert_eq!(history.keys_len(), 0);
        assert_eq!(history.canonical_bytes_len(), 0);
        for (index, topology) in topologies[..2].iter().enumerate() {
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
            queries: [key(0x44), key(0x55)],
            slots: [ConceptualSlot::new(7), ConceptualSlot::new(8)],
            registry: PlacementRegistry::new(universe(), 2).unwrap(),
            epochs: [
                ClientPresentation::new(universe(), CLIENTS[0]).unwrap(),
                ClientPresentation::new(universe(), CLIENTS[1]).unwrap(),
            ],
            history,
            outputs: Vec::new(),
        };
        value.assert_fixed(0);
        value
    }

    fn snapshot(&self) -> Snapshot {
        let (h, o, s) = self.queries[0];
        Snapshot {
            registry: self.registry.clone(),
            entries: self.registry.entries().to_vec(),
            stable: StableImage {
                topologies: self.topologies.clone(),
                literals: self.literals.clone(),
                limits: limit_values(self.limits),
                queries: self.queries,
                slots: self.slots,
                epochs: self.epochs.clone(),
                history: self.history.clone(),
                records: self.history.records().to_vec(),
                latest: self.history.latest(h, o, s).cloned(),
                outputs: self.outputs.clone(),
                getters: self.outputs.iter().map(output_image).collect(),
            },
        }
    }

    fn assert_fixed(&self, state: usize) {
        assert_eq!(limit_values(self.limits), [128, 64, 128, 65_536]);
        assert_eq!(self.queries, [key(0x44), key(0x55)]);
        assert_eq!([self.slots[0].value(), self.slots[1].value()], [7, 8]);
        for index in 0..3 {
            assert_input(
                &self.topologies[index],
                &self.literals[index],
                index,
                self.limits,
            );
        }
        assert_history(&self.history, 2);
        for (index, epoch) in self.epochs.iter().enumerate() {
            assert_eq!(epoch.profile(), PresentationProfile::Grid256V1);
            assert_eq!(epoch.universe(), universe());
            assert_eq!(epoch.domain(), CLIENTS[index]);
            assert_eq!(
                format!("{epoch:?}"),
                "ClientPresentation { profile: Grid256V1 }"
            );
        }
        assert_eq!(self.registry.universe(), universe());
        assert_eq!(self.registry.capacity(), 2);
        let expected = REGISTRY_ROWS[state];
        assert_eq!(self.registry.entries().len(), expected.len());
        for (entry, row) in self.registry.entries().iter().zip(expected) {
            let (system, slot, revision, sha) = *row;
            assert_eq!(
                (entry.history(), entry.owner(), entry.system()),
                key(system)
            );
            assert_eq!(entry.slot().value(), slot);
            assert_eq!(entry.revision(), revision);
            assert_eq!(entry.topology_sha256().as_slice(), bytes(sha).as_slice());
        }
        assert!(self.outputs.len() <= 7);
        for (index, output) in self.outputs.iter().enumerate() {
            assert_output(output, index);
        }
    }

    fn project(&mut self, client: usize, query: usize, state: usize) {
        self.assert_fixed(state);
        let before = self.snapshot();
        let (h, o, s) = self.queries[query];
        let output = self.epochs[client]
            .project(&self.registry, h, o, s)
            .unwrap();
        self.assert_fixed(state);
        assert_eq!(self.snapshot(), before);
        assert_output(&output, self.outputs.len());
        self.outputs.push(output);
        self.assert_fixed(state);
    }

    fn assert_missing(&self, query: usize, state: usize) {
        self.assert_fixed(state);
        let before = self.snapshot();
        let (h, o, s) = self.queries[query];
        assert_eq!(
            self.epochs[0].project(&self.registry, h, o, s),
            Err(PresentationError::MissingPlacement)
        );
        self.assert_fixed(state);
        assert_eq!(self.snapshot(), before);
    }
}

#[test]
fn real_second_reservation_preserves_client_outputs_and_reprojection() {
    let mut case = Case::new();
    case.assert_missing(0, 0);
    let before_insert = case.snapshot();
    assert_eq!(
        case.registry
            .reserve(&case.topologies[0], case.slots[0], case.limits),
        Ok(ReservationOutcome::Inserted)
    );
    case.assert_fixed(1);
    assert_eq!(case.snapshot().stable, before_insert.stable);
    case.project(0, 0, 1);
    case.project(1, 0, 1);
    assert_eq!(case.outputs.len(), 2);

    let before_update = case.snapshot();
    assert_eq!(
        case.registry
            .reserve(&case.topologies[1], case.slots[0], case.limits),
        Ok(ReservationOutcome::Updated)
    );
    // Check both owned A1 results immediately, before either new projection.
    case.assert_fixed(2);
    assert_eq!(case.snapshot().stable, before_update.stable);
    case.project(0, 0, 2);
    case.project(1, 0, 2);
    assert_eq!(case.outputs.len(), 4);

    let before_collision = case.snapshot();
    assert_eq!(
        case.registry
            .reserve(&case.topologies[2], case.slots[0], case.limits),
        Err(PlacementError::SlotOccupied)
    );
    case.assert_fixed(2);
    assert_eq!(case.snapshot(), before_collision);
    case.assert_missing(1, 2);
    assert_eq!(case.snapshot(), before_collision);

    let before_second = case.snapshot();
    assert_eq!(
        case.registry
            .reserve(&case.topologies[2], case.slots[1], case.limits),
        Ok(ReservationOutcome::Inserted)
    );
    // R3 may change only the registry; all four older outputs remain exact.
    case.assert_fixed(3);
    assert_eq!(case.snapshot().stable, before_second.stable);
    case.project(0, 1, 3);
    case.project(1, 1, 3);
    assert_eq!(case.outputs.len(), 6);
    let before_repeat = case.snapshot();
    case.project(0, 0, 3);
    assert_eq!(case.outputs.len(), 7);
    assert_eq!(case.outputs[..6], before_repeat.stable.outputs);
    assert_eq!(
        case.outputs[..6]
            .iter()
            .map(output_image)
            .collect::<Vec<_>>(),
        before_repeat.stable.getters
    );
    assert_nonoverlap(&case.outputs);
}
