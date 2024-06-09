mod error;
mod old_types;
use std::net::SocketAddr;

use crate::{auth::JwtInterceptor, RedisPool};
use candid::Principal;
use error::{ApiError, Result};
use futures::{stream::FuturesUnordered, StreamExt};
use ntex::web::{
    self,
    types::{Json, Path, State},
};
use ntex_cors::Cors;
use old_types::{
    ApiResult, BulkUsers, DeleteMetadataBulkRes, GetUserMetadataRes, SetUserMetadataReq,
    SetUserMetadataRes, UserMetadata,
};
use redis::{AsyncCommands, RedisError};

#[derive(Clone)]
pub struct AppState {
    pub redis: RedisPool,
    pub jwt: JwtInterceptor,
}

const METADATA_FIELD: &str = "metadata";

#[web::post("/metadata/{user_principal}")]
async fn set_user_metadata(
    state: State<AppState>,
    user_principal: Path<Principal>,
    req: Json<SetUserMetadataReq>,
) -> Result<Json<ApiResult<SetUserMetadataRes>>> {
    let signature = req.0.signature;
    let metadata = req.0.metadata;
    signature.verify_identity(*user_principal.as_ref(), metadata.clone().into())?;

    let user = user_principal.to_text();
    let mut conn = state.redis.get().await?;
    let meta_raw = serde_json::to_vec(&metadata).expect("failed to serialize user metadata?!");
    let _replaced: bool = conn.hset(user, METADATA_FIELD, &meta_raw).await?;

    Ok(Json(Ok(())))
}

#[web::get("/metadata/{user_principal}")]
async fn get_user_metadata(
    state: State<AppState>,
    path: Path<Principal>,
) -> Result<Json<ApiResult<GetUserMetadataRes>>> {
    let user = path.to_text();

    let mut conn = state.redis.get().await?;
    let meta_raw: Option<Box<[u8]>> = conn.hget(&user, METADATA_FIELD).await?;
    let Some(meta_raw) = meta_raw else {
        return Ok(Json(Ok(None)));
    };
    let meta: UserMetadata = serde_json::from_slice(&meta_raw).map_err(crate::Error::Deser)?;

    Ok(Json(Ok(Some(meta))))
}

#[web::delete("/metadata/bulk")]
async fn delete_metadata_bulk(
    state: State<AppState>,
    req: Json<BulkUsers>,
    http_req: web::HttpRequest,
) -> Result<Json<ApiResult<DeleteMetadataBulkRes>>> {
    // verify token
    let token = http_req
        .headers()
        .get("Authorization")
        .ok_or(crate::Error::AuthTokenMissing)?
        .to_str()
        .map_err(|_| crate::Error::AuthTokenInvalid)?;
    let token = token.trim_start_matches("Bearer ");
    state.jwt.verify_token(token)?;

    let keys = &req.users;

    let conn = state.redis.get().await?;

    let futures: FuturesUnordered<_> = keys
        .iter()
        .map(|key| async {
            conn.clone()
                .hdel::<_, _, bool>(key.to_text(), METADATA_FIELD)
                .await
                .map_err(|e| (key.to_text(), e))
        })
        .collect();
    let results = futures
        .collect::<Vec<Result<_, (String, RedisError)>>>()
        .await;
    let errors = results
        .into_iter()
        .filter_map(|res| match res {
            Ok(_) => None,
            Err((key, e)) => Some((key, e)),
        })
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        log::error!("failed to delete keys: {:?}", errors);
        return Ok(Json(Err(ApiError::DeleteKeys)));
    }

    Ok(Json(Ok(())))
}

pub async fn start_legacy_server(
    bind_address: SocketAddr,
    redis: RedisPool,
    jwt: JwtInterceptor,
) -> Result<()> {
    let state = AppState { redis, jwt };

    web::HttpServer::new(move || {
        web::App::new()
            .wrap(Cors::default())
            .state(state.clone())
            .service(set_user_metadata)
            .service(get_user_metadata)
            .service(delete_metadata_bulk)
    })
    .bind(bind_address)?
    .run()
    .await?;

    Ok(())
}
