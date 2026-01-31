use aws_config::{BehaviorVersion, Region};
use aws_sdk_sesv2::Client;

use crate::config::resolve_with_default;
use crate::error::AppError;

pub use crate::aws::kms::{AwsConfig, AwsConfigBuilder};

const DEFAULT_REGION: &str = "ap-south-1";

pub async fn get_ses_client(config: impl Into<Option<AwsConfig>>) -> Client {
    let config = config.into().unwrap_or_default();
    let region_str = resolve_with_default(config.region, "AWS_REGION", DEFAULT_REGION.to_string());
    let region = Region::new(region_str);

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    Client::new(&aws_config)
}

pub struct SESClient {
    client: Client,
}

impl SESClient {
    pub fn new(client: Client) -> Self {
        SESClient { client }
    }

    #[allow(dead_code)]
    pub async fn new_with_region(region_name: &str) -> Self {
        let region = Region::new(region_name.to_string());
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await;
        let client = Client::new(&config);

        SESClient { client }
    }

    pub async fn send_email(
        &self,
        from_email: &str,
        to_email: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
    ) -> Result<String, AppError> {
        let text_content = aws_sdk_sesv2::types::Content::builder()
            .data(body_text)
            .build()
            .map_err(|e| {
                AppError::internal_error(format!("Failed to build text content: {:?}", e), None)
            })?;

        let mut body_builder = aws_sdk_sesv2::types::Body::builder().text(text_content);

        if let Some(html) = body_html {
            let html_content = aws_sdk_sesv2::types::Content::builder()
                .data(html)
                .build()
                .map_err(|e| {
                    AppError::internal_error(
                        "Failed to build HTML".to_string(),
                        Some(format!("Failed to build HTML content: {:?}", e)),
                    )
                })?;
            body_builder = body_builder.html(html_content);
        }

        let subject_content = aws_sdk_sesv2::types::Content::builder()
            .data(subject)
            .build()
            .map_err(|e| {
                AppError::internal_error(
                    "Failed to build subject".to_string(),
                    Some(format!("Failed to build subject content: {:?}", e)),
                )
            })?;

        let message = aws_sdk_sesv2::types::Message::builder()
            .subject(subject_content)
            .body(body_builder.build())
            .build();

        let destination = aws_sdk_sesv2::types::Destination::builder()
            .to_addresses(to_email)
            .build();

        let email_content = aws_sdk_sesv2::types::EmailContent::builder()
            .simple(message)
            .build();

        let response = self
            .client
            .send_email()
            .from_email_address(from_email)
            .destination(destination)
            .content(email_content)
            .send()
            .await
            .map_err(|e| {
                AppError::internal_error(format!("SES send_email failed: {:?}", e), None)
            })?;

        let message_id = response.message_id().unwrap_or("").to_string();
        Ok(message_id)
    }

    #[allow(dead_code)]
    pub async fn send_templated_email(
        &self,
        from_email: &str,
        to_email: &str,
        template_name: &str,
        template_data: &str,
    ) -> Result<String, AppError> {
        let response = self
            .client
            .send_email()
            .from_email_address(from_email)
            .destination(
                aws_sdk_sesv2::types::Destination::builder()
                    .to_addresses(to_email)
                    .build(),
            )
            .content(
                aws_sdk_sesv2::types::EmailContent::builder()
                    .template(
                        aws_sdk_sesv2::types::Template::builder()
                            .template_name(template_name)
                            .template_data(template_data)
                            .build(),
                    )
                    .build(),
            )
            .send()
            .await
            .map_err(|_| AppError::Unavailable)?;

        Ok(response.message_id().unwrap_or("").to_string())
    }
}
