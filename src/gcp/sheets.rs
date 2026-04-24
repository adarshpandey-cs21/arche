use google_sheets4::{
    Sheets,
    hyper_util::{self, client::legacy::Client},
    yup_oauth2,
};

use crate::config::resolve_required_string;
use crate::error::AppError;
use crate::gcp::auth::{ProxiedConnector, build_proxy_aware_connector, build_sa_authenticator};

pub use crate::config::gcp::{GcpSheetsConfig, GcpSheetsConfigBuilder};

pub type GCPSheetsClient = Sheets<ProxiedConnector>;

#[allow(dead_code)]
pub async fn get_sheets_client(
    config: impl Into<Option<GcpSheetsConfig>>,
) -> Result<GCPSheetsClient, AppError> {
    let config = config.into().unwrap_or_default();

    let gcp_sheets_key = resolve_required_string(
        config.service_account_key_path,
        "GCP_SHEETS_KEY",
        "service_account_key_path",
    )?;

    let sa_key = yup_oauth2::read_service_account_key(&gcp_sheets_key)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                service = "google_sheets",
                "Failed to read service account key"
            );
            AppError::internal_error(
                format!(
                    "Config error [service_account_key/GCP_SHEETS_KEY]: Failed to read GCP key: {}",
                    e
                ),
                None,
            )
        })?;

    let authenticator = build_sa_authenticator("google_sheets", sa_key).await?;
    let connector = build_proxy_aware_connector("google_sheets")?;
    let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build(connector);

    Ok(Sheets::new(client, authenticator))
}
