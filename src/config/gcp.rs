use crate::config::resolve_required_string;
use crate::error::AppError;

#[derive(Debug, Clone, Default)]
pub struct GcpKmsConfig {
    pub project_id: Option<String>,
    pub location: Option<String>,
    pub kms_base_url: Option<String>,
}

impl GcpKmsConfig {
    pub fn builder() -> GcpKmsConfigBuilder {
        GcpKmsConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct GcpKmsConfigBuilder {
    project_id: Option<String>,
    location: Option<String>,
    kms_base_url: Option<String>,
}

impl GcpKmsConfigBuilder {
    pub fn project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    pub fn kms_base_url(mut self, url: impl Into<String>) -> Self {
        self.kms_base_url = Some(url.into());
        self
    }

    pub fn build(self) -> GcpKmsConfig {
        GcpKmsConfig {
            project_id: self.project_id,
            location: self.location,
            kms_base_url: self.kms_base_url,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GcpKmsKey {
    pub key_ring: String,
    pub key_name: String,
}

impl GcpKmsKey {
    pub fn new(key_ring: impl Into<String>, key_name: impl Into<String>) -> Self {
        Self {
            key_ring: key_ring.into(),
            key_name: key_name.into(),
        }
    }

    pub fn from_env() -> Result<Self, AppError> {
        let key_ring = resolve_required_string(None, "GCP_KMS_KEY_RING", "key_ring")?;
        let key_name = resolve_required_string(None, "GCP_KMS_KEY_NAME", "key_name")?;
        Ok(Self { key_ring, key_name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builder_collects_fields() {
        let config = GcpKmsConfig::builder()
            .project_id("p")
            .location("us-east1")
            .kms_base_url("https://example.test")
            .build();

        assert_eq!(config.project_id.as_deref(), Some("p"));
        assert_eq!(config.location.as_deref(), Some("us-east1"));
        assert_eq!(config.kms_base_url.as_deref(), Some("https://example.test"));
    }

    #[test]
    fn config_default_is_all_none() {
        let config = GcpKmsConfig::default();
        assert!(config.project_id.is_none());
        assert!(config.location.is_none());
        assert!(config.kms_base_url.is_none());
    }

    #[test]
    fn key_new_stores_fields() {
        let key = GcpKmsKey::new("ring", "name");
        assert_eq!(key.key_ring, "ring");
        assert_eq!(key.key_name, "name");
    }
}
