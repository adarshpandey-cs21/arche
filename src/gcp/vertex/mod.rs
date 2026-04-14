mod client;
mod config;
mod providers;
pub mod types;

pub use client::VertexClient;
pub use config::VertexConfig;
pub use types::*;

use crate::error::AppError;

pub async fn get_vertex_client(
    config: impl Into<Option<VertexConfig>>,
) -> Result<VertexClient, AppError> {
    let auth = config::resolve_auth(config.into()).await?;
    let http = reqwest::Client::new();
    Ok(VertexClient::new(http, auth))
}
