use std::sync::Arc;

use gpui::{AppContext, Entity, IntoElement, ParentElement, Render, Styled};
use gpui_component::StyledExt;
use tokio::task;

use crate::{
    components::track_list::{TrackList, TrackListDelegate},
    player::{PLAYER, Track},
    preferences::PREFERENCES,
    providers,
};

pub struct HomeView {
    track_list: Entity<TrackList>,
}

impl HomeView {
    pub fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
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

                if let Some(player_mutex) = PLAYER.get() {
                    let mut player = player_mutex.lock().await;
                    println!("Playing track: {:?}", track.title);
                    player.add_track(track);
                    player.play();
                } else {
                    eprintln!("Player not initialized");
                }
            });
        });

        let initial_delegate =
            TrackListDelegate::new(vec![]).with_on_play(on_play_callback.clone());

        let track_list = cx.new(|cx| TrackList::new(window, cx, initial_delegate));

        let track_list_clone = track_list.clone();
        let on_play_for_load = on_play_callback.clone();
        cx.spawn(async move |_, app| {
            let tracks = task::spawn(async move {
                if let Some(prefs) = PREFERENCES.get() {
                    let prefs = prefs.read().await;
                    prefs.all_tracks()
                } else {
                    vec![]
                }
            })
            .await
            .unwrap_or_default();

            let new_delegate = TrackListDelegate::new(tracks).with_on_play(on_play_for_load);
            app.update_entity(&track_list_clone, |e, cx| {
                e.update_delegate(cx, new_delegate)
            })
            .ok();
        })
        .detach();

        Self { track_list }
    }
}

impl Render for HomeView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        gpui::div()
            .w_full()
            .h_full()
            .v_flex()
            .px_5()
            .py_2()
            .gap_4()
            .child(
                gpui::div()
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Your Library"),
            )
            .child(self.track_list.clone())
    }
}
