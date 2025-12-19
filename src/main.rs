#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod components;
pub mod controls;
pub mod lyrics;
pub mod player;
pub mod preferences;
pub mod providers;
pub mod resources;
pub mod views;

use std::{path::PathBuf, time::Duration};

use gpui::*;
use gpui_component::*;
use gpui_component_assets::Assets;
use once_cell::sync::OnceCell;
use souvlaki::{MediaMetadata, MediaPlayback, OsMediaControls};
use tokio::{
    sync::{Mutex, RwLock},
    task, time,
};

use crate::{
    components::sidebar::NavigationState,
    player::{PLAYER, Player, PlayerEvent},
    preferences::{PREFERENCES, read_preferences},
    resources::Resources,
};

pub struct App {
    player: Entity<components::player::Player>,
    sidebar: Entity<components::sidebar::Sidebar>,
    home_view: Entity<views::HomeView>,
    search_view: Entity<views::SearchView>,
    lyrics_view: Entity<views::LyricsView>,
}

impl App {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let player = cx.new(|cx| components::player::Player::new(cx));

        let sidebar = cx.new(|cx| components::sidebar::Sidebar::new(cx));
        let home_view = cx.new(|cx| views::HomeView::new(window, cx));
        let player_for_search = player.clone();
        let search_view = cx.new(|cx| views::SearchView::new(window, cx, player_for_search));
        let lyrics_view = cx.new(|cx| views::LyricsView::new(window, cx));
        Self {
            player,
            sidebar,
            home_view,
            search_view,
            lyrics_view,
        }
    }
}
impl Render for App {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = self.sidebar.read(cx);
        let navitem = sidebar.navigation_state.read(cx);
        let render = match navitem {
            NavigationState::Home => self.home_view.clone().into_any_element(),
            NavigationState::Search => self.search_view.clone().into_any_element(),
            NavigationState::Lyrics => self.lyrics_view.clone().into_any_element(),
        };
        // This is a weird bug as "DM Sans" works perfectly fine on Linux, but
        // Windows only recognises the font as "DM Sans 14pt" for some reason.
        #[cfg(target_os = "windows")]
        let font_name = "DM Sans 14pt";
        #[cfg(not(target_os = "windows"))]
        let font_name = "DM Sans";
        div()
            .font(font(font_name))
            .bg(linear_gradient(
                135.0,
                linear_color_stop(rgb(0x1D0034), 0.0),
                linear_color_stop(rgb(0x31001D), 1.0),
            ))
            .text_color(rgb(16777215))
            .v_flex()
            .size_full()
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .child(self.sidebar.clone())
                    .child(div().flex_1().min_h_0().h_full().child(render)),
            )
            .child(self.player.clone())
    }
}

// const DM_SANS: &[u8] = include_bytes!("./resources/fonts/dm-sans-variable.ttf");
// const DM_SANS_ITALIC: &[u8] = include_bytes!("./resources/fonts/dm-sans-italic-variable.ttf");

static CONTROLS: OnceCell<Mutex<OsMediaControls>> = OnceCell::new();

