use std::{fs::File, path::PathBuf, time::Duration};

use anyhow::Result;
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::Utc;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::ItemKey,
};
use once_cell::sync::OnceCell;
use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{
        Mutex,
        broadcast::{Receiver, Sender, channel},
        mpsc::{UnboundedSender, unbounded_channel},
    },
    task, time,
};
use ulid::Ulid;

use crate::preferences::PREFERENCES;

/// Global player instance
pub static PLAYER: OnceCell<Mutex<Player>> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct Player {
    pub current_track: Option<Track>,
    pub queue: Vec<Track>,
    pub in_cmd: UnboundedSender<PlayerCommand>,
    pub in_evt: Sender<PlayerEvent>,
}

pub enum PlayerCommand {
    Play(Track),
    Pause,
    Stop,
    Seek(f32),
    SetVolume(f32),
    SetMuted(bool),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackLoaded(Track),
    Progress(f64),
    Paused,
    Resumed,
    End,
    // QueueEnd,
    // TrackChanged(Option<Track>),
    // Error(String),
}

impl Player {
    pub fn new(volume: f32) -> Self {
        let (in_cmd, mut out_cmd) = unbounded_channel::<PlayerCommand>();
        let (in_evt, out_evt) = channel::<PlayerEvent>(25);
        let in_evt_clone = in_evt.clone();
        task::spawn(async move {
            let stream = OutputStreamBuilder::open_default_stream().unwrap();
            let sink = Sink::connect_new(stream.mixer());
            let mut current_duration = 0.0;
            let mut global_volume = volume;
            sink.set_volume(global_volume);
            let mut last_progress_updated: i64 = 0;
            loop {
                if let Ok(cmd) = out_cmd.try_recv() {
                    match cmd {
                        PlayerCommand::Play(track) => {
                            in_evt_clone
                                .send(PlayerEvent::TrackLoaded(track.clone()))
                                .unwrap_or_else(|_| {
                                    println!("Failed to send track loaded event");
                                    0
                                });
                            let Some(path) = track.path else {
                                println!("Track path is None, skipping playback");
                                continue;
                            };
                            if !PathBuf::from(&path).exists() {
                                println!("Track file does not exist: {}", path);
                                continue;
                            }
                            let file = File::open(&path).unwrap();
                            let source = Decoder::try_from(file).unwrap();
                            current_duration = source
                                .total_duration()
                                .map(|d| d.as_secs_f32())
                                .unwrap_or(0.0);
                            println!("Playing track: {}", current_duration);
                            sink.append(source);
                            sink.play();
                            in_evt_clone.send(PlayerEvent::Resumed).unwrap_or_else(|_| {
                                println!("Failed to send unpause event");
                                0
                            });
                        }
                        PlayerCommand::Seek(pos) => {
                            println!(
                                "Seeking to position: {:?} of {}",
                                Duration::from_secs_f32(pos * current_duration),
                                current_duration
                            );
                            sink.try_seek(Duration::from_secs_f32(pos * current_duration))
                                .unwrap_or_else(|e| {
                                    println!("Failed to seek to position: {:?}", e);
                                });
                        }
                        PlayerCommand::Stop => {
                            sink.clear();
                            current_duration = 0.0;
                        }
                        PlayerCommand::Pause => {
                            if sink.is_paused() {
                                sink.play();
                                in_evt_clone.send(PlayerEvent::Resumed).unwrap_or_else(|_| {
                                    println!("Failed to send unpause event");
                                    0
                                });
                            } else {
                                sink.pause();
                                let position = sink.get_pos().as_secs_f32() / current_duration;
                                in_evt_clone
                                    .send(PlayerEvent::Progress(position.into()))
                                    .unwrap();
                                in_evt_clone.send(PlayerEvent::Paused).unwrap_or_else(|_| {
                                    println!("Failed to send pause event");
                                    0
                                });
                            }
                        }
                        PlayerCommand::SetVolume(volume) => {
                            sink.set_volume(volume);
                            global_volume = volume;
                            let mut preferences = PREFERENCES
                                .get()
                                .expect("Preferences not initialized")
                                .write()
                                .await;
                            preferences.volume = volume;
                            drop(preferences);
                        }
                        PlayerCommand::SetMuted(muted) => {
                            if muted {
                                sink.set_volume(0.0);
                            } else {
                                sink.set_volume(global_volume); // Default volume, adjust as needed
                            }
                        }
                    }
                }
                if sink.empty() && current_duration > 0.0 {
                    current_duration = 0.0;
                    in_evt_clone.send(PlayerEvent::End).unwrap_or_else(|_| {
                        println!("Failed to send end event");
                        0
                    });
                } else if !sink.empty() && !sink.is_paused() {
                    if Utc::now().timestamp_millis() - last_progress_updated < 100 {
                        time::sleep(std::time::Duration::from_millis(100)).await;
                        continue; // Skip if the last update was too recent
                    }
                    // Emit progress event based on current position
                    let position = sink.get_pos().as_secs_f32() / current_duration;
                    in_evt_clone
                        .send(PlayerEvent::Progress(position.into()))
                        .unwrap();
                    last_progress_updated = Utc::now().timestamp_millis();
                }
                time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
        Player {
            current_track: None,
            queue: Vec::new(),
            in_cmd,
            in_evt,
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.queue.push(track);
    }

    pub fn play(&mut self) {
        if self.queue.is_empty() {
            println!("No tracks in the queue to play.");
            return;
        }
        if self.current_track.is_none() {
            self.current_track = Some(self.queue.remove(0));
        } else {
            self.in_cmd.send(PlayerCommand::Stop).unwrap_or_else(|e| {
                panic!("Failed to send stop command: {}", e);
            });
            // .expect("Failed to send stop command");
            self.current_track = Some(self.queue.remove(0));
        }
        if let Some(track) = &self.current_track {
            let cmd = PlayerCommand::Play(track.clone());
            self.in_cmd.send(cmd).expect("Failed to send play command");
            println!("Playing track: {:?}", track);
        } else {
            println!("No track is currently set to play.");
        }
    }

    pub fn seek(&mut self, pos: f32) {
        if self.current_track.is_some() {
            let cmd = PlayerCommand::Seek(pos);
            self.in_cmd.send(cmd).expect("Failed to send seek command");
            println!("Seeking to position: {}", pos);
        } else {
            println!("No track is currently set to seek.");
        }
    }

    pub fn pause(&mut self) {
        let cmd = PlayerCommand::Pause;
        self.in_cmd.send(cmd).expect("Failed to send pause command");
        println!("Pause command sent.");
    }

    pub fn set_volume(&mut self, volume: f32) {
        let cmd = PlayerCommand::SetVolume(volume);
        self.in_cmd
            .send(cmd)
            .expect("Failed to send set volume command");
        println!("Volume set to: {}", volume);
    }

    pub fn set_muted(&mut self, muted: bool) {
        let cmd = PlayerCommand::SetMuted(muted);
        self.in_cmd
            .send(cmd)
            .expect("Failed to send set muted command");
        println!("Muted set to: {}", muted);
    }

    pub fn out_evt_receiver(&self) -> Receiver<PlayerEvent> {
        self.in_evt.subscribe()
    }

    pub fn resolve_track(&self, path: String) -> Result<Track> {
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
            .map(|p| p.data())
            .map(|d| BASE64_STANDARD.encode(d));
        Ok(Track {
            id,
            title: tag.and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from)),
            artists,
            album: tag.and_then(|t| t.get_string(&ItemKey::AlbumTitle).map(String::from)),
            album_art,
            duration: properties.duration().as_secs_f64(),
            path: Some(path.to_string_lossy().to_string()),
            yt_id: None,
        })
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.current_track = None;
        let cmd = PlayerCommand::Stop;
        self.in_cmd.send(cmd).expect("Failed to send stop command");
        println!("Queue cleared and playback stopped.");
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Track {
    pub id: String,
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_art: Option<String>, // base64 encoded image data
    pub duration: f64,
    pub path: Option<String>,
    pub yt_id: Option<String>,
}
