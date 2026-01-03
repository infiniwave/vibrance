use std::sync::Arc;

use gpui::{AppContext, Entity, IntoElement, ParentElement, Render, Styled};
use gpui_component::StyledExt;
use tokio::task;

use crate::{
    components::track_list::{TrackList, TrackListDelegate},
    library::{LIBRARY, LibraryEvent, Track},
    player::PLAYER,
};

pub struct HomeView {
    track_list: Entity<TrackList<Track>>,
}

impl HomeView {
    pub fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        let on_play_callback: Arc<dyn Fn(Track) + Send + Sync> = Arc::new(move |track: Track| {
            task::spawn(async move {
                if let Some(player) = PLAYER.get() {
                    println!("Playing track: {:?}", track.title);
                    player.clear_queue();
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
                let library = LIBRARY
                    .get()
                    .ok_or(anyhow::anyhow!("Library not initialized"))?;
                Ok(library.all_tracks().await?)
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to join task: {}", e))
            .flatten()
            .unwrap_or_default();

            let new_delegate = TrackListDelegate::new(tracks).with_on_play(on_play_for_load);
            app.update_entity(&track_list_clone, |e, cx| {
                e.update_delegate(cx, new_delegate)
            })
            .ok();
        })
        .detach();

        let track_list_for_events = track_list.clone();
        let on_play_for_events = on_play_callback.clone();
        cx.spawn(async move |_, app| {
            let library = LIBRARY.get().expect("Library not initialized");
            let mut recv = library.subscribe();
            while let Ok(event) = recv.recv().await {
                match event {
                    LibraryEvent::TracksAdded(_) => {
                        let tracks = task::spawn(async move {
                            let library = LIBRARY
                                .get()
                                .ok_or(anyhow::anyhow!("Library not initialized"))?;
                            Ok(library.all_tracks().await?)
                        })
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to join task: {}", e))
                            .flatten()
                            .unwrap_or_default();
                        
                        let new_delegate = TrackListDelegate::new(tracks).with_on_play(on_play_for_events.clone());
                        app.update_entity(&track_list_for_events, |e, cx| {
                            e.update_delegate(cx, new_delegate)
                        })
                        .ok();
                    }
                }
            }
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
                    .child("Library"),
            )
            .child(self.track_list.clone())
    }
}
