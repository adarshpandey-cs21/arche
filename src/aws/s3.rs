use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, config::Credentials};

use crate::config::{resolve_required_string, resolve_with_default};
use crate::error::AppError;

pub use crate::config::s3::{S3Config, S3ConfigBuilder};

const DEFAULT_REGION: &str = "ap-south-1";

pub async fn get_s3_client(config: impl Into<Option<S3Config>>) -> Result<Client, AppError> {
    let config = config.into().unwrap_or_default();

    let region_str = resolve_with_default(config.region, "S3_REGION", DEFAULT_REGION.to_string());
    let region = Region::new(region_str);

    let cred_source = resolve_with_default(
        config.credential_source,
        "S3_CRED_SOURCE",
        "IAM".to_string(),
    );

    if cred_source.to_lowercase() == "env" {
        let credentials = load_s3_cred_from_config(config.access_key_id, config.secret_access_key)?;
        Ok(Client::new(
            &aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(credentials)
                .load()
                .await,
        ))
    } else {
        Ok(Client::new(
            &aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .load()
                .await,
        ))
    }
}

fn load_s3_cred_from_config(
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
) -> Result<Credentials, AppError> {
    let access_key_id =
        resolve_required_string(access_key_id, "S3_ACCESS_KEY_ID", "access_key_id")?;
    let secret_access_key = resolve_required_string(
        secret_access_key,
        "S3_SECRET_ACCESS_KEY",
        "secret_access_key",
    )?;

    Ok(Credentials::new(
        access_key_id,
        secret_access_key,
        None,
        None,
        "",
    ))
}
