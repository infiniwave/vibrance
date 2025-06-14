#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod album_art;
pub mod controls;
pub mod lyrics;
pub mod player;
pub mod preferences;
pub mod providers;

use std::{fs, sync::Mutex, time::Duration};

use cxx;
use lyrics::LyricSource;
use once_cell::sync::OnceCell;
use player::{Player, PlayerEvent};
use preferences::{PREFERENCES, read_preferences};
use providers::youtube;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback};
use tokio::{runtime::Runtime, time::timeout};

unsafe extern "C" {
    unsafe fn get_mainwindow_hwnd() -> *mut std::ffi::c_void;
}

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cpp/window.h");
        unsafe fn show_widget_window(argc: i32, argv: *mut *mut i8);
        unsafe fn get_mainwindow_mediaplayer() -> usize;
        unsafe fn get_mainwindow() -> usize;
        unsafe fn mediaplayer_set_progress(mediaplayer: usize, value: f64);
        unsafe fn mediaplayer_set_track(
            mediaplayer: usize,
            title: String,
            artists: String,
            album: String,
            duration: f64,
        );
        unsafe fn mediaplayer_set_paused(mediaplayer: usize, paused: bool);
        unsafe fn add_track(
            mainwindow: usize,
            id: String,
            title: String,
            artists: String,
            album_art: String,
        );
    }
    extern "Rust" {
        fn process_audio_file(path: &str);
        fn open_media_directory(path: &str);
        fn pause();
        fn seek(duration: f64);
        fn set_volume(volume: i32);
        fn get_initial_volume() -> i32;
        fn initialize_controls();
        fn get_track_list() -> Vec<TrackInfo>;
        fn play(id: &str);
        fn get_lyrics_for_current_track() -> Vec<LyricLine>;
        
        fn yt_search(query: &str) -> Vec<TrackInfo>;
        fn yt_download(id: &str);
    }

    #[derive(Debug)]
    pub struct TrackInfo {
        id: String,
        title: String,
        artists: String,
        album: String,
        album_art: String,
        duration: f64,
    }

    #[derive(Debug, Clone)]
    pub struct LyricLine {
        timestamp: f64, // seconds
        text: String,
    }
}

static PLAYER: OnceCell<Mutex<Player>> = OnceCell::new();
static CONTROLS: OnceCell<Mutex<MediaControls>> = OnceCell::new();

pub fn yt_search(query: &str) -> Vec<ffi::TrackInfo> {
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        match providers::youtube::search_tracks(query).await {
            Ok(tracks) => tracks
                .into_iter()
                .map(|track| ffi::TrackInfo {
                    id: track.id,
                    title: track.title,
                    artists: track.artist,
                    album: track.album,
                    album_art: track.album_art.unwrap_or_default(),
                    duration: track.duration as f64,
                })
                .collect(),
            Err(e) => {
                eprintln!("Failed to search YouTube tracks: {}", e);
                vec![]
            }
        }
    })
}

pub fn yt_download(id: &str) {
    let preferences = PREFERENCES
        .get()
        .expect("Preferences not initialized")
        .lock()
        .expect("Failed to lock preferences mutex");
    if let Some(track) = preferences.find_track_by_yt_id(id) {
        drop(preferences);
        let mut player = PLAYER
            .get()
            .expect("Player not initialized")
            .lock()
            .expect("Failed to lock player mutex");
        player.add_track(track);
        player.play();
    } else {
        drop(preferences);
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        match providers::youtube::download_track_default(id).await {
            Ok(track) => {
                let mut player = PLAYER
                    .get()
                    .expect("Player not initialized")
                    .lock()
                    .expect("Failed to lock player mutex");
                let mainwindow = unsafe { ffi::get_mainwindow() };
                add_track_to_mainwindow(&track, mainwindow);
                player.add_track(track);
                player.play();
                println!("YouTube track downloaded and playback started.");
            }
            Err(e) => {
                eprintln!("Failed to download YouTube track: {}", e);
            }
        }
    });
    }
}

