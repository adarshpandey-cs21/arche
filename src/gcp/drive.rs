use crate::error::AppError;
use crate::gcp::client::GcpClient;
use crate::gcp::token::ServiceAccountKey;

const SCOPE: &str = "https://www.googleapis.com/auth/drive";

pub async fn client(
    key: Option<ServiceAccountKey>,
    path: Option<String>,
) -> Result<GcpClient, AppError> {
    GcpClient::new(key, path, [SCOPE]).await
}
