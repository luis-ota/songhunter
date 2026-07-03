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


#[derive(Debug, Clone, Deserialize)]
pub struct AcoustIdConfig {
    #[serde(default = "default_acoustid_url")]
    pub url: String,
    pub api_key: String,
    #[serde(default = "default_fpcalc_path")]
    pub fpcalc_path: String,
}

impl AcoustIdConfig {
    pub fn enabled(&self) -> bool {
        !self.api_key.is_empty() && self.api_key != "sua_chave_aqui"
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcrCloudConfig {
    pub host: String,
    pub access_key: String,
    pub access_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShazamConfig {}

#[derive(Debug, Clone, Deserialize)]
pub struct HouseAdSlot {
    pub slot: String,
    pub image_url: String,
    pub dest_url: String,
    pub alt: Option<String>,
    pub cpm: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdsterraConfig {
    pub slot: String,
    pub script_src: Option<String>,
    pub redirect_url: Option<String>,
    pub container_id: Option<String>,
    pub cpm: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdsConfig {
    #[serde(default)]
    pub house: Vec<HouseAdSlot>,
    #[serde(default)]
    pub adsterra: Vec<AdsterraConfig>,
    #[serde(default)]
    pub adsense_client: Option<String>,
    #[serde(default)]
    pub carbon_uid: Option<String>,
    #[serde(default = "default_ad_refresh_secs")]
    pub refresh_secs: u64,
}

fn default_ad_refresh_secs() -> u64 { 60 }

impl AdsConfig {
    pub fn enabled_providers(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if !self.house.is_empty() { v.push("house"); }
        if !self.adsterra.is_empty() { v.push("adsterra"); }
        if self.adsense_client.is_some() { v.push("adsense"); }
        if self.carbon_uid.is_some() { v.push("carbon"); }
        v
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default)]
    pub acoustid: Option<AcoustIdConfig>,
    #[serde(default)]
    pub acrcloud: Option<AcrCloudConfig>,
    #[serde(default)]
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
    #[serde(default)]
    pub ytdlp_extra_args: Vec<String>,
    #[serde(default)]
    pub ytdlp_cookies_file: Option<String>,
    #[serde(default)]
    pub ads: AdsConfig,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::File::with_name("config/ads").required(false))
            .add_source(config::Environment::with_prefix("SONGFINDER").separator("__"))
            .build()?;

        let mut config: Self = cfg.try_deserialize()?;

        if config.ytdlp_cookies_file.is_none()
            && let Ok(cookies) = std::env::var("SONGFINDER_YTDLP_COOKIES_FILE")
            && !cookies.is_empty()
        {
            config.ytdlp_cookies_file = Some(cookies);
        }

        if config.acoustid.is_none()
            && let Ok(api_key) = std::env::var("SONGFINDER_ACOUSTID__API_KEY")
            && !api_key.is_empty() && api_key != "sua_chave_aqui"
        {
            config.acoustid = Some(AcoustIdConfig {
                url: std::env::var("SONGFINDER_ACOUSTID__URL")
                    .unwrap_or_else(|_| default_acoustid_url()),
                api_key,
                fpcalc_path: std::env::var("SONGFINDER_ACOUSTID__FPCALC_PATH")
                    .unwrap_or_else(|_| default_fpcalc_path()),
            });
        }

        if config.acrcloud.is_none()
            && let (Ok(host), Ok(access_key), Ok(access_secret)) = (
                std::env::var("SONGFINDER_ACRCLOUD__HOST"),
                std::env::var("SONGFINDER_ACRCLOUD__ACCESS_KEY"),
                std::env::var("SONGFINDER_ACRCLOUD__ACCESS_SECRET"),
            )
            && !host.is_empty() && !access_key.is_empty() && !access_secret.is_empty()
        {
            config.acrcloud = Some(AcrCloudConfig {
                host,
                access_key,
                access_secret,
            });
        }

        if config.shazam.is_none() {
            config.shazam = Some(ShazamConfig {});
        }

        if config.ytdlp_extra_args.is_empty()
            && let Ok(extra) = std::env::var("SONGFINDER_YTDLP_EXTRA_ARGS")
        {
            config.ytdlp_extra_args = shlex::split(&extra)
                .unwrap_or_default()
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
        }

        Ok(config)
    }

    pub fn addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("invalid host/port")
    }
}
