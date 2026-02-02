use super::error::AppError;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use time::format_description::well_known::Iso8601;

#[allow(dead_code)]
pub trait ToOffsetDateTime {
    fn _to_offset_date_time_from_epoch(&self) -> Result<OffsetDateTime, AppError>;
    fn _to_offset_date_time_from_iso(&self) -> Result<OffsetDateTime, AppError>;
}

impl ToOffsetDateTime for String {
    fn _to_offset_date_time_from_epoch(&self) -> Result<OffsetDateTime, AppError> {
        let timestamp = i64::from_str(self).map_err(|_| AppError::UnprocessableEntity {
            error_values: None,
            message: Some("Invalid timestamp".into()),
            description: None,
        })?;

        OffsetDateTime::from_unix_timestamp(timestamp).map_err(|_| AppError::UnprocessableEntity {
            error_values: None,
            message: Some("Invalid timestamp".into()),
            description: None,
        })
    }

    fn _to_offset_date_time_from_iso(&self) -> Result<OffsetDateTime, AppError> {
        OffsetDateTime::parse(self, &Iso8601::DEFAULT).map_err(|_| AppError::UnprocessableEntity {
            error_values: None,
            message: Some("Invalid timestamp".into()),
            description: None,
        })
    }
}

pub trait FromOffsetDateTime {
    fn _to_unix_string(&self) -> Result<String, AppError>;
    fn to_iso_string(&self) -> Result<String, AppError>;
}

impl FromOffsetDateTime for OffsetDateTime {
    fn _to_unix_string(&self) -> Result<String, AppError> {
        Ok(self.unix_timestamp().to_string())
    }

    fn to_iso_string(&self) -> Result<String, AppError> {
        self.format(&Iso8601::DEFAULT)
            .map(|s| s.to_string())
            .map_err(|_| AppError::UnprocessableEntity {
                error_values: None,
                message: Some("Invalid timestamp".into()),
                description: None,
            })
    }
}

pub fn validate_timestamp(timestamp: i64, is_in_past: bool) -> Result<bool, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            AppError::internal_error(
                e.to_string(),
                Some("Failed to get current time".to_string()),
            )
        })?
        .as_secs() as i64;

    if is_in_past {
        Ok(timestamp < now)
    } else {
        Ok(timestamp > now)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaginationParams {
    pub page_number: Option<i32>,
    pub page_size: Option<i32>,
}
