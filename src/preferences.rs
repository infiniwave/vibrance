use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::player::Track;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Preferences {
    user_library: HashMap<String, HashMap<String, Track>>, // folder path -> (file name -> Track)
    unorganized_tracks: HashMap<String, Track>, // file name -> Track
    use_system_audio_controls: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            user_library: HashMap::new(),
            unorganized_tracks: HashMap::new(),
            use_system_audio_controls: true,
        }
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