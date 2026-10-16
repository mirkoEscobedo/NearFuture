use Outcome::{PublicationAppended, TombstoneAppended, Unchanged};
use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_publication::{
        PublicationError as Error, PublicationHistory as History, PublicationKind as Kind,
        PublicationLimits as ArchiveLimits, PublicationOutcome as Outcome, PublicationProfile,
        PublicationRecord,
    },
    system_topology::{self, Entity, Kind as EntityKind, Limits, Topology},
};

type Key = (HistoryId, AccountId, EntityId);
type Result = core::result::Result<Outcome, Error>;
const A1: usize = 0;
const A2: usize = 1;
const CONFLICT: usize = 2;
const HIGH: usize = 3;
const MAX: usize = 4;
const B1: usize = 5;
const A4: usize = 6;
const A5: usize = 7;
const H7F: usize = 8;
const H80: usize = 9;
const O7F: usize = 10;
const O80: usize = 11;
const S7F: usize = 12;
const S80: usize = 13;

// Fixed policy literals: expectations never come from History or its encoder.
struct Fixture {
    hex: &'static str,
    sha: &'static str,
    history: u8,
    owner: u8,
    system: u8,
    revision: u64,
    day: u64,
}
const FIXTURES: [Fixture; 14] = [
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183",
        history: 0x22,
        owner: 0x33,
        system: 0x44,
        revision: 1,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d",
        history: 0x22,
        owner: 0x33,
        system: 0x44,
        revision: 2,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000030000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "52ed6079f4b31e1aa71f17a3740bfd085f56bc6bd8f4df1bb4ec31ae136fa5be",
        history: 0x22,
        owner: 0x33,
        system: 0x44,
        revision: 2,
        day: 3,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440000000000000080ffffffffffffffff01010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "840c1706efbc0e083b7743c01e9964f65b7698767e5bb5f983e7a2762b77fc53",
        history: 0x22,
        owner: 0x33,
        system: 0x44,
        revision: 9_223_372_036_854_775_808,
        day: u64::MAX,
    },
    Fixture {
        hex: "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222222222223333333333333333333333333333333344444444444444444444444444444444ffffffffffffffffffffffffffffffff01010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "af7535b29a044a03adf92107775dd55b2c4e14f03fac34ecca4e216d53151345",
        history: 0x22,
        owner: 0x33,
        system: 0x44,
        revision: u64::MAX,
        day: u64::MAX,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333555555555555555555555555555555550100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "6ea8fd3113fc299431367483e01f808086cdc28062ac9daf9d03a761fdcde821",
        history: 0x22,
        owner: 0x33,
        system: 0x55,
        revision: 1,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440400000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "1fa957a0c4e5780f76d86618a6617482c9ce7fff1bad4d534c85085423dc11e1",
        history: 0x22,
        owner: 0x33,
        system: 0x44,
        revision: 4,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440500000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "704bed23c8ed3a9d2b3fc3fc0393834a9f972c0d8c6203fc4b770ab7deb9b732",
        history: 0x22,
        owner: 0x33,
        system: 0x44,
        revision: 5,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111117f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7ffefefefefefefefefefefefefefefefefefefefefefefefefefefefefefefefe0100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "8100c2283ecae91958b27c51ecfeb2929f6043abf3eed3e647a15a9d95ba605c",
        history: 0x7f,
        owner: 0xfe,
        system: 0xfe,
        revision: 1,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111118080808080808080808080808080808001010101010101010101010101010101010101010101010101010101010101010100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "d7186064df291f76d01812d127343cd4018e2f878db7246699063d8fea90dd46",
        history: 0x80,
        owner: 0x01,
        system: 0x01,
        revision: 1,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222222222227f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7ffefefefefefefefefefefefefefefefe0100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "a340e718ecda294ff8d6777e9b29cdc514a1f55eb1d81e73c38d9a684f2d2aee",
        history: 0x22,
        owner: 0x7f,
        system: 0xfe,
        revision: 1,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222280808080808080808080808080808080010101010101010101010101010101010100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "dd259684a3b3516aad76496ab44c036d0a4e19c52db748d5a635abd3e3b493bb",
        history: 0x22,
        owner: 0x80,
        system: 0x01,
        revision: 1,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222222222333333333333333333333333333333337f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f0100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "bc100d592da430de5eedd1c87a228a1f0073d6d2e9912438deffd2bfbd1ecb24",
        history: 0x22,
        owner: 0x33,
        system: 0x7f,
        revision: 1,
        day: 2,
    },
    Fixture {
        hex: "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333808080808080808080808080808080800100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        sha: "c2dacf84b51533e15caedd89d249aa45907d85771fd4e226bdb481fb6172e5d0",
        history: 0x22,
        owner: 0x33,
        system: 0x80,
        revision: 1,
        day: 2,
    },
];

