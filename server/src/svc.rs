use candid::Principal;
use futures::{prelude::*, stream::FuturesUnordered};
use redis::AsyncCommands;
use tonic::{Request, Response, Status};
use types::{
    private_yral_metadata_server::PrivateYralMetadata,
    public_yral_metadata_server::PublicYralMetadata, BulkDeleteReq, GetUserMetadataReq,
    SetUserMetadataReq, UserMetadata, UserMetadataJson,
};

use crate::Error;

const METADATA_FIELD: &str = "metadata";

pub type RedisPool = bb8::Pool<bb8_redis::RedisConnectionManager>;

#[derive(Debug, Clone)]
pub struct YralMetadataServer {
    pub redis: RedisPool,
}

impl YralMetadataServer {
    pub fn new(redis: RedisPool) -> Self {
        Self { redis }
    }

    async fn set_user_metadata_inner(&self, req: SetUserMetadataReq) -> Result<(), Error> {
        let signature = req.signature()?;
        let user_principal = req.user_principal()?;
        let metadata = req.user_metadata.ok_or(Error::MetadataMissing)?;
        let msg = metadata.clone().try_into()?;
        signature.verify_identity(user_principal, msg)?;

        let user = user_principal.to_text();
        let mut conn = self.redis.get().await?;
        let meta_json = UserMetadataJson::try_from(metadata)?;
        let meta_raw = serde_json::to_vec(&meta_json)?;
        let _replaced: bool = conn.hset(user, METADATA_FIELD, &meta_raw).await?;
        Ok(())
    }

    async fn get_user_metadata_inner(
        &self,
        req: GetUserMetadataReq,
    ) -> Result<Option<UserMetadata>, Error> {
        let user = Principal::try_from_slice(&req.user_principal_id)?.to_text();
        let mut conn = self.redis.get().await?;
        let meta_raw: Option<Box<[u8]>> = conn.hget(&user, METADATA_FIELD).await?;
        let Some(meta_raw) = meta_raw else {
            return Ok(None);
        };
        let meta: UserMetadataJson = serde_json::from_slice(&meta_raw)?;
        Ok(Some(UserMetadata {
            user_canister_id: meta.user_canister_id.as_slice().to_vec(),
            user_name: meta.user_name,
        }))
    }

    async fn delete_bulk_inner(&self, req: BulkDeleteReq) -> Result<(), Error> {
        let conn = self.redis.get().await?;
        let keys: Vec<_> = req
            .users
            .into_iter()
            .map(|key| {
                let principal = Principal::try_from_slice(&key)?;
                Ok::<_, Error>(principal.to_text())
            })
            .collect::<Result<_, _>>()?;

        let deletes: FuturesUnordered<_> = keys
            .into_iter()
            .map(|key| async {
                conn.clone()
                    .hdel::<_, _, bool>(key.clone(), METADATA_FIELD)
                    .await
                    .map_err(|e| (key, Error::from(e)))
            })
            .collect();

        let results = deletes.collect::<Vec<Result<_, _>>>().await;
        let errors = results
            .into_iter()
            .filter_map(|res| res.err())
            .collect::<Vec<_>>();

        if !errors.is_empty() {
            log::error!("failed to delete keys: {:?}", errors);
            return Err(Error::DeleteKeys);
        }

        Ok(())
    }
}

#[tonic::async_trait]
impl PublicYralMetadata for YralMetadataServer {
    async fn set_user_metadata(
        &self,
        req: Request<SetUserMetadataReq>,
    ) -> Result<Response<()>, Status> {
        let req = req.into_inner();
        self.set_user_metadata_inner(req).await?;

        Ok(Response::new(()))
    }

    async fn get_user_metadata(
        &self,
        req: Request<GetUserMetadataReq>,
    ) -> Result<Response<UserMetadata>, Status> {
        let req = req.into_inner();
        let Some(meta) = self.get_user_metadata_inner(req).await? else {
            return Err(Status::not_found("metadata not found"));
        };
        Ok(Response::new(meta))
    }
}

#[tonic::async_trait]
impl PrivateYralMetadata for YralMetadataServer {
    async fn bulk_delete(&self, req: Request<BulkDeleteReq>) -> Result<Response<()>, Status> {
        let req = req.into_inner();
        self.delete_bulk_inner(req).await?;
        Ok(Response::new(()))
    }
}
