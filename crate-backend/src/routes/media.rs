use std::{cmp::Ordering, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing, Json,
};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, trace};
use types::MediaSize;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::{Error, Result},
    services::media::MAX_SIZE,
    types::{Media, MediaCreate, MediaCreated, MediaId},
    ServerState,
};

use super::util::Auth;

/// Media create
///
/// Create a new url to upload media to. Use the media upload endpoint for actually uploading media. Media not referenced/used in other api calls will be removed after a period of time.
#[utoipa::path(
    post,
    path = "/media",
    tags = ["media"],
    responses(
        (status = StatusCode::CREATED, description = "Create media success", body = MediaCreated)
    )
)]
async fn media_create(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(r): Json<MediaCreate>,
) -> Result<impl IntoResponse> {
    if r.size > MAX_SIZE {
        return Err(Error::TooBig);
    }

    let media_id = MediaId(uuid::Uuid::now_v7());
    let srv = s.services();
    srv.media
        .create_upload(media_id, user_id, r.clone())
        .await?;
    if let Some(source_url) = r.source_url {
        let res = MediaCreated {
            media_id,
            upload_url: None,
        };
        let req = reqwest::get(&source_url).await?.error_for_status()?;
        // if req.content_length().is_some_and(|s| s > MAX_SIZE) {
        if req.content_length().is_some_and(|s| s > r.size) {
            return Err(Error::TooBig);
        }

        let mut up = srv
            .media
            .uploads
            .get_mut(&media_id)
            .ok_or(Error::NotFound)?;

        debug!(
            "download media {} from {}, file {:?}",
            media_id,
            source_url,
            up.temp_file.file_path()
        );

        let mut bytes = req.bytes_stream();
        // let stat = up.temp_file.metadata().await?;
        // let current_size = stat.len();
        // let current_off: u64 = 0;
        // if current_size != current_off {
        //     return Err(Error::CantOverwrite);
        // }
        // up.seek(current_off).await?;

        while let Some(chunk) = bytes.next().await {
            match up.write(&chunk?).await {
                Err(err) => {
                    srv.media.uploads.remove(&media_id);
                    return Err(err);
                }
                Ok(_) => {}
            };
        }

        info!("finished stream download end_size={}", up.current_size);

        match up.current_size.cmp(&up.create.size) {
            Ordering::Greater => {
                s.services().media.uploads.remove(&media_id);
                Err(Error::TooBig)
            }
            Ordering::Equal => {
                trace!("flush media");
                up.temp_writer.flush().await?;
                trace!("flushed media");
                drop(up);
                trace!("dropped upload");
                let (_, up) = s
                    .services()
                    .media
                    .uploads
                    .remove(&media_id)
                    .expect("it was there a few milliseconds ago");
                trace!("processing upload");
                let mut media = s
                    .services()
                    .media
                    .process_upload(up, media_id, user_id)
                    .await?;
                debug!("finished processing media");
                s.presign(&mut media).await?;
                let mut headers = HeaderMap::new();
                let size = match media.source.size {
                    MediaSize::Bytes(b) => b,
                    MediaSize::BytesPerSecond(_) => panic!("BytesPerSecond invalid for upload?"),
                };
                headers.insert("content-length", size.into());
                Ok((StatusCode::CREATED, HeaderMap::new(), Json(res)))
            }
            Ordering::Less => Err(Error::BadStatic("failed to download content")),
        }
    } else {
        let upload_url = Some(
            s.config()
                .base_url
                .join(&format!("/api/v1/internal/media-upload/{media_id}"))?,
        );
        let res = MediaCreated {
            media_id,
            upload_url,
        };
        let mut res_headers = HeaderMap::new();
        res_headers.insert("upload-length", r.size.into());
        res_headers.insert("upload-offset", 0.into());
        Ok((StatusCode::CREATED, res_headers, Json(res)))
    }
}

// /// Media patch
// #[utoipa::path(
//     patch,
//     path = "/media/{media_id}",
//     tags = ["media"],
//     params(("media_id", description = "Media id")),
//     responses(
//         (status = NOT_MODIFIED, description = "Not modified"),
//         (status = OK, body = Media, description = "Success"),
//     )
// )]
// async fn media_patch(
//     Path(media_id): Path<MediaId>,
//     Auth(user_id): Auth,
//     State(s): State<Arc<ServerState>>,
// ) -> Result<impl IntoResponse> {
// }

