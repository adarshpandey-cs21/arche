use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{config::Credentials, Client};

pub async fn get_s3_client() -> Client {
    let region = Region::new("ap-south-1");

    let s3_cred_source = std::env::var("S3_CRED_SOURCE").unwrap_or("IAM".to_string());

    if s3_cred_source.to_lowercase() == "env" {
        Client::new(
            &aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(load_s3_cred_from_env())
                .load()
                .await,
        )
    } else {
        Client::new(
            &aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .load()
                .await,
        )
    }
}

fn load_s3_cred_from_env() -> Credentials {
    let access_key_id = std::env::var("S3_ACCESS_KEY_ID").expect("S3_ACCESS_KEY_ID is not set");
    let secret_access_key =
        std::env::var("S3_SECRET_ACCESS_KEY").expect("S3_SECRET_ACCESS_KEY is not set");

    Credentials::new(access_key_id, secret_access_key, None, None, "")
}
