use std::{fs::File, io::BufReader, path::PathBuf, sync::{mpsc::channel, Arc, Mutex}, thread, time::Duration};

use anyhow::Result;
use chrono::Utc;
use lofty::{file::{AudioFile, TaggedFileExt}, probe::Probe, tag::ItemKey};
use rodio::{Decoder, OutputStream, Sink, Source};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::PREFERENCES;

#[derive(Debug, Clone)]
pub struct Player {
    pub current_track: Option<Track>,
    pub queue: Vec<Track>,
    pub in_cmd: std::sync::mpsc::Sender<PlayerCommand>,
    pub out_evt: Arc<Mutex<std::sync::mpsc::Receiver<PlayerEvent>>>,
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
        let (in_cmd, out_cmd) = channel::<PlayerCommand>();
        let (in_evt, out_evt) = channel::<PlayerEvent>();
        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();
            let mut current_duration = 0.0;
            let mut global_volume = volume; 
            sink.set_volume(global_volume);
            let mut last_progress_updated: i64 = 0;
            loop {
                if let Ok(cmd) = out_cmd.try_recv() {
                    match cmd {
                        PlayerCommand::Play(track) => {            
                            in_evt.send(PlayerEvent::TrackLoaded(track.clone())).unwrap_or_else(|_| {
                                println!("Failed to send track loaded event");
                            });
                            let path = match track.sources.first() {
                                Some(TrackSource::File(path)) => {
                                    if !PathBuf::from(path).exists() {
                                        println!("Track file does not exist: {}", path);
                                        continue;
                                    }
                                    path.clone()
                                },
                                _ => {
                                    println!("No valid track source found");
                                    continue;
                                }
                            };
                            let file = File::open(&path).unwrap();
                            let source = Decoder::new(BufReader::new(file)).unwrap();
                            current_duration = source.total_duration().map(|d| d.as_secs_f32()).unwrap_or(0.0);
                            println!("Playing track: {}", current_duration);
                            sink.append(source);
                            sink.play();
                            in_evt.send(PlayerEvent::Resumed).unwrap_or_else(|_| {
                                println!("Failed to send unpause event");
                            });
                        },
                        PlayerCommand::Seek(pos) => {
                            sink.try_seek(Duration::from_secs_f32(pos * current_duration)).unwrap_or_else(|_| {
                                println!("Failed to seek to position: {}", pos);
                            });
                        },
                        PlayerCommand::Stop => {
                            sink.clear();
                            current_duration = 0.0;
                        },
                        PlayerCommand::Pause => {
                            if sink.is_paused() {
                                sink.play();
                                in_evt.send(PlayerEvent::Resumed).unwrap_or_else(|_| {
                                    println!("Failed to send unpause event");
                                });
                            } else {
                                sink.pause();
                                let position = sink.get_pos().as_secs_f32() / current_duration;
                                in_evt.send(PlayerEvent::Progress(position.into())).unwrap();
                                in_evt.send(PlayerEvent::Paused).unwrap_or_else(|_| {
                                    println!("Failed to send pause event");
                                });
                            }
                        },
                        PlayerCommand::SetVolume(volume) => {
                            sink.set_volume(volume);
                            global_volume = volume;
                            let mut preferences = PREFERENCES.get().expect("Preferences not initialized")
                                .lock()
                                .expect("Failed to lock preferences mutex");
                            preferences.volume = volume;
                            drop(preferences);
                        },
                        PlayerCommand::SetMuted(muted) => {
                            if muted {
                                sink.set_volume(0.0);
                            } else {
                                sink.set_volume(global_volume); // Default volume, adjust as needed
                            }
                        },
                    }
                }
                if sink.empty() && current_duration > 0.0 {
                    current_duration = 0.0;
                    in_evt.send(PlayerEvent::End).unwrap_or_else(|_| {
                        println!("Failed to send end event");
                    });
                } else if !sink.empty() && !sink.is_paused() {
                    if Utc::now().timestamp_millis() - last_progress_updated < 100 {
                        thread::sleep(std::time::Duration::from_millis(100));
                        continue; // Skip if the last update was too recent
                    }
                    // Emit progress event based on current position
                    let position = sink.get_pos().as_secs_f32() / current_duration;
                    in_evt.send(PlayerEvent::Progress(position.into())).unwrap();
                    last_progress_updated = Utc::now().timestamp_millis();
                }
                thread::sleep(std::time::Duration::from_millis(100));
            }
        });
        Player {
            current_track: None,
            queue: Vec::new(),
            in_cmd,
            out_evt: Arc::new(Mutex::new(out_evt)),
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
        self.in_cmd.send(cmd).expect("Failed to send set volume command");
        println!("Volume set to: {}", volume);
    }

    pub fn set_muted(&mut self, muted: bool) {
        let cmd = PlayerCommand::SetMuted(muted);
        self.in_cmd.send(cmd).expect("Failed to send set muted command");
        println!("Muted set to: {}", muted);
    }

    pub fn out_evt_receiver(&self) -> Arc<Mutex<std::sync::mpsc::Receiver<PlayerEvent>>> {
        self.out_evt.clone()
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
        let mut artists = tag.map_or_else(Vec::new, |t| t.get_strings(&ItemKey::TrackArtists).map(String::from).collect());
        if artists.is_empty() {
            let artist = tag.and_then(|t| t.get_string(&ItemKey::TrackArtist).map(String::from));
            if let Some(artist) = artist {
                artists.push(artist);
            }
        }
        Ok(Track {
            id,
            title: tag.and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from)),
            artists,
            album: tag.and_then(|t| t.get_string(&ItemKey::AlbumTitle).map(String::from)),
            duration: properties.duration().as_secs_f64(),
            sources: vec![TrackSource::File(path.to_string_lossy().to_string())],
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
    pub duration: f64,
    pub sources: Vec<TrackSource>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum TrackSource {
    File(String)
}
