mod client;
mod config;
mod providers;

pub use client::{VertexClient, VertexProvider};
pub use config::VertexConfig;

use std::time::Duration;

use crate::error::AppError;

pub async fn get_vertex_client(
    provider: VertexProvider,
    config: Option<VertexConfig>,
) -> Result<VertexClient, AppError> {
    let http = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| {
            AppError::internal_error(format!("Failed to build Vertex HTTP client: {e}"), None)
        })?;
    let auth = config::resolve_auth(config, http.clone()).await?;
    Ok(VertexClient::new(http, auth, provider))
}
