#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod controls;
pub mod player;
pub mod preferences;
use std::sync::Mutex;

use cxx;
use once_cell::sync::OnceCell;
use player::{Player, PlayerEvent};
use preferences::read_preferences;
use souvlaki::MediaControls;

unsafe extern "C" {
    unsafe fn get_mainwindow_hwnd() -> *mut std::ffi::c_void;
}

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cpp/window.h");
        unsafe fn show_widget_window(argc: i32, argv: *mut *mut i8);
        unsafe fn get_mainwindow_mediaplayer() -> usize;
        unsafe fn mediaplayer_set_progress(mediaplayer: usize, value: f64);
        unsafe fn mediaplayer_set_track(mediaplayer: usize, title: String, artists: String, album: String, duration: f64);
        // unsafe fn mediaplayer_set_paused(mediaplayer: usize, paused: bool);
    }
    extern "Rust" {
        fn process_audio_file(path: &str);
        fn open_media_directory(path: &str);
        fn pause();
        fn seek(duration: f64);
    }
}

static PLAYER: OnceCell<Mutex<Player>> = OnceCell::new();
static CONTROLS: OnceCell<Mutex<MediaControls>> = OnceCell::new();

pub fn process_audio_file(path: &str) {
    println!("Rust received file path: {}", path);
    
    let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    let track = player.resolve_track(path.to_string()).expect("Failed to resolve track");
    player.add_track(track);
    player.play();
    println!("Track added and playback started.");
}

pub fn open_media_directory(path: &str) {
    println!("Rust received directory path: {}", path);
}

pub fn pause() {
    let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    player.pause();
}

pub fn seek(duration: f64) {
    let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    player.seek(duration as f32);
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
    // Initialize the player
    let player = Player::new();
    let recv = player.out_evt.clone();

    run_threaded(move || {
        // Initialize media controls if enabled in preferences
        if preferences.use_system_audio_controls {
            while CONTROLS.get().is_none() {
                // TODO: Implement a more robust way to wait for the controls to be initialized
                // (currently, there is a chance of a segfault if Qt is not initialized yet)
                std::thread::sleep(std::time::Duration::from_millis(1000));
                let controls = controls::initialize();
                if let Ok(controls) = controls {
                    CONTROLS.set(Mutex::new(controls)).expect("Failed to set media controls");
                    println!("Media controls initialized successfully.");
                }
            }
        } else {
            println!("System audio controls are disabled in preferences.");
        }
    });
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
                        drop(player); // Release the lock after playing
                    },
                    PlayerEvent::TrackLoaded(track) => {
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
                }
            }
        }
    });
    let player = Mutex::new(player);
    PLAYER.set(player).expect("Failed to initialize player");
    println!("Player initialized successfully.");
    // Start the Qt application
    let args: Vec<std::ffi::CString> = std::env::args()
        .map(|arg| std::ffi::CString::new(arg).unwrap())
        .collect();
    let mut raw_args: Vec<*mut i8> = args.iter().map(|arg| arg.as_ptr() as *mut i8).collect();
    raw_args.push(std::ptr::null_mut());
    unsafe {
        ffi::show_widget_window(args.len() as i32, raw_args.as_mut_ptr());
    }
}
