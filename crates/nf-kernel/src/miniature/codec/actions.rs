use super::*;
use nf_world::WorldAction;
pub(super) fn write_action(w: &mut Writer<'_>, action: &WorldAction) -> MiniatureResult<()> {
    match action {
        WorldAction::CreateColony { site, faction } => {
            w.byte(1)?;
            w.raw(site.as_bytes())?;
            w.raw(faction.as_bytes())?;
        }
        WorldAction::BuildIndustry { market, kind } => {
            w.byte(2)?;
            w.raw(market.as_bytes())?;
            w.byte(*kind as u8)?;
        }
        WorldAction::SetRelationship {
            faction,
            other,
            score,
        } => {
            w.byte(3)?;
            w.raw(faction.as_bytes())?;
            w.raw(other.as_bytes())?;
            w.raw(&i64::from(*score).to_le_bytes())?;
        }
        WorldAction::Travel { fleet, destination } => {
            w.byte(4)?;
            w.raw(fleet.as_bytes())?;
            w.raw(destination.as_bytes())?;
        }
    }
    Ok(())
}
pub(super) fn read_action(r: &mut Reader<'_, '_>) -> MiniatureResult<nf_world::WorldAction> {
    use nf_contract::identity::EntityId;
    use nf_world::{IndustryKind, WorldAction};
    Ok(match r.byte()? {
        1 => WorldAction::CreateColony {
            site: EntityId::from_bytes(r.fixed()?),
            faction: EntityId::from_bytes(r.fixed()?),
        },
        2 => WorldAction::BuildIndustry {
            market: EntityId::from_bytes(r.fixed()?),
            kind: match r.byte()? {
                1 => IndustryKind::Farming,
                2 => IndustryKind::Workshop,
                _ => return Err(MiniatureRejection::UnsupportedProvider),
            },
        },
        3 => WorldAction::SetRelationship {
            faction: EntityId::from_bytes(r.fixed()?),
            other: EntityId::from_bytes(r.fixed()?),
            score: i64::from_le_bytes(r.fixed()?)
                .try_into()
                .map_err(|_| MiniatureRejection::InvalidValue)?,
        },
        4 => WorldAction::Travel {
            fleet: EntityId::from_bytes(r.fixed()?),
            destination: EntityId::from_bytes(r.fixed()?),
        },
        _ => return Err(MiniatureRejection::UnsupportedProvider),
    })
}
