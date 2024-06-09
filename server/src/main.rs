mod auth;
mod config;
mod consts;
mod error;
mod legacy;
mod svc;

use auth::{init_jwt, JwtInterceptor};
use config::AppConfig;

use error::*;
use svc::*;
use tokio::sync::oneshot;
use tonic::transport::Server;

pub async fn init_redis(conf: &AppConfig) -> RedisPool {
    let manager = bb8_redis::RedisConnectionManager::new(conf.redis_url.clone())
        .expect("failed to open connection to redis");
    RedisPool::builder().build(manager).await.unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conf = AppConfig::load()?;
    env_logger::init();
    let redis = init_redis(&conf).await;
    let jwt_details = init_jwt(&conf);

    let server = YralMetadataServer::new(redis.clone());
    let auth_interceptor = JwtInterceptor::new(jwt_details);

    let pub_svc = types::public_yral_metadata_server::PublicYralMetadataServer::new(server.clone());
    let priv_svc = types::private_yral_metadata_server::PrivateYralMetadataServer::with_interceptor(
        server,
        auth_interceptor.clone(),
    );

    let (legacy_cancel_tx, legacy_cancel_rx) = oneshot::channel();
    let legacy_thread = std::thread::spawn(move || {
        ntex::rt::System::new("legacy_thread").block_on(async move {
            let legacy_fut =
                legacy::start_legacy_server(conf.legacy_bind_address, redis, auth_interceptor);
            tokio::select! {
                legacy_res = legacy_fut => {
                    panic!("legacy server died: {:?}", legacy_res);
                },
                _ = legacy_cancel_rx => {
                    Ok::<_, Error>(())
                }
            }
        })
    });

    let grpc_fut = Server::builder()
        .accept_http1(true)
        .add_service(tonic_web::enable(pub_svc))
        .add_service(tonic_web::enable(priv_svc))
        .serve(conf.bind_address);
    let ctrl_c_fut = tokio::signal::ctrl_c();

    tokio::select! {
        tonic_res = grpc_fut => {
            panic!("tonic server died: {:?}", tonic_res);
        }
        _ = ctrl_c_fut => {
            log::info!("Received Ctrl-C, shutting down");
            _ = legacy_cancel_tx.send(());
        }
    };

    legacy_thread.join().unwrap().expect("legacy server died");

    Ok(())
}