fn bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn namespace() -> UniverseId {
    UniverseId::from_bytes([0x11; 16])
}

fn key(index: usize) -> Key {
    let f = &FIXTURES[index];
    (
        HistoryId::from_bytes([f.history; 16]),
        AccountId::from_bytes([f.owner; 16]),
        EntityId::from_bytes([f.system; 16]),
    )
}

fn limits(keys: usize, records: usize, canonical_bytes: usize) -> ArchiveLimits {
    ArchiveLimits {
        keys,
        records,
        canonical_bytes,
    }
}

fn topology_limits(value: Limits) -> [usize; 4] {
    [
        value.entities,
        value.dependencies,
        value.label_bytes,
        value.document_bytes,
    ]
}

fn assert_input(value: &Topology, index: usize) {
    let f = &FIXTURES[index];
    let (history, owner, system) = key(index);
    assert_eq!(
        (value.universe, value.history, value.owner, value.system),
        (namespace(), history, owner, system)
    );
    assert_eq!(
        (value.revision, value.campaign_day_millis),
        (f.revision, f.day)
    );
    let literal = bytes(f.hex);
    assert_eq!(literal.len(), 199);
    assert_eq!(system_topology::validate(value, Limits::default()), Ok(()));
    assert_eq!(
        system_topology::encode(value, Limits::default()).unwrap(),
        literal
    );
    assert_eq!(
        system_topology::digest(value, Limits::default())
            .unwrap()
            .as_slice(),
        bytes(f.sha).as_slice()
    );
}

fn input(index: usize) -> Topology {
    let value = system_topology::decode(&bytes(FIXTURES[index].hex), Limits::default()).unwrap();
    assert_input(&value, index);
    value
}

#[derive(Clone, Copy)]
struct Expected {
    identity: usize,
    event: u64,
    reference: usize,
    kind: Kind,
}

fn p(index: usize) -> Expected {
    Expected {
        identity: index,
        event: FIXTURES[index].revision,
        reference: index,
        kind: Kind::PublishedVersion,
    }
}

fn t(identity: usize, event: u64, reference: usize) -> Expected {
    Expected {
        identity,
        event,
        reference,
        kind: Kind::Tombstone,
    }
}

fn assert_record(record: &PublicationRecord, expected: Expected) {
    let (history, owner, system) = key(expected.identity);
    assert_eq!(
        (
            record.universe(),
            record.history(),
            record.owner(),
            record.system(),
        ),
        (namespace(), history, owner, system)
    );
    assert_eq!(record.profile(), PublicationProfile::SelectedTopologyV1);
    assert_eq!(record.kind(), expected.kind);
    assert_eq!(record.revision(), expected.event);
    assert_eq!(
        record.referenced_publication_revision(),
        FIXTURES[expected.reference].revision
    );
    assert_eq!(
        record.referenced_topology_sha256().as_slice(),
        bytes(FIXTURES[expected.reference].sha).as_slice()
    );
    let body = bytes(FIXTURES[expected.identity].hex);
    assert_eq!(
        record.canonical_topology_bytes(),
        if expected.kind == Kind::PublishedVersion {
            Some(body.as_slice())
        } else {
            None
        }
    );
}

