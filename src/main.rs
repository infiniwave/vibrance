#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod controls;
pub mod player;
pub mod preferences;

use std::{fs, sync::Mutex, time::Duration};

use cxx;
use lrc::Lyrics;
use once_cell::sync::OnceCell;
use player::{Player, PlayerEvent};
use preferences::{read_preferences, PREFERENCES};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback};

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
        unsafe fn mediaplayer_set_track(mediaplayer: usize, title: String, artists: String, album: String, duration: f64);
        unsafe fn mediaplayer_set_paused(mediaplayer: usize, paused: bool);
        unsafe fn add_track(mainwindow: usize, id: String, title: String, artists: String);
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
    }

    #[derive(Debug)]
    struct TrackInfo {
        id: String,
        title: String,
        artists: String,
        album: String,
        album_art_path: String,
        duration: f64,
    }

    #[derive(Debug)]
    struct LyricLine {
        timestamp: f64, // seconds
        text: String,
    }
}

static PLAYER: OnceCell<Mutex<Player>> = OnceCell::new();
static CONTROLS: OnceCell<Mutex<MediaControls>> = OnceCell::new();
// TODO: periodically save preferences to disk

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
    let sample_lyrics = "[ti:可惜夜]\n[ar:tayori]\n[al:可惜夜]\n[by:]\n[offset:0]\n[kana:2あたら1よ1し1きょく1とお1かん1むね1おく1ざわ1よる1ねつ1う1おも1は1はな1は1ひと1こころ1うば1だれ1さわ1くう1そう1か1なた1きみ1さら1そう1ぞう1かな1かみ1さま1ぼく1き1たい1ねむ1よる1ふけ1わす1き1のろ1も1むね1おく1こ1め1こえ1おぼ1つな1て1さぐ1あ1ふた1り1よる1つき1て1うそ1み1ぬ1かな1かみ1さま1ぼく1き1たい1つづ1ねが1な1あふ1おも1し1だい1あつ1うた1かた1と1よる1ゆ1およ2とわ1おど1だれ1さわ1くう1そう1か1なた1きみ1さら1そう1ぞう1かな1かみ1さま1ぼく1き1たい1ひ1きみ1とも1ねむ1よる1ふけ]\n[00:00.20]可惜夜 - tayori\n[00:01.20]词：tazuneru\n[00:01.87]曲：tazuneru\n[00:18.88]ずっと遠くに感じていた\n[00:22.45]胸の奥の騒めき\n[00:26.12]夜の熱に浮かされて\n[00:29.78]想いは馳せる\n[00:32.99]花も恥じらうような人\n[00:40.79]その心を奪えたなら\n[00:50.95]誰にも障れない空想の彼方\n[00:58.44]君を攫ってなんて想像している\n[01:05.67]叶うなら神様って\n[01:10.42]僕はただ期待して\n[01:13.38]眠れないまま夜に耽る\n[01:25.14]忘れようとしたって\n[01:28.01]ずっと消えなくて\n[01:32.62]呪いのように 燃ゆるように\n[01:36.28]胸の奥を焦がしていく\n[01:42.70]その瞳を その声を 憶えている\n[01:53.84]繋ぐ手を探り合った\n[01:58.45]二人だけの夜を\n[02:01.25]月が照らした嘘を\n[02:03.97]見抜けないまま\n[02:08.58]叶うなら神様って\n[02:13.21]僕はまた期待して\n[02:16.23]あの続きを願ってる\n[02:38.94]どうしようも無いくらい\n[02:42.78]溢れるこの想いが\n[02:46.44]次第に熱くなって\n[02:50.21]泡沫に溶けてゆく\n[02:56.58]この夜を揺られ游いで\n[03:03.94]永遠に踊ろう\n[03:07.66]誰にも障れない空想の彼方\n[03:15.00]君を攫ってなんて想像している\n[03:22.39]叶うなら神様って\n[03:26.97]僕はただ期待して\n[03:29.92]あの日のように君を灯す\n[03:37.24]眠れないまま夜に耽る";
    let parsed = Lyrics::from_str(sample_lyrics).expect("Failed to parse lyrics");
    parsed.get_timed_lines().iter().map(|line| {
        ffi::LyricLine {
            timestamp: line.0.get_timestamp() as f64,
            text: line.1.to_string(),
        }
    }).collect::<Vec<_>>()
}

