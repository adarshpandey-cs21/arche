use crate::config::{resolve_optional, resolve_with_default};
use crate::error::AppError;
use crate::gcp::token::{ServiceAccountKey, TokenSource};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct VertexConfig {
    pub api_key: Option<String>,
    pub project_id: Option<String>,
    pub region: Option<String>,
    pub service_account_key: Option<ServiceAccountKey>,
    pub service_account_key_path: Option<String>,
}

impl VertexConfig {
    pub fn with_api_key(mut self, val: impl Into<String>) -> Self {
        self.api_key = Some(val.into());
        self
    }

    pub fn with_project_id(mut self, val: impl Into<String>) -> Self {
        self.project_id = Some(val.into());
        self
    }

    pub fn with_region(mut self, val: impl Into<String>) -> Self {
        self.region = Some(val.into());
        self
    }

    pub fn with_service_account_key(mut self, key: ServiceAccountKey) -> Self {
        self.service_account_key = Some(key);
        self
    }

    pub fn with_service_account_key_path(mut self, val: impl Into<String>) -> Self {
        self.service_account_key_path = Some(val.into());
        self
    }
}

pub(crate) enum ResolvedAuth {
    ApiKey {
        api_key: String,
    },
    ServiceAccount {
        project_id: String,
        region: String,
        token_source: Arc<TokenSource>,
    },
}

pub(crate) async fn resolve_auth(
    config: Option<VertexConfig>,
    http: reqwest::Client,
) -> Result<ResolvedAuth, AppError> {
    let config = config.unwrap_or_default();

    let api_key = resolve_optional(config.api_key, "VERTEX_API_KEY")
        .or_else(|| resolve_optional::<String>(None, "GEMINI_API_KEY"));

    let project_id = resolve_optional(config.project_id, "VERTEX_PROJECT_ID");

    if let Some(api_key) = api_key {
        return Ok(ResolvedAuth::ApiKey { api_key });
    }

    if let Some(project_id) = project_id {
        let region =
            resolve_with_default(config.region, "VERTEX_REGION", "asia-south1".to_string());

        let sa_key = match (config.service_account_key, config.service_account_key_path) {
            (Some(key), _) => key,
            (None, Some(path)) => ServiceAccountKey::from_path(&path).await?,
            (None, None) => {
                return Err(AppError::internal_error(
                    "Vertex AI service-account auth requires service_account_key or service_account_key_path".into(),
                    None,
                ));
            }
        };

        let token_source = Arc::new(TokenSource::new(http, sa_key));

        return Ok(ResolvedAuth::ServiceAccount {
            project_id,
            region,
            token_source,
        });
    }

    Err(AppError::internal_error(
        "Vertex AI requires either VERTEX_API_KEY/GEMINI_API_KEY, or VERTEX_PROJECT_ID + service_account_key/service_account_key_path".into(),
        None,
    ))
}