pub fn get_lyrics_for_current_track() -> Vec<ffi::LyricLine> {
    // let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
    // let player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    // if let Some(track) = player.current_track() {
    //     if let Some(lyrics) = preferences.lyrics.get(&track.id) {
    //         return lyrics.iter().map(|line| ffi::LyricLine {
    //             timestamp: line.timestamp,
    //             text: line.text.clone(),
    //         }).collect();
    //     }
    // }
    let player = PLAYER
        .get()
        .expect("Player not initialized")
        .lock()
        .expect("Failed to lock player mutex");
    if let Some(track) = &player.current_track {
        if track.artists.is_empty() || track.title.is_none() {
            eprintln!("Track artists or title is missing, cannot fetch lyrics.");
            return vec![];
        }
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            timeout(Duration::from_secs(4),async move {
                let lyrics = providers::qq::QQProvider::fetch_lyrics(
                    &track.artists.join(", "),
                    &track.title.as_ref().unwrap(),
                )
                .await;
                match lyrics {
                    Ok(lyrics) => {
                        if lyrics.is_empty() {
                            eprintln!("No lyrics found for the current track.");
                            return vec![];
                        }
                        let lyrics = lyrics.get(0).unwrap().0.clone();
                        println!("{:?}", lyrics);
                        lyrics
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch lyrics: {}", e);
                        vec![]
                    }
                }
            }).await
        }).unwrap_or(vec![])
    } else {
        eprintln!("No current track to fetch lyrics for.");
        return vec![];
    }
}

pub fn play(id: &str) {
    let preferences = PREFERENCES
        .get()
        .expect("Preferences not initialized")
        .lock()
        .expect("Failed to lock preferences mutex");
    let track = preferences.find_track_by_id(id);
    drop(preferences);
    if let Some(track) = track {
        let mut player = PLAYER
            .get()
            .expect("Player not initialized")
            .lock()
            .expect("Failed to lock player mutex");
        player.clear_queue();
        player.add_track(track);
        player.play();
        println!("Playback started for track with ID: {}", id);
    } else {
        eprintln!("Track with ID {} not found, assuming yt track", id);
        yt_download(id);    
    }
}

impl From<player::Track> for ffi::TrackInfo {
    fn from(track: player::Track) -> Self {
        let artists = if track.artists.is_empty() {
            "Unknown Artist".to_string()
        } else {
            track.artists.join(", ")
        };
        Self {
            id: track.id,
            title: track.title.unwrap_or_else(|| "Unknown Title".to_string()),
            artists,
            album: track.album.unwrap_or_else(|| "Unknown Album".to_string()),
            album_art: track.album_art.unwrap_or_default(),
            duration: track.duration,
        }
    }
}

fn add_track_to_mainwindow(track: &player::Track, mainwindow: usize) {
    unsafe {
        ffi::add_track(
            mainwindow,
            track.id.clone(),
            track
                .title
                .clone()
                .unwrap_or_else(|| "Unknown Title".to_string()),
            track.artists.join(", "),
            track.album_art.clone().unwrap_or_default(),
        );
    }
}

pub fn get_track_list() -> Vec<ffi::TrackInfo> {
    let preferences = PREFERENCES
        .get()
        .expect("Preferences not initialized")
        .lock()
        .expect("Failed to lock preferences mutex");
    preferences
        .all_tracks()
        .into_iter()
        .par_bridge()
        .map(|track| track.into())
        .collect()
}

pub fn initialize_controls() {
    // Initialize media controls if enabled in preferences
    let preferences = PREFERENCES
        .get()
        .expect("Preferences not initialized")
        .lock()
        .expect("Failed to lock preferences mutex");
    if preferences.use_system_audio_controls {
        CONTROLS
            .get_or_try_init::<_, Box<dyn std::error::Error + Send + Sync>>(|| {
                Ok(Mutex::new(controls::initialize()?))
            })
            .expect("Failed to initialize media controls");
        println!("Media controls initialized successfully.");
    } else {
        println!("System audio controls are disabled in preferences.");
    }
}

pub fn get_initial_volume() -> i32 {
    let preferences = PREFERENCES
        .get()
        .expect("Preferences not initialized")
        .lock()
        .expect("Failed to lock preferences mutex");
    (preferences.volume * 100.0) as i32
}

