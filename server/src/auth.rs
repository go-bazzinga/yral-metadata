use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tonic::{service::Interceptor, Request, Status};

use crate::{
    config::AppConfig,
    consts::{OFF_CHAIN_AGENT_SUBJECT, YRAL_METADATA_COMPANY},
    Error,
};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    pub sub: String,
    pub company: String,
    pub exp: u64,
}

#[derive(Clone)]
pub struct JwtDetails {
    pub decoding_key: DecodingKey,
    pub validation: Validation,
}

#[derive(Clone)]
pub struct JwtInterceptor {
    jwt_details: Arc<JwtDetails>,
}

impl JwtInterceptor {
    pub fn new(jwt_details: JwtDetails) -> Self {
        Self {
            jwt_details: Arc::new(jwt_details),
        }
    }

    pub(crate) fn verify_token(&self, token: &str) -> Result<(), Error> {
        let JwtDetails {
            decoding_key,
            validation,
        } = self.jwt_details.as_ref();

        let token_message =
            decode::<Claims>(token, decoding_key, validation).map_err(Error::Jwt)?;

        let claims = token_message.claims;
        if claims.sub != OFF_CHAIN_AGENT_SUBJECT || claims.company != YRAL_METADATA_COMPANY {
            return Err(Error::AuthTokenInvalid);
        }

        Ok(())
    }

    fn check_auth(&self, req: Request<()>) -> Result<Request<()>, Error> {
        let auth = req
            .metadata()
            .get("authorization")
            .ok_or(Error::AuthTokenMissing)?;
        let token = auth.to_str().map_err(|_| Error::AuthTokenInvalid)?;
        let token = token.trim_start_matches("Bearer ");
        self.verify_token(token)?;

        Ok(req)
    }
}

impl Interceptor for JwtInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let req = self.check_auth(request)?;
        Ok(req)
    }
}

pub fn init_jwt(conf: &AppConfig) -> JwtDetails {
    let decoding_key = DecodingKey::from_ed_pem(conf.jwt_public_key.as_bytes())
        .expect("failed to create decoding key");

    let validation = Validation::new(Algorithm::EdDSA);

    JwtDetails {
        decoding_key,
        validation,
    }
}
