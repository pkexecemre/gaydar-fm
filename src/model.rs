//! Shared domain types for GAYDAR FM.

use serde::{Deserialize, Serialize};

/// A SoundCloud track, normalised for the player and the Gaydar algorithm.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<u64>,
    pub artist_avatar_url: Option<String>,
    pub artwork_url: Option<String>,
    pub duration_ms: u64,
    pub genre: Option<String>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub playback_count: u64,
    pub likes_count: u64,
    pub user_followers: u64,
    pub created_at: Option<String>,
    pub permalink_url: String,
    pub streamable: bool,
    pub policy: String,
}

impl Track {
    pub fn duration_secs(&self) -> u64 {
        self.duration_ms / 1000
    }

    pub fn duration_label(&self) -> String {
        format_time(self.duration_ms)
    }

    /// A 500px artwork URL when SoundCloud gives us one of its
    /// -large.jpg / -t500x500.jpg URLs.
    pub fn artwork_large(&self) -> Option<String> {
        let url = self.artwork_url.as_deref()?;
        Some(url.replace("-large.", "-t500x500."))
    }

    pub fn seed_text(&self) -> String {
        let mut s = self.title.clone();
        s.push(' ');
        s.push_str(&self.artist);
        if let Some(g) = &self.genre {
            s.push(' ');
            s.push_str(g);
        }
        for t in &self.tags {
            s.push(' ');
            s.push_str(t);
        }
        s
    }
}

pub fn format_time(ms: u64) -> String {
    let secs = ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamKind {
    #[default]
    Progressive,
    Hls,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StreamSource {
    pub url: String,
    pub kind: StreamKind,
}

/// The four dials behind the Gaydar. All values are 0.0..=1.0.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Dials {
    pub camp: f32,
    pub club: f32,
    pub soft: f32,
    pub chaos: f32,
}

impl Default for Dials {
    fn default() -> Self {
        Self {
            camp: 0.82,
            club: 0.74,
            soft: 0.46,
            chaos: 0.38,
        }
    }
}

impl Dials {
    pub fn clamped(mut self) -> Self {
        self.camp = self.camp.clamp(0.0, 1.0);
        self.club = self.club.clamp(0.0, 1.0);
        self.soft = self.soft.clamp(0.0, 1.0);
        self.chaos = self.chaos.clamp(0.0, 1.0);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub track: Track,
    pub reason: String,
    pub score: f32,
}
