use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub mpd_host: String,
    pub mpd_port: u16,
    pub mpd_music_dir: String,
    pub visualizer_data_source: String,
    pub visualizer_output_name: String,
    pub visualizer_in_stereo: String,
    pub visualizer_type: String,
    pub visualizer_look: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path();
        let contents = fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read config at {}: {}", path.display(), e))?;
        let config: Config = toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
        Ok(config)
    }
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/music-player/config.toml")
}