struct View<'a> {
    rows: &'a [Expected],
    keys: usize,
    bytes: usize,
    latest: &'a [Expected],
}

fn v<'a>(rows: &'a [Expected], keys: usize, bytes: usize, latest: &'a [Expected]) -> View<'a> {
    View {
        rows,
        keys,
        bytes,
        latest,
    }
}

struct Case {
    history: History,
    limits: ArchiveLimits,
}

impl Case {
    fn new(limits: ArchiveLimits) -> Self {
        let case = Self {
            history: History::new(namespace(), limits).unwrap(),
            limits,
        };
        case.state(v(&[], 0, 0, &[]));
        case
    }

    fn state(&self, expected: View<'_>) {
        assert_eq!(self.history.universe(), namespace());
        assert_eq!(self.history.limits(), self.limits);
        assert_eq!(self.history.keys_len(), expected.keys);
        assert_eq!(self.history.canonical_bytes_len(), expected.bytes);
        assert_eq!(self.history.records().len(), expected.rows.len());
        for (record, row) in self.history.records().iter().zip(expected.rows) {
            assert_record(record, *row);
        }
        for row in expected.latest {
            let (h, o, s) = key(row.identity);
            assert_record(self.history.latest(h, o, s).unwrap(), *row);
        }
    }

    fn retained(&self, before: &History) {
        for record in before.records() {
            assert!(self.history.records().contains(record));
        }
    }

    fn publication(&mut self, index: usize, expected: Result, state: View<'_>) {
        self.publication_value(
            input(index),
            Limits::default(),
            Some(index),
            expected,
            state,
        );
    }

    fn publication_value(
        &mut self,
        value: Topology,
        tl: Limits,
        fixture: Option<usize>,
        expected: Result,
        state: View<'_>,
    ) {
        let before = self.history.clone();
        let original = value.clone();
        let original_limits = topology_limits(tl);
        if let Some(index) = fixture {
            assert_input(&value, index);
        }
        assert_eq!(self.history.append_publication(&value, tl), expected);
        if matches!(expected, Err(_) | Ok(Unchanged)) {
            assert_eq!(self.history, before);
        }
        assert_eq!(value, original);
        assert_eq!(topology_limits(tl), original_limits);
        if let Some(index) = fixture {
            assert_input(&value, index);
        }
        self.state(state);
        self.retained(&before);
    }

    fn direct(&mut self, value: Topology, tl: Limits, error: Error, state: View<'_>) {
        self.publication_value(value, tl, None, Err(error), state);
    }

    fn tombstone(&mut self, key: Key, revision: u64, expected: Result, state: View<'_>) {
        let before = self.history.clone();
        let arguments = (key, revision);
        assert_eq!(
            self.history.append_tombstone(key.0, key.1, key.2, revision),
            expected
        );
        if matches!(expected, Err(_) | Ok(Unchanged)) {
            assert_eq!(self.history, before);
        }
        assert_eq!((key, revision), arguments);
        self.state(state);
        self.retained(&before);
    }

    fn query(&self, key: Key, expected: Option<Expected>, state: View<'_>) {
        let before = self.history.clone();
        let original = key;
        let actual = self.history.latest(key.0, key.1, key.2);
        if let Some(row) = expected {
            assert_record(actual.unwrap(), row);
        } else {
            assert!(actual.is_none());
        }
        assert_eq!(self.history, before);
        assert_eq!(key, original);
        self.state(state);
    }
}

#[derive(Clone, Copy)]
enum Seed {
    A1,
    A1A2,
    A1A2T3,
    A2,
    A2T3,
}

