//! Explicit bounded preview of a single redacted support document.
//! No private-file collection, filesystem export, repair or authority is granted.
use crate::{StatusProjectionError, project_miniature_status};
use nf_store::miniature::MiniatureSnapshot;

/// Closed preview bytes. This value grants no file or administration capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedactedMiniatureSupportPreviewV1(String);
impl RedactedMiniatureSupportPreviewV1 {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Preview exactly one allowlisted document from supplied public snapshot counters.
/// Package version is build metadata; source revision and other coverage are unknown.
pub fn preview_miniature_support(
    snapshot: &MiniatureSnapshot,
) -> Result<RedactedMiniatureSupportPreviewV1, StatusProjectionError> {
    let version = env!("CARGO_PKG_VERSION");
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return Err(StatusProjectionError::OutputLimit);
    }
    let status = project_miniature_status(snapshot)?;
    let json = format!(
        concat!(
            "{{\"format\":\"NF-OPERATOR-SUPPORT-PREVIEW-1\",",
            "\"coverage\":\"miniature_status_only\",",
            "\"software\":{{\"package\":\"nf-world-driver\",\"version\":\"{}\",",
            "\"source_revision\":null}},",
            "\"files\":[{{\"name\":\"status.json\",\"media_type\":\"application/json\",",
            "\"content\":{}}}],",
            "\"excluded\":[\"keys\",\"private_chat\",\"save_content\",\"sensitive_paths\"],",
            "\"unavailable\":[\"logs\",\"manifests\",\"transfers\",\"backup_recovery\"]}}"
        ),
        version,
        status.as_str(),
    );
    if json.len() > 4096 {
        return Err(StatusProjectionError::OutputLimit);
    }
    Ok(RedactedMiniatureSupportPreviewV1(json))
}
