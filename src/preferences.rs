use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

pub static PREFERENCES: OnceCell<RwLock<Preferences>> = OnceCell::new();

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Preferences {
    pub use_system_audio_controls: bool,
    pub volume: f32,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
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