fn seed(limits: ArchiveLimits, seed: Seed) -> Case {
    let mut c = Case::new(limits);
    match seed {
        Seed::A1 | Seed::A1A2 | Seed::A1A2T3 => {
            c.publication(A1, Ok(PublicationAppended), v(&[p(A1)], 1, 199, &[p(A1)]));
            if matches!(seed, Seed::A1A2 | Seed::A1A2T3) {
                c.publication(
                    A2,
                    Ok(PublicationAppended),
                    v(&[p(A1), p(A2)], 1, 398, &[p(A2)]),
                );
            }
            if matches!(seed, Seed::A1A2T3) {
                c.tombstone(
                    key(A1),
                    3,
                    Ok(TombstoneAppended),
                    v(&[p(A1), p(A2), t(A2, 3, A2)], 1, 398, &[t(A2, 3, A2)]),
                );
            }
        }
        Seed::A2 | Seed::A2T3 => {
            c.publication(A2, Ok(PublicationAppended), v(&[p(A2)], 1, 199, &[p(A2)]));
            if matches!(seed, Seed::A2T3) {
                c.tombstone(
                    key(A1),
                    3,
                    Ok(TombstoneAppended),
                    v(&[p(A2), t(A2, 3, A2)], 1, 199, &[t(A2, 3, A2)]),
                );
            }
        }
    }
    c
}

#[test]
fn constructor_and_admission_refusals_preserve_history_and_inputs() {
    let zero = UniverseId::from_bytes([0; 16]);
    let invalid = limits(0, 0, 0);
    let arguments = (zero, invalid);
    assert_eq!(History::new(zero, invalid), Err(Error::InvalidNamespace));
    assert_eq!((zero, invalid), arguments);
    for bad in [
        limits(0, 1, 199),
        limits(257, 1, 199),
        limits(1, 0, 199),
        limits(1, 513, 199),
        limits(1, 1, 0),
        limits(1, 1, 8_388_609),
    ] {
        let original = bad;
        assert_eq!(History::new(namespace(), bad), Err(Error::Bounds));
        assert_eq!(bad, original);
    }
    for good in [limits(256, 512, 8_388_608), limits(256, 1, 1)] {
        Case::new(good).query(key(A1), None, v(&[], 0, 0, &[]));
    }
    let mut empty = Case::new(limits(1, 1, 1));
    empty.tombstone(
        key(A1),
        1,
        Err(Error::MissingPublication),
        v(&[], 0, 0, &[]),
    );
    let a = key(A1);
    for (key, revision) in [
        ((HistoryId::from_bytes([0; 16]), a.1, a.2), 1),
        ((a.0, AccountId::from_bytes([0; 16]), a.2), 1),
        ((a.0, a.1, EntityId::from_bytes([0; 16])), 1),
        (a, 0),
    ] {
        empty.tombstone(key, revision, Err(Error::InvalidInput), v(&[], 0, 0, &[]));
    }
    let mut c = seed(limits(1, 1, 199), Seed::A1);
    let mut invalid = input(A1);
    invalid.revision = 0;
    invalid.universe = UniverseId::from_bytes([0x66; 16]);
    c.direct(
        invalid,
        Limits::default(),
        Error::Topology(system_topology::Error::Invalid),
        v(&[p(A1)], 1, 199, &[p(A1)]),
    );
    let mut foreign = input(A1);
    foreign.universe = UniverseId::from_bytes([0x66; 16]);
    c.direct(
        foreign,
        Limits::default(),
        Error::UniverseMismatch,
        v(&[p(A1)], 1, 199, &[p(A1)]),
    );
    for lower in [
        Limits {
            document_bytes: 198,
            ..Limits::default()
        },
        Limits {
            entities: 0,
            ..Limits::default()
        },
    ] {
        c.direct(
            input(A1),
            lower,
            Error::Topology(system_topology::Error::Limit),
            v(&[p(A1)], 1, 199, &[p(A1)]),
        );
    }
    let mut cycle = input(A1);
    cycle.entities[1].parent = Some(EntityId::from_bytes([3; 16]));
    cycle.entities.push(Entity {
        id: EntityId::from_bytes([3; 16]),
        kind: EntityKind::Station,
        parent: Some(EntityId::from_bytes([2; 16])),
        dependency: None,
        label: None,
    });
    c.direct(
        cycle,
        Limits::default(),
        Error::Topology(system_topology::Error::Cycle),
        v(&[p(A1)], 1, 199, &[p(A1)]),
    );
}

