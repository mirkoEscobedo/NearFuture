//! Fixed negative provenance diagnostics; no native certification or admission grant.
//!
//! A trusted receiver-side owner must establish the selected reason. Choosing a
//! variant verifies no detector, campaign checkpoint, lineage or network frontier.
//! This module performs no I/O and cannot grant an economic capability.
//! Chat/view authorization and availability are outside this module.

/// Negative provenance vocabulary. No clean, ready or admitted state is provided.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceRefusal {
    UnsupportedHook,
    MissingCheckpoint,
    UnverifiedDescendant,
    CopiedOrOlderSave,
    MetadataLost,
    NetworkTainted,
}

impl ProvenanceRefusal {
    /// Stable code with no caller-controlled details.
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedHook => "unsupported_hook",
            Self::MissingCheckpoint => "missing_checkpoint",
            Self::UnverifiedDescendant => "unverified_descendant",
            Self::CopiedOrOlderSave => "copied_or_older_save",
            Self::MetadataLost => "metadata_lost",
            Self::NetworkTainted => "network_tainted",
        }
    }

    /// Fixed explanation, accepting no paths, identifiers or command arguments.
    pub const fn message(self) -> &'static str {
        match self {
            Self::NetworkTainted => "The verified network lineage has recorded taint.",
            _ => "Campaign provenance is unverified.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProvenanceRefusal;

    #[test]
    fn six_negative_reasons_have_distinct_stable_codes() {
        let cases = [
            (ProvenanceRefusal::UnsupportedHook, "unsupported_hook"),
            (ProvenanceRefusal::MissingCheckpoint, "missing_checkpoint"),
            (
                ProvenanceRefusal::UnverifiedDescendant,
                "unverified_descendant",
            ),
            (ProvenanceRefusal::CopiedOrOlderSave, "copied_or_older_save"),
            (ProvenanceRefusal::MetadataLost, "metadata_lost"),
            (ProvenanceRefusal::NetworkTainted, "network_tainted"),
        ];
        for (index, (reason, expected)) in cases.iter().enumerate() {
            assert_eq!(reason.code(), *expected);
            assert!(expected.len() <= 32);
            assert!(
                expected
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
            );
            for (other, _) in &cases[..index] {
                assert_ne!(reason.code(), other.code());
            }
        }
    }

    #[test]
    fn missing_evidence_is_distinct_from_recorded_network_taint() {
        let missing = [
            ProvenanceRefusal::UnsupportedHook,
            ProvenanceRefusal::MissingCheckpoint,
            ProvenanceRefusal::UnverifiedDescendant,
            ProvenanceRefusal::CopiedOrOlderSave,
            ProvenanceRefusal::MetadataLost,
        ];
        let tainted = ProvenanceRefusal::NetworkTainted;
        assert_eq!(
            tainted.message(),
            "The verified network lineage has recorded taint."
        );
        for reason in missing {
            assert_ne!(reason, tainted);
            assert_ne!(reason.message(), tainted.message());
            assert!(!reason.message().contains("has recorded taint"));
            assert!(reason.message().len() <= 96);
        }
    }
}
