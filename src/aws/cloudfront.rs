use aws_config::{BehaviorVersion, Region};
use aws_sdk_cloudfront::{
    Client,
    types::{InvalidationBatch, Paths},
};

use crate::config::{resolve_optional_string, resolve_with_default};
use crate::error::AppError;

pub use crate::config::cloudfront::{CloudFrontConfig, CloudFrontConfigBuilder};

const DEFAULT_REGION: &str = "ap-south-1";

const MAX_PATHS_PER_INVALIDATION: usize = 3000;

pub async fn get_cloudfront_client(config: impl Into<Option<CloudFrontConfig>>) -> Client {
    let config = config.into().unwrap_or_default();
    let region_str = resolve_with_default(config.region, "AWS_REGION", DEFAULT_REGION.to_string());
    let region = Region::new(region_str);

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    Client::new(&aws_config)
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct InvalidationResult {
    pub id: String,
    pub status: String,
}

pub struct CloudFrontClient {
    client: Client,
    default_distribution_id: Option<String>,
}

impl CloudFrontClient {
    pub fn new(client: Client, config: impl Into<Option<CloudFrontConfig>>) -> Self {
        let config = config.into().unwrap_or_default();
        let default_distribution_id =
            resolve_optional_string(config.distribution_id, "CLOUDFRONT_DISTRIBUTION_ID");
        CloudFrontClient {
            client,
            default_distribution_id,
        }
    }

    fn resolve_distribution_id(
        &self,
        distribution_id: Option<&str>,
        method: &str,
    ) -> Result<String, AppError> {
        distribution_id
            .map(|s| s.to_string())
            .or_else(|| self.default_distribution_id.clone())
            .ok_or_else(|| {
                AppError::bad_request(
                    None,
                    Some(format!(
                        "CloudFront {}: distribution_id not provided and no default configured",
                        method
                    )),
                    None,
                )
            })
    }

    /// If `caller_reference` is `None`, a fresh nanoid is generated, so retries will create
    /// duplicate invalidations. Pass a stable reference for retry-safe idempotency.
    pub async fn invalidate_paths(
        &self,
        distribution_id: Option<&str>,
        paths: Vec<String>,
        caller_reference: Option<&str>,
    ) -> Result<InvalidationResult, AppError> {
        let distribution_id = self.resolve_distribution_id(distribution_id, "invalidate_paths")?;

        if paths.is_empty() {
            return Err(AppError::bad_request(
                None,
                Some("CloudFront invalidate_paths: paths must not be empty".to_string()),
                None,
            ));
        }

        if paths.len() > MAX_PATHS_PER_INVALIDATION {
            return Err(AppError::bad_request(
                None,
                Some(format!(
                    "CloudFront invalidate_paths: {} paths exceeds the {}-path per-invalidation limit",
                    paths.len(),
                    MAX_PATHS_PER_INVALIDATION
                )),
                None,
            ));
        }

        let caller_reference = caller_reference
            .map(|s| s.to_string())
            .unwrap_or_else(|| nanoid::nanoid!());

        let paths_obj = Paths::builder()
            .quantity(paths.len() as i32)
            .set_items(Some(paths))
            .build()
            .map_err(|e| {
                AppError::internal_error(
                    format!(
                        "CloudFront invalidate_paths: failed to build Paths: {:?}",
                        e
                    ),
                    None,
                )
            })?;

        let batch = InvalidationBatch::builder()
            .paths(paths_obj)
            .caller_reference(caller_reference)
            .build()
            .map_err(|e| {
                AppError::internal_error(
                    format!(
                        "CloudFront invalidate_paths: failed to build InvalidationBatch: {:?}",
                        e
                    ),
                    None,
                )
            })?;

        let response = self
            .client
            .create_invalidation()
            .distribution_id(distribution_id)
            .invalidation_batch(batch)
            .send()
            .await
            .map_err(|e| {
                AppError::dependency_failed(
                    "cloudfront",
                    format!("CloudFront create_invalidation failed: {:?}", e),
                )
            })?;

        let invalidation = response.invalidation().ok_or_else(|| {
            AppError::dependency_failed(
                "cloudfront",
                "CloudFront create_invalidation returned no invalidation",
            )
        })?;

        Ok(InvalidationResult {
            id: invalidation.id().to_string(),
            status: invalidation.status().to_string(),
        })
    }

    pub async fn get_invalidation(
        &self,
        distribution_id: Option<&str>,
        invalidation_id: &str,
    ) -> Result<InvalidationResult, AppError> {
        let distribution_id = self.resolve_distribution_id(distribution_id, "get_invalidation")?;

        if invalidation_id.is_empty() {
            return Err(AppError::bad_request(
                None,
                Some("CloudFront get_invalidation: invalidation_id must not be empty".to_string()),
                None,
            ));
        }

        let response = self
            .client
            .get_invalidation()
            .distribution_id(distribution_id)
            .id(invalidation_id)
            .send()
            .await
            .map_err(|e| {
                AppError::dependency_failed(
                    "cloudfront",
                    format!("CloudFront get_invalidation failed: {:?}", e),
                )
            })?;

        let invalidation = response.invalidation().ok_or_else(|| {
            AppError::dependency_failed(
                "cloudfront",
                "CloudFront get_invalidation returned no invalidation",
            )
        })?;

        Ok(InvalidationResult {
            id: invalidation.id().to_string(),
            status: invalidation.status().to_string(),
        })
    }
}