#[test]
fn quota_refusals_preserve_full_history_and_new_key_accounting() {
    let mut c = seed(limits(1, 2, 398), Seed::A1A2);
    c.publication(
        B1,
        Err(Error::KeyLimit),
        v(&[p(A1), p(A2)], 1, 398, &[p(A2)]),
    );
    c.query(key(B1), None, v(&[p(A1), p(A2)], 1, 398, &[p(A2)]));
    let mut c = seed(limits(2, 1, 199), Seed::A1);
    c.publication(A2, Err(Error::RecordLimit), v(&[p(A1)], 1, 199, &[p(A1)]));
    let mut c = seed(limits(1, 3, 199), Seed::A1);
    c.publication(A2, Err(Error::ByteLimit), v(&[p(A1)], 1, 199, &[p(A1)]));
    let mut c = seed(limits(1, 3, 199), Seed::A1);
    c.tombstone(
        key(A1),
        2,
        Ok(TombstoneAppended),
        v(&[p(A1), t(A1, 2, A1)], 1, 199, &[t(A1, 2, A1)]),
    );
    let mut c = seed(limits(1, 3, 398), Seed::A1A2T3);
    c.publication(
        A1,
        Ok(Unchanged),
        v(&[p(A1), p(A2), t(A2, 3, A2)], 1, 398, &[t(A2, 3, A2)]),
    );
    let mut c = seed(limits(1, 3, 398), Seed::A1A2T3);
    c.publication(
        CONFLICT,
        Err(Error::RevisionConflict),
        v(&[p(A1), p(A2), t(A2, 3, A2)], 1, 398, &[t(A2, 3, A2)]),
    );
    let mut c = seed(limits(1, 2, 199), Seed::A2T3);
    c.publication(
        A1,
        Err(Error::StaleRevision),
        v(&[p(A2), t(A2, 3, A2)], 1, 199, &[t(A2, 3, A2)]),
    );
    let mut c = seed(limits(2, 1, 199), Seed::A1);
    c.publication(B1, Err(Error::RecordLimit), v(&[p(A1)], 1, 199, &[p(A1)]));
    c.query(key(B1), None, v(&[p(A1)], 1, 199, &[p(A1)]));
    let mut c = seed(limits(2, 3, 199), Seed::A1);
    c.publication(B1, Err(Error::ByteLimit), v(&[p(A1)], 1, 199, &[p(A1)]));
    c.query(key(B1), None, v(&[p(A1)], 1, 199, &[p(A1)]));
    let mut c = seed(limits(1, 1, 199), Seed::A1);
    c.tombstone(
        key(A1),
        2,
        Err(Error::RecordLimit),
        v(&[p(A1)], 1, 199, &[p(A1)]),
    );
    let mut c = seed(limits(1, 2, 199), Seed::A2T3);
    c.tombstone(
        key(A1),
        1,
        Err(Error::StaleRevision),
        v(&[p(A2), t(A2, 3, A2)], 1, 199, &[t(A2, 3, A2)]),
    );
    let mut c = seed(limits(1, 1, 199), Seed::A1);
    c.tombstone(
        key(B1),
        2,
        Err(Error::MissingPublication),
        v(&[p(A1)], 1, 199, &[p(A1)]),
    );
    c.query(key(B1), None, v(&[p(A1)], 1, 199, &[p(A1)]));
}