/// Media upload
async fn media_upload(
    Path(media_id): Path<MediaId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    body: Body,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let mut up = srv
        .media
        .uploads
        .get_mut(&media_id)
        .ok_or(Error::NotFound)?;
    if up.user_id != user_id {
        return Err(Error::NotFound);
    }
    debug!(
        "continue upload for {}, file {:?}",
        media_id,
        up.temp_file.file_path()
    );
    let stat = up.temp_file.metadata().await?;
    let current_size = stat.len();
    let current_off: u64 = headers
        .get("upload-offset")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;
    let part_length: u64 = headers
        .get("content-length")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;
    if current_size != current_off {
        return Err(Error::CantOverwrite);
    }
    if current_size + part_length > up.create.size {
        return Err(Error::TooBig);
    }
    up.seek(current_off).await?;
    let mut stream = body.into_data_stream();

    while let Some(chunk) = stream.next().await {
        match up.write(&chunk?).await {
            Err(err) => {
                srv.media.uploads.remove(&media_id);
                return Err(err);
            }
            Ok(_) => {}
        };
    }

    info!("finished stream upload end_size={}", up.current_size);

    match up.current_size.cmp(&up.create.size) {
        Ordering::Greater => {
            s.services().media.uploads.remove(&media_id);
            Err(Error::TooBig)
        }
        Ordering::Equal => {
            trace!("flush media");
            up.temp_writer.flush().await?;
            trace!("flushed media");
            drop(up);
            trace!("dropped upload");
            let (_, up) = s
                .services()
                .media
                .uploads
                .remove(&media_id)
                .expect("it was there a few milliseconds ago");
            trace!("processing upload");
            let mut media = s
                .services()
                .media
                .process_upload(up, media_id, user_id)
                .await?;
            debug!("finished processing media");
            s.presign(&mut media).await?;
            let mut headers = HeaderMap::new();
            let size = match media.source.size {
                MediaSize::Bytes(b) => b,
                MediaSize::BytesPerSecond(_) => panic!("BytesPerSecond invalid for upload?"),
            };
            headers.insert("upload-offset", size.into());
            headers.insert("upload-length", size.into());
            Ok((StatusCode::OK, headers, Json(Some(media))))
        }
        Ordering::Less => {
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", up.current_size.into());
            headers.insert("upload-length", up.create.size.into());
            Ok((StatusCode::NO_CONTENT, headers, Json(None)))
        }
    }
}

/// Media get
// TODO: restrict media visibility? or make it always public?
#[utoipa::path(
    get,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    responses(
        (status = OK, body = Media, description = "Success"),
    )
)]
async fn media_get(
    Path((media_id,)): Path<(MediaId,)>,
    Auth(_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let mut media = s.data().media_select(media_id).await?;
    s.presign(&mut media).await?;
    Ok(Json(media))
}

/// Media check
///
/// Get headers useful for resuming an upload
#[utoipa::path(
    head,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    responses(
        (status = NO_CONTENT, description = "no content", headers(("upload-length" = u64), ("upload-offset" = u64))),
    )
)]
async fn media_check(
    Path(media_id): Path<MediaId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    if let Some(up) = s.services().media.uploads.get_mut(&media_id) {
        if up.user_id == user_id {
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", up.temp_file.metadata().await?.len().into());
            headers.insert("upload-length", up.create.size.into());
            return Ok((StatusCode::NO_CONTENT, headers));
        }
    }
    let media = s.data().media_select(media_id).await?;
    let mut headers = HeaderMap::new();
    let size = match media.source.size {
        MediaSize::Bytes(b) => b,
        MediaSize::BytesPerSecond(_) => panic!("BytesPerSecond invalid for upload?"),
    };
    headers.insert("upload-offset", size.into());
    headers.insert("upload-length", size.into());
    Ok((StatusCode::NO_CONTENT, headers))
}

/// Media delete
///
/// Delete unlinked media. If its linked to a message, delete that message instead.
#[utoipa::path(
    delete,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    responses(
        (status = NO_CONTENT, description = "no content"),
        (status = CONFLICT, description = "media is linked to another resource (ie. a message)"),
    )
)]
async fn media_delete(
    Path(media_id): Path<MediaId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    if let Some(up) = s.services().media.uploads.get_mut(&media_id) {
        if up.user_id == user_id {
            s.services().media.uploads.remove(&media_id);
        }
        Ok(StatusCode::NO_CONTENT)
    } else {
        let links = s.data().media_link_select(media_id).await?;
        if links.is_empty() {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Ok(StatusCode::CONFLICT)
        }
    }
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(media_create))
        // .routes(routes!(media_patch))
        .routes(routes!(media_get))
        .routes(routes!(media_check))
        .routes(routes!(media_delete))
        .route(
            "/internal/media-upload/{media_id}",
            routing::patch(media_upload),
        )
}
