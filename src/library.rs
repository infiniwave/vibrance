use std::{fs::File, path::PathBuf};

use anyhow::Result;
use once_cell::sync::OnceCell;
use rodio::{Decoder, Source};
use serde::{Deserialize, Serialize};
use tokio::{
    fs,
    sync::{
        broadcast::{channel, Sender as BroadcastSender},
        mpsc, oneshot,
    },
};
use turso::{Builder, Connection, Value};
use ulid::Ulid;

use crate::providers::youtube;

pub static LIBRARY: OnceCell<Library> = OnceCell::new();

#[derive(Debug, Clone)]
pub enum LibraryEvent {
    TracksAdded(Vec<Track>),
}

enum DbCommand {
    ExecuteBatch {
        sql: String,
        respond_to: oneshot::Sender<Result<(), String>>,
    },
    Cacheflush {
        respond_to: oneshot::Sender<Result<(), String>>,
    },
    Execute {
        sql: String,
        params: Vec<Value>,
        respond_to: oneshot::Sender<Result<(), String>>,
    },
    Query {
        sql: String,
        params: Vec<Value>,
        respond_to: oneshot::Sender<Result<Vec<Vec<Value>>, String>>,
    },
}

async fn row_to_values(row: &turso::Row) -> Result<Vec<Value>> {
    let mut values = Vec::new();
    let column_count = row.column_count();
    for i in 0..column_count {
        let value = row.get_value(i)?;
        values.push(value);
    }
    Ok(values)
}

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
    pub fn artists_string(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artists: Vec<Artist>,
    pub release_year: Option<i32>,
    pub album_art: Option<Vec<u8>>, // binary image data
}

