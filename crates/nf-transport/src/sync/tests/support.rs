use super::super::*;
use crate::records::PeerContext;
use nf_contract::identity::*;
use nf_identity::model::{DeviceProof, Scope};
use sha2::{Digest, Sha256};
const TSV: &str = include_str!("../../../../../docs/transport/sync-vectors/sync-v1.tsv");
pub struct Row {
    pub fields: Vec<&'static str>,
}
impl Row {
    pub fn category(&self) -> &str {
        self.fields[0]
    }
    pub fn name(&self) -> &str {
        self.fields[1]
    }
    pub fn layer(&self) -> &str {
        self.fields[2]
    }
    pub fn bytes(&self) -> Vec<u8> {
        hex(self.fields[4])
    }
    pub fn number(&self, n: usize, default: u32) -> u32 {
        if self.fields[n].is_empty() {
            default
        } else {
            self.fields[n].parse().unwrap()
        }
    }
    pub fn policy(&self) -> SyncWirePolicy {
        for suffix in ["-truncated", "-trailing"] {
            if let Some(name) = self.name().strip_suffix(suffix) {
                return row(name).policy();
            }
        }
        let raw = self.bytes();
        let inferred = if raw.len() > 13 && &raw[..10] == b"NF-SYNC-1\0" {
            u32::from(raw[13])
        } else {
            1
        };
        let lane = if self.number(7, inferred) == 1 {
            SyncLane::Control
        } else {
            SyncLane::Transfer
        };
        SyncWirePolicy {
            lane,
            pins: pins(),
            limits: SyncLimits {
                chunk_bytes: self.number(8, 8192),
                transfer_frame: self.number(9, 9216),
                ..SyncLimits::default()
            },
        }
    }
}
pub fn rows() -> Vec<Row> {
    TSV.lines()
        .skip(2)
        .map(|line| Row {
            fields: line.split('\t').collect(),
        })
        .collect()
}
pub fn row(name: &str) -> Row {
    rows().into_iter().find(|r| r.name() == name).unwrap()
}
pub fn hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| {
            let a = char::from(v[0]).to_digit(16).unwrap();
            let b = char::from(v[1]).to_digit(16).unwrap();
            (a * 16 + b) as u8
        })
        .collect()
}
pub fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn pins() -> ExpectedProfilePins {
    ExpectedProfilePins {
        implementation: [0x30; 32],
        schema: hex("7e06491204d7afa6935f76f769cb3fbf981c8b7c9c65100673ce3e3201236ebb")
            .try_into()
            .unwrap(),
    }
}
pub fn scope() -> Scope {
    Scope {
        universe: UniverseId::from_bytes([6; 16]),
        history: HistoryId::from_bytes([7; 16]),
    }
}
pub fn context(lane: SyncLane, hello: bool) -> PeerContext {
    PeerContext {
        session: if hello {
            [0; 16]
        } else if lane == SyncLane::Control {
            [5; 16]
        } else {
            [29; 16]
        },
        scope: scope(),
        ruleset: [8; 32],
        content: [9; 32],
    }
}
pub fn peer(client: bool) -> libp2p::PeerId {
    libp2p::PeerId::from_bytes(&hex(if client {
        "002408011220d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
    } else {
        "0024080112203d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"
    }))
    .unwrap()
}
pub fn key(client: bool) -> [u8; 32] {
    hex(if client {
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
    } else {
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"
    })
    .try_into()
    .unwrap()
}
pub fn point(n: u8) -> Point {
    match n {
        0 => Point {
            store_revision: 199,
            event: EventSeq(99),
            world: [20; 32],
            state: [21; 32],
            journal: [22; 32],
        },
        1 => Point {
            store_revision: 200,
            event: EventSeq(100),
            world: [23; 32],
            state: [24; 32],
            journal: [25; 32],
        },
        _ => Point {
            store_revision: 201,
            event: EventSeq(100),
            world: [23; 32],
            state: [26; 32],
            journal: [27; 32],
        },
    }
}
pub fn stamp() -> MemberStamp {
    MemberStamp {
        revision: 12,
        digest: [13; 32],
    }
}
/// Only copy fixed public cryptographic fields from the independently signed fixture.
/// No production record decoder is used to construct the expected typed body.
pub fn proof(r: &Row) -> DeviceProof {
    let raw = r.bytes();
    let raw = &raw[raw.len() - 297..];
    let client = raw[32] == 1;
    assert_eq!(&raw[..32], &[&[6; 16][..], &[7; 16]].concat());
    assert_eq!(raw[72] as usize, 38);
    DeviceProof {
        scope: scope(),
        account: AccountId::from_bytes([if client { 1 } else { 3 }; 16]),
        device: DeviceId::from_bytes([if client { 2 } else { 4 }; 16]),
        frontier: 12,
        peer: peer(client).to_bytes(),
        challenge: raw[201..233].try_into().unwrap(),
        signature: raw[233..297].try_into().unwrap(),
    }
}
pub fn transcript(name: &str) -> [u8; 32] {
    hash(&row(name).bytes())
}
