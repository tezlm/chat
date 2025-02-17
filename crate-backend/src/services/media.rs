use std::{
    io::{Cursor, SeekFrom},
    process::Stdio,
    sync::Arc,
};

use async_tempfile::TempFile;
use dashmap::DashMap;
use ffprobe::{MediaType, Metadata};
use futures_util::{stream::FuturesUnordered, StreamExt};
use image::{codecs::avif::AvifEncoder, DynamicImage};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter},
    process::Command,
};
use tracing::{debug, error, info, span, trace, Level};
use types::{
    Media, MediaCreate, MediaId, MediaSize, MediaTrack, MediaTrackInfo, TrackSource, UserId,
};

use crate::{
    error::{Error, Result},
    ServerStateInner,
};

mod ffprobe;

const MEGABYTE: usize = 1024 * 1024;
pub const MAX_SIZE: u64 = 1024 * 1024 * 16;

pub struct ServiceMedia {
    pub state: Arc<ServerStateInner>,
    pub uploads: DashMap<MediaId, MediaUpload>,
}

pub struct MediaUpload {
    pub create: MediaCreate,
    pub user_id: UserId,
    pub temp_file: TempFile,
    pub temp_writer: BufWriter<TempFile>,
    pub current_size: u64,
}

impl MediaUpload {
    pub async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let len = bytes.len() as u64;
        if self.current_size + len > MAX_SIZE {
            return Err(Error::TooBig);
        }

        self.temp_writer.write_all(bytes).await?;
        self.current_size += len;
        Ok(())
    }

    pub async fn seek(&mut self, off: u64) -> Result<()> {
        self.temp_writer.seek(SeekFrom::Start(off)).await?;
        Ok(())
    }
}

