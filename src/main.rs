#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod player;
pub mod preferences;
use std::sync::Mutex;

use cxx;
use once_cell::sync::OnceCell;
use player::{Player, PlayerEvent, Track};
use preferences::read_preferences;

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cpp/window.h");
        unsafe fn show_widget_window(argc: i32, argv: *mut *mut i8);
        unsafe fn get_mainwindow_mediaplayer() -> usize;
        unsafe fn mediaplayer_set_progress(mediaplayer: usize, value: f64);
        unsafe fn mediaplayer_set_track(mediaplayer: usize, track: String);
    }
    extern "Rust" {
        fn process_audio_file(path: &str);
        fn pause();
        fn seek(duration: f64);
    }
}

static PLAYER: OnceCell<Mutex<Player>> = OnceCell::new();

pub fn process_audio_file(path: &str) {
    println!("Rust received file path: {}", path);
    
    let mut player = PLAYER.get().expect("Player not initialized").lock().expect("Failed to lock player mutex");
    player.add_track(Track {
        file_path: path.to_string()
    });
    player.play();
    println!("Track added and playback started.");
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
                        // play(());
                    },
                    PlayerEvent::TrackLoaded(track) => {
                        let media_player = unsafe { ffi::get_mainwindow_mediaplayer() };
                        unsafe {
                            ffi::mediaplayer_set_track(media_player, track);
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
