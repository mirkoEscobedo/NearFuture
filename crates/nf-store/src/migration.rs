//! Pure local composition identity. Supplied digests bind descriptors, not authority.
use nf_contract::identity::ProviderId;
use sha2::{Digest, Sha256};

const MAX_PROVIDERS: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionError {
    Limit,
    ZeroVersion,
    DuplicateProvider,
    UnsupportedProvider,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderVersion {
    pub provider: ProviderId,
    pub implementation: [u8; 32],
    pub version: u32,
    pub state_schema: u32,
}

#[derive(Debug)]
pub struct Composition {
    storage_schema: u32,
    schema: [u8; 32],
    config: [u8; 32],
    semantic_content: [u8; 32],
    providers: Vec<ProviderVersion>,
}

impl Composition {
    pub fn new(
        storage_schema: u32,
        schema: [u8; 32],
        config: [u8; 32],
        semantic_content: [u8; 32],
        mut providers: Vec<ProviderVersion>,
    ) -> Result<Self, CompositionError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDERS {
            return Err(CompositionError::Limit);
        }
        if storage_schema == 0
            || providers
                .iter()
                .any(|p| p.version == 0 || p.state_schema == 0)
        {
            return Err(CompositionError::ZeroVersion);
        }
        providers.sort_unstable_by_key(|p| p.provider);
        if providers
            .windows(2)
            .any(|pair| pair[0].provider == pair[1].provider)
        {
            return Err(CompositionError::DuplicateProvider);
        }
        Ok(Self {
            storage_schema,
            schema,
            config,
            semantic_content,
            providers,
        })
    }

    /// Closed local record; no NF-CANON wire registration or provider support claim.
    pub fn digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"NF-MIGRATION-COMPOSITION-1\0");
        hash.update(self.storage_schema.to_le_bytes());
        hash.update(self.schema);
        hash.update(self.config);
        hash.update(self.semantic_content);
        hash.update((self.providers.len() as u32).to_le_bytes());
        for provider in &self.providers {
            hash.update(provider.provider.as_bytes());
            hash.update(provider.implementation);
            hash.update(provider.version.to_le_bytes());
            hash.update(provider.state_schema.to_le_bytes());
        }
        hash.finalize().into()
    }
}

impl Composition {
    /// Compare required descriptors with an explicit caller-supplied support set.
    ///
    /// Implementation bytes remain opaque: matching grants no binary authenticity,
    /// route, activation, migration or recovery authority. Inputs are not changed.
    pub fn require_supported(&self, supported: &[ProviderVersion]) -> Result<(), CompositionError> {
        if self
            .providers
            .iter()
            .any(|required| !supported.contains(required))
        {
            return Err(CompositionError::UnsupportedProvider);
        }
        Ok(())
    }
}
