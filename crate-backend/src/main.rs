use std::{str::FromStr, sync::Arc, time::Duration};

use axum::{extract::DefaultBodyLimit, response::Html, routing::get, Json};
use dashmap::{DashMap, DashSet};
use data::{postgres::Postgres, Data};
use figment::providers::{Env, Format, Toml};
use serde::Deserialize;
use services::Services;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::broadcast::Sender;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::debug;
use tracing_subscriber::EnvFilter;
use types::{MediaId, MediaUpload, MessageServer};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable as _};

pub mod data;
pub mod error;
mod routes;
pub mod types;
pub mod services;

use error::Result;
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(components(schemas(
    types::Room,
    types::RoomPatch,
    types::User,
    types::Thread,
    types::ThreadPatch,
    types::Message,
    types::RoomMember,
    types::Role,
)))]
struct ApiDoc;

pub struct ServerState {
    // should this be global?
    pub config: Config,
    
    // TODO: move some of these into the db? or use something like redis? 
    pub uploads: Arc<DashMap<MediaId, MediaUpload>>,
    pub valid_oauth2_states: Arc<DashSet<Uuid>>,
    
    // this is fine probably
    pub sushi: Sender<MessageServer>,
    // channel_user: Arc<DashMap<UserId, (Sender<MessageServer>, Receiver<MessageServer>)>>,
    pub pool: PgPool,
    pub blobs: opendal::Operator,
}

impl ServerState {
    fn new(config: Config, pool: PgPool, blobs: opendal::Operator) -> Self {
        Self {
            config,
            uploads: Arc::new(DashMap::new()),
            valid_oauth2_states: Arc::new(DashSet::new()),
            pool,
            sushi: tokio::sync::broadcast::channel(100).0,
            // channel_user: Arc::new(DashMap::new()),
            blobs,
        }
    }

    fn data(&self) -> Box<dyn Data> {
        Box::new(Postgres {
            pool: self.pool.clone(),
        })
    }

    fn services(self: &Arc<Self>) -> Services {
        Services::new(self.clone(), self.data())
    }

    fn blobs(&self) -> &opendal::Operator {
        &self.blobs
    }

    async fn presign(&self, url: &str) -> Result<String> {
        // Ok(self
        //     .blobs
        //     .presign_read(&media_id.to_string(), Duration::from_secs(60 * 60 * 24))
        //     .await?
        //     .uri()
        //     .to_string())
        // HACK: temporary thing for better caching
        // TODO: i should use serviceworkers to cache while ignoring signature params
        Ok(format!("https://chat-files.celery.eu.org/{url}"))
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    rust_log: String,
    database_url: String,
    s3: ConfigS3,
    discord: ConfigDiscord,
}

#[derive(Debug, Deserialize)]
pub struct ConfigS3 {
    bucket: String,
    endpoint: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigDiscord {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let config: Config = figment::Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    debug!("Starting with config: {:#?}", config);
    
    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_str(&config.rust_log)?)
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let blobs_builder = opendal::services::S3::default()
        .bucket(&config.s3.bucket)
        .endpoint(&config.s3.endpoint)
        .region(&config.s3.region)
        .access_key_id(&config.s3.access_key_id)
        .secret_access_key(&config.s3.secret_access_key);
    let blobs = opendal::Operator::new(blobs_builder).unwrap().finish();

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routes::routes())
        .with_state(Arc::new(ServerState::new(config, pool, blobs)))
        .split_for_parts();
    let api1 = api.clone();
    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(Scalar::with_url("/scalar", api1).to_html()) }),
        )
        .layer(CorsLayer::very_permissive())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await?;
    axum::serve(listener, router).await?;
    Ok(())
}
