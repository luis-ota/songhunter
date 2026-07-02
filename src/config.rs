use serde::Deserialize;
use std::net::SocketAddr;

fn default_host() -> String {
    "0.0.0.0".into()
}

fn default_port() -> u16 {
    3000
}

fn default_temp_dir() -> String {
    "./tmp".into()
}

fn default_cache_ttl_hours() -> i64 {
    168
}

fn default_max_concurrent_tasks() -> usize {
    4
}

fn default_ytdlp_path() -> String {
    "yt-dlp".into()
}

fn default_ffmpeg_path() -> String {
    "ffmpeg".into()
}

fn default_fpcalc_path() -> String {
    "fpcalc".into()
}

fn default_acoustid_url() -> String {
    "https://api.acoustid.org/v2/lookup".into()
}

pub fn default_shazam_url() -> String {
    "https://amp.shazam.com/discovery/v5/en/US/iphone/-/tag".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcoustIdConfig {
    #[serde(default = "default_acoustid_url")]
    pub url: String,
    pub api_key: String,
    #[serde(default = "default_fpcalc_path")]
    pub fpcalc_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcrCloudConfig {
    pub host: String,
    pub access_key: String,
    pub access_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShazamConfig {
    #[serde(default = "default_shazam_url")]
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,

    pub acoustid: Option<AcoustIdConfig>,
    pub acrcloud: Option<AcrCloudConfig>,
    pub shazam: Option<ShazamConfig>,

    #[serde(default = "default_temp_dir")]
    pub temp_dir: String,
    #[serde(default = "default_cache_ttl_hours")]
    pub cache_ttl_hours: i64,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,
    #[serde(default = "default_ytdlp_path")]
    pub ytdlp_path: String,
    #[serde(default = "default_ffmpeg_path")]
    pub ffmpeg_path: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(
                config::File::with_name(".env")
                    .required(false)
                    .format(config::FileFormat::Ini),
            )
            .add_source(config::Environment::with_prefix("SONGFINDER").separator("__"))
            .build()?;

        Ok(cfg.try_deserialize()?)
    }

    pub fn addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("invalid host/port")
    }
}
