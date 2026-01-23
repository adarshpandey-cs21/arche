use google_sheets4::{
    hyper_rustls, hyper_util,
    yup_oauth2::{self, ServiceAccountAuthenticator},
    Sheets,
};

pub type GCPSheetsClient =
    Sheets<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

#[allow(dead_code)]
pub async fn get_sheets_client() -> Result<GCPSheetsClient, String> {
    let gcp_sheets_key = std::env::var("GCP_SHEETS_KEY").map_err(|e| {
        tracing::error!(
            error = %e,
            env_var = "GCP_SHEETS_KEY",
            "Missing GCP Sheets configuration"
        );
        format!("GCP_SHEETS_KEY not configured: {}", e)
    })?;

    let auth = yup_oauth2::read_service_account_key(gcp_sheets_key)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                service = "google_sheets",
                "Failed to read service account key"
            );
            format!("Failed to read GCP key: {}", e)
        })?;

    let authenticator = ServiceAccountAuthenticator::builder(auth)
        .build()
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                service = "google_sheets",
                "Failed to build authenticator"
            );
            format!("Failed to build GCP auth: {}", e)
        })?;

    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| {
            tracing::error!(
                error = %e,
                service = "google_sheets",
                "Failed to build HTTPS connector"
            );
            format!("Failed to build HTTPS connector: {}", e)
        })?
        .https_or_http()
        .enable_http1()
        .build();

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector);

    Ok(Sheets::new(client, authenticator))
}
