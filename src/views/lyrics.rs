use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{AppContext, IntoElement, ParentElement, Render, Styled, Timer, uniform_list};
use gpui_component::StyledExt;
use tokio::task;

use crate::{
    lyrics::{LyricLine, LyricSource},
    player::{PLAYER, PlayerEvent, Track},
    providers::qq::QQProvider,
};

pub struct LyricsView {
    lyrics: Vec<LyricLine>,
    current_track: Option<Track>,
    loading: bool,
    error: Option<String>,
}

impl LyricsView {
    pub fn new(_window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            let receiver = PLAYER
                .get()
                .expect("Player not initialized")
                .out_evt_receiver();

            let mut receiver = receiver;
            loop {
                Timer::after(Duration::from_millis(50)).await;
                loop {
                    match receiver.try_recv() {
                        Ok(event) => {
                            if let PlayerEvent::TrackLoaded(track) = event {
                                if let Some(this_entity) = this.upgrade() {
                                    let _ = cx.update_entity(&this_entity, |view, cx| {
                                        view.current_track = Some(track.clone());
                                        view.loading = true;
                                        view.lyrics.clear();
                                        view.error = None;
                                        cx.notify();
                                    });
                                }
                                let track_clone = track.clone();
                                let lyrics_result = task::spawn(async move {
                                    let artist = track_clone
                                        .artists
                                        .first()
                                        .map(|s| s.as_str())
                                        .unwrap_or("");
                                    let title = track_clone
                                        .title
                                        .as_deref()
                                        .unwrap_or("");

                                    QQProvider::fetch_lyrics(artist, title).await
                                })
                                .await;
                                if let Some(this_entity) = this.upgrade() {
                                    let _ = cx.update_entity(&this_entity, |view, cx| {
                                        match lyrics_result {
                                            Ok(Ok(lyrics_list)) => {
                                                view.loading = false;
                                                if let Some(first_lyrics) =
                                                    lyrics_list.first()
                                                {
                                                    view.lyrics = first_lyrics.0.clone();
                                                } else {
                                                    view.error =
                                                        Some("No lyrics found".to_string());
                                                }
                                            }
                                            Ok(Err(e)) => {
                                                view.loading = false;
                                                view.error = Some(format!(
                                                    "Failed to fetch lyrics: {}",
                                                    e
                                                ));
                                            }
                                            Err(e) => {
                                                view.loading = false;
                                                view.error = Some(format!(
                                                    "Task error: {}",
                                                    e
                                                ));
                                            }
                                        }
                                        cx.notify();
                                    });
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                            eprintln!("Lyrics view: Broadcast receiver lagged by {} messages", n);
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                            break;
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                            return;
                        }
                    }
                }
            }
        })
        .detach();

        Self {
            lyrics: Vec::new(),
            current_track: None,
            loading: false,
            error: None,
        }
    }
}

impl Render for LyricsView {
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
            .gap_2()
            .child(
                gpui::div()
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Lyrics"),
            )
            .when_some(self.current_track.as_ref(), |div, track| {
                div.child(
                    gpui::div()
                        .text_sm()
                        .text_color(gpui::rgb(0x888888))
                        .child(format!(
                            "{} - {}",
                            track.title.as_deref().unwrap_or("Unknown"),
                            track.artists.join(", ")
                        )),
                )
            })
            .when(self.loading, |div| {
                div.child(gpui::div().text_sm().child("Loading lyrics..."))
            })
            .when_some(self.error.as_ref(), |div, error| {
                div.child(gpui::div().text_sm().text_color(gpui::rgb(0xff6666)).child(error.clone()))
            })
            .when(!self.loading && self.error.is_none() && self.lyrics.is_empty() && self.current_track.is_some(), |div| {
                div.child(gpui::div().text_sm().child("No lyrics available"))
            })
            .when(!self.loading && !self.lyrics.is_empty(), |div| {
                let lyrics = self.lyrics.clone();
                div.child(
                    uniform_list(
                        "lyrics-list",
                        lyrics.len(),
                        move |range, _window, _cx| {
                            range
                                .map(|i| {
                                    gpui::div()
                                        .text_base()
                                        .py_1()
                                        .child(lyrics[i].text.clone())
                                })
                                .collect()
                        },
                    )
                    .flex_1()
                )
            })
    }
}
