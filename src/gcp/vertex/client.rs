use crate::error::AppError;
use futures::Stream;
use std::pin::Pin;

use super::config::ResolvedAuth;
use super::providers;
use super::types::*;

pub struct VertexClient {
    pub(crate) http: reqwest::Client,
    pub(crate) auth: ResolvedAuth,
}

impl VertexClient {
    pub(crate) fn new(http: reqwest::Client, auth: ResolvedAuth) -> Self {
        Self { http, auth }
    }

    pub async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, AppError> {
        match &request.provider {
            Provider::Gemini => providers::gemini::generate(self, request).await,
            Provider::Anthropic => providers::anthropic::generate(self, request).await,
        }
    }

    pub async fn stream_generate(
        &self,
        request: &GenerateRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AppError>> + Send>>, AppError> {
        match &request.provider {
            Provider::Gemini => providers::gemini::stream_generate(self, request).await,
            Provider::Anthropic => providers::anthropic::stream_generate(self, request).await,
        }
    }

    pub(crate) async fn auth_header(&self) -> Result<Option<String>, AppError> {
        match &self.auth {
            ResolvedAuth::ApiKey { .. } => Ok(None),
            ResolvedAuth::ServiceAccount { authenticator, .. } => {
                let token = authenticator
                    .token(&["https://www.googleapis.com/auth/cloud-platform"])
                    .await
                    .map_err(|e| {
                        AppError::internal_error(
                            format!("Failed to fetch Vertex AI access token: {e}"),
                            None,
                        )
                    })?;

                let bearer = token.token().ok_or_else(|| {
                    AppError::internal_error(
                        "Vertex AI token response contained no access token".into(),
                        None,
                    )
                })?;

                Ok(Some(format!("Bearer {bearer}")))
            }
        }
    }

    pub(crate) async fn send(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, AppError> {
        let resp = req.send().await.map_err(|e| {
            AppError::dependency_failed("vertex-ai", format!("Request failed: {e}"))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::dependency_failed(
                "vertex-ai",
                format!("API error ({status}): {body}"),
            ));
        }

        Ok(resp)
    }
}