pub fn play(id: &str) {
    let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
    let mut track = preferences.unorganized_tracks.get(id).map(|track| track.clone());
    if track.is_none() {
        track = preferences.user_library.par_iter().find_map_any(|(_, tracks)| {
            tracks.par_iter().find_any(|track| track.1.id == id)
        }).map(|(_, track)| track.clone());
    }
    drop(preferences);
    if let Some(track) = track {
        let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
        player.clear_queue();
        player.add_track(track);
        player.play();
        println!("Playback started for track with ID: {}", id);
    } else {
        eprintln!("Track with ID {} not found", id);
        return;
    }
}

pub fn get_track_list() -> Vec<ffi::TrackInfo> {
    let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
    let tracks = preferences.unorganized_tracks.values().collect::<Vec<_>>();
    
    
    tracks.iter().map(|track| {
        let artists = if track.artists.is_empty() {
            "Unknown Artist".to_string()
        } else {
            track.artists.join(", ")
        };
        ffi::TrackInfo {
            id: track.id.clone(),
            title: track.title.clone().unwrap_or_else(|| "Unknown Title".to_string()),
            artists,
            album: track.album.clone().unwrap_or_else(|| "Unknown Album".to_string()),
            album_art_path: "default_album_art.png".to_string(),
            duration: track.duration.clone(),
        }
    }).collect()
}

pub fn initialize_controls() {
    // Initialize media controls if enabled in preferences
    let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
    if preferences.use_system_audio_controls {
        CONTROLS.get_or_try_init::<_, Box<dyn std::error::Error + Send + Sync>>(|| {
            Ok(Mutex::new(controls::initialize()?))
        }).expect("Failed to initialize media controls");
        println!("Media controls initialized successfully.");
    } else {
        println!("System audio controls are disabled in preferences.");
    }
}

pub fn get_initial_volume() -> i32 {
    let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
    (preferences.volume * 100.0) as i32
}

pub fn process_audio_file(path: &str) {
    println!("Rust received file path: {}", path);
    
    let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    let track = player.resolve_track(path.to_string()).expect("Failed to resolve track");
    let preferences = PREFERENCES.get().expect("Preferences not initialized");
    let mut preferences = preferences.lock().expect("Failed to lock preferences mutex");
    preferences.add_unorganized_track(track.clone());
    drop(preferences);
    player.add_track(track.clone());
    let mainwindow = unsafe { ffi::get_mainwindow() };
    unsafe {
        ffi::add_track(mainwindow, track.id.clone(), track.title.clone().unwrap_or("Unknown Title".to_string()), track.artists.join(", "));
    }
    player.play();
    println!("Track added and playback started.");
}

pub fn open_media_directory(directory_path: &str) {
    println!("Rust received directory path: {}", directory_path);
    let files = fs::read_dir(directory_path).expect("Failed to read directory");
    let player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    let files = files
        .par_bridge()
        .into_par_iter()
        .filter_map(|entry| {
            match entry {
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
                },
                Err(_) => None,
            }
        })
        .map(|path| {
            player.resolve_track(path).expect("Failed to resolve track")
        })
        .collect::<Vec<_>>();
    let mainwindow = unsafe { ffi::get_mainwindow() };
    unsafe {
        for track in &files {
            ffi::add_track(mainwindow, track.id.clone(), track.title.clone().unwrap_or("Unknown Title".to_string()), track.artists.join(", "));
        }
    }
    let preferences = PREFERENCES.get().expect("Preferences not initialized");
    let mut preferences = preferences.lock().expect("Failed to lock preferences mutex");
    println!("Processed {} audio files from directory: {}", files.len(), directory_path);
    preferences.add_tracks_to_library(directory_path.to_string(), files);
    drop(preferences);
    println!("Media directory opened and tracks added to library.");
}

pub fn pause() {
    let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    player.pause();
}

pub fn seek(duration: f64) {
    let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    player.seek(duration as f32);
}

pub fn set_volume(volume: i32) {
    let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    player.set_volume((volume as f32) / 100.0);
}

