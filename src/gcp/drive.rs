use google_drive3::{
    DriveHub,
    hyper_util::{self, client::legacy::Client},
    yup_oauth2,
};

use crate::config::resolve_required_string;
use crate::error::AppError;
use crate::gcp::auth::{ProxiedConnector, build_proxy_aware_connector, build_sa_authenticator};

pub use crate::config::gcp::{GcpDriveConfig, GcpDriveConfigBuilder};

pub type GCPDriveClient = DriveHub<ProxiedConnector>;

#[allow(dead_code)]
pub async fn get_drive_client(
    config: impl Into<Option<GcpDriveConfig>>,
) -> Result<GCPDriveClient, AppError> {
    let config = config.into().unwrap_or_default();

    let gcp_drive_key = resolve_required_string(
        config.service_account_key_path,
        "GCP_DRIVE_KEY",
        "service_account_key_path",
    )?;

    let sa_key = yup_oauth2::read_service_account_key(&gcp_drive_key)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                service = "google_drive",
                "Failed to read service account key"
            );
            AppError::internal_error(
                format!(
                    "Config error [service_account_key/GCP_DRIVE_KEY]: Failed to read GCP key: {}",
                    e
                ),
                None,
            )
        })?;

    let authenticator = build_sa_authenticator("google_drive", sa_key).await?;
    let connector = build_proxy_aware_connector("google_drive")?;
    let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build(connector);

    Ok(DriveHub::new(client, authenticator))
}
