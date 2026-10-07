use crate::ActionSelection;
use nf_kernel::miniature::{MiniatureRejection, MiniatureWorld};
use nf_world::WorldAction;
/// Resolve immutable bounded labels only. Actor control and resources remain kernel admission policy.
pub fn resolve_action(
    world: &MiniatureWorld,
    selection: &ActionSelection,
) -> Result<WorldAction, MiniatureRejection> {
    let component = world.component();
    let faction = |ordinal: u8| {
        component
            .factions()
            .iter()
            .find(|f| f.ordinal == ordinal)
            .map(|f| f.id)
            .ok_or(MiniatureRejection::InvalidReference)
    };
    let market = |ordinal: u8| {
        component
            .markets()
            .iter()
            .find(|m| m.ordinal == ordinal)
            .map(|m| m.id)
            .ok_or(MiniatureRejection::InvalidReference)
    };
    Ok(match *selection {
        ActionSelection::Colony {
            site,
            faction: owner,
        } => WorldAction::CreateColony {
            site: market(site)?,
            faction: faction(owner)?,
        },
        ActionSelection::Build { market: site, kind } => WorldAction::BuildIndustry {
            market: market(site)?,
            kind,
        },
        ActionSelection::Relationship {
            faction: owner,
            other,
            score,
        } => {
            if !(-10000..=10000).contains(&score) {
                return Err(MiniatureRejection::InvalidValue);
            }
            WorldAction::SetRelationship {
                faction: faction(owner)?,
                other: faction(other)?,
                score,
            }
        }
        ActionSelection::Travel {
            faction: owner,
            destination,
        } => {
            let owner = faction(owner)?;
            let fleet = component
                .fleets()
                .iter()
                .find(|f| f.faction == owner)
                .ok_or(MiniatureRejection::InvalidReference)?
                .id;
            let destination = component
                .systems()
                .iter()
                .find(|s| s.ordinal == destination)
                .ok_or(MiniatureRejection::InvalidReference)?
                .id;
            WorldAction::Travel { fleet, destination }
        }
    })
}
