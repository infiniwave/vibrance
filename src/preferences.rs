use std::{collections::HashMap, sync::Mutex};

use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::player::Track;

pub static PREFERENCES: OnceCell<Mutex<Preferences>> = OnceCell::new();

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Preferences {
    pub user_library: HashMap<String, HashMap<String, Track>>, // folder path -> (file name -> Track)
    pub unorganized_tracks: HashMap<String, Track>, // file name -> Track
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
    pub fn save(&self) -> Result<()> {
        let data = dirs::config_dir().ok_or(anyhow::anyhow!("Could not find config directory"))?;
        let config_path = data.join("Vibrance").join("vibrance.json");
        std::fs::create_dir_all(config_path.parent().ok_or(anyhow::anyhow!("Could not find parent directory"))?)?;
        std::fs::write(&config_path, serde_json::to_string(self)?)?;
        Ok(())
    }
    pub fn add_track_to_library(&mut self, folder: String, track: Track) {
        self.user_library.entry(folder).or_default().insert(track.id.clone(), track);
    }
    pub fn add_unorganized_track(&mut self, track: Track) {
        // check if the track already exists by file name
        let existing_track = self.unorganized_tracks.iter().find(|(_, t)| 
            t.sources.iter().any(|source| 
                if let crate::player::TrackSource::File(path) = source {
                    if let crate::player::TrackSource::File(existing_path) = &track.sources[0] {
                        existing_path == path
                    } else {
                        false
                    }
                } else {
                    false
                }
            )
        );
        if let Some((_, existing_track)) = existing_track {
            // ignore if the track already exists
            return;
        }
        self.unorganized_tracks.insert(track.id.clone(), track);
    }
}

pub fn read_preferences() -> Result<Preferences> {
    let data = dirs::config_dir().ok_or(anyhow::anyhow!("Could not find config directory"))?;
    let config_path = data.join("Vibrance").join("vibrance.json");
    if !config_path.exists() {
        // create the config directory and file
        std::fs::create_dir_all(config_path.parent().ok_or(anyhow::anyhow!("Could not find parent directory"))?)?;
        std::fs::write(&config_path, serde_json::to_string(&Preferences::default())?)?;
    }
    let data = std::fs::read_to_string(config_path)?;
    let preferences: Preferences = serde_json::from_str(&data)?;
    Ok(preferences)
}