lazy_static::lazy_static! {
    static ref THREAD_POOL: rayon::ThreadPool = rayon::ThreadPoolBuilder::new().build().unwrap();
}
pub fn run_threaded<F>(cb: F) where F: FnOnce() + Send + 'static {
    THREAD_POOL.spawn(cb);
}

fn main() {
    // Read data from configuration
    let preferences = read_preferences().expect("Failed to read preferences");
    PREFERENCES.set(Mutex::new(preferences.clone())).expect("Failed to set preferences");
    println!("Preferences loaded successfully.");
    run_threaded(move || {
        let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
        let mut previous_preferences = preferences.clone();
        drop(preferences); 
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60)); 
            let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
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
        let recv = recv.lock();
        println!("Starting progress receiver thread");
        if recv.is_err() {
            println!("Failed to lock receiver");
            return;
        }
        if let Ok(recv) = recv {
            while let Ok(event) = recv.recv() {
                match event {
                    PlayerEvent::Progress(progress_value) => {
                        let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                        unsafe {
                            ffi::mediaplayer_set_progress(media_player, progress_value);
                        }
                        // println!("Progress: {}", progress_value);
                    },
                    PlayerEvent::End => {
                        println!("Playback ended");
                        let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
                        player.play();
                        drop(player);
                    },
                    PlayerEvent::TrackLoaded(track) => {
                        let mut controls = CONTROLS.get().expect("Media controls not initialized").lock().expect("Failed to lock media controls mutex");
                        let title = track.title.clone();
                        let album = track.album.clone();
                        controls.set_metadata(MediaMetadata {
                            title: Some(&title.unwrap_or("Unknown Title".to_string())),
                            album: Some(&album.unwrap_or("Unknown Album".to_string())),
                            duration: Some(Duration::from_secs_f64(track.duration)),
                            artist: Some(&track.artists.join(", ")),
                            cover_url: None
                        });
                        let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                        unsafe {
                            ffi::mediaplayer_set_track(
                                media_player, 
                                track.title.unwrap_or("Unknown Title".to_string()), 
                                track.artists.join(", "), 
                                track.album.unwrap_or_default(), 
                                track.duration
                            );
                        }
                        // println!("Track loaded: {}", track.file_path);
                    },
                    PlayerEvent::Paused => {
                        let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                        unsafe {
                            ffi::mediaplayer_set_paused(media_player, true);
                        }
                        let mut controls = CONTROLS.get().expect("Media controls not initialized").lock().expect("Failed to lock media controls mutex");
                        controls.set_playback(MediaPlayback::Paused { progress: None });
                        drop(controls); 
                        // println!("Playback paused: {}", paused);
                    },
                    PlayerEvent::Resumed => {
                        let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                        unsafe {
                            ffi::mediaplayer_set_paused(media_player, false);
                        }
                        let mut controls = CONTROLS.get().expect("Media controls not initialized").lock().expect("Failed to lock media controls mutex");
                        controls.set_playback(MediaPlayback::Playing { progress: None });
                        drop(controls);
                        // println!("Playback resumed");
                    },
                }
            }
        }
    });
    let player = Mutex::new(player);
    PLAYER.set(player).expect("Failed to initialize player");
    println!("Player initialized successfully.");
    std::panic::set_hook(Box::new(|info| {
        eprintln!("Panic occurred: {:?}", info);
        let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
        if let Err(e) = preferences.save() {
            eprintln!("Failed to save preferences on panic: {}", e);
        } else {
            println!("Preferences saved successfully on panic.");
        }
    }));
    // Start the Qt application
    let args: Vec<std::ffi::CString> = std::env::args()
        .map(|arg| std::ffi::CString::new(arg).unwrap())
        .collect();
    let mut raw_args: Vec<*mut i8> = args.iter().map(|arg| arg.as_ptr() as *mut i8).collect();
    raw_args.push(std::ptr::null_mut());
    unsafe {
        ffi::show_widget_window(args.len() as i32, raw_args.as_mut_ptr());
    }

    let preferences = PREFERENCES.get().expect("Preferences not initialized").lock().expect("Failed to lock preferences mutex");
    if let Err(e) = preferences.save() {
        eprintln!("Failed to save preferences on exit: {}", e);
    } else {
        println!("Preferences saved successfully on exit.");
    }
    println!("Bye");
}
