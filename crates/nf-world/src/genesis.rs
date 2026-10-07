use crate::*;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};
pub(crate) fn entity(g: Genesis, kind: u8, ordinal: u32) -> EntityId {
    let mut h = Sha256::new();
    h.update(b"NF-MINI-ID-1\0");
    h.update(g.universe.as_bytes());
    h.update(g.history.as_bytes());
    h.update(g.seed);
    h.update([kind]);
    h.update(ordinal.to_le_bytes());
    let digest = h.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    EntityId::from_bytes(id)
}
pub fn generate(g: Genesis) -> Result<State, WorldError> {
    if g.accounts[0] == g.accounts[1]
        || g.accounts[0] == g.accounts[2]
        || g.accounts[1] == g.accounts[2]
    {
        return Err(WorldError::Duplicate);
    }
    let mut systems: Vec<_> = (0..3)
        .map(|i| System {
            id: entity(g, 1, i),
            ordinal: i as u8,
        })
        .collect();
    let mut factions: Vec<_> = (0..3)
        .map(|i| Faction {
            id: entity(g, 2, i),
            ordinal: i as u8,
            account: g.accounts[i as usize],
            credits: 200,
            supplies: 100,
            spent_credits: 0,
            spent_supplies: 0,
        })
        .collect();
    let mut markets: Vec<_> = (0..6)
        .map(|i| Market {
            id: entity(g, 3, i),
            ordinal: i as u8,
            system: entity(g, 1, i / 2),
            owner: if i % 2 == 0 {
                Some(entity(g, 2, i / 2))
            } else {
                None
            },
            industries: [IndustryState::Absent; 2],
        })
        .collect();
    let mut fleets: Vec<_> = (0..3)
        .map(|i| Fleet {
            id: entity(g, 4, i),
            faction: entity(g, 2, i),
            location: FleetLocation::Docked(entity(g, 1, i)),
        })
        .collect();
    systems.sort_by_key(|v| v.id);
    factions.sort_by_key(|v| v.id);
    markets.sort_by_key(|v| v.id);
    fleets.sort_by_key(|v| v.id);
    let mut relations = Vec::new();
    for (index, (a, b)) in [(0, 1), (0, 2), (1, 2)].into_iter().enumerate() {
        let x = entity(g, 2, a);
        let y = entity(g, 2, b);
        relations.push(Relation {
            id: entity(g, 5, index as u32),
            left: x.min(y),
            right: x.max(y),
            score: 0,
        });
    }
    relations.sort_by_key(|r| r.id);
    let state = State {
        genesis: g,
        revision: nf_contract::identity::AggregateRevision(0),
        systems,
        factions,
        markets,
        fleets,
        relations,
        schedules: Vec::new(),
    };
    validate_at(&state, nf_contract::identity::WorldTick(0))?;
    Ok(state)
}
use nf_contract::identity::EntityId;
