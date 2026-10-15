//! Bounded actual catalog data, never an owner, support-export witness or catalog-file copy.
use super::{engine::Engine, error::{ManagedError, Result}, receipt::Receipt,
    registry::RegistryRow, stamp::CatalogFrontier};
use std::path::Path;

#[derive(Clone)]
pub struct RegistryObservation { row: RegistryRow }
impl RegistryObservation {
    pub fn kind(&self) -> u32 { self.row.kind }
    pub fn artifact(&self) -> [u8; 16] { self.row.artifact }
    /// Local operator data. A support encoder must exclude host paths and private identifiers.
    pub fn path(&self) -> &Path { &self.row.path }
    pub fn byte_count(&self) -> u64 { self.row.bytes }
    pub fn digest(&self) -> [u8; 32] { self.row.digest }
    pub fn canonical_manifest(&self) -> &[u8] { &self.row.manifest }
    pub fn manifest_digest(&self) -> [u8; 32] { self.row.manifest_digest() }
}
#[derive(Clone)]
pub struct ManagedCatalogObservation {
    frontier: CatalogFrontier,
    registry: Vec<RegistryObservation>,
    receipts: Vec<Receipt>,
    pending: Option<[u8; 32]>,
}
impl ManagedCatalogObservation {
    pub fn catalog_frontier(&self) -> CatalogFrontier { self.frontier }
    pub fn registry(&self) -> &[RegistryObservation] { &self.registry }
    pub fn receipts(&self) -> &[Receipt] { &self.receipts }
    pub fn pending_identity(&self) -> Option<[u8; 32]> { self.pending }
    pub(super) fn exact_receipt(&self, sequence: u64, operation: [u8; 16]) -> Result<Option<Receipt>> {
        if sequence == 0 { return Err(ManagedError::OperationConflict); }
        let Some(receipt) = self.receipts.iter().find(|r| r.sequence() == sequence) else { return Ok(None); };
        if receipt.operation() != operation { return Err(ManagedError::OperationConflict); }
        Ok(Some(receipt.clone()))
    }
}
pub(super) fn read(engine: &mut Engine) -> Result<ManagedCatalogObservation> {
    engine.gate(false)?;
    engine.guard(|engine| {
        engine.catalog.maps.encode()?;
        let mut registry = Vec::new();
        for row in engine.catalog.maps.registry.values() { registry.push(RegistryObservation { row: row.clone() }); }
        let mut receipts = Vec::new();
        for row in engine.catalog.maps.receipts.values() {
            let receipt = Receipt::decode(&row.result)?;
            if receipt.sequence() != row.sequence || receipt.operation() != row.operation
                || super::bytes::hash(&row.result) != row.digest { return Err(ManagedError::Corrupt); }
            receipts.push(receipt);
        }
        Ok(ManagedCatalogObservation { frontier: engine.catalog.frontier, registry, receipts,
            pending: engine.catalog.state.pending })
    })
}
