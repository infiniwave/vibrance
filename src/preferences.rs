use std::collections::HashMap;

use anyhow::Result;
use once_cell::sync::OnceCell;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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
        let existing_track = self
            .unorganized_tracks
            .iter()
            .find(|(_, t)| t.path == track.path || t.yt_id == track.yt_id);
        if let Some((_, existing_track)) = existing_track {
            // ignore if the track already exists
            return;
        }
        self.unorganized_tracks.insert(track.id.clone(), track);
    }
    pub fn find_track_by_id(&self, id: &str) -> Option<Track> {
        let mut track = self.unorganized_tracks.get(id).map(|track| track.clone());
        if track.is_none() {
            track = self
                .user_library
                .par_iter()
                .find_map_any(|(_, tracks)| tracks.par_iter().find_any(|track| track.1.id == id))
                .map(|(_, track)| track.clone());
        }
        track
    }
    pub fn find_track_by_yt_id(&self, yt_id: &str) -> Option<Track> {
        let mut track = self
            .unorganized_tracks
            .values()
            .find(|t| t.yt_id.as_deref() == Some(yt_id))
            .cloned();
        if track.is_none() {
            track = self
                .user_library
                .par_iter()
                .find_map_any(|(_, tracks)| {
                    tracks.values().find(|t| t.yt_id.as_deref() == Some(yt_id))
                })
                .cloned();
        }
        track
    }
    pub fn all_tracks(&self) -> Vec<Track> {
        let tracks = self.unorganized_tracks.values().collect::<Vec<_>>();
        let library = self
            .user_library
            .values()
            .flat_map(|tracks| tracks.values())
            .collect::<Vec<_>>();
        tracks
            .into_iter()
            .chain(library.into_iter())
            .map(|t| t.clone())
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
