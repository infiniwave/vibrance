use std::path::PathBuf;

use lofty::{file::{AudioFile, TaggedFileExt}, probe::Probe, tag::ItemKey};
use ulid::Ulid;

use crate::library::{Album, Artist, Track, TrackSource};

pub fn resolve_track(path: &str) -> anyhow::Result<Track> {
    if path.is_empty() {
        return Err(anyhow::anyhow!("Path is empty"));
    }
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist: {}", path.display()));
    }
    let tag = Probe::open(&path)?.read()?;
    let properties = tag.properties();
    let tag = match tag.primary_tag() {
        Some(primary_tag) => Some(primary_tag),
        None => tag.first_tag(),
    };
    let id = Ulid::new().to_string();
    let mut artists = tag.map_or_else(Vec::new, |t| {
        t.get_strings(&ItemKey::TrackArtists)
            .map(String::from)
            .collect()
    });
    if artists.is_empty() {
        let artist = tag.and_then(|t| t.get_string(&ItemKey::TrackArtist).map(String::from));
        if let Some(artist) = artist {
            artists.push(artist);
        }
    }
    let album_art = tag
        .and_then(|t| t.pictures().get(0))
        .map(|p| p.data().to_vec());
    let album = tag.map(|t| {
        t
            .get_string(&ItemKey::AlbumTitle)
            .map(String::from)
            
    }).flatten().unwrap_or("Unknown Album".to_string());
    let release_year = tag.and_then(|t| {
        t.get_string(&ItemKey::Year)
            .and_then(|date_str| date_str.get(0..4))
            .and_then(|year_str| year_str.parse::<i32>().ok())
    });
    let artists: Vec<Artist> = artists.iter().map(|a| Artist::new(a.to_string())).collect();
    Ok(Track {
        id,
        title: tag.and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from)).unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string()),
        album: Album::new(album,artists.first().into_iter().cloned().collect(), release_year,album_art),
        artists,
        duration: properties.duration().as_secs_f64(),
        path: Some(path.to_string_lossy().to_string()),
        source: TrackSource::Local,
        source_id: None,
        track_number: tag.and_then(|t| t.get_string(&ItemKey::TrackNumber).map(|i| i.parse().ok())).flatten(),
    })
}