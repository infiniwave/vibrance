use std::{io::SeekFrom, sync::Arc};

use anyhow::Result;
use futures::{
    future::{join_all, try_join_all},
    lock::Mutex,
};
use once_cell::sync::OnceCell;
use rustypipe::{
    client::RustyPipe,
    model::{AudioFormat, TrackItem},
    param::StreamFilter,
};
use tokio::{
    fs::{self, File},
    io::{AsyncSeekExt, AsyncWriteExt},
};
use ulid::Ulid;

use crate::{
    library::{Album, Artist, LIBRARY, Track, TrackSource},
    lyrics::{self},
};

pub static YT_CLIENT: OnceCell<RustyPipe> = OnceCell::new();

pub fn get_client() -> &'static RustyPipe {
    YT_CLIENT.get_or_init(|| RustyPipe::new())
}

#[derive(Debug, Clone)]
pub struct YtTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_art: Option<Vec<u8>>,
    pub duration: u32, // in seconds
}

pub async fn search_tracks(query: &str) -> Result<Vec<YtTrack>> {
    let client = get_client();
    let results = client.query().music_search_tracks(query).await?;
    let map_track = |track: TrackItem| async move {
        let album_art_url = track.cover.first().map(|c| c.url.clone());
        let album_art = match album_art_url {
            Some(url) => {
                async {
                    let client = lyrics::get_client().ok()?;
                    let response = client.get(url).send().await.ok()?;
                    if response.status().is_success() {
                        Some(response.bytes().await.ok()?.to_vec())
                    } else {
                        None
                    }
                }
                .await
            }
            None => None,
        };
        let artist = track
            .artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let album = track
            .album
            .as_ref()
            .map_or_else(|| "Unknown Album".to_string(), |a| a.name.clone());
        YtTrack {
            id: track.id,
            title: track.name,
            artist,
            album,
            album_art,
            duration: track.duration.unwrap(),
        }
    };
    let tracks: Vec<_> = results.items.items.into_iter().map(map_track).collect();
    let tracks = join_all(tracks).await;
    Ok(tracks)
}

pub async fn download_track(id: &str, output_path: &str) -> Result<()> {
    let client = get_client();
    let track = client.query().player(id).await?;
    let audio = track
        .select_audio_stream(&StreamFilter::new().audio_formats(vec![AudioFormat::M4a]))
        .ok_or(anyhow::anyhow!("No suitable audio stream found"))?;
    let file = File::create(output_path).await?;
    let client = lyrics::get_client()?;
    println!("Downloading track: {}", audio.url);
    // download this file using parallel requests
    // youtube throttles each request to a woeful 30 KB/s
    let response = client.head(&audio.url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download track: {}",
            response.status()
        ));
    }
    let total_size = response
        .headers()
        .get("Content-Length")
        .ok_or(anyhow::anyhow!("Failed to get content length"))?
        .to_str()?
        .parse::<u64>()?;
    println!("Total size: {} bytes", total_size);
    let file = Arc::new(Mutex::new(file));
    let chunk_size = 1024 * 256; // 256 KB
    let semaphore = Arc::new(tokio::sync::Semaphore::new(8));
    let mut futures = Vec::new();
    let mut start = 0;
    while start < total_size {
        let end = std::cmp::min(start + chunk_size as u64, total_size);
        let file_clone = Arc::clone(&file);
        let url = audio.url.clone();
        let semaphore_clone = Arc::clone(&semaphore);
        futures.push(tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await?;
            let range_header = format!("bytes={}-{}", start, end - 1);
            let response = client
                .get(&url)
                .header("Range", range_header)
                .send()
                .await?;
            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Failed to download chunk: {}",
                    response.status()
                ));
            }
            println!("Downloading chunk: {}-{}", start, end - 1);
            let bytes = response.bytes().await?;
            {
                let mut file = file_clone.lock().await;
                file.seek(SeekFrom::Start(start)).await?;
                file.write_all(&bytes).await?;
                file.flush().await?;
            }
            Ok(())
        }));
        start += chunk_size as u64;
    }
    try_join_all(futures)
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    println!("Track downloaded to {}", output_path);
    Ok(())
}

pub async fn query_track(id: &str) -> Result<Track> {
    let client = get_client();
    let track = client.query().music_details(id).await?;
    let mut covers = track.track.cover.clone();
    covers.sort_by(|a, b| a.width.cmp(&b.width));
    let album_cover = match covers.last() {
        Some(cover) => {
            let client = lyrics::get_client()?;
            let response = client.get(&cover.url).send().await?;
            if response.status().is_success() {
                Some(response.bytes().await?.to_vec())
            } else {
                None
            }
        }
        None => None,
    };
    let id = Ulid::new().to_string();
    let artists: Vec<Artist> = track
        .track
        .artists
        .iter()
        .map(|a| Artist::new(a.name.clone()))
        .collect();
    let track = Track {
        id,
        title: track.track.name,
        album: track
            .track
            .album
            .as_ref()
            .map(|a| {
                Album::new(
                    a.name.clone(),
                    artists.first().cloned().into_iter().collect(),
                    None,
                    album_cover,
                )
            })
            .unwrap_or(Album::new(
                "Unknown Album".to_string(),
                artists.first().cloned().into_iter().collect(),
                None,
                None,
            )),
        artists,
        duration: track.track.duration.unwrap_or(0) as f64,
        path: None,
        source_id: Some(track.track.id),
        source: TrackSource::YouTube,
        track_number: None,
    };
    Ok(track)
}

pub async fn get_or_query_track(id: &str) -> Result<Track> {
    let library = LIBRARY
        .get()
        .ok_or(anyhow::anyhow!("Library not initialized"))?;
    if let Some(track) = library
        .find_track_by_source(TrackSource::YouTube, id)
        .await?
    {
        Ok(track)
    } else {
        let track = query_track(id).await?;
        Ok(track)
    }
}

pub async fn get_default_download_path(id: &str) -> Result<String> {
    let data = dirs::config_dir().ok_or(anyhow::anyhow!("Could not find config directory"))?;
    let path = data
        .join("Vibrance")
        .join("yt_tracks")
        .join(format!("{}.m4a", id));
    fs::create_dir_all(
        path.parent()
            .ok_or(anyhow::anyhow!("Could not find parent directory"))?,
    )
    .await?;
    let path = path.to_str().ok_or(anyhow::anyhow!("Invalid path"))?;
    Ok(path.to_string())
}

pub async fn download_track_and_save(track: &Track, path: &str) -> Result<()> {
    if track.source != TrackSource::YouTube {
        return Err(anyhow::anyhow!("Track is not a YouTube track"));
    }
    let Some(video_id) = track.source_id.as_ref() else {
        return Err(anyhow::anyhow!("Track does not have a YouTube ID"));
    };
    download_track(video_id, &path).await?;
    let library = LIBRARY
        .get()
        .ok_or(anyhow::anyhow!("Library not initialized"))?;
    library.add_track(&track).await?;
    Ok(())
}