impl Album {
    pub fn new(
        title: String,
        artists: Vec<Artist>,
        release_year: Option<i32>,
        album_art: Option<Vec<u8>>,
    ) -> Self {
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
    db_sender: mpsc::Sender<DbCommand>,
    event_sender: BroadcastSender<LibraryEvent>,
}

async fn db_worker(mut rx: mpsc::Receiver<DbCommand>, connection: Connection) {
    'outer: while let Some(cmd) = rx.recv().await {
        match cmd {
            DbCommand::ExecuteBatch { sql, respond_to } => {
                let result = connection
                    .execute_batch(&sql)
                    .await
                    .map_err(|e| e.to_string());
                let _ = respond_to.send(result);
            }
            DbCommand::Cacheflush { respond_to } => {
                let result = connection.cacheflush().map_err(|e| e.to_string());
                let _ = respond_to.send(result);
            }
            // these are a bit ugly and could use some macros 
            // but turso's execute/query methods don't take slices directly
            DbCommand::Execute {
                sql,
                params,
                respond_to,
            } => {
                let result = match params.len() {
                    0 => connection.execute(&sql, ()).await,
                    1 => connection.execute(&sql, [params[0].clone()]).await,
                    2 => connection.execute(&sql, [params[0].clone(), params[1].clone()]).await,
                    3 => connection.execute(&sql, [params[0].clone(), params[1].clone(), params[2].clone()]).await,
                    4 => connection.execute(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone()]).await,
                    5 => connection.execute(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone(), params[4].clone()]).await,
                    6 => connection.execute(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone(), params[4].clone(), params[5].clone()]).await,
                    7 => connection.execute(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone(), params[4].clone(), params[5].clone(), params[6].clone()]).await,
                    8 => connection.execute(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone(), params[4].clone(), params[5].clone(), params[6].clone(), params[7].clone()]).await,
                    _ => {
                        let _ = respond_to.send(Err("Too many parameters".to_string()));
                        continue;
                    }
                };
                let _ = respond_to.send(result.map(|_| ()).map_err(|e| e.to_string()));
            }
            DbCommand::Query {
                sql,
                params,
                respond_to,
            } => {
                let result = match params.len() {
                    0 => connection.query(&sql, ()).await,
                    1 => connection.query(&sql, [params[0].clone()]).await,
                    2 => connection.query(&sql, [params[0].clone(), params[1].clone()]).await,
                    3 => connection.query(&sql, [params[0].clone(), params[1].clone(), params[2].clone()]).await,
                    4 => connection.query(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone()]).await,
                    5 => connection.query(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone(), params[4].clone()]).await,
                    6 => connection.query(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone(), params[4].clone(), params[5].clone()]).await,
                    7 => connection.query(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone(), params[4].clone(), params[5].clone(), params[6].clone()]).await,
                    8 => connection.query(&sql, [params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone(), params[4].clone(), params[5].clone(), params[6].clone(), params[7].clone()]).await,
                    _ => {
                        let _ = respond_to.send(Err("Too many parameters".to_string()));
                        continue;
                    }
                };
                
                match result {
                    Ok(mut rows) => {
                        let mut all_rows = Vec::new();
                        while let Ok(Some(row)) = rows.next().await {
                            let values = row_to_values(&row).await;
                            let values = match values {
                                Ok(v) => v,
                                Err(e) => {
                                    let _ = respond_to.send(Err(e.to_string()));
                                    continue 'outer;
                                }
                            };
                            all_rows.push(values);
                        }
                        let _ = respond_to.send(Ok(all_rows));
                    }
                    Err(e) => {
                        let _ = respond_to.send(Err(e.to_string()));
                    }
                }
            }
        }
    }
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
        let connection =
            Builder::new_local(db_path.to_str().ok_or(anyhow::anyhow!("Invalid path"))?)
                .build()
                .await?
                .connect()?;
        let (tx, rx) = mpsc::channel::<DbCommand>(100);
        tokio::spawn(db_worker(rx, connection));
        let (respond_tx, respond_rx) = oneshot::channel();
        tx.send(DbCommand::ExecuteBatch {
            sql: CREATE_DB.to_string(),
            respond_to: respond_tx,
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        respond_rx
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Failed to create database: {}", e))?;
        let (event_sender, _) = channel::<LibraryEvent>(25);
        Ok(Self {
            db_sender: tx,
            event_sender,
        })
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LibraryEvent> {
        self.event_sender.subscribe()
    }

    pub async fn write(&self) -> anyhow::Result<()> {
        let (respond_tx, respond_rx) = oneshot::channel();
        self.db_sender
            .send(DbCommand::Cacheflush {
                respond_to: respond_tx,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        respond_rx
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Cacheflush failed: {}", e))?;
        Ok(())
    }

    async fn execute(&self, sql: &str, params: Vec<Value>) -> anyhow::Result<()> {
        let (respond_tx, respond_rx) = oneshot::channel();
        self.db_sender
            .send(DbCommand::Execute {
                sql: sql.to_string(),
                params,
                respond_to: respond_tx,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        respond_rx
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Execute failed: {}", e))?;
        Ok(())
    }

    async fn query(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Vec<Value>>> {
        let (respond_tx, respond_rx) = oneshot::channel();
        self.db_sender
            .send(DbCommand::Query {
                sql: sql.to_string(),
                params,
                respond_to: respond_tx,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        
        respond_rx
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Query failed: {}", e))
    }

    fn get_string(value: &Value) -> Result<String> {
        match value {
            Value::Text(s) => Ok(s.clone()),
            _ => Err(anyhow::anyhow!("Expected text value")),
        }
    }

    fn get_optional_string(value: &Value) -> Option<String> {
        match value {
            Value::Text(s) => Some(s.clone()),
            Value::Null => None,
            _ => None,
        }
    }

    fn get_i64(value: &Value) -> Result<i64> {
        match value {
            Value::Integer(i) => Ok(*i),
            _ => Err(anyhow::anyhow!("Expected integer value")),
        }
    }

    fn get_optional_i64(value: &Value) -> Option<i64> {
        match value {
            Value::Integer(i) => Some(*i),
            Value::Null => None,
            _ => None,
        }
    }

    fn get_f64(value: &Value) -> Result<f64> {
        match value {
            Value::Real(f) => Ok(*f),
            _ => Err(anyhow::anyhow!("Expected real value")),
        }
    }

    fn get_optional_blob(value: &Value) -> Option<Vec<u8>> {
        match value {
            Value::Blob(b) => Some(b.clone()),
            Value::Null => None,
            _ => None,
        }
    }

    /// Add an artist to the library, returns the artist (with existing id if already exists)
    pub async fn add_artist(&self, artist: &Artist) -> anyhow::Result<Artist> {
        if let Some(existing) = self.find_artist_by_name(&artist.name).await? {
            return Ok(existing);
        }

        self.execute(
            "INSERT INTO artists (id, name) VALUES (?, ?)",
            vec![
                Value::Text(artist.id.clone()),
                Value::Text(artist.name.clone()),
            ],
        )
        .await?;

        Ok(artist.clone())
    }

    /// Find an artist by ID
    pub async fn find_artist_by_id(&self, id: &str) -> anyhow::Result<Option<Artist>> {
        let rows = self
            .query(
                "SELECT id, name FROM artists WHERE id = ?",
                vec![Value::Text(id.to_string())],
            )
            .await?;

        if let Some(row) = rows.first() {
            let id = Self::get_string(&row[0])?;
            let name = Self::get_string(&row[1])?;
            Ok(Some(Artist { id, name }))
        } else {
            Ok(None)
        }
    }

    /// Find an artist by name
    pub async fn find_artist_by_name(&self, name: &str) -> anyhow::Result<Option<Artist>> {
        let rows = self
            .query(
                "SELECT id, name FROM artists WHERE name = ?",
                vec![Value::Text(name.to_string())],
            )
            .await?;

        if let Some(row) = rows.first() {
            let id = Self::get_string(&row[0])?;
            let name = Self::get_string(&row[1])?;
            Ok(Some(Artist { id, name }))
        } else {
            Ok(None)
        }
    }

    /// Get all artists
    pub async fn all_artists(&self) -> anyhow::Result<Vec<Artist>> {
        let rows = self
            .query("SELECT id, name FROM artists ORDER BY name", vec![])
            .await?;

        let mut artists = Vec::new();
        for row in rows {
            let id = Self::get_string(&row[0])?;
            let name = Self::get_string(&row[1])?;
            artists.push(Artist { id, name });
        }
        Ok(artists)
    }

    /// Add an album to the library
    pub async fn add_album(&self, album: &Album) -> anyhow::Result<Album> {
        if let Some(existing) = self.find_album_by_title(&album.title).await? {
            return Ok(existing);
        }

        self.execute(
            "INSERT INTO albums (id, title, release_year, album_art) VALUES (?, ?, ?, ?)",
            vec![
                Value::Text(album.id.clone()),
                Value::Text(album.title.clone()),
                album
                    .release_year
                    .map(|y| Value::Integer(y as i64))
                    .unwrap_or(Value::Null),
                album
                    .album_art
                    .clone()
                    .map(Value::Blob)
                    .unwrap_or(Value::Null),
            ],
        )
        .await?;

        for artist in &album.artists {
            let artist = self.add_artist(artist).await?;
            self.execute(
                "INSERT OR IGNORE INTO album_artists (album_id, artist_id) VALUES (?, ?)",
                vec![Value::Text(album.id.clone()), Value::Text(artist.id)],
            )
            .await?;
        }

        Ok(album.clone())
    }

    /// Find an album by ID
    pub async fn find_album_by_id(&self, id: &str) -> anyhow::Result<Option<Album>> {
        let rows = self
            .query(
                "SELECT id, title, release_year, album_art FROM albums WHERE id = ?",
                vec![Value::Text(id.to_string())],
            )
            .await?;

        if let Some(row) = rows.first() {
            let id = Self::get_string(&row[0])?;
            let title = Self::get_string(&row[1])?;
            let release_year = Self::get_optional_i64(&row[2]).map(|y| y as i32);
            let album_art = Self::get_optional_blob(&row[3]);

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
        let rows = self
            .query(
                "SELECT id, title, release_year, album_art FROM albums WHERE title = ?",
                vec![Value::Text(title.to_string())],
            )
            .await?;

        if let Some(row) = rows.first() {
            let id = Self::get_string(&row[0])?;
            let title = Self::get_string(&row[1])?;
            let release_year = Self::get_optional_i64(&row[2]).map(|y| y as i32);
            let album_art = Self::get_optional_blob(&row[3]);

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
        let rows = self
            .query(
                "SELECT a.id, a.name FROM artists a 
                 INNER JOIN album_artists aa ON a.id = aa.artist_id 
                 WHERE aa.album_id = ?",
                vec![Value::Text(album_id.to_string())],
            )
            .await?;

        let mut artists = Vec::new();
        for row in rows {
            let id = Self::get_string(&row[0])?;
            let name = Self::get_string(&row[1])?;
            artists.push(Artist { id, name });
        }
        Ok(artists)
    }

    /// Get all albums
    pub async fn all_albums(&self) -> anyhow::Result<Vec<Album>> {
        let rows = self
            .query(
                "SELECT id, title, release_year, album_art FROM albums ORDER BY title",
                vec![],
            )
            .await?;

        let mut albums = Vec::new();
        for row in rows {
            let id = Self::get_string(&row[0])?;
            let title = Self::get_string(&row[1])?;
            let release_year = Self::get_optional_i64(&row[2]).map(|y| y as i32);
            let album_art = Self::get_optional_blob(&row[3]);

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
        let track = self.add_track_internal(track).await?;
        let _ = self.event_sender.send(LibraryEvent::TracksAdded(vec![track.clone()]));
        Ok(track)
    }

    async fn add_track_internal(&self, track: &Track) -> anyhow::Result<Track> {
        let album = self.add_album(&track.album).await?;

        let mut artists_with_ids = Vec::new();
        for artist in &track.artists {
            let artist = self.add_artist(artist).await?;
            artists_with_ids.push(artist);
        }

        self.execute(
            "INSERT INTO tracks (id, title, album_id, duration, path, source, source_id, track_number) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                Value::Text(track.id.clone()),
                Value::Text(track.title.clone()),
                Value::Text(album.id.clone()),
                Value::Real(track.duration),
                track.path.clone().map(Value::Text).unwrap_or(Value::Null),
                Value::Text(track.source.as_str().to_string()),
                track
                    .source_id
                    .clone()
                    .map(Value::Text)
                    .unwrap_or(Value::Null),
                track
                    .track_number
                    .map(|n| Value::Integer(n as i64))
                    .unwrap_or(Value::Null),
            ],
        )
        .await?;

        for artist in &artists_with_ids {
            self.execute(
                "INSERT OR IGNORE INTO track_artists (track_id, artist_id) VALUES (?, ?)",
                vec![
                    Value::Text(track.id.clone()),
                    Value::Text(artist.id.clone()),
                ],
            )
            .await?;
        }

        let mut result = track.clone();
        result.artists = artists_with_ids;
        Ok(result)
    }

    /// Add multiple tracks to the library
    pub async fn add_tracks(&self, tracks: &[Track]) -> anyhow::Result<Vec<Track>> {
        let mut results = Vec::new();
        for track in tracks {
            results.push(self.add_track_internal(track).await?);
        }
        if !results.is_empty() {
            let _ = self.event_sender.send(LibraryEvent::TracksAdded(results.clone()));
        }
        Ok(results)
    }

    /// Find a track by ID
    pub async fn find_track_by_id(&self, id: &str) -> anyhow::Result<Option<Track>> {
        let rows = self
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE id = ?",
                vec![Value::Text(id.to_string())],
            )
            .await?;

        if let Some(row) = rows.first() {
            self.row_to_track(&row).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Find a track by source ID
    pub async fn find_track_by_source(
        &self,
        source: TrackSource,
        id: &str,
    ) -> anyhow::Result<Option<Track>> {
        let rows = self
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE source = ? AND source_id = ?",
                vec![
                    Value::Text(source.as_str().to_string()),
                    Value::Text(id.to_string()),
                ],
            )
            .await?;

        if let Some(row) = rows.first() {
            self.row_to_track(&row).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Find tracks by source
    pub async fn find_tracks_by_source(&self, source: TrackSource) -> anyhow::Result<Vec<Track>> {
        let rows = self
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE source = ?",
                vec![Value::Text(source.as_str().to_string())],
            )
            .await?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Get all tracks in the library
    pub async fn all_tracks(&self) -> anyhow::Result<Vec<Track>> {
        let rows = self
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks ORDER BY title",
                vec![],
            )
            .await?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Get all tracks not in any playlist
    pub async fn all_unorganized_tracks(&self) -> anyhow::Result<Vec<Track>> {
        let rows = self
            .query(
                "SELECT t.id, t.title, t.album_id, t.duration, t.path, t.source, t.source_id, t.track_number 
                 FROM tracks t
                 LEFT JOIN playlist_tracks pt ON t.id = pt.track_id
                 WHERE pt.track_id IS NULL
                 ORDER BY t.title",
                vec![],
            )
            .await?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Get artists for a track
    async fn get_track_artists(&self, track_id: &str) -> anyhow::Result<Vec<Artist>> {
        let rows = self
            .query(
                "SELECT a.id, a.name FROM artists a 
                 INNER JOIN track_artists ta ON a.id = ta.artist_id 
                 WHERE ta.track_id = ?",
                vec![Value::Text(track_id.to_string())],
            )
            .await?;

        let mut artists = Vec::new();
        for row in rows {
            let id = Self::get_string(&row[0])?;
            let name = Self::get_string(&row[1])?;
            artists.push(Artist { id, name });
        }
        Ok(artists)
    }

    /// Convert a database row to a Track
    async fn row_to_track(&self, row: &Vec<Value>) -> anyhow::Result<Track> {
        let id = Self::get_string(&row[0])?;
        let title = Self::get_string(&row[1])?;
        let album_id = Self::get_string(&row[2])?;
        let duration = Self::get_f64(&row[3])?;
        let path = Self::get_optional_string(&row[4]);
        let source_str = Self::get_string(&row[5])?;
        let source_id = Self::get_optional_string(&row[6]);
        let track_number = Self::get_optional_i64(&row[7]).map(|n| n as i32);

        let source = TrackSource::from_str(&source_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid track source: {}", source_str))?;

        let artists = self.get_track_artists(&id).await?;
        let album = self
            .find_album_by_id(&album_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Album not found for track: {}", id))?;

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
        self.execute(
            "DELETE FROM tracks WHERE id = ?",
            vec![Value::Text(id.to_string())],
        )
        .await?;
        Ok(())
    }

    /// Create a new playlist
    pub async fn create_playlist(&self, playlist: &Playlist) -> anyhow::Result<Playlist> {
        self.execute(
            "INSERT INTO playlists (id, name, description) VALUES (?, ?, ?)",
            vec![
                Value::Text(playlist.id.clone()),
                Value::Text(playlist.name.clone()),
                playlist
                    .description
                    .clone()
                    .map(Value::Text)
                    .unwrap_or(Value::Null),
            ],
        )
        .await?;

        Ok(playlist.clone())
    }

    /// Find a playlist by ID
    pub async fn find_playlist_by_id(&self, id: &str) -> anyhow::Result<Option<Playlist>> {
        let rows = self
            .query(
                "SELECT id, name, description FROM playlists WHERE id = ?",
                vec![Value::Text(id.to_string())],
            )
            .await?;

        if let Some(row) = rows.first() {
            let id = Self::get_string(&row[0])?;
            let name = Self::get_string(&row[1])?;
            let description = Self::get_optional_string(&row[2]);

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
        let rows = self
            .query(
                "SELECT id, name, description FROM playlists ORDER BY name",
                vec![],
            )
            .await?;

        let mut playlists = Vec::new();
        for row in rows {
            let id = Self::get_string(&row[0])?;
            let name = Self::get_string(&row[1])?;
            let description = Self::get_optional_string(&row[2]);

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
        let rows = self
            .query(
                "SELECT t.id, t.title, t.album_id, t.duration, t.path, t.source, t.source_id, t.track_number 
                 FROM tracks t
                 INNER JOIN playlist_tracks pt ON t.id = pt.track_id
                 WHERE pt.playlist_id = ?
                 ORDER BY pt.position",
                vec![Value::Text(playlist_id.to_string())],
            )
            .await?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Add a track to a playlist
    pub async fn add_track_to_playlist(
        &self,
        playlist_id: &str,
        track_id: &str,
    ) -> anyhow::Result<()> {
        let rows = self
            .query(
                "SELECT COALESCE(MAX(position), 0) + 1 FROM playlist_tracks WHERE playlist_id = ?",
                vec![Value::Text(playlist_id.to_string())],
            )
            .await?;

        let position: i64 = if let Some(row) = rows.first() {
            Self::get_i64(&row[0]).unwrap_or(1)
        } else {
            1
        };

        self.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
            vec![
                Value::Text(playlist_id.to_string()),
                Value::Text(track_id.to_string()),
                Value::Integer(position),
            ],
        )
        .await?;

        Ok(())
    }

    /// Remove a track from a playlist
    pub async fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        track_id: &str,
    ) -> anyhow::Result<()> {
        self.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?",
            vec![
                Value::Text(playlist_id.to_string()),
                Value::Text(track_id.to_string()),
            ],
        )
        .await?;

        Ok(())
    }

    /// Delete a playlist
    pub async fn delete_playlist(&self, id: &str) -> anyhow::Result<()> {
        self.execute(
            "DELETE FROM playlists WHERE id = ?",
            vec![Value::Text(id.to_string())],
        )
        .await?;
        Ok(())
    }

    /// Update a playlist
    pub async fn update_playlist(&self, playlist: &Playlist) -> anyhow::Result<()> {
        self.execute(
            "UPDATE playlists SET name = ?, description = ? WHERE id = ?",
            vec![
                Value::Text(playlist.name.clone()),
                playlist
                    .description
                    .clone()
                    .map(Value::Text)
                    .unwrap_or(Value::Null),
                Value::Text(playlist.id.clone()),
            ],
        )
        .await?;
        Ok(())
    }

    /// Search tracks by title
    pub async fn search_tracks(&self, query: &str) -> anyhow::Result<Vec<Track>> {
        let pattern = format!("%{}%", query);
        let rows = self
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE title LIKE ? ORDER BY title",
                vec![Value::Text(pattern)],
            )
            .await?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Search artists by name
    pub async fn search_artists(&self, query: &str) -> anyhow::Result<Vec<Artist>> {
        let pattern = format!("%{}%", query);
        let rows = self
            .query(
                "SELECT id, name FROM artists WHERE name LIKE ? ORDER BY name",
                vec![Value::Text(pattern)],
            )
            .await?;

        let mut artists = Vec::new();
        for row in rows {
            let id = Self::get_string(&row[0])?;
            let name = Self::get_string(&row[1])?;
            artists.push(Artist { id, name });
        }
        Ok(artists)
    }

    /// Search albums by title
    pub async fn search_albums(&self, query: &str) -> anyhow::Result<Vec<Album>> {
        let pattern = format!("%{}%", query);
        let rows = self
            .query(
                "SELECT id, title, release_year, album_art FROM albums WHERE title LIKE ? ORDER BY title",
                vec![Value::Text(pattern)],
            )
            .await?;

        let mut albums = Vec::new();
        for row in rows {
            let id = Self::get_string(&row[0])?;
            let title = Self::get_string(&row[1])?;
            let release_year = Self::get_optional_i64(&row[2]).map(|y| y as i32);
            let album_art = Self::get_optional_blob(&row[3]);

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
        let rows = self
            .query(
                "SELECT id, title, album_id, duration, path, source, source_id, track_number 
                 FROM tracks WHERE album_id = ? ORDER BY track_number, title",
                vec![Value::Text(album_id.to_string())],
            )
            .await?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }

    /// Get tracks by artist
    pub async fn get_tracks_by_artist(&self, artist_id: &str) -> anyhow::Result<Vec<Track>> {
        let rows = self
            .query(
                "SELECT t.id, t.title, t.album_id, t.duration, t.path, t.source, t.source_id, t.track_number 
                 FROM tracks t
                 INNER JOIN track_artists ta ON t.id = ta.track_id
                 WHERE ta.artist_id = ?
                 ORDER BY t.title",
                vec![Value::Text(artist_id.to_string())],
            )
            .await?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(self.row_to_track(&row).await?);
        }
        Ok(tracks)
    }
}

impl Track {
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