#[tokio::main]
async fn main() {
    // Read data from configuration
    let preferences = read_preferences()
        .await
        .expect("Failed to read preferences");
    PREFERENCES
        .set(RwLock::new(preferences.clone()))
        .expect("Failed to set preferences");
    println!("Preferences loaded successfully.");
    task::spawn(async move {
        let preferences = PREFERENCES
            .get()
            .expect("Preferences not initialized")
            .read()
            .await;
        let mut previous_preferences = preferences.clone();
        drop(preferences);
        loop {
            time::sleep(std::time::Duration::from_secs(60)).await;
            let preferences = PREFERENCES
                .get()
                .expect("Preferences not initialized")
                .read()
                .await;
            if *preferences == previous_preferences {
                break;
            }
            println!("Preferences changed, saving...");
            if let Err(e) = preferences.save().await {
                eprintln!("Failed to save preferences: {}", e);
            } else {
                println!("Preferences saved successfully.");
                previous_preferences.clone_from(&*preferences);
            }
        }
    });
    // Initialize the player
    let player = Player::new(preferences.volume.clone());
    let mut recv = player.out_evt_receiver();
    // Initialize the player helper
    task::spawn(async move {
        println!("Starting progress receiver thread");
        while let Ok(event) = recv.recv().await {
            match event {
                PlayerEvent::Progress(progress_value, len) => {
                    // TODO: Update media controls progress
                    println!("Progress: {}", progress_value);
                }
                PlayerEvent::End => {}
                PlayerEvent::TrackLoaded(track) => {
                    let mut controls = CONTROLS
                        .get()
                        .expect("Media controls not initialized")
                        .lock()
                        .await;
                    let title = track.title.clone();
                    let album = track.album.clone();
                    #[cfg(target_os = "windows")]
                    {
                        // On Windows, also set the album art if available
                        if let Some(album_art) = track.album_art.clone() {
                            use base64::{Engine, prelude::BASE64_STANDARD};
                            use souvlaki::platform::windows::WindowsCover;
                            let cover = BASE64_STANDARD
                                .decode(album_art)
                                .expect("Failed to decode album art");
                            controls.set_cover(Some(WindowsCover::Bytes(cover))).
                                unwrap_or_else(|e| {
                                    eprintln!("Failed to set album art: {:?}", e);
                                });
                        }
                    }
                    let c = controls.set_metadata(MediaMetadata {
                        title: Some(title.unwrap_or("Unknown Title".to_string())),
                        album_title: Some(album.unwrap_or("Unknown Album".to_string())),    
                        duration: Some(Duration::from_secs_f64(track.duration)),
                        artist: Some(track.artists.join(", ")),
                        artists: Some(track.artists.clone()),
                        ..Default::default()
                    });
                    if let Err(e) = c {
                        eprintln!("Failed to set metadata: {:?}", e);
                    }
                    // TODO: Update media controls playback state with track
                    // println!("Track loaded: {}", track.file_path);
                }
                PlayerEvent::Paused => {
                    // TODO: Update media controls playback state to paused
                    let mut controls = CONTROLS
                        .get()
                        .expect("Media controls not initialized")
                        .lock()
                        .await;
                    let c = controls.set_playback(MediaPlayback::Paused { progress: None });
                    if let Err(e) = c {
                        eprintln!("Failed to set playback state: {:?}", e);
                    }
                    drop(controls);
                    // println!("Playback paused: {}", paused);
                }
                PlayerEvent::Resumed => {
                    // TODO: Update media controls playback state to playing
                    let mut controls = CONTROLS
                        .get()
                        .expect("Media controls not initialized")
                        .lock()
                        .await;
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
    PLAYER.set(player).expect("Failed to initialize player");
    println!("Player initialized successfully.");
    let resources = Resources;
    let dm_sans = resources
        .load("fonts/dm-sans-variable.ttf")
        .expect("Missing font")
        .expect("Missing font");
    let dm_sans_italic = resources
        .load("fonts/dm-sans-italic-variable.ttf")
        .expect("Missing font")
        .expect("Missing font");
    let app = Application::new()
        .with_assets(Assets)
        .with_assets(resources);
    app.text_system()
        .add_fonts(vec![dm_sans, dm_sans_italic])
        .expect("Failed to load fonts");

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);
        let theme_name = SharedString::from("Tokyo Night"); // TODO: theme preferences
        if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
            if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
                Theme::global_mut(cx).apply_config(&theme);
            }
        }) {
            println!("Failed to watch themes directory: {}", err);
        }

        cx.on_window_closed(move |cx| {
            if cx.windows().is_empty() {
                task::block_in_place(|| {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        // Save preferences before exiting
                        if let Some(preferences_mutex) = PREFERENCES.get() {
                            let preferences = preferences_mutex.read().await;
                            if let Err(e) = preferences.save().await {
                                eprintln!("Failed to save preferences on exit: {}", e);
                            } else {
                                println!("Preferences saved successfully on exit.");
                            }
                        }
                    });
                });
                println!("Bye");
                std::process::exit(0);
            }
        })
        .detach();

        cx.open_window(WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(SharedString::new("Vibrance")),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(900.0), px(700.0)), cx))),
            ..Default::default()
        }, |window, cx| {
            let view = cx.new(|cx| App::new(window, cx));
            if preferences.use_system_audio_controls {
                let controls =
                    controls::initialize(window).expect("Failed to initialize media controls");
                let controls_mutex = Mutex::new(controls);
                CONTROLS
                    .set(controls_mutex)
                    .expect("Failed to initialize media controls");
                println!("Media controls initialized successfully.");
            }
            // This first level on the window, should be a Root.
            cx.new(|cx| Root::new(view, window, cx))
        })
        .ok();
    });
}
