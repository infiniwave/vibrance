use std::{io::SeekFrom, sync::Arc};

use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use futures::{future::{join_all, try_join_all}, lock::Mutex};
use once_cell::sync::OnceCell;
use rustypipe::{client::RustyPipe, model::{AudioFormat, TrackItem}, param::StreamFilter};
use tokio::{fs::File, io::{AsyncSeekExt, AsyncWriteExt}};
use ulid::Ulid;

use crate::{lyrics::CLIENT, player::Track, preferences::PREFERENCES};

pub static YT_CLIENT: OnceCell<RustyPipe> = OnceCell::new();

pub fn initialize_client() -> &'static RustyPipe {
    YT_CLIENT.get_or_init(|| RustyPipe::new())
}

#[derive(Debug, Clone)]
pub struct YtTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_art: Option<String>, // URL to album art, if available
    pub duration: u32, // in seconds
}

pub async fn search_tracks(query: &str) -> Result<Vec<YtTrack>> {
    let client = YT_CLIENT.get().ok_or(anyhow::anyhow!("YouTube client not initialized"))?;
    let results = client.query().music_search_tracks(query).await?;
    let map_track = |track: TrackItem| async move {
        let album_art_url = track.cover.first().as_ref().map(|c| c.url.clone());
        let album_art = match album_art_url {
            Some(url) => {
                async {
                    let client = CLIENT.get()?;
                    let response = client.get(url).send().await.ok()?;
                    if response.status().is_success() {
                        Some(BASE64_STANDARD.encode(response.bytes().await.ok()?))
                    } else {
                        None
                    }
                }.await
            }
            None => None,
        };
        YtTrack {
            id: track.id,
            title: track.name,
            artist: track.artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", "),
            album: track.album.as_ref().map_or("Unknown Album".to_string(), |a| a.name.clone()),
            album_art,
            duration: track.duration.unwrap(),
        }
    };
    let tracks: Vec<_> = results.items.items
        .into_iter()
        .map(map_track)
        .collect();
    let tracks = join_all(tracks).await;
    Ok(tracks)       

}

pub async fn download_track(id: &str, output_path: &str) -> Result<()> {
    let client = YT_CLIENT.get().ok_or(anyhow::anyhow!("YouTube client not initialized"))?;
    let track = client.query().player(id).await?;
    let audio = track
        .select_audio_stream(&StreamFilter::new().audio_formats(vec![AudioFormat::M4a]))
        .ok_or(anyhow::anyhow!("No suitable audio stream found"))?;
    let file = File::create(output_path).await?;
    let client = CLIENT.get().ok_or(anyhow::anyhow!("HTTP client not initialized"))?;
    println!("Downloading track: {}", audio.url);
    // download this file using parallel requests
    // youtube throttles each request to a woeful 30 KB/s
    let response = client.head(&audio.url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download track: {}", response.status()));
    }
    let total_size = response.headers().get("Content-Length").ok_or(anyhow::anyhow!("Failed to get content length"))?.to_str()?.parse::<u64>()?;
    println!("Total size: {} bytes", total_size);
    let file = Arc::new(Mutex::new(file));
    let mut futures = Vec::new();
    let chunk_size = 1024 * 128; // 128 KB
    let mut start = 0;
    while start < total_size {
        let end = std::cmp::min(start + chunk_size as u64, total_size);
        let file_clone = Arc::clone(&file);
        let url = audio.url.clone();
        futures.push(tokio::spawn(async move {
            let range_header = format!("bytes={}-{}", start, end - 1);
            let response = client.get(&url).header("Range", range_header).send().await?;
            if !response.status().is_success() {
                return Err(anyhow::anyhow!("Failed to download chunk: {}", response.status()));
            }
            println!("Downloading chunk: {}-{}", start, end - 1);
            let mut file = file_clone.lock().await;
            let bytes = response.bytes().await?;
            file.seek(SeekFrom::Start(start)).await?;
            file.write_all(&bytes).await?;
            file.flush().await?;
            Ok(())
        }));
        start += chunk_size as u64;
    }
    try_join_all(futures).await?.into_iter().collect::<Result<Vec<_>, _>>()?;    
    println!("Track downloaded to {}", output_path);
    Ok(())
}

pub async fn download_track_default(id: &str) -> Result<Track> {
    let mut preferences = PREFERENCES.get().ok_or(anyhow::anyhow!("Preferences not initialized"))?.lock().map_err(|_| anyhow::anyhow!("Failed to lock preferences"))?;
    if let Some(track) = preferences.find_track_by_yt_id(id) {
        return Ok(track);
    }
    drop(preferences);
    let client = YT_CLIENT.get().ok_or(anyhow::anyhow!("YouTube client not initialized"))?;
    let track = client.query().music_details(id).await?;
    let album_cover = match track.track.cover.first() {
        Some(cover) => {
            let client = CLIENT.get().ok_or(anyhow::anyhow!("HTTP client not initialized"))?;
            let response = client.get(&cover.url).send().await?;
            if response.status().is_success() {
                Some(BASE64_STANDARD.encode(response.bytes().await?))
            } else {
                None
            }
        }
        None => None,
    };
    let id = Ulid::new().to_string();
    let data = dirs::config_dir().ok_or(anyhow::anyhow!("Could not find config directory"))?;
    let path = data.join("Vibrance").join("yt_tracks").join(format!("{}.m4a", id));
    std::fs::create_dir_all(
        path
            .parent()
            .ok_or(anyhow::anyhow!("Could not find parent directory"))?,
    )?;
    let path = path.to_str().unwrap().to_string();
    download_track(&track.track.id, &path).await?;
    let track = Track {
        id,
        title: Some(track.track.name),
        artists: track.track.artists.iter().map(|a| a.name.clone()).collect(),
        album: track.track.album.as_ref().map(|a| a.name.clone()),
        album_art: album_cover,
        duration: track.track.duration.unwrap_or(0) as f64,
        path: Some(path),
        yt_id: Some(track.track.id.clone()),
    };
    let mut preferences = PREFERENCES.get().ok_or(anyhow::anyhow!("Preferences not initialized"))?.lock().map_err(|_| anyhow::anyhow!("Failed to lock preferences"))?;
    preferences.add_unorganized_track(track.clone());
    drop(preferences);
    Ok(track)
}
