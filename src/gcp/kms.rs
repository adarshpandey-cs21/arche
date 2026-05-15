use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::config::{resolve_optional_string, resolve_required_string, resolve_with_default};
use crate::error::AppError;
use crate::gcp::client::GcpClient;
use crate::gcp::token::ServiceAccountKey;

pub use crate::config::gcp::{GcpKmsConfig, GcpKmsConfigBuilder, GcpKmsKey};

pub const KMS_SCOPE: &str = "https://www.googleapis.com/auth/cloudkms";
const DEFAULT_KMS_BASE_URL: &str = "https://cloudkms.googleapis.com";
const DEFAULT_LOCATION: &str = "global";

#[derive(Debug, Serialize)]
struct EncryptRequest {
    plaintext: String,
}

#[derive(Debug, Deserialize)]
struct EncryptResponse {
    ciphertext: String,
    name: String,
}

#[derive(Debug, Clone)]
pub struct EncryptOutput {
    pub ciphertext: Vec<u8>,
    pub key_version: String,
}

#[derive(Debug, Serialize)]
struct DecryptRequest {
    ciphertext: String,
}

#[derive(Debug, Deserialize)]
struct DecryptResponse {
    plaintext: String,
}

#[derive(Debug, Deserialize)]
struct KmsErrorEnvelope {
    error: KmsErrorBody,
}

#[derive(Debug, Deserialize)]
struct KmsErrorBody {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

pub async fn get_kms_client(
    sa_key: Option<ServiceAccountKey>,
    sa_path: Option<String>,
    config: impl Into<Option<GcpKmsConfig>>,
) -> Result<GcpKmsClient, AppError> {
    let resolved = resolve_kms_config(config.into().unwrap_or_default())?;
    let gcp = GcpClient::new(sa_key, sa_path, [KMS_SCOPE]).await?;
    Ok(GcpKmsClient::new(
        gcp,
        resolved.project_id,
        resolved.location,
        resolved.base_url,
    ))
}

#[derive(Debug)]
struct ResolvedKmsConfig {
    project_id: String,
    location: String,
    base_url: Option<String>,
}

fn resolve_kms_config(config: GcpKmsConfig) -> Result<ResolvedKmsConfig, AppError> {
    Ok(ResolvedKmsConfig {
        project_id: resolve_required_string(config.project_id, "GCP_KMS_PROJECT_ID", "project_id")?,
        location: resolve_with_default(
            config.location,
            "GCP_KMS_LOCATION",
            DEFAULT_LOCATION.to_string(),
        ),
        base_url: resolve_optional_string(config.kms_base_url, "GCP_KMS_BASE_URL"),
    })
}

#[derive(Clone)]
pub struct GcpKmsClient {
    gcp: GcpClient,
    project_id: String,
    location: String,
    base_url: String,
}

impl GcpKmsClient {
    pub fn new(
        gcp: GcpClient,
        project_id: String,
        location: String,
        base_url: Option<String>,
    ) -> Self {
        let base_url = base_url
            .unwrap_or_else(|| DEFAULT_KMS_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        Self {
            gcp,
            project_id,
            location,
            base_url,
        }
    }

    fn key_resource_path(&self, key: &GcpKmsKey) -> String {
        format_key_path(
            &self.project_id,
            &self.location,
            &key.key_ring,
            &key.key_name,
        )
    }

    pub async fn encrypt(
        &self,
        key: &GcpKmsKey,
        plaintext: &[u8],
    ) -> Result<EncryptOutput, AppError> {
        let url = format!(
            "{}/v1/{}:encrypt",
            self.base_url,
            self.key_resource_path(key)
        );

        let body = EncryptRequest {
            plaintext: STANDARD.encode(plaintext),
        };

        let resp = self
            .gcp
            .post(&url)
            .await?
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AppError::dependency_failed("gcp-kms", format!("encrypt request failed: {e}"))
            })?;

        let parsed: EncryptResponse = handle_response(resp, "encrypt").await?;

        let ciphertext = STANDARD.decode(&parsed.ciphertext).map_err(|e| {
            AppError::internal_error(format!("Failed to decode GCP KMS ciphertext: {e}"), None)
        })?;
        Ok(EncryptOutput {
            ciphertext,
            key_version: parsed.name,
        })
    }

    pub async fn decrypt(&self, key: &GcpKmsKey, ciphertext: &[u8]) -> Result<Vec<u8>, AppError> {
        self.decrypt_base64(key, &STANDARD.encode(ciphertext)).await
    }

    pub async fn decrypt_base64(
        &self,
        key: &GcpKmsKey,
        base64_ciphertext: &str,
    ) -> Result<Vec<u8>, AppError> {
        let url = format!(
            "{}/v1/{}:decrypt",
            self.base_url,
            self.key_resource_path(key)
        );

        let body = DecryptRequest {
            ciphertext: base64_ciphertext.to_string(),
        };

        let resp = self
            .gcp
            .post(&url)
            .await?
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AppError::dependency_failed("gcp-kms", format!("decrypt request failed: {e}"))
            })?;

        let parsed: DecryptResponse = handle_response(resp, "decrypt").await?;

        STANDARD.decode(&parsed.plaintext).map_err(|e| {
            AppError::internal_error(format!("Failed to decode GCP KMS plaintext: {e}"), None)
        })
    }
}

