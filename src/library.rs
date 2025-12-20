use std::{fs::File, path::PathBuf};

use once_cell::sync::OnceCell;
use rodio::{Decoder, Source};
use serde::{Deserialize, Serialize};
use tokio::fs;
use turso::{Builder, Connection, Value};
use ulid::Ulid;

use crate::providers::youtube;

pub static LIBRARY: OnceCell<Library> = OnceCell::new();

const CREATE_DB: &str = r#"
CREATE TABLE IF NOT EXISTS artists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS albums (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    release_year INTEGER,
    album_art BLOB
);

CREATE TABLE IF NOT EXISTS tracks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    album_id TEXT NOT NULL,
    duration REAL NOT NULL,
    path TEXT,
    source TEXT NOT NULL,
    source_id TEXT,
    track_number INTEGER,
    FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS playlists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT
);

CREATE TABLE IF NOT EXISTS album_artists (
    album_id TEXT NOT NULL,
    artist_id TEXT NOT NULL,
    PRIMARY KEY (album_id, artist_id),
    FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS track_artists (
    track_id TEXT NOT NULL,
    artist_id TEXT NOT NULL,
    PRIMARY KEY (track_id, artist_id),
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS playlist_tracks (
    playlist_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    position INTEGER,
    added_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (playlist_id, track_id),
    FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album_id);
CREATE INDEX IF NOT EXISTS idx_tracks_source ON tracks(source);
CREATE INDEX IF NOT EXISTS idx_playlist_tracks_position ON playlist_tracks(playlist_id, position);
CREATE INDEX IF NOT EXISTS idx_album_artists_artist ON album_artists(artist_id);
CREATE INDEX IF NOT EXISTS idx_track_artists_artist ON track_artists(artist_id);
"#;

/// Source of a track (local file or online provider)
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum TrackSource {
    Local,
    YouTube,
}

impl TrackSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrackSource::Local => "local",
            TrackSource::YouTube => "youtube",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "local" => Some(TrackSource::Local),
            "youtube" => Some(TrackSource::YouTube),
            _ => None,
        }
    }
}

/// A track in the library
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: f64,
    pub path: Option<String>,
    pub source: TrackSource,
    pub source_id: Option<String>,
    pub track_number: Option<i32>,
}

impl Track {
    /// Create a new local track
    pub fn new_local(
        title: String,
        artists: Vec<Artist>,
        album: Album,
        duration: f64,
        path: String,
        track_number: Option<i32>,
    ) -> Self {
        Self {
            id: Ulid::new().to_string(),
            title,
            artists,
            album,
            duration,
            path: Some(path),
            source: TrackSource::Local,
            source_id: None,
            track_number,
        }
    }

    /// Create a new YouTube track
    pub fn new_youtube(
        title: String,
        artists: Vec<Artist>,
        album: Album,
        duration: f64,
        yt_id: String,
    ) -> Self {
        Self {
            id: Ulid::new().to_string(),
            title,
            artists,
            album,
            duration,
            path: None,
            source: TrackSource::YouTube,
            source_id: Some(yt_id),
            track_number: None,
        }
    }

    pub fn artists_string(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// An artist in the library
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

impl Artist {
    pub fn new(name: String) -> Self {
        Self {
            id: Ulid::new().to_string(),
            name,
        }
    }
}

impl ToString for Artist {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}

/// An album in the library
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artists: Vec<Artist>,
    pub release_year: Option<i32>,
    pub album_art: Option<Vec<u8>>, // binary image data
}

impl Album {
    pub fn new(title: String, artists: Vec<Artist>, release_year: Option<i32>, album_art: Option<Vec<u8>>) -> Self {
        Self {
            id: Ulid::new().to_string(),
            title,
            artists,
            release_year,
            album_art,
        }
    }
}

impl ToString for Album {
    fn to_string(&self) -> String {
        self.title.clone()
    }
}

/// A playlist in the library
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tracks: Vec<Track>,
}

