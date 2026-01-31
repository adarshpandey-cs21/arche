use aws_config::{BehaviorVersion, Region};
use aws_sdk_kms::{Client, primitives::Blob};
use base64::{Engine, engine::general_purpose::STANDARD};

use crate::config::resolve_with_default;
use crate::error::AppError;

pub use crate::config::aws::{AwsConfig, AwsConfigBuilder};

const DEFAULT_REGION: &str = "ap-south-1";

pub async fn get_kms_client(config: impl Into<Option<AwsConfig>>) -> Client {
    let config = config.into().unwrap_or_default();
    let region_str = resolve_with_default(config.region, "AWS_REGION", DEFAULT_REGION.to_string());
    let region = Region::new(region_str);

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    Client::new(&aws_config)
}

pub struct KMSClient {
    client: Client,
}

impl KMSClient {
    pub fn new(client: Client) -> Self {
        KMSClient { client }
    }

    #[allow(dead_code)]
    pub async fn new_with_region(region_name: &str) -> Self {
        let region = Region::new(region_name.to_string());
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await;
        let client = Client::new(&config);

        KMSClient { client }
    }

    pub async fn encrypt(&self, key_id: &str, plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
        let response = self
            .client
            .encrypt()
            .key_id(key_id)
            .plaintext(Blob::new(plaintext))
            .send()
            .await
            .map_err(|e| AppError::internal_error(format!("KMS encrypt failed: {:?}", e), None))?;

        let ciphertext = response
            .ciphertext_blob()
            .ok_or_else(|| {
                AppError::internal_error("KMS encrypt returned no ciphertext".to_string(), None)
            })?
            .as_ref()
            .to_vec();

        Ok(ciphertext)
    }

    pub async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, AppError> {
        let response = self
            .client
            .decrypt()
            .ciphertext_blob(Blob::new(ciphertext))
            .send()
            .await
            .map_err(|e| AppError::internal_error(format!("KMS decrypt failed: {:?}", e), None))?;

        let plaintext = response
            .plaintext()
            .ok_or_else(|| {
                AppError::internal_error("KMS decrypt returned no plaintext".to_string(), None)
            })?
            .as_ref()
            .to_vec();

        Ok(plaintext)
    }

    pub async fn decrypt_base64(&self, ciphertext: &str) -> Result<Vec<u8>, AppError> {
        let decoded = STANDARD.decode(ciphertext).map_err(|e| {
            AppError::internal_error(format!("Base64 decode failed: {:?}", e), None)
        })?;
        self.decrypt(&decoded).await
    }
}