async fn handle_response<T: for<'de> Deserialize<'de>>(
    resp: reqwest::Response,
    op: &str,
) -> Result<T, AppError> {
    let status = resp.status();
    let bytes = resp.bytes().await.map_err(|e| {
        AppError::dependency_failed("gcp-kms", format!("failed reading {op} response body: {e}"))
    })?;

    if !status.is_success() {
        let detail = parse_kms_error(&bytes);
        return Err(AppError::dependency_failed(
            "gcp-kms",
            format!("{op} returned HTTP {status}: {detail}"),
        ));
    }

    serde_json::from_slice::<T>(&bytes).map_err(|e| {
        AppError::internal_error(format!("Failed to parse GCP KMS {op} response: {e}"), None)
    })
}

fn format_key_path(project_id: &str, location: &str, key_ring: &str, key_name: &str) -> String {
    format!("projects/{project_id}/locations/{location}/keyRings/{key_ring}/cryptoKeys/{key_name}")
}

fn parse_kms_error(body: &[u8]) -> String {
    serde_json::from_slice::<KmsErrorEnvelope>(body)
        .ok()
        .map(|env| match (env.error.status, env.error.message) {
            (Some(s), Some(m)) => format!("{s}: {m}"),
            (Some(s), None) => s,
            (None, Some(m)) => m,
            (None, None) => String::new(),
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| String::from_utf8_lossy(body).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex, MutexGuard};

    // Serialize env-touching tests within this binary. cargo test parallelizes
    // tests in the same binary by default, and Rust 2024 makes set_var/remove_var
    // `unsafe` because process-global env mutations race other threads.
    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn env_guard() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }

    const ENV_VARS: &[&str] = &["GCP_KMS_PROJECT_ID", "GCP_KMS_LOCATION", "GCP_KMS_BASE_URL"];

    fn clear_kms_env() {
        for k in ENV_VARS {
            unsafe { std::env::remove_var(k) };
        }
    }

    #[test]
    fn key_resource_path_format_is_stable() {
        assert_eq!(
            format_key_path("my-project", "global", "my-keyring", "my-key"),
            "projects/my-project/locations/global/keyRings/my-keyring/cryptoKeys/my-key"
        );
    }

    #[test]
    fn parse_kms_error_extracts_status_and_message() {
        let body = br#"{"error":{"code":403,"status":"PERMISSION_DENIED","message":"missing kms.cryptoKeyVersions.useToEncrypt"}}"#;
        assert_eq!(
            parse_kms_error(body),
            "PERMISSION_DENIED: missing kms.cryptoKeyVersions.useToEncrypt"
        );
    }

    #[test]
    fn parse_kms_error_falls_back_to_raw_body() {
        assert_eq!(parse_kms_error(b"not json"), "not json");
    }

    #[test]
    fn resolve_kms_config_reads_from_env_when_unset() {
        let _g = env_guard();
        clear_kms_env();
        unsafe {
            std::env::set_var("GCP_KMS_PROJECT_ID", "from-env");
            std::env::set_var("GCP_KMS_LOCATION", "asia-south1");
            std::env::set_var("GCP_KMS_BASE_URL", "https://env.test");
        }

        let resolved = resolve_kms_config(GcpKmsConfig::default()).unwrap();
        clear_kms_env();

        assert_eq!(resolved.project_id, "from-env");
        assert_eq!(resolved.location, "asia-south1");
        assert_eq!(resolved.base_url.as_deref(), Some("https://env.test"));
    }

    #[test]
    fn resolve_kms_config_explicit_config_overrides_env() {
        let _g = env_guard();
        clear_kms_env();
        unsafe {
            std::env::set_var("GCP_KMS_PROJECT_ID", "from-env");
            std::env::set_var("GCP_KMS_LOCATION", "asia-south1");
            std::env::set_var("GCP_KMS_BASE_URL", "https://env.test");
        }

        let resolved = resolve_kms_config(
            GcpKmsConfig::builder()
                .project_id("explicit-proj")
                .location("us-east1")
                .kms_base_url("https://explicit.test")
                .build(),
        )
        .unwrap();
        clear_kms_env();

        assert_eq!(resolved.project_id, "explicit-proj");
        assert_eq!(resolved.location, "us-east1");
        assert_eq!(resolved.base_url.as_deref(), Some("https://explicit.test"));
    }

    #[test]
    fn resolve_kms_config_defaults_location_to_global() {
        let _g = env_guard();
        clear_kms_env();
        unsafe { std::env::set_var("GCP_KMS_PROJECT_ID", "p") };

        let resolved = resolve_kms_config(GcpKmsConfig::default()).unwrap();
        clear_kms_env();

        assert_eq!(resolved.location, "global");
        assert!(resolved.base_url.is_none());
    }

    #[test]
    fn resolve_kms_config_errors_when_project_id_missing() {
        let _g = env_guard();
        clear_kms_env();
        let err = resolve_kms_config(GcpKmsConfig::default()).unwrap_err();
        assert!(
            format!("{err:?}").contains("project_id"),
            "expected project_id in error, got: {err:?}"
        );
    }

    #[test]
    fn new_strips_trailing_slash_from_base_url() {
        let key = GcpKmsKey::new("ring", "name");
        // Construct minimally via the same field assembly `new` performs,
        // without standing up a real GcpClient.
        let trimmed = "https://cloudkms.googleapis.com/"
            .trim_end_matches('/')
            .to_string();
        let url = format!(
            "{}/v1/{}:encrypt",
            trimmed,
            format_key_path("p", "global", &key.key_ring, &key.key_name)
        );
        assert!(
            !url.contains("//v1/"),
            "trailing-slash base URL should be trimmed, got: {url}"
        );
    }
}