impl Playlist {
    pub fn new(name: String, description: Option<String>) -> Self {
        Self {
            id: Ulid::new().to_string(),
            name,
            description,
            tracks: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Library {
    connection: Connection,
}

impl Library {
    pub async fn initialize() -> anyhow::Result<Self> {
        let db_path = dirs::data_dir()
            .ok_or(anyhow::anyhow!("Could not find data directory"))?
            .join("Vibrance")
            .join("library.db");
        fs::create_dir_all(
            db_path
                .parent()
                .ok_or(anyhow::anyhow!("Could not find parent directory"))?,
        )
        .await?;
        let connection = Builder::new_local(
            db_path
                .to_str()
                .ok_or(anyhow::anyhow!("Invalid path"))?,
        )
        .build()
        .await?
        .connect()?;
        connection
            .execute_batch(CREATE_DB)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create database: {}", e))?;
        Ok(Self { connection })
    }

    pub fn write(&self) -> anyhow::Result<()> {
        self.connection.cacheflush()?;
        Ok(())
    }

    /// Add an artist to the library, returns the artist (with existing id if already exists)
    pub async fn add_artist(&self, artist: &Artist) -> anyhow::Result<Artist> {
        // Check if artist already exists by name
        if let Some(existing) = self.find_artist_by_name(&artist.name).await? {
            return Ok(existing);
        }

        self.connection
            .execute(
                "INSERT INTO artists (id, name) VALUES (?, ?)",
                [
                    Value::Text(artist.id.clone()),
                    Value::Text(artist.name.clone()),
                ],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to insert artist: {}", e))?;

        Ok(artist.clone())
    }

    /// Find an artist by ID
    pub async fn find_artist_by_id(&self, id: &str) -> anyhow::Result<Option<Artist>> {
        let mut rows = self
            .connection
            .query("SELECT id, name FROM artists WHERE id = ?", [id])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query artist: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let name: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get name: {}", e))?;
            Ok(Some(Artist { id, name }))
        } else {
            Ok(None)
        }
    }

    /// Find an artist by name
    pub async fn find_artist_by_name(&self, name: &str) -> anyhow::Result<Option<Artist>> {
        let mut rows = self
            .connection
            .query("SELECT id, name FROM artists WHERE name = ?", [name])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query artist: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let name: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get name: {}", e))?;
            Ok(Some(Artist { id, name }))
        } else {
            Ok(None)
        }
    }

    /// Get all artists
    pub async fn all_artists(&self) -> anyhow::Result<Vec<Artist>> {
        let mut rows = self
            .connection
            .query("SELECT id, name FROM artists ORDER BY name", ())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query artists: {}", e))?;

        let mut artists = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let name: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get name: {}", e))?;
            artists.push(Artist { id, name });
        }
        Ok(artists)
    }

    /// Add an album to the library
    pub async fn add_album(&self, album: &Album) -> anyhow::Result<Album> {
        // Check if album already exists by title
        if let Some(existing) = self.find_album_by_title(&album.title).await? {
            return Ok(existing);
        }

        self.connection
            .execute(
                "INSERT INTO albums (id, title, release_year, album_art) VALUES (?, ?, ?, ?)",
                [
                    Value::Text(album.id.clone()),
                    Value::Text(album.title.clone()),
                    album.release_year.map(|y| Value::Integer(y as i64)).unwrap_or(Value::Null),
                    album.album_art.clone().map(Value::Blob).unwrap_or(Value::Null),
                ],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to insert album: {}", e))?;

        // Add album artists
        for artist in &album.artists {
            let artist = self.add_artist(artist).await?;
            self.connection
                .execute(
                    "INSERT OR IGNORE INTO album_artists (album_id, artist_id) VALUES (?, ?)",
                    [Value::Text(album.id.clone()), Value::Text(artist.id)],
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to insert album artist: {}", e))?;
        }

        Ok(album.clone())
    }

    /// Find an album by ID
    pub async fn find_album_by_id(&self, id: &str) -> anyhow::Result<Option<Album>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, release_year, album_art FROM albums WHERE id = ?",
                [id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query album: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let title: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get title: {}", e))?;
            let release_year: Option<i32> = row.get::<Option<i64>>(2)
                .map_err(|e| anyhow::anyhow!("Failed to get release_year: {}", e))?
                .map(|y| y as i32);
            let album_art: Option<Vec<u8>> = row.get(3).map_err(|e| anyhow::anyhow!("Failed to get album_art: {}", e))?;

            let artists = self.get_album_artists(&id).await?;

            Ok(Some(Album {
                id,
                title,
                artists,
                release_year,
                album_art,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find an album by title
    pub async fn find_album_by_title(&self, title: &str) -> anyhow::Result<Option<Album>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, release_year, album_art FROM albums WHERE title = ?",
                [title],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query album: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let title: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get title: {}", e))?;
            let release_year: Option<i32> = row.get::<Option<i64>>(2)
                .map_err(|e| anyhow::anyhow!("Failed to get release_year: {}", e))?
                .map(|y| y as i32);
            let album_art: Option<Vec<u8>> = row.get(3).map_err(|e| anyhow::anyhow!("Failed to get album_art: {}", e))?;

            let artists = self.get_album_artists(&id).await?;

            Ok(Some(Album {
                id,
                title,
                artists,
                release_year,
                album_art,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get artists for an album
    async fn get_album_artists(&self, album_id: &str) -> anyhow::Result<Vec<Artist>> {
        let mut rows = self
            .connection
            .query(
                "SELECT a.id, a.name FROM artists a 
                 INNER JOIN album_artists aa ON a.id = aa.artist_id 
                 WHERE aa.album_id = ?",
                [album_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query album artists: {}", e))?;

        let mut artists = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let name: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get name: {}", e))?;
            artists.push(Artist { id, name });
        }
        Ok(artists)
    }

    /// Get all albums
    pub async fn all_albums(&self) -> anyhow::Result<Vec<Album>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, release_year, album_art FROM albums ORDER BY title",
                (),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query albums: {}", e))?;

        let mut albums = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let title: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get title: {}", e))?;
            let release_year: Option<i32> = row.get::<Option<i64>>(2)
                .map_err(|e| anyhow::anyhow!("Failed to get release_year: {}", e))?
                .map(|y| y as i32);
            let album_art: Option<Vec<u8>> = row.get(3).map_err(|e| anyhow::anyhow!("Failed to get album_art: {}", e))?;

            let artists = self.get_album_artists(&id).await?;

            albums.push(Album {
                id,
                title,
                artists,
                release_year,
                album_art,
            });
        }
        Ok(albums)
    }

    /// Add a track to the library
    pub async fn add_track(&self, track: &Track) -> anyhow::Result<Track> {
        // Add album if present
        let album = self.add_album(&track.album).await?;

        // Add artists
        let mut artists_with_ids = Vec::new();
        for artist in &track.artists {
            let artist = self.add_artist(artist).await?;
            artists_with_ids.push(artist);
        }

        self.connection
            .execute(
                "INSERT INTO tracks (id, title, album_id, duration, path, source, source_id, track_number) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    Value::Text(track.id.clone()),
                    Value::Text(track.title.clone()),
                    Value::Text(album.id.clone()),
                    Value::Real(track.duration),
                    track.path.clone().map(Value::Text).unwrap_or(Value::Null),
                    Value::Text(track.source.as_str().to_string()),
                    track.source_id.clone().map(Value::Text).unwrap_or(Value::Null),
                    track.track_number.map(|n| Value::Integer(n as i64)).unwrap_or(Value::Null),
                ],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to insert track: {}", e))?;

        // Add track artists
        for artist in &artists_with_ids {
            self.connection
                .execute(
                    "INSERT OR IGNORE INTO track_artists (track_id, artist_id) VALUES (?, ?)",
                    [Value::Text(track.id.clone()), Value::Text(artist.id.clone())],
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to insert track artist: {}", e))?;
        }

        // Return track with updated artists
        let mut result = track.clone();
        result.artists = artists_with_ids;
        Ok(result)
    }

    /// Add multiple tracks to the library
    pub async fn add_tracks(&self, tracks: &[Track]) -> anyhow::Result<Vec<Track>> {
        let mut results = Vec::new();
        for track in tracks {
            results.push(self.add_track(track).await?);
        }
        Ok(results)
    }

    /// Find a track by ID 
    pub async fn find_track_by_id(&self, id: &str) -> anyhow::Result<Option<Track>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE id = ?",
                [id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query track: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            self.row_to_track(&row).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Find a track by source ID
    pub async fn find_track_by_source(&self, source: TrackSource, id: &str) -> anyhow::Result<Option<Track>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE source = ? AND source_id = ?",
                [source.as_str(), id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query track: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            self.row_to_track(&row).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Find tracks by source
    pub async fn find_tracks_by_source(&self, source: TrackSource) -> anyhow::Result<Vec<Track>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE source = ?",
                [source.as_str()],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query tracks: {}", e))?;

        let mut tracks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Get all tracks in the library
    pub async fn all_tracks(&self) -> anyhow::Result<Vec<Track>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks ORDER BY title",
                (),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query tracks: {}", e))?;

        let mut tracks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Get all tracks not in any playlist
    pub async fn all_unorganized_tracks(&self) -> anyhow::Result<Vec<Track>> {
        let mut rows = self
            .connection
            .query(
                "SELECT t.id, t.title, t.album_id, t.duration, t.path, t.source, t.source_id, t.track_number 
                 FROM tracks t
                 LEFT JOIN playlist_tracks pt ON t.id = pt.track_id
                 WHERE pt.track_id IS NULL
                 ORDER BY t.title",
                (),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query unorganized tracks: {}", e))?;

        let mut tracks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Get artists for a track
    async fn get_track_artists(&self, track_id: &str) -> anyhow::Result<Vec<Artist>> {
        let mut rows = self
            .connection
            .query(
                "SELECT a.id, a.name FROM artists a 
                 INNER JOIN track_artists ta ON a.id = ta.artist_id 
                 WHERE ta.track_id = ?",
                [track_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query track artists: {}", e))?;

        let mut artists = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let name: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get name: {}", e))?;
            artists.push(Artist { id, name });
        }
        Ok(artists)
    }

    /// Convert a database row to a Track
    async fn row_to_track(&self, row: &turso::Row) -> anyhow::Result<Track> {
        let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
        let title: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get title: {}", e))?;
        let album_id: String = row.get(2).map_err(|e| anyhow::anyhow!("Failed to get album_id: {}", e))?;
        let duration: f64 = row.get(3).map_err(|e| anyhow::anyhow!("Failed to get duration: {}", e))?;
        let path: Option<String> = row.get(4).map_err(|e| anyhow::anyhow!("Failed to get path: {}", e))?;
        let source_str: String = row.get(5).map_err(|e| anyhow::anyhow!("Failed to get source: {}", e))?;
        let source_id: Option<String> = row.get(6).map_err(|e| anyhow::anyhow!("Failed to get source_id: {}", e))?;
        let track_number: Option<i32> = row.get::<Option<i64>>(7)
            .map_err(|e| anyhow::anyhow!("Failed to get track_number: {}", e))?
            .map(|n| n as i32);

        let source = TrackSource::from_str(&source_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid track source: {}", source_str))?;

        let artists = self.get_track_artists(&id).await?;
        let album = self.find_album_by_id(&album_id).await?.ok_or_else(|| anyhow::anyhow!("Album not found for track: {}", id))?;

        Ok(Track {
            id,
            title,
            artists,
            album,
            duration,
            path,
            source,
            source_id,
            track_number,
        })
    }

    /// Delete a track by ID
    pub async fn delete_track(&self, id: &str) -> anyhow::Result<()> {
        self.connection
            .execute("DELETE FROM tracks WHERE id = ?", [id])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete track: {}", e))?;
        Ok(())
    }

    /// Create a new playlist
    pub async fn create_playlist(&self, playlist: &Playlist) -> anyhow::Result<Playlist> {
        self.connection
            .execute(
                "INSERT INTO playlists (id, name, description) VALUES (?, ?, ?)",
                [
                    Value::Text(playlist.id.clone()),
                    Value::Text(playlist.name.clone()),
                    playlist.description.clone().map(Value::Text).unwrap_or(Value::Null),
                ],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create playlist: {}", e))?;

        Ok(playlist.clone())
    }

    /// Find a playlist by ID
    pub async fn find_playlist_by_id(&self, id: &str) -> anyhow::Result<Option<Playlist>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, name, description FROM playlists WHERE id = ?",
                [id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query playlist: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let name: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get name: {}", e))?;
            let description: Option<String> = row.get(2).map_err(|e| anyhow::anyhow!("Failed to get description: {}", e))?;

            let tracks = self.get_playlist_tracks(&id).await?;

            Ok(Some(Playlist {
                id,
                name,
                description,
                tracks,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all playlists
    pub async fn all_playlists(&self) -> anyhow::Result<Vec<Playlist>> {
        let mut rows = self
            .connection
            .query("SELECT id, name, description FROM playlists ORDER BY name", ())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query playlists: {}", e))?;

        let mut playlists = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let name: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get name: {}", e))?;
            let description: Option<String> = row.get(2).map_err(|e| anyhow::anyhow!("Failed to get description: {}", e))?;

            let tracks = self.get_playlist_tracks(&id).await?;

            playlists.push(Playlist {
                id,
                name,
                description,
                tracks,
            });
        }
        Ok(playlists)
    }

    /// Get tracks in a playlist
    async fn get_playlist_tracks(&self, playlist_id: &str) -> anyhow::Result<Vec<Track>> {
        let mut rows = self
            .connection
            .query(
                "SELECT t.id, t.title, t.album_id, t.duration, t.path, t.source, t.source_id, t.track_number 
                 FROM tracks t
                 INNER JOIN playlist_tracks pt ON t.id = pt.track_id
                 WHERE pt.playlist_id = ?
                 ORDER BY pt.position",
                [playlist_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query playlist tracks: {}", e))?;

        let mut tracks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Add a track to a playlist
    pub async fn add_track_to_playlist(&self, playlist_id: &str, track_id: &str) -> anyhow::Result<()> {
        // Get the next position
        let mut rows = self
            .connection
            .query(
                "SELECT COALESCE(MAX(position), 0) + 1 FROM playlist_tracks WHERE playlist_id = ?",
                [playlist_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get next position: {}", e))?;

        let position: i64 = if let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            row.get(0).unwrap_or(1)
        } else {
            1
        };

        self.connection
            .execute(
                "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
                [
                    Value::Text(playlist_id.to_string()),
                    Value::Text(track_id.to_string()),
                    Value::Integer(position),
                ],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to add track to playlist: {}", e))?;

        Ok(())
    }

    /// Remove a track from a playlist
    pub async fn remove_track_from_playlist(&self, playlist_id: &str, track_id: &str) -> anyhow::Result<()> {
        self.connection
            .execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?",
                [playlist_id, track_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to remove track from playlist: {}", e))?;

        Ok(())
    }

    /// Delete a playlist
    pub async fn delete_playlist(&self, id: &str) -> anyhow::Result<()> {
        self.connection
            .execute("DELETE FROM playlists WHERE id = ?", [id])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete playlist: {}", e))?;
        Ok(())
    }

    /// Update a playlist
    pub async fn update_playlist(&self, playlist: &Playlist) -> anyhow::Result<()> {
        self.connection
            .execute(
                "UPDATE playlists SET name = ?, description = ? WHERE id = ?",
                [
                    Value::Text(playlist.name.clone()),
                    playlist.description.clone().map(Value::Text).unwrap_or(Value::Null),
                    Value::Text(playlist.id.clone()),
                ],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update playlist: {}", e))?;
        Ok(())
    }

    /// Search tracks by title
    pub async fn search_tracks(&self, query: &str) -> anyhow::Result<Vec<Track>> {
        let pattern = format!("%{}%", query);
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE title LIKE ? ORDER BY title",
                [pattern.as_str()],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to search tracks: {}", e))?;

        let mut tracks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Search artists by name
    pub async fn search_artists(&self, query: &str) -> anyhow::Result<Vec<Artist>> {
        let pattern = format!("%{}%", query);
        let mut rows = self
            .connection
            .query(
                "SELECT id, name FROM artists WHERE name LIKE ? ORDER BY name",
                [pattern.as_str()],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to search artists: {}", e))?;

        let mut artists = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let name: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get name: {}", e))?;
            artists.push(Artist { id, name });
        }
        Ok(artists)
    }

    /// Search albums by title
    pub async fn search_albums(&self, query: &str) -> anyhow::Result<Vec<Album>> {
        let pattern = format!("%{}%", query);
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, release_year, album_art FROM albums WHERE title LIKE ? ORDER BY title",
                [pattern.as_str()],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to search albums: {}", e))?;

        let mut albums = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            let id: String = row.get(0).map_err(|e| anyhow::anyhow!("Failed to get id: {}", e))?;
            let title: String = row.get(1).map_err(|e| anyhow::anyhow!("Failed to get title: {}", e))?;
            let release_year: Option<i32> = row.get::<Option<i64>>(2)
                .map_err(|e| anyhow::anyhow!("Failed to get release_year: {}", e))?
                .map(|y| y as i32);
            let album_art: Option<Vec<u8>> = row.get(3).map_err(|e| anyhow::anyhow!("Failed to get album_art: {}", e))?;

            let artists = self.get_album_artists(&id).await?;

            albums.push(Album {
                id,
                title,
                artists,
                release_year,
                album_art,
            });
        }
        Ok(albums)
    }

    /// Get tracks by album
    pub async fn get_tracks_by_album(&self, album_id: &str) -> anyhow::Result<Vec<Track>> {
        let mut rows = self
            .connection
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE album_id = ? ORDER BY track_number, title",
                [album_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query tracks by album: {}", e))?;

        let mut tracks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Get tracks by artist
    pub async fn get_tracks_by_artist(&self, artist_id: &str) -> anyhow::Result<Vec<Track>> {
        let mut rows = self
            .connection
            .query(
                "SELECT t.id, t.title, t.album_id, t.duration, t.path, t.source, t.source_id, t.track_number 
                 FROM tracks t
                 INNER JOIN track_artists ta ON t.id = ta.track_id
                 WHERE ta.artist_id = ?
                 ORDER BY t.title",
                [artist_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query tracks by artist: {}", e))?;

        let mut tracks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| anyhow::anyhow!("Failed to fetch row: {}", e))? {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }
}

impl  Track {
    pub async fn load(&self) -> anyhow::Result<impl Source + use<>> {
        match self.source {
            TrackSource::Local => {
                let Some(path) = &self.path else {
                    return Err(anyhow::anyhow!("Local track missing path"));
                };
                if !PathBuf::from(&path).exists() {
                    Err(anyhow::anyhow!("Local file does not exist: {}", path))
                } else {
                    let file = File::open(&path).unwrap();
                    Ok(Decoder::try_from(file).unwrap())
                }
            }
            TrackSource::YouTube => {
                let Some(source_id) = &self.source_id else {
                    return Err(anyhow::anyhow!("YouTube track missing source ID"));
                };
                let path = youtube::get_default_download_path(source_id).await?;
                if !PathBuf::from(&path).exists() {
                    // doesn't exist, so download it
                    youtube::download_track_and_save(&self, &path).await?;
                }
                let file = File::open(path).unwrap();
                Ok(Decoder::try_from(file).unwrap())
            }
        }
    }
}

pub async fn get_library() -> anyhow::Result<&'static Library> {
    let library = LIBRARY.get();
    if let Some(conn) = library {
        Ok(conn)
    } else {
        let lib = Library::initialize().await?;
        LIBRARY
            .set(lib)
            .map_err(|_| anyhow::anyhow!("Failed to set library connection"))?;
        Ok(LIBRARY
            .get()
            .ok_or(anyhow::anyhow!("Library connection not set"))?)
    }
}

