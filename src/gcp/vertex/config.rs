use crate::config::{resolve_optional, resolve_required, resolve_with_default};
use crate::error::AppError;
use google_drive3::{
    hyper_rustls::HttpsConnector, hyper_util::client::legacy::connect::HttpConnector, yup_oauth2,
};

#[derive(Debug, Clone, Default)]
pub struct VertexConfig {
    pub api_key: Option<String>,
    pub project_id: Option<String>,
    pub region: Option<String>,
    pub service_account_path: Option<String>,
    pub service_account_json: Option<String>,
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

    pub fn with_service_account_path(mut self, val: impl Into<String>) -> Self {
        self.service_account_path = Some(val.into());
        self
    }

    pub fn with_service_account_json(mut self, val: impl Into<String>) -> Self {
        self.service_account_json = Some(val.into());
        self
    }
}

pub(crate) type VertexAuthenticator =
    yup_oauth2::authenticator::Authenticator<HttpsConnector<HttpConnector>>;

pub(crate) enum ResolvedAuth {
    ApiKey {
        api_key: String,
    },
    ServiceAccount {
        project_id: String,
        region: String,
        authenticator: VertexAuthenticator,
    },
}

pub(crate) async fn resolve_auth(config: Option<VertexConfig>) -> Result<ResolvedAuth, AppError> {
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

        let sa_key = if let Some(json) = config.service_account_json {
            yup_oauth2::parse_service_account_key(json).map_err(|e| {
                tracing::error!(
                    error = %e,
                    service = "vertex_ai",
                    "Failed to parse service account JSON"
                );
                AppError::internal_error(
                    format!("Failed to parse Vertex AI service account JSON: {e}"),
                    None,
                )
            })?
        } else {
            let path = resolve_required(
                config.service_account_path,
                "GOOGLE_APPLICATION_CREDENTIALS",
                "service_account_path or service_account_json",
            )?;
            yup_oauth2::read_service_account_key(&path)
                .await
                .map_err(|e| {
                    tracing::error!(
                        error = %e,
                        service = "vertex_ai",
                        "Failed to read service account key"
                    );
                    AppError::internal_error(
                        format!("Failed to read Vertex AI service account key: {e}"),
                        None,
                    )
                })?
        };

        let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(sa_key)
            .build()
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    service = "vertex_ai",
                    "Failed to build authenticator"
                );
                AppError::internal_error(
                    format!("Failed to build Vertex AI authenticator: {e}"),
                    None,
                )
            })?;

        return Ok(ResolvedAuth::ServiceAccount {
            project_id,
            region,
            authenticator,
        });
    }

    Err(AppError::internal_error(
        "Vertex AI requires either VERTEX_API_KEY/GEMINI_API_KEY or VERTEX_PROJECT_ID + GOOGLE_APPLICATION_CREDENTIALS".into(),
        None,
    ))
}
