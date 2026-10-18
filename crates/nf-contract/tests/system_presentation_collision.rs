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
const OUTPUTS: [(PresentationDomain, u64, &str, i64, i64); 4] = [
    (CLIENTS[0], 1, A1_SHA, -784, -2016),
    (CLIENTS[1], 1, A1_SHA, 4576, -4080),
    (CLIENTS[0], 2, A2_SHA, -784, -2016),
    (CLIENTS[1], 2, A2_SHA, 4576, -4080),
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
    let (domain, revision, sha, x, y) = OUTPUTS[index];
    assert_eq!(output.profile(), PresentationProfile::Grid256V1);
    assert_eq!(output.universe(), universe());
    assert_eq!(
        (output.history(), output.owner(), output.system()),
        key(0x44)
    );
    assert_eq!(output.slot().value(), 7);
    assert_eq!(output.revision(), revision);
    assert_eq!(output.topology_sha256().as_slice(), bytes(sha).as_slice());
    assert_eq!(output.domain(), domain);
    assert_eq!((output.x(), output.y()), (x, y));
    assert_eq!(
        format!("{output:?}"),
        "ProjectedPlacement { profile: Grid256V1 }"
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StableImage {
    topologies: [Topology; 3],
    literals: [Vec<u8>; 3],
    limits: [usize; 4],
    queries: [Key; 2],
    slot: ConceptualSlot,
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
    slot: ConceptualSlot,
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
            slot: ConceptualSlot::new(7),
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
                slot: self.slot,
                epochs: self.epochs.clone(),
                history: self.history.clone(),
                records: self.history.records().to_vec(),
                latest: self.history.latest(h, o, s).cloned(),
                outputs: self.outputs.clone(),
                getters: self.outputs.iter().map(output_image).collect(),
            },
        }
    }

    fn assert_fixed(&self, revision: u64) {
        assert_eq!(limit_values(self.limits), [128, 64, 128, 65_536]);
        assert_eq!(self.queries, [key(0x44), key(0x55)]);
        assert_eq!(self.slot.value(), 7);
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
        if revision == 0 {
            assert!(self.registry.entries().is_empty());
        } else {
            let sha = match revision {
                1 => A1_SHA,
                2 => A2_SHA,
                _ => panic!("only fixed R1 and R2 are admitted by this fixture"),
            };
            assert_eq!(self.registry.entries().len(), 1);
            let entry = &self.registry.entries()[0];
            assert_eq!((entry.history(), entry.owner(), entry.system()), key(0x44));
            assert_eq!(entry.slot().value(), 7);
            assert_eq!(entry.revision(), revision);
            assert_eq!(entry.topology_sha256().as_slice(), bytes(sha).as_slice());
        }
        assert!(self.outputs.len() <= 4);
        for (index, output) in self.outputs.iter().enumerate() {
            assert_output(output, index);
        }
    }

    fn project_a(&mut self, client: usize, revision: u64) {
        let before = self.snapshot();
        let (h, o, s) = self.queries[0];
        let output = self.epochs[client]
            .project(&self.registry, h, o, s)
            .unwrap();
        self.assert_fixed(revision);
        assert_eq!(self.snapshot(), before);
        assert_output(&output, self.outputs.len());
        self.outputs.push(output);
        self.assert_fixed(revision);
    }
}

#[test]
fn real_collision_refuses_without_creating_a_projectable_placement() {
    let mut case = Case::new();
    let before_insert = case.snapshot();
    assert_eq!(
        case.registry
            .reserve(&case.topologies[0], case.slot, case.limits),
        Ok(ReservationOutcome::Inserted)
    );
    case.assert_fixed(1);
    assert_eq!(case.snapshot().stable, before_insert.stable);
    case.project_a(0, 1);
    case.project_a(1, 1);
    assert_eq!(case.outputs.len(), 2);

    let before_update = case.snapshot();
    assert_eq!(
        case.registry
            .reserve(&case.topologies[1], case.slot, case.limits),
        Ok(ReservationOutcome::Updated)
    );
    // Check both owned A1 results immediately, before either new projection.
    case.assert_fixed(2);
    assert_eq!(case.snapshot().stable, before_update.stable);
    case.project_a(0, 2);
    case.project_a(1, 2);
    assert_eq!(case.outputs.len(), 4);

    let before_collision = case.snapshot();
    assert_eq!(
        case.registry
            .reserve(&case.topologies[2], case.slot, case.limits),
        Err(PlacementError::SlotOccupied)
    );
    case.assert_fixed(2);
    assert_eq!(case.snapshot(), before_collision);

    let before_missing = case.snapshot();
    let (h, o, s) = case.queries[1];
    assert_eq!(
        case.epochs[0].project(&case.registry, h, o, s),
        Err(PresentationError::MissingPlacement)
    );
    case.assert_fixed(2);
    assert_eq!(case.snapshot(), before_missing);
    assert_eq!(case.snapshot(), before_collision);
}