impl ServiceMedia {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            uploads: DashMap::new(),
        }
    }

    pub async fn create_upload(
        &self,
        media_id: MediaId,
        user_id: UserId,
        create: MediaCreate,
    ) -> Result<()> {
        let temp_file = TempFile::new().await.expect("failed to create temp file!");
        let temp_writer = BufWriter::new(temp_file.open_rw().await?);
        trace!("create temp_file {:?}", temp_file.file_path());
        self.uploads.insert(
            media_id,
            MediaUpload {
                create,
                user_id,
                temp_file,
                temp_writer,
                current_size: 0,
            },
        );
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_metadata_and_mime(
        &self,
        file: &std::path::Path,
    ) -> Result<(Option<Metadata>, String)> {
        let meta = match ffprobe::extract(file).await {
            Ok(meta) => meta,
            Err(Error::Ffprobe) => {
                let mime = get_mime(file).await?;
                return Ok((None, mime));
            }
            Err(err) => return Err(err),
        };
        let mut mime = get_mime(file).await?;
        // HACK: fix webm
        if meta.is_video() {
            mime = mime.replace("video/webm", "audio/webm");
        }
        Ok((Some(meta), mime))
    }

    #[tracing::instrument(skip(self))]
    pub async fn generate_thumbnails(
        &self,
        media: &mut Media,
        meta: &Metadata,
        path: &std::path::Path,
        force: bool,
    ) -> Result<()> {
        if media
            .all_tracks()
            .any(|i| matches!(i.info, MediaTrackInfo::Thumbnail(_)))
            && !force
        {
            return Ok(());
        }
        let media_id = media.id;
        let mime = get_mime(path).await?;
        let img = if mime.starts_with("image/") {
            let bytes = tokio::fs::read(path).await?;
            let cursor = Cursor::new(bytes);
            image::ImageReader::new(cursor)
                .with_guessed_format()?
                .decode()?
        } else if let Some(thumb) = meta.get_thumb_stream() {
            if thumb.codec_type == MediaType::Attachment {
                let cmd = Command::new("ffmpeg")
                    .args([
                        "-v",
                        "quiet",
                        &format!("-dump_attachment:{}", thumb.index),
                        "/dev/stdout",
                        "-y",
                        "-i",
                    ])
                    .arg(path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .output()
                    .await?;
                let cursor = Cursor::new(cmd.stdout);
                image::ImageReader::new(cursor)
                    .with_guessed_format()?
                    .decode()?
            } else if thumb.codec_type == MediaType::Video {
                let cmd = Command::new("ffmpeg")
                    .args(["-v", "quiet", "-i"])
                    .arg(path)
                    .args([
                        "-vf",
                        "thumbnail,scale=300:300",
                        "-frames:v",
                        "1",
                        "-f",
                        "webp",
                        "-",
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .output()
                    .await?;
                let cursor = Cursor::new(cmd.stdout);
                image::ImageReader::new(cursor)
                    .with_guessed_format()?
                    .decode()?
            } else {
                error!("no suitable thumbnail codec");
                return Ok(());
            }
        } else {
            error!("no suitable thumbnail stream");
            return Ok(());
        };
        let mut fut = FuturesUnordered::new();
        for size in [64, 320, 640] {
            let state = self.state.clone();
            fut.push(generate_and_upload_thumb(state, &img, media_id, size, size));
        }
        while let Some(track) = fut.next().await {
            if let Some(track) = track? {
                media.tracks.push(track);
            }
        }
        Ok(())
    }

    pub async fn process_upload(
        &self,
        up: MediaUpload,
        media_id: MediaId,
        user_id: UserId,
    ) -> Result<Media> {
        let tmp = up.temp_file;
        let p = tmp.file_path().to_owned();
        let url = format!("media/{media_id}");
        let services = self.state.services();
        let (meta, mime) = &services.media.get_metadata_and_mime(&p).await?;
        let mut media = Media {
            alt: up.create.alt.clone(),
            id: media_id,
            filename: up.create.filename.clone(),
            source: MediaTrack {
                url: url.clone(),
                mime: mime.clone(),
                // TODO: use correct MediaTrackInfo type
                info: if mime.starts_with("image/") {
                    types::MediaTrackInfo::Image(types::Image {
                        height: meta
                            .as_ref()
                            .and_then(|m| m.height())
                            .expect("all images have a height"),
                        width: meta
                            .as_ref()
                            .and_then(|m| m.width())
                            .expect("all images have a width"),
                        language: None,
                    })
                } else {
                    types::MediaTrackInfo::Mixed(types::Mixed {
                        height: meta.as_ref().and_then(|m| m.height()),
                        width: meta.as_ref().and_then(|m| m.width()),
                        duration: meta.as_ref().and_then(|m| m.duration().map(|d| d as u64)),
                        language: None,
                    })
                },
                size: MediaSize::Bytes(up.create.size),
                source: TrackSource::Uploaded,
            },
            tracks: vec![],
        };
        debug!("finish upload for {}, mime {}", media_id, mime);
        if let Some(meta) = &meta {
            self.generate_thumbnails(&mut media, meta, &p, false)
                .await?;
        }
        debug!("finish generating thumbnails for {}", media_id);
        let upload_s3 = async {
            let mut f = tokio::fs::OpenOptions::new().read(true).open(&p).await?;
            let mut buf = vec![0u8; MEGABYTE];
            let mut w = self
                .state
                .blobs
                .writer_with(&url)
                .cache_control("public, max-age=604800, immutable, stale-while-revalidate=86400")
                // FIXME: sometimes this fails with "failed to parse header"
                // .content_type(&mime)
                .await?;
            while f.read(&mut buf).await? != 0 {
                w.write(buf.to_vec()).await?;
            }
            w.close().await?;
            info!("uploaded {} bytes to s3", up.create.size);
            Result::Ok(())
        };
        upload_s3.await?;
        drop(tmp);
        self.state
            .data()
            .media_insert(user_id, media.clone())
            .await?;
        Ok(media)
    }
}

async fn generate_and_upload_thumb(
    state: Arc<ServerStateInner>,
    img: &DynamicImage,
    media_id: MediaId,
    width: u32,
    height: u32,
) -> Result<Option<MediaTrack>> {
    if img.width() < width && img.height() < height {
        return Ok(None);
    }
    let mut out = Cursor::new(Vec::new());
    // currently using the default
    let span_gen_thumb = span!(Level::INFO, "generate thumb");
    let _s = span_gen_thumb.enter();
    let enc = AvifEncoder::new_with_speed_quality(&mut out, 4, 80);
    img.thumbnail(width, height).write_with_encoder(enc)?;
    drop(_s);
    let url = format!("thumb/{media_id}/{width}x{height}");
    let len = out.get_ref().len();
    let span_upload_thumb = span!(Level::INFO, "upload thumb");
    let _s = span_upload_thumb.enter();
    let mut w = state
        .blobs
        .writer_with(&url)
        .cache_control("public, max-age=604800, immutable, stale-while-revalidate=86400")
        .content_type("image/avif")
        .await?;
    w.write(out.into_inner()).await?;
    w.close().await?;
    drop(_s);
    let track = MediaTrack {
        info: MediaTrackInfo::Thumbnail(types::Image {
            height: height as u64,
            width: width as u64,
            language: None,
        }),
        url,
        size: MediaSize::Bytes(len as u64),
        mime: "image/avif".to_owned(),
        source: TrackSource::Generated,
    };
    Ok(Some(track))
}

async fn get_mime(file: &std::path::Path) -> Result<String> {
    let out = Command::new("file").arg("-ib").arg(file).output().await?;
    let mime = String::from_utf8(out.stdout).expect("file has failed me");
    Ok(mime)
}
