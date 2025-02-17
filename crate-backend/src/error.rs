use std::num::{ParseFloatError, ParseIntError};

use axum::{extract::ws::Message, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use tracing::error;
use types::MessageEnvelope;

use crate::types::MessageSync;

#[derive(thiserror::Error, Debug)]
// TODO: avoid returning actual error messages to prevent leaking stuff
pub enum Error {
    #[error("missing authentication")]
    MissingAuth,
    #[error("bad header")]
    BadHeader,
    #[error("session not yet authenticated")]
    UnauthSession,
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    MissingPermissions,
    #[error("bad request: {0}")]
    BadStatic(&'static str),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("too big :(")]
    TooBig,
    #[error("internal error: {0}")]
    Internal(String),
    #[error("internal error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("can't overwrite already uploaded data!")]
    CantOverwrite,
    #[error("internal error: {0}")]
    Tempfile(#[from] async_tempfile::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("axum error")]
    Axum(#[from] axum::Error),
    #[error("sushi send error: {0}")]
    SushiSend(#[from] tokio::sync::broadcast::error::SendError<MessageSync>),
    #[error("parse int error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("parse float error: {0}")]
    ParseFloat(#[from] ParseFloatError),
    #[error("opendal error: {0}")]
    Opendal(#[from] opendal::Error),
    #[error("migrate error: {0}")]
    SqlxMigrate(#[from] sqlx::migrate::MigrateError),
    #[error("tracing subscriber error: {0}")]
    TracingSubscriber(#[from] tracing::subscriber::SetGlobalDefaultError),
    #[error("log format parse error: {0}")]
    LogFormatParse(#[from] tracing_subscriber::filter::ParseError),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("figment error: {0}")]
    Figment(#[from] figment::Error),
    #[error("url parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("unmodified")]
    // HACK: not really an error, but still kind of helpful to have here
    NotModified,

    #[error("not yet implemented...")]
    // NOTE: also not really an error, but put here for convenience
    Ffprobe,

    #[error("not yet implemented...")]
    Unimplemented,

    #[error("image error: {0}")]
    ImageError(#[from] image::ImageError),
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::RowNotFound => Error::NotFound,
            err => Error::Internal(err.to_string()),
        }
    }
}

impl From<axum::http::header::ToStrError> for Error {
    fn from(_value: axum::http::header::ToStrError) -> Self {
        Error::BadHeader
    }
}

impl Error {
    fn get_status(&self) -> StatusCode {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::BadHeader => StatusCode::BAD_REQUEST,
            Error::BadStatic(_) => StatusCode::BAD_REQUEST,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Serde(_) => StatusCode::BAD_REQUEST,
            Error::MissingAuth => StatusCode::UNAUTHORIZED,
            Error::UnauthSession => StatusCode::UNAUTHORIZED,
            Error::TooBig => StatusCode::PAYLOAD_TOO_LARGE,
            Error::MissingPermissions => StatusCode::FORBIDDEN,
            Error::CantOverwrite => StatusCode::CONFLICT,
            Error::ParseInt(_) => StatusCode::BAD_REQUEST,
            Error::ParseFloat(_) => StatusCode::BAD_REQUEST,
            Error::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            Error::NotModified => StatusCode::NOT_MODIFIED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        if let Error::NotModified = self {
            return self.get_status().into_response();
        };
        error!(
            "Response error: status {}, message {:?}",
            self.get_status(),
            self
        );
        (
            self.get_status(),
            Json(json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}

impl From<Error> for Message {
    fn from(val: Error) -> Self {
        Message::text(
            serde_json::to_string(&MessageEnvelope {
                payload: types::MessagePayload::Error {
                    error: val.to_string(),
                },
            })
            .expect("error should always be able to be serialized"),
        )
    }
}

pub type Result<T> = std::result::Result<T, Error>;