#[test]
fn unsigned_revisions_conflicts_and_max_preserve_event_identity() {
    let mut c = Case::new(limits(2, 12, 2388));
    let s1 = [p(A1)];
    let s2 = [p(A1), p(A2)];
    let s3 = [p(A1), p(A2), t(A2, 3, A2)];
    let s4 = [p(A1), p(A2), t(A2, 3, A2), p(A4)];
    let sh = [p(A1), p(A2), t(A2, 3, A2), p(A4), p(HIGH)];
    let sm = [p(A1), p(A2), t(A2, 3, A2), p(A4), p(HIGH), p(MAX)];
    c.publication(A1, Ok(PublicationAppended), v(&s1, 1, 199, &[p(A1)]));
    c.publication(A2, Ok(PublicationAppended), v(&s2, 1, 398, &[p(A2)]));
    c.publication(
        CONFLICT,
        Err(Error::RevisionConflict),
        v(&s2, 1, 398, &[p(A2)]),
    );
    c.tombstone(
        key(A1),
        3,
        Ok(TombstoneAppended),
        v(&s3, 1, 398, &[t(A2, 3, A2)]),
    );
    let old_t3 = c.history.records()[2].clone();
    c.publication(A1, Ok(Unchanged), v(&s3, 1, 398, &[t(A2, 3, A2)]));
    c.publication(A2, Ok(Unchanged), v(&s3, 1, 398, &[t(A2, 3, A2)]));
    c.tombstone(key(A1), 3, Ok(Unchanged), v(&s3, 1, 398, &[t(A2, 3, A2)]));
    c.publication(A4, Ok(PublicationAppended), v(&s4, 1, 597, &[p(A4)]));
    c.tombstone(key(A1), 3, Ok(Unchanged), v(&s4, 1, 597, &[p(A4)]));
    c.publication(HIGH, Ok(PublicationAppended), v(&sh, 1, 796, &[p(HIGH)]));
    c.publication(A5, Err(Error::StaleRevision), v(&sh, 1, 796, &[p(HIGH)]));
    c.publication(MAX, Ok(PublicationAppended), v(&sm, 1, 995, &[p(MAX)]));
    c.publication(MAX, Ok(Unchanged), v(&sm, 1, 995, &[p(MAX)]));
    c.tombstone(
        key(A1),
        u64::MAX,
        Err(Error::RevisionConflict),
        v(&sm, 1, 995, &[p(MAX)]),
    );
    c.tombstone(
        key(A1),
        9_223_372_036_854_775_809,
        Err(Error::StaleRevision),
        v(&sm, 1, 995, &[p(MAX)]),
    );
    c.tombstone(
        key(A1),
        0,
        Err(Error::InvalidInput),
        v(&sm, 1, 995, &[p(MAX)]),
    );
    assert_record(&old_t3, t(A2, 3, A2));
    assert_eq!(c.history.records()[2], old_t3);
    let mut c = seed(limits(1, 3, 199), Seed::A2);
    let state = [p(A2), t(A2, u64::MAX, A2)];
    c.tombstone(
        key(A1),
        u64::MAX,
        Ok(TombstoneAppended),
        v(&state, 1, 199, &[t(A2, u64::MAX, A2)]),
    );
    c.tombstone(
        key(A1),
        u64::MAX,
        Ok(Unchanged),
        v(&state, 1, 199, &[t(A2, u64::MAX, A2)]),
    );
    c.publication(
        MAX,
        Err(Error::RevisionConflict),
        v(&state, 1, 199, &[t(A2, u64::MAX, A2)]),
    );
    c.publication(
        A1,
        Err(Error::StaleRevision),
        v(&state, 1, 199, &[t(A2, u64::MAX, A2)]),
    );
    c.publication(A2, Ok(Unchanged), v(&state, 1, 199, &[t(A2, u64::MAX, A2)]));
}

