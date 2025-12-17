use std::sync::Arc;

use gpui::{AppContext, Entity, IntoElement, ParentElement, Render, Styled, Subscription};
use gpui_component::{
    Icon as GpuiIcon, StyledExt,
    input::{Input, InputEvent, InputState},
};
use tokio::task;

use crate::{
    components::{
        icon::Icon,
        player::Player as PlayerComponent,
        track_list::{TrackList, TrackListDelegate},
    },
    player::{PLAYER, Track},
    providers,
};

pub struct SearchView {
    input_state: Entity<InputState>,
    track_list: Entity<TrackList>,
    _s: Vec<Subscription>,
}

impl SearchView {
    pub fn new(
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
        _player_component: Entity<PlayerComponent>,
    ) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

        let on_play_callback: Arc<dyn Fn(Track) + Send + Sync> = Arc::new(move |track: Track| {
            let track_clone = track.clone();

            task::spawn(async move {
                let track = if track_clone.yt_id.is_some() && track_clone.path.is_none() {
                    println!("Downloading YouTube track: {:?}", track_clone.yt_id);
                    match providers::youtube::download_track_default(
                        track_clone.yt_id.as_ref().unwrap(),
                    )
                    .await
                    {
                        Ok(downloaded_track) => downloaded_track,
                        Err(e) => {
                            eprintln!("Failed to download track: {}", e);
                            return;
                        }
                    }
                } else {
                    track_clone
                };

                if let Some(player) = PLAYER.get() {
                    println!("Playing track: {:?}", track.title);
                    player.add_track(track);
                    player.play();
                } else {
                    eprintln!("Player not initialized");
                }
            });
        });

        let initial_delegate = TrackListDelegate::new(vec![Track {
            album: None,
            album_art: None,
            artists: vec![],
            duration: 0.0,
            id: "".to_string(),
            path: None,
            title: Some("No results".to_string()),
            yt_id: None,
        }])
        .with_on_play(on_play_callback.clone());

        let track_list = cx.new(|cx| TrackList::new(window, cx, initial_delegate));
        let x = track_list.clone();
        let on_play_for_search = on_play_callback.clone();
        let _s = vec![cx.subscribe_in(
            &input_state,
            window,
            move |view, state, event, window, cx| {
                let y = x.clone();
                let on_play = on_play_for_search.clone();
                match event {
                    InputEvent::PressEnter { secondary } => {
                        let query = state.read(cx).value().trim().to_string();
                        println!("Searching for: {}", query);
                        cx.spawn(async move |e, app| {
                            println!("Searching for: {}", query);
                            // spawn the search on tokio's runtime to avoid conflicts with gpui's executor
                            let search_result = task::spawn(async move {
                                providers::youtube::search_tracks(&query).await
                            })
                            .await;

                            match search_result {
                                Ok(Ok(tracks)) => {
                                    println!("Found {} tracks", tracks.len());
                                    let items = tracks
                                        .iter()
                                        .map(|t| Track {
                                            title: Some(t.title.clone()),
                                            artists: vec![t.artist.clone()],
                                            album: Some(t.album.clone()),
                                            album_art: t.album_art.clone(),
                                            duration: t.duration as f64,
                                            id: t.id.clone(),
                                            yt_id: Some(t.id.clone()),
                                            path: None,
                                        })
                                        .collect::<Vec<_>>();
                                    // update the track list delegate with the callback
                                    let new_delegate =
                                        TrackListDelegate::new(items).with_on_play(on_play);
                                    app.update_entity(&y, |e, cx| {
                                        e.update_delegate(cx, new_delegate)
                                    })
                                    .unwrap();
                                }
                                Ok(Err(e)) => {
                                    eprintln!("Error searching tracks: {}", e);
                                }
                                Err(e) => {
                                    eprintln!("Task join error: {}", e);
                                }
                            }
                        })
                        .detach();
                    }
                    _ => {}
                }
            },
        )];
        Self {
            input_state,
            track_list,
            _s,
        }
    }
}

impl Render for SearchView {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        gpui::div()
            .w_full()
            .h_full()
            .v_flex()
            .px_5()
            .py_2()
            .gap_4()
            .child(Input::new(&self.input_state).prefix(GpuiIcon::new(Icon::Search)))
            .child(self.track_list.clone())
    }
}