pub fn process_audio_file(path: &str) {
    println!("Rust received file path: {}", path);
    let mut player = PLAYER
        .get()
        .expect("Player not initialized")
        .lock()
        .expect("Failed to lock player mutex");
    let track = player
        .resolve_track(path.to_string())
        .expect("Failed to resolve track");
    let preferences = PREFERENCES.get().expect("Preferences not initialized");
    let mut preferences = preferences
        .lock()
        .expect("Failed to lock preferences mutex");
    preferences.add_unorganized_track(track.clone());
    drop(preferences);
    player.add_track(track.clone());
    let mainwindow = unsafe { ffi::get_mainwindow() };
    add_track_to_mainwindow(&track, mainwindow);
    player.play();
    println!("Track added and playback started.");
}

pub fn open_media_directory(directory_path: &str) {
    println!("Rust received directory path: {}", directory_path);
    let files = fs::read_dir(directory_path).expect("Failed to read directory");
    let player = PLAYER
        .get()
        .expect("Player not initialized")
        .lock()
        .expect("Failed to lock player mutex");
    let files = files
        .par_bridge()
        .into_par_iter()
        .filter_map(|entry| match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "mp3" || ext == "wav" || ext == "flac" || ext == "ogg" {
                            Some(path.to_string_lossy().to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        })
        .filter_map(|path| player.resolve_track(path).ok())
        .collect::<Vec<_>>();
    let mainwindow = unsafe { ffi::get_mainwindow() };
    for track in &files {
        add_track_to_mainwindow(track, mainwindow);
    }
    let preferences = PREFERENCES.get().expect("Preferences not initialized");
    let mut preferences = preferences
        .lock()
        .expect("Failed to lock preferences mutex");
    println!(
        "Processed {} audio files from directory: {}",
        files.len(),
        directory_path
    );
    preferences.add_tracks_to_library(directory_path.to_string(), files);
    drop(preferences);
    println!("Media directory opened and tracks added to library.");
}

pub fn pause() {
    let mut player = PLAYER
        .get()
        .expect("Player not initialized")
        .lock()
        .expect("Failed to lock player mutex");
    player.pause();
}

pub fn seek(duration: f64) {
    let mut player = PLAYER
        .get()
        .expect("Player not initialized")
        .lock()
        .expect("Failed to lock player mutex");
    player.seek(duration as f32);
}

pub fn set_volume(volume: i32) {
    let mut player = PLAYER
        .get()
        .expect("Player not initialized")
        .lock()
        .expect("Failed to lock player mutex");
    player.set_volume((volume as f32) / 100.0);
}

lazy_static::lazy_static! {
    static ref THREAD_POOL: rayon::ThreadPool = rayon::ThreadPoolBuilder::new().build().unwrap();
}
pub fn run_threaded<F>(cb: F)
where
    F: FnOnce() + Send + 'static,
{
    THREAD_POOL.spawn(cb);
}

fn main() {
    // Read data from configuration
    let preferences = read_preferences().expect("Failed to read preferences");
    PREFERENCES
        .set(Mutex::new(preferences.clone()))
        .expect("Failed to set preferences");
    println!("Preferences loaded successfully.");
    lyrics::initialize().expect("Failed to initialize lyrics client");
    youtube::initialize_client();
    run_threaded(move || {
        let preferences = PREFERENCES
            .get()
            .expect("Preferences not initialized")
            .lock()
            .expect("Failed to lock preferences mutex");
        let mut previous_preferences = preferences.clone();
        drop(preferences);
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            let preferences = PREFERENCES
                .get()
                .expect("Preferences not initialized")
                .lock()
                .expect("Failed to lock preferences mutex");
            if *preferences == previous_preferences {
                break;
            }
            println!("Preferences changed, saving...");
            if let Err(e) = preferences.save() {
                eprintln!("Failed to save preferences: {}", e);
            } else {
                println!("Preferences saved successfully.");
                previous_preferences.clone_from(&*preferences);
            }
        }
    });
    // Initialize the player
    let player = Player::new(preferences.volume.clone());
    let recv = player.out_evt.clone();
    // Initialize the player helper
    run_threaded(move || {
        let recv = recv.lock().expect("Failed to lock player event receiver");
        println!("Starting progress receiver thread");
        while let Ok(event) = recv.recv() {
            match event {
                PlayerEvent::Progress(progress_value) => {
                    let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                    unsafe {
                        ffi::mediaplayer_set_progress(media_player, progress_value);
                    }
                    // println!("Progress: {}", progress_value);
                }
                PlayerEvent::End => {
                    println!("Playback ended");
                    let mut player = PLAYER
                        .get()
                        .expect("Player not initialized")
                        .lock()
                        .expect("Failed to lock player mutex");
                    player.current_track = None;
                    player.play();
                    drop(player);
                }
                PlayerEvent::TrackLoaded(track) => {
                    let mut controls = CONTROLS
                        .get()
                        .expect("Media controls not initialized")
                        .lock()
                        .expect("Failed to lock media controls mutex");
                    let title = track.title.clone();
                    let album = track.album.clone();
                    let c = controls.set_metadata(MediaMetadata {
                        title: Some(&title.unwrap_or("Unknown Title".to_string())),
                        album: Some(&album.unwrap_or("Unknown Album".to_string())),
                        duration: Some(Duration::from_secs_f64(track.duration)),
                        artist: Some(&track.artists.join(", ")),
                        cover_url: None,
                    });
                    if let Err(e) = c {
                        eprintln!("Failed to set metadata: {:?}", e);
                    }
                    let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                    unsafe {
                        ffi::mediaplayer_set_track(
                            media_player,
                            track.title.unwrap_or("Unknown Title".to_string()),
                            track.artists.join(", "),
                            track.album_art.unwrap_or(String::new()),
                            track.duration,
                        );
                    }
                    // println!("Track loaded: {}", track.file_path);
                }
                PlayerEvent::Paused => {
                    let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                    unsafe {
                        ffi::mediaplayer_set_paused(media_player, true);
                    }
                    let mut controls = CONTROLS
                        .get()
                        .expect("Media controls not initialized")
                        .lock()
                        .expect("Failed to lock media controls mutex");
                    let c = controls.set_playback(MediaPlayback::Paused { progress: None });
                    if let Err(e) = c {
                        eprintln!("Failed to set playback state: {:?}", e);
                    }
                    drop(controls);
                    // println!("Playback paused: {}", paused);
                }
                PlayerEvent::Resumed => {
                    let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                    unsafe {
                        ffi::mediaplayer_set_paused(media_player, false);
                    }
                    let mut controls = CONTROLS
                        .get()
                        .expect("Media controls not initialized")
                        .lock()
                        .expect("Failed to lock media controls mutex");
                    let c = controls.set_playback(MediaPlayback::Playing { progress: None });
                    if let Err(e) = c {
                        eprintln!("Failed to set playback state: {:?}", e);
                    }
                    drop(controls);
                    // println!("Playback resumed");
                }
            }
        }
    });
    let player = Mutex::new(player);
    PLAYER.set(player).expect("Failed to initialize player");
    println!("Player initialized successfully.");
    // std::panic::set_hook(Box::new(|info| {
    //     eprintln!("Panic occurred: {:?} {:?}", info, info.payload());
    //     let preferences = PREFERENCES
    //         .get()
    //         .expect("Preferences not initialized")
    //         .lock()
    //         .expect("Failed to lock preferences mutex");
    //     if let Err(e) = preferences.save() {
    //         eprintln!("Failed to save preferences on panic: {}", e);
    //     } else {
    //         println!("Preferences saved successfully on panic.");
    //     }
    // }));
    // Start the Qt application
    let args: Vec<std::ffi::CString> = std::env::args()
        .map(|arg| std::ffi::CString::new(arg).unwrap())
        .collect();
    let mut raw_args: Vec<*mut i8> = args.iter().map(|arg| arg.as_ptr() as *mut i8).collect();
    raw_args.push(std::ptr::null_mut());
    unsafe {
        ffi::show_widget_window(args.len() as i32, raw_args.as_mut_ptr());
    }

    let preferences = PREFERENCES
        .get()
        .expect("Preferences not initialized")
        .lock()
        .expect("Failed to lock preferences mutex");
    if let Err(e) = preferences.save() {
        eprintln!("Failed to save preferences on exit: {}", e);
    } else {
        println!("Preferences saved successfully on exit.");
    }
    println!("Bye");
}
