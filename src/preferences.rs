use std::collections::HashMap;

use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

use crate::player::Track;

pub static PREFERENCES: OnceCell<RwLock<Preferences>> = OnceCell::new();

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Preferences {
    pub user_library: HashMap<String, HashMap<String, Track>>, // folder path -> (file name -> Track)
    pub unorganized_tracks: HashMap<String, Track>,            // file name -> Track
    pub use_system_audio_controls: bool,
    pub volume: f32,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            user_library: HashMap::new(),
            unorganized_tracks: HashMap::new(),
            use_system_audio_controls: true,
            volume: 0.5,
        }
    }
}

impl Preferences {
    pub async fn save(&self) -> Result<()> {
        let data = dirs::config_dir().ok_or(anyhow::anyhow!("Could not find config directory"))?;
        let config_path = data.join("Vibrance").join("vibrance.json");
        fs::create_dir_all(
            config_path
                .parent()
                .ok_or(anyhow::anyhow!("Could not find parent directory"))?,
        )
        .await?;
        fs::write(&config_path, serde_json::to_string(self)?).await?;
        Ok(())
    }
    pub fn add_track_to_library(&mut self, folder: String, track: Track) {
        self.add_tracks_to_library(folder, vec![track])
    }
    pub fn add_tracks_to_library(&mut self, folder: String, tracks: Vec<Track>) {
        let folder_tracks = self.user_library.entry(folder).or_default();
        for track in tracks {
            folder_tracks.insert(track.id.clone(), track);
        }
    }
    pub fn add_unorganized_track(&mut self, track: Track) {
        // check if the track already exists by file name
        let exists = self
            .unorganized_tracks
            .values()
            .any(|t| t.path == track.path || t.yt_id == track.yt_id);
        if exists {
            // ignore if the track already exists
            return;
        }
        self.unorganized_tracks.insert(track.id.clone(), track);
    }
    pub fn find_track_by_id(&self, id: &str) -> Option<&Track> {
        if let Some(track) = self.unorganized_tracks.get(id) {
            return Some(track);
        }
        self.user_library
            .values()
            .find_map(|tracks| tracks.get(id))
    }
    pub fn find_track_by_yt_id(&self, yt_id: &str) -> Option<&Track> {
        if let Some(track) = self
            .unorganized_tracks
            .values()
            .find(|t| t.yt_id.as_deref() == Some(yt_id))
        {
            return Some(track);
        }
        self.user_library
            .values()
            .find_map(|tracks| tracks.values().find(|t| t.yt_id.as_deref() == Some(yt_id)))
    }
    pub fn all_tracks(&self) -> Vec<&Track> {
        self.unorganized_tracks
            .values()
            .chain(self.user_library.values().flat_map(|tracks| tracks.values()))
            .collect()
    }
}

pub async fn read_preferences() -> Result<Preferences> {
    let data = dirs::config_dir().ok_or(anyhow::anyhow!("Could not find config directory"))?;
    let config_path = data.join("Vibrance").join("vibrance.json");
    if !config_path.exists() {
        // create the config directory and file
        fs::create_dir_all(
            config_path
                .parent()
                .ok_or(anyhow::anyhow!("Could not find parent directory"))?,
        )
        .await?;
        fs::write(
            &config_path,
            serde_json::to_string(&Preferences::default())?,
        )
        .await?;
    }
    let data = fs::read_to_string(config_path).await?;
    let preferences: Preferences = serde_json::from_str(&data)?;
    Ok(preferences)
}
