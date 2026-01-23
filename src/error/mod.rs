use axum::http::header::WWW_AUTHENTICATE;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::collections::HashMap;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Authentication Required")]
    Unauthorized,
    #[error("Unprocessable Entity")]
    UnprocessableEntity {
        error_values: Option<HashMap<String, String>>,
        message: Option<String>,
        description: Option<String>,
    },
    #[error("Access Denied")]
    _Forbidden,
    #[error("DB Error")]
    DBError {
        error_values: Option<HashMap<String, String>>,
        message: Option<String>,
        description: Option<String>,
    },
    #[error("Internal Error")]
    InternalError {
        error: String,
        message: Option<String>,
    },
    #[error("Service Unavailable")]
    #[allow(dead_code)]
    Unavailable,
    #[error("Bad Request")]
    BadRequest {
        error_values: Option<HashMap<String, String>>,
        message: Option<String>,
        description: Option<String>,
    },
}

impl AppError {
    // Mapper for ENUM to HTTP Status Code
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::_Forbidden => StatusCode::FORBIDDEN,
            Self::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::DBError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
        }
    }

    // Helper for generating bad keys in the request
    pub fn _unprocessable_entity(
        errors: Option<HashMap<String, String>>,
        message: Option<String>,
        description: Option<String>,
    ) -> Self {
        Self::UnprocessableEntity {
            error_values: errors,
            message,
            description,
        }
    }

    pub fn _db_error(
        errors: Option<HashMap<String, String>>,
        message: Option<String>,
        description: Option<String>,
    ) -> Self {
        Self::DBError {
            error_values: errors,
            message,
            description,
        }
    }

    pub fn _internal_error(error: String, message: Option<String>) -> Self {
        Self::InternalError { error, message }
    }

    pub fn bad_request(
        errors: Option<HashMap<String, String>>,
        message: Option<String>,
        description: Option<String>,
    ) -> Self {
        Self::BadRequest {
            error_values: errors,
            message,
            description,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::UnprocessableEntity {
                error_values,
                message,
                description,
            } => {
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(ErrorDetails {
                        error_values,
                        message,
                        description,
                    }),
                )
                    .into_response();
            }
            Self::Unauthorized => {
                return (
                    self.status_code(),
                    [(WWW_AUTHENTICATE, "Token")],
                    self.to_string(),
                )
                    .into_response();
            }
            Self::DBError {
                error_values,
                message,
                description,
            } => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorDetails {
                        error_values,
                        message,
                        description,
                    }),
                )
                    .into_response();
            }
            Self::InternalError { error, message } => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(InternalErrorDetails { error, message }),
                )
                    .into_response();
            }
            Self::Unavailable => {
                return (StatusCode::SERVICE_UNAVAILABLE, self.to_string()).into_response();
            }
            Self::BadRequest {
                error_values,
                message,
                description,
            } => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorDetails {
                        error_values,
                        message,
                        description,
                    }),
                )
                    .into_response();
            }
            _ => (),
        }

        (self.status_code(), self.to_string()).into_response()
    }
}

#[derive(serde::Serialize)]
struct ErrorDetails {
    error_values: Option<HashMap<String, String>>,
    message: Option<String>,
    description: Option<String>,
}

#[derive(serde::Serialize)]
struct InternalErrorDetails {
    error: String,
    message: Option<String>,
}
