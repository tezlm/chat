use async_tempfile::TempFile;
use tokio::{io::BufWriter, process::Command};
use types::{MediaCreate, UserId};

use crate::error::Result;

mod ffprobe {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Metadata {
        pub streams: Vec<Stream>,
        pub format: Format,
    }

    #[derive(Debug, Deserialize)]
    pub struct Format {
        pub duration: Option<String>,
        // #[serde(default)]
        // pub tags: HashMap<String, String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Stream {
        // pub codec_name: String,
        // pub codec_type: String,
        pub width: Option<u64>,
        pub height: Option<u64>,
        // #[serde(default)]
        // pub tags: HashMap<String, String>,
        pub disposition: Disposition,
    }

    #[derive(Debug, Deserialize)]
    pub struct Disposition {
        pub default: u8,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Metadata {
    pub height: Option<u64>,
    pub width: Option<u64>,
    pub duration: Option<u64>,
}

pub struct ServiceMedia;

pub struct MediaUpload {
    pub create: MediaCreate,
    pub user_id: UserId,
    pub temp_file: TempFile,
    pub temp_writer: BufWriter<TempFile>,
}

// const UPLOADS: OnceCell<DashMap<MediaId, MediaUpload>> = OnceCell::const_new();

impl ServiceMedia {
    pub fn new() -> Self {
        Self
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_metadata(&self, file: &std::path::Path) -> Result<Option<Metadata>> {
        let out = Command::new("ffprobe")
            .args([
                "-v",
                "quiet",
                "-of",
                "json",
                "-show_format",
                "-show_streams",
                "-i",
            ])
            .arg(file)
            .output()
            .await?;
        if !out.status.success() {
            return Ok(None);
        }
        let json: ffprobe::Metadata = serde_json::from_slice(&out.stdout)?;
        let duration: Option<f64> = match json.format.duration {
            Some(s) => Some(s.parse::<f64>()? * 1000.),
            None => None,
        };
        let dims = json
            .streams
            .iter()
            .find(|i| i.disposition.default == 1 && i.width.is_some())
            .or_else(|| json.streams.iter().find(|i| i.width.is_some()));
        Ok(Some(Metadata {
            height: dims.and_then(|i| i.height),
            width: dims.and_then(|i| i.width),
            duration: duration.map(|i| i as u64),
        }))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mime_type(&self, file: &std::path::Path) -> Result<String> {
        let out = Command::new("file").arg("-ib").arg(file).output().await?;
        let mime = String::from_utf8(out.stdout).expect("file has failed me");
        Ok(mime)
    }
}
