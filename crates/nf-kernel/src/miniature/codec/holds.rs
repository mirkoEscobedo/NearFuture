use super::*;
pub(super) fn write_hold(w: &mut Writer<'_>, hold: MiniatureHold) -> MiniatureResult<()> {
    w.raw(hold.operation.as_bytes())?;
    w.raw(hold.faction.as_bytes())?;
    w.u64(hold.credits)?;
    w.u64(hold.supplies)?;
    Ok(())
}
pub(super) fn read_hold(r: &mut Reader<'_, '_>) -> MiniatureResult<MiniatureHold> {
    use nf_contract::identity::{EntityId, OperationId};
    Ok(MiniatureHold {
        operation: OperationId::from_bytes(r.fixed()?),
        faction: EntityId::from_bytes(r.fixed()?),
        credits: r.u64()?,
        supplies: r.u64()?,
    })
}
