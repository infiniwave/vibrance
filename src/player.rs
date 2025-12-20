use std::{fs::File, path::PathBuf, time::Duration};

use chrono::Utc;
use once_cell::sync::OnceCell;
use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
use tokio::{
    sync::{
        broadcast::{Receiver, Sender, channel},
        mpsc::{UnboundedSender, unbounded_channel},
    },
    task, time,
};

use crate::{library::Track, preferences::PREFERENCES};

pub static PLAYER: OnceCell<Player> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct Player {
    pub in_cmd: UnboundedSender<PlayerCommand>,
    pub in_evt: Sender<PlayerEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Repeat {
    Off,
    All,
    One,
}

pub enum PlayerCommand {
    AddTrack(Track),
    RemoveTrack(usize),
    ClearQueue,
    SetRepeat(Repeat),
    Play,
    Pause,
    Stop,
    Seek(f32),
    SetVolume(f32),
    SetMuted(bool),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackLoaded(Track),
    Progress(f32, f32),
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
            let mut pending_volume_save: Option<f32> = None;
            let mut last_volume_change: i64 = 0;
            let mut current_track: Option<Track> = None;
            let mut queue: Vec<Track> = Vec::new();
            let mut repeat_mode = Repeat::Off;
            loop {
                // drain all pending commands
                let mut commands = Vec::new();
                while let Ok(cmd) = out_cmd.try_recv() {
                    commands.push(cmd);
                }
                // keep latest of each to reduce lag
                let mut latest_volume: Option<f32> = None;
                let mut latest_seek: Option<f32> = None;
                let mut latest_muted: Option<bool> = None;
                let mut filtered_commands = Vec::new();
                for cmd in commands {
                    match cmd {
                        PlayerCommand::SetVolume(v) => latest_volume = Some(v),
                        PlayerCommand::Seek(s) => latest_seek = Some(s),
                        PlayerCommand::SetMuted(m) => latest_muted = Some(m),
                        other => filtered_commands.push(other),
                    }
                }
                if let Some(v) = latest_volume {
                    filtered_commands.push(PlayerCommand::SetVolume(v));
                }
                if let Some(s) = latest_seek {
                    filtered_commands.push(PlayerCommand::Seek(s));
                }
                if let Some(m) = latest_muted {
                    filtered_commands.push(PlayerCommand::SetMuted(m));
                }
                for cmd in filtered_commands {
                    match cmd {
                        PlayerCommand::SetRepeat(mode) => {
                            repeat_mode = mode;
                        }
                        PlayerCommand::AddTrack(track) => {
                            queue.push(track);
                        }
                        PlayerCommand::RemoveTrack(index) => {
                            if index < queue.len() {
                                queue.remove(index);
                            }
                        }
                        PlayerCommand::ClearQueue => {
                            queue.clear();
                            current_track = None;
                            sink.clear();
                            current_duration = 0.0;
                        }
                        PlayerCommand::Play => {
                            if queue.is_empty() {
                                println!("No tracks in the queue to play.");
                                continue;
                            }
                            if let Some(current) = &current_track {
                                sink.clear();
                                current_duration = 0.0;
                                if repeat_mode == Repeat::All {
                                    queue.push(current.clone());
                                }
                            }
                            current_track = Some(queue.remove(0));
                            if let Some(ref track) = current_track {
                                in_evt_clone
                                    .send(PlayerEvent::TrackLoaded(track.clone()))
                                    .unwrap_or_else(|_| {
                                        println!("Failed to send track loaded event");
                                        0
                                    });
                                let source = track.load().await;
                                let source = match source {
                                    Ok(source) => source,
                                    Err(e) => {
                                        println!("Failed to load track source: {:?}", e);
                                        continue;
                                    }
                                };
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
                                println!("Playing track: {:?}", track);
                            } else {
                                println!("No track is currently set to play.");
                            }
                        }
                        PlayerCommand::Seek(pos) => {
                            if current_track.is_none() {
                                println!("No track is currently set to seek.");
                                continue;
                            }
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
                                in_evt_clone
                                    .send(PlayerEvent::Progress(
                                        sink.get_pos().as_secs_f32(),
                                        current_duration,
                                    ))
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
                            pending_volume_save = Some(volume);
                            last_volume_change = Utc::now().timestamp_millis();
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
                // debounce saving volume
                if let Some(volume) = pending_volume_save {
                    if Utc::now().timestamp_millis() - last_volume_change >= 300 {
                        let mut preferences = PREFERENCES
                            .get()
                            .expect("Preferences not initialized")
                            .write()
                            .await;
                        preferences.volume = volume;
                        drop(preferences);
                        pending_volume_save = None;
                    }
                }
                if sink.empty() && current_duration > 0.0 {
                    current_duration = 0.0;
                    in_evt_clone.send(PlayerEvent::End).unwrap_or_else(|_| {
                        println!("Failed to send end event");
                        0
                    });
                    if repeat_mode == Repeat::One {
                        if let Some(ref track) = current_track {
                            queue.insert(0, track.clone());
                        }
                    } else if repeat_mode == Repeat::All {
                        if let Some(ref track) = current_track {
                            queue.push(track.clone());
                        }
                    }
                    if let Some(next_track) = queue.first().cloned() {
                        current_track = Some(next_track);
                        sink.clear();
                        current_duration = 0.0;
                        if let Some(ref track) = current_track {
                            in_evt_clone
                                .send(PlayerEvent::TrackLoaded(track.clone()))
                                .unwrap_or_else(|_| {
                                    println!("Failed to send track loaded event");
                                    0
                                });
                            let Some(ref path) = track.path else {
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
                            println!("Playing track: {:?}", track);
                        }
                    } else {
                        current_track = None;
                    }
                } else if !sink.empty() && !sink.is_paused() {
                    if Utc::now().timestamp_millis() - last_progress_updated < 100 {
                        time::sleep(std::time::Duration::from_millis(100)).await;
                        continue; // Skip if the last update was too recent
                    }
                    // Emit progress event based on current position
                    in_evt_clone
                        .send(PlayerEvent::Progress(
                            sink.get_pos().as_secs_f32(),
                            current_duration,
                        ))
                        .unwrap();
                    last_progress_updated = Utc::now().timestamp_millis();
                }
                time::sleep(std::time::Duration::from_millis(200)).await;
            }
        });
        Player { in_cmd, in_evt }
    }

    pub fn add_track(&self, track: Track) {
        self.in_cmd
            .send(PlayerCommand::AddTrack(track))
            .expect("Failed to send add track command");
        println!("Track added to queue.");
    }

    pub fn play(&self) {
        self.in_cmd
            .send(PlayerCommand::Play)
            .expect("Failed to send play command");
        println!("Play command sent.");
    }

    pub fn seek(&self, pos: f32) {
        let cmd = PlayerCommand::Seek(pos);
        self.in_cmd.send(cmd).expect("Failed to send seek command");
        println!("Seeking to position: {}", pos);
    }

    pub fn pause(&self) {
        let cmd = PlayerCommand::Pause;
        self.in_cmd.send(cmd).expect("Failed to send pause command");
        println!("Pause command sent.");
    }

    pub fn set_volume(&self, volume: f32) {
        let cmd = PlayerCommand::SetVolume(volume);
        self.in_cmd
            .send(cmd)
            .expect("Failed to send set volume command");
        println!("Volume set to: {}", volume);
    }

    pub fn set_muted(&self, muted: bool) {
        let cmd = PlayerCommand::SetMuted(muted);
        self.in_cmd
            .send(cmd)
            .expect("Failed to send set muted command");
        println!("Muted set to: {}", muted);
    }

    pub fn out_evt_receiver(&self) -> Receiver<PlayerEvent> {
        self.in_evt.subscribe()
    }

    pub fn clear_queue(&self) {
        self.in_cmd
            .send(PlayerCommand::ClearQueue)
            .expect("Failed to send stop command");
        println!("Queue cleared and playback stopped.");
    }

    pub fn set_repeat(&self, mode: Repeat) {
        self.in_cmd
            .send(PlayerCommand::SetRepeat(mode))
            .expect("Failed to send set repeat command");
        println!("Repeat mode set.");
    }
}
