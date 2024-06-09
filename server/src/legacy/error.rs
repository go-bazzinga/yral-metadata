use ntex::{
    http::{header, StatusCode},
    web,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::old_types::ApiResult;

#[derive(Deserialize, Serialize, Error, Debug)]
#[non_exhaustive]
pub enum ApiError {
    #[error("invalid signature provided")]
    InvalidSignature,
    #[error("internal error: redis")]
    Redis,
    #[error("internal error: deser")]
    Deser,
    #[error("jwt error - invalid token")]
    Jwt,
    #[error("invalid authentication token")]
    AuthToken,
    #[error("missing authentication token")]
    AuthTokenMissing,
    #[error("failed to delete keys (redis)")]
    DeleteKeys,
    #[error("unknown: {0}")]
    Unknown(String),
}

impl From<&crate::Error> for ApiResult<()> {
    fn from(value: &crate::Error) -> Self {
        use crate::Error;

        let err = match value {
            Error::IO(_) | Error::Config(_) => {
                log::warn!("internal error {value}");
                ApiError::Unknown("internal error, reported".into())
            }
            Error::Identity(_) => ApiError::InvalidSignature,
            Error::Redis(e) => {
                log::warn!("redis error {e}");
                ApiError::Redis
            }
            Error::Bb8(e) => {
                log::warn!("bb8 error {e}");
                ApiError::Redis
            }
            Error::Deser(e) => {
                log::warn!("deserialization error {e}");
                ApiError::Deser
            }
            Error::Jwt(_) => ApiError::Jwt,
            Error::AuthTokenMissing => ApiError::AuthTokenMissing,
            Error::AuthTokenInvalid => ApiError::AuthToken,
            Error::MetadataMissing | Error::Principal(_) | Error::DeleteKeys => {
                log::error!("unexpected error in legacy code {value}");
                ApiError::Unknown("internal".into())
            }
        };
        ApiResult::Err(err)
    }
}

impl web::error::WebResponseError for crate::Error {
    fn error_response(&self, _: &web::HttpRequest) -> web::HttpResponse {
        let api_error = ApiResult::from(self);
        web::HttpResponse::build(self.status_code())
            .set_header(header::CONTENT_TYPE, "application/json")
            .json(&api_error)
    }

    fn status_code(&self) -> StatusCode {
        use crate::Error;

        match self {
            Error::IO(_) | Error::Config(_) | Error::Redis(_) | Error::Deser(_) | Error::Bb8(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::Identity(_)
            | Error::Jwt(_)
            | Error::AuthTokenInvalid
            | Error::AuthTokenMissing => StatusCode::UNAUTHORIZED,
            Error::MetadataMissing | Error::Principal(_) | Error::DeleteKeys => {
                StatusCode::NOT_IMPLEMENTED
            }
        }
    }
}

pub type Result<T, E = crate::Error> = std::result::Result<T, E>;
