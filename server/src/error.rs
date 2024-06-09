use candid::types::principal::PrincipalError;
use redis::RedisError;
use thiserror::Error;
use tonic::Status;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("failed to load config {0}")]
    Config(#[from] config::ConfigError),
    #[error("invalid identity: {0}")]
    Identity(#[from] yral_identity::Error),
    #[error("{0}")]
    Redis(#[from] RedisError),
    #[error("{0}")]
    Bb8(#[from] bb8::RunError<RedisError>),
    #[error("failed to deserialize json {0}")]
    Deser(#[from] serde_json::Error),
    #[error("jwt {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("auth token missing")]
    AuthTokenMissing,
    #[error("auth token invalid")]
    AuthTokenInvalid,
    #[error("metadata missing")]
    MetadataMissing,
    #[error("invalid principal {0}")]
    Principal(#[from] PrincipalError),
    #[error("internal error: redis")]
    DeleteKeys,
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        use Error::*;
        match &value {
            IO(e) => {
                log::warn!("io error: {e}");
                Status::internal("internal error: IO")
            }
            Config(e) => {
                log::warn!("config error: {e}");
                Status::internal("internal error")
            }
            Identity(_) => Status::invalid_argument(value.to_string()),
            Redis(e) => {
                log::warn!("redis error: {e}");
                Status::internal("internal error: redis")
            }
            Bb8(e) => {
                log::warn!("redis pool error: {e}");
                Status::internal("internal error: redis")
            }
            Deser(_) => Status::invalid_argument(value.to_string()),
            Jwt(_) => Status::invalid_argument(value.to_string()),
            AuthTokenMissing => Status::unauthenticated(value.to_string()),
            AuthTokenInvalid => Status::unauthenticated(value.to_string()),
            MetadataMissing => Status::invalid_argument(value.to_string()),
            Principal(_) => Status::invalid_argument(value.to_string()),
            DeleteKeys => Status::internal(value.to_string()),
        }
    }
}
