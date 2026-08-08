use mpd::{Client, Song, Status};
use std::net::TcpStream;
use std::time::Duration;

use crate::config::Config;

pub struct MpdClient {
    client: Client<TcpStream>,
}

impl MpdClient {
    pub fn connect(config: &Config) -> anyhow::Result<Self> {
        let addr = format!("{}:{}", config.mpd_host, config.mpd_port);
        let client = Client::connect(addr.as_str())
            .map_err(|e| anyhow::anyhow!("Failed to connect to MPD at {}: {}", addr, e))?;
        Ok(Self { client })
    }

    pub fn queue(&mut self) -> anyhow::Result<Vec<Song>> {
        self.client
            .queue()
            .map_err(|e| anyhow::anyhow!("Failed to fetch queue: {}", e))
    }

    pub fn play_at(&mut self, pos: u32) -> anyhow::Result<()> {
        self.client
            .switch(pos)
            .map_err(|e| anyhow::anyhow!("Failed to play track: {}", e))
    }

    pub fn delete_at(&mut self, pos: u32) -> anyhow::Result<()> {
        self.client
            .delete(mpd::Id(pos))
            .map_err(|e| anyhow::anyhow!("Failed to delete track: {}", e))
    }

    pub fn swap_tracks(&mut self, pos_a: u32, pos_b: u32) -> anyhow::Result<()> {
        self.client
            .swap(pos_a, pos_b)
            .map_err(|e| anyhow::anyhow!("Failed to swap tracks: {}", e))
    }

    pub fn clear_queue(&mut self) -> anyhow::Result<()> {
        self.client
            .clear()
            .map_err(|e| anyhow::anyhow!("Failed to clear queue: {}", e))
    }

    pub fn update_database(&mut self) -> anyhow::Result<()> {
        self.client
            .update()
            .map_err(|e| anyhow::anyhow!("Failed to trigger database update: {}", e))?;
        Ok(())
    }

    pub fn status(&mut self) -> anyhow::Result<Status> {
        self.client
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to get status: {}", e))
    }

    pub fn current_song(&mut self) -> anyhow::Result<Option<Song>> {
        self.client
            .currentsong()
            .map_err(|e| anyhow::anyhow!("Failed to get current song: {}", e))
    }

    pub fn all_songs(&mut self) -> anyhow::Result<Vec<Song>> {
        let mut query = mpd::Query::new();
        query.and(mpd::Term::Any, "");
        self.client
            .search(&query, None)
            .map_err(|e| anyhow::anyhow!("Failed to list all songs: {}", e))
    }

    pub fn seek(&mut self, pos: Duration) -> anyhow::Result<()> {
        self.client
            .rewind(pos.as_secs_f64())
            .map_err(|e| anyhow::anyhow!("Failed to seek: {}", e))
    }

    pub fn append_song(&mut self, song: &Song) -> anyhow::Result<()> {
        self.client
            .push(song)
            .map_err(|e| anyhow::anyhow!("Failed to append song: {}", e))?;
        Ok(())
    }

    pub fn queue_len(&mut self) -> anyhow::Result<u32> {
        let status = self.status()?;
        Ok(status.queue_len)
    }

    pub fn toggle_pause(&mut self) -> anyhow::Result<()> {
        self.client
            .toggle_pause()
            .map_err(|e| anyhow::anyhow!("Failed to toggle pause: {}", e))
    }

    pub fn prev(&mut self) -> anyhow::Result<()> {
        self.client
            .prev()
            .map_err(|e| anyhow::anyhow!("Failed to skip to previous: {}", e))
    }

    pub fn next(&mut self) -> anyhow::Result<()> {
        self.client
            .next()
            .map_err(|e| anyhow::anyhow!("Failed to skip to next: {}", e))
    }

}
