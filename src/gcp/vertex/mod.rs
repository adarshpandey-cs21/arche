mod client;
mod config;
mod providers;

pub use client::{VertexClient, VertexProvider};
pub use config::VertexConfig;

use crate::error::AppError;

pub async fn get_vertex_client(
    provider: VertexProvider,
    config: Option<VertexConfig>,
) -> Result<VertexClient, AppError> {
    let auth = config::resolve_auth(config).await?;
    let http = reqwest::Client::new();
    Ok(VertexClient::new(http, auth, provider))
}
