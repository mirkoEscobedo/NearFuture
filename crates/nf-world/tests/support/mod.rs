use nf_contract::identity::*;
use nf_world::*;
pub fn genesis() -> Genesis {
    Genesis {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
        seed: [3; 32],
        accounts: [
            AccountId::from_bytes([4; 16]),
            AccountId::from_bytes([5; 16]),
            AccountId::from_bytes([6; 16]),
        ],
    }
}
pub fn state() -> State {
    generate(genesis()).unwrap()
}
#[allow(dead_code)]
pub fn colony(s: &State, ordinal: u8, operation: u8) -> Candidate {
    let site = s
        .markets()
        .iter()
        .find(|m| m.ordinal == ordinal)
        .unwrap()
        .id;
    let faction = s
        .factions()
        .iter()
        .find(|f| f.ordinal == ordinal / 2)
        .unwrap();
    Candidate {
        request: RequestId::from_bytes([operation; 16]),
        operation: OperationId::from_bytes([operation; 16]),
        job: JobId::from_bytes([operation; 16]),
        actor: faction.account,
        expected_revision: s.revision(),
        action: WorldAction::CreateColony {
            site,
            faction: faction.id,
        },
    }
}