#[test]
fn full_key_order_queries_and_repeated_tombstones_preserve_references() {
    for order in [[A1, B1], [B1, A1]] {
        let mut c = Case::new(limits(2, 2, 398));
        c.publication(
            order[0],
            Ok(PublicationAppended),
            v(&[p(order[0])], 1, 199, &[p(order[0])]),
        );
        c.publication(
            order[1],
            Ok(PublicationAppended),
            v(&[p(A1), p(B1)], 2, 398, &[p(A1), p(B1)]),
        );
    }
    let forward: &[(usize, &[usize], usize, usize)] = &[
        (A1, &[A1], 1, 199),
        (B1, &[A1, B1], 2, 398),
        (S7F, &[A1, B1, S7F], 3, 597),
        (S80, &[A1, B1, S7F, S80], 4, 796),
        (O7F, &[A1, B1, S7F, S80, O7F], 5, 995),
        (O80, &[A1, B1, S7F, S80, O7F, O80], 6, 1194),
        (H7F, &[A1, B1, S7F, S80, O7F, O80, H7F], 7, 1393),
        (H80, &[A1, B1, S7F, S80, O7F, O80, H7F, H80], 8, 1592),
    ];
    let reverse: &[(usize, &[usize], usize, usize)] = &[
        (H80, &[H80], 1, 199),
        (H7F, &[H7F, H80], 2, 398),
        (O80, &[O80, H7F, H80], 3, 597),
        (O7F, &[O7F, O80, H7F, H80], 4, 796),
        (S80, &[S80, O7F, O80, H7F, H80], 5, 995),
        (S7F, &[S7F, S80, O7F, O80, H7F, H80], 6, 1194),
        (B1, &[B1, S7F, S80, O7F, O80, H7F, H80], 7, 1393),
        (A1, &[A1, B1, S7F, S80, O7F, O80, H7F, H80], 8, 1592),
    ];
    for stages in [forward, reverse] {
        let mut c = Case::new(limits(8, 8, 1592));
        for &(index, expected_indices, keys, byte_count) in stages {
            // Policy constants specify every stage; there is no test-side sort/model.
            let rows: Vec<Expected> = expected_indices.iter().map(|&i| p(i)).collect();
            c.publication(
                index,
                Ok(PublicationAppended),
                v(&rows, keys, byte_count, &rows),
            );
        }
    }
    let c = seed(limits(1, 1, 199), Seed::A1);
    let a = key(A1);
    for absent in [
        (HistoryId::from_bytes([0; 16]), a.1, a.2),
        (a.0, AccountId::from_bytes([0; 16]), a.2),
        (a.0, a.1, EntityId::from_bytes([0; 16])),
        key(B1),
    ] {
        c.query(absent, None, v(&[p(A1)], 1, 199, &[p(A1)]));
    }
    let mut h = [0x22; 16];
    h[15] = 0x23;
    let mut o = [0x33; 16];
    o[15] = 0x34;
    let mut s = [0x44; 16];
    s[15] = 0x45;
    for absent in [
        (HistoryId::from_bytes(h), a.1, a.2),
        (a.0, AccountId::from_bytes(o), a.2),
        (a.0, a.1, EntityId::from_bytes(s)),
    ] {
        c.query(absent, None, v(&[p(A1)], 1, 199, &[p(A1)]));
    }
    c.query(a, Some(p(A1)), v(&[p(A1)], 1, 199, &[p(A1)]));
    let mut c = seed(limits(1, 3, 199), Seed::A2T3);
    let old_rows = c.history.records().to_vec();
    let old_t3 = c.history.records()[1].clone();
    c.tombstone(
        key(A1),
        4,
        Ok(TombstoneAppended),
        v(
            &[p(A2), t(A2, 3, A2), t(A2, 4, A2)],
            1,
            199,
            &[t(A2, 4, A2)],
        ),
    );
    assert_eq!(&c.history.records()[..2], old_rows.as_slice());
    assert_record(&old_t3, t(A2, 3, A2));
    assert_record(&c.history.records()[2], t(A2, 4, A2));
}
