use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, Entity, ImageSource, IntoElement, ParentElement, Render, Styled, Timer,
    div, img,
};
use gpui_component::popover::Popover;
use gpui_component::{
    StyledExt,
    button::Button,
    group_box::{GroupBox, GroupBoxVariants},
    slider::{Slider, SliderEvent, SliderState},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::components::icon::Icon;
use crate::components::render_image;
use crate::library::Track;
use crate::player::{PLAYER, PlayerCommand, PlayerEvent, Repeat};

pub struct Player {
    playback_position: f32,
    playback_state: Entity<SliderState>,
    current_track: Option<Track>,
    playback_position_secs: f64,
    duration_secs: f64,
    is_seeking: bool,
    cmd_sender: Option<UnboundedSender<PlayerCommand>>,
    paused: bool,
    volume_state: Entity<SliderState>,
    repeat: Repeat,
    album_art_source: Option<ImageSource>,
}

impl Player {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let playback_state = cx.new(|_| SliderState::new().min(0.0).max(100.0).step(0.1));
        let volume_state = cx.new(|_| SliderState::new().min(0.0).max(100.0).step(1.0));
        cx.subscribe(
            &playback_state,
            |this: &mut Self, _, event: &SliderEvent, cx| {
                let SliderEvent::Change(value) = event;
                // TODO: only seek if the user is done dragging (on mouse up)
                if this.duration_secs > 0.0 && !this.is_seeking {
                    this.is_seeking = true;
                    let position = value.end() / 100.0; // convert from 0-100 to 0-1
                    if let Some(player) = PLAYER.get() {
                        player.seek(position);
                    }
                    this.playback_position_secs = position as f64 * this.duration_secs;
                    this.is_seeking = false;
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe(
            &volume_state,
            |this: &mut Self, _, event: &SliderEvent, cx| {
                let SliderEvent::Change(value) = event;
                let position = value.end() / 100.0; // convert from 0-100 to 0-1
                if let Some(player) = PLAYER.get() {
                    player.set_volume(position);
                }
                cx.notify();
            },
        )
        .detach();

        cx.spawn(async move |this, cx| {
            // wait for player to be initialized and subscribe
            let player = PLAYER.get().expect("Player not initialized");
            let mut receiver = player.out_evt_receiver();
            if let Some(this_entity) = this.upgrade() {
                let _ = cx.update_entity(&this_entity, |player_component: &mut Player, cx| {
                    player_component.cmd_sender = Some(player.in_cmd.clone());
                });
            }

            // need to loop twice. inner loop to drain all messages
            loop {
                Timer::after(Duration::from_millis(50)).await;
                loop {
                    match receiver.try_recv() {
                        Ok(event) => {
                            if let Some(this_entity) = this.upgrade() {
                                let _ = cx.update_entity(
                                    &this_entity,
                                    |player_component: &mut Player, cx| match event {
                                        PlayerEvent::Progress(position, length) => {
                                            if !player_component.is_seeking {
                                                player_component.playback_position =
                                                    position / length as f32;
                                                player_component.playback_position_secs =
                                                    position as f64;
                                                cx.notify();
                                            }
                                        }
                                        PlayerEvent::TrackLoaded(track) => {
                                            player_component.update_track(track, cx);
                                        }
                                        PlayerEvent::End => {
                                            player_component.playback_position_secs = 0.0;
                                            player_component.playback_position = 0.0;
                                            cx.notify();
                                        }
                                        PlayerEvent::Paused => {
                                            player_component.paused = true;
                                        }
                                        PlayerEvent::Resumed => {
                                            player_component.paused = false;
                                        }
                                    },
                                );
                            }
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                            eprintln!("Broadcast receiver lagged by {} messages", n);
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
            playback_position: 0.0,
            playback_state,
            current_track: None,
            playback_position_secs: 0.0,
            duration_secs: 0.0,
            is_seeking: false,
            cmd_sender: None,
            paused: false,
            volume_state,
            repeat: Repeat::Off,
            album_art_source: None,
        }
    }

    pub fn update_track(&mut self, track: Track, cx: &mut Context<Self>) {
        self.duration_secs = track.duration;
        self.playback_position_secs = 0.0;
        self.playback_position = 0.0;
        self.album_art_source = track.album.album_art.clone().map(|album_art| {
            ImageSource::Custom(Arc::new(move |w, a| {
                Some(render_image(w, a, album_art.clone()))
            }))
        });
        self.current_track = Some(track);
        cx.notify();
    }

    fn format_time(seconds: f64) -> String {
        let mins = (seconds / 60.0).floor() as u32;
        let secs = (seconds % 60.0).floor() as u32;
        format!("{}:{:02}", mins, secs)
    }
}

impl Render for Player {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        let title = self
            .current_track
            .as_ref()
            .and_then(|t| Some(t.title.clone()))
            .unwrap_or_else(|| "No track playing".to_string());
        let artist = self
            .current_track
            .as_ref()
            .map(|t| t.artists_string())
            .unwrap_or_default();

        // format current position and duration
        let current_time = Self::format_time(self.playback_position_secs);
        let total_time = Self::format_time(self.duration_secs);

        // update slider value based on current playback position
        let slider_value = (self.playback_position * 100.0) as f32;
        self.playback_state.update(cx, |state, cx| {
            state.set_value(slider_value, window, cx);
        });

        let album_art_source = self.album_art_source.clone();

        GroupBox::new().outline().child(
            div()
                .w_full()
                .v_flex()
                .gap_4()
                .child(
                    div()
                        .gap_4()
                        .h_flex()
                        .text_sm()
                        .child(current_time)
                        .child(Slider::new(&self.playback_state))
                        .child(total_time),
                )
                .child(
                    gpui::div()
                        .w_full()
                        .h_flex()
                        .child(
                            div()
                                .h_flex()
                                .gap_4()
                                .flex_1()
                                .mr_4()
                                .min_w_0()
                                .child(
                                    div()
                                        .h_flex()
                                        .when_some(album_art_source, |el, source| {
                                            el.child(img(source).rounded_md())
                                        })
                                        .w_20()
                                        .h_20(),
                                )
                                .child(
                                    div()
                                        .v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .w_full()
                                        .child(
                                            div()
                                                .child(title)
                                                .text_lg()
                                                .text_ellipsis()
                                                .font_semibold(),
                                        )
                                        .child(div().child(artist).text_sm().text_ellipsis()),
                                ),
                        )
                        .child(
                            div()
                                .h_flex()
                                .justify_center()
                                .gap_4()
                                .child(
                                    Button::new("previous")
                                        .icon(Icon::Previous)
                                        .on_click(cx.listener(|t, _, _, _| {})),
                                )
                                .child(
                                    Button::new("pause")
                                        .when(self.paused, |s| s.icon(Icon::Play))
                                        .when(!self.paused, |s| s.icon(Icon::Pause))
                                        .on_click(cx.listener(|t, _, _, _| {
                                            t.cmd_sender.as_ref().map(|sender| {
                                                let _ = sender.send(PlayerCommand::Pause);
                                            });
                                        })),
                                )
                                .child(
                                    Button::new("next")
                                        .icon(Icon::Next)
                                        .on_click(cx.listener(|t, _, _, _| {})),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .h_flex()
                                .gap_4()
                                .ml_4()
                                .justify_end()
                                .child(
                                    Button::new("repeat")
                                        .when(self.repeat == Repeat::Off, |s| {
                                            s.icon(Icon::ArrowRepeatOff)
                                        })
                                        .when(self.repeat == Repeat::All, |s| {
                                            s.icon(Icon::ArrowRepeatAll)
                                        })
                                        .when(self.repeat == Repeat::One, |s| {
                                            s.icon(Icon::ArrowRepeatOne)
                                        })
                                        .on_click(cx.listener(|t, _, _, _| {
                                            t.repeat = match t.repeat {
                                                Repeat::Off => Repeat::All,
                                                Repeat::All => Repeat::One,
                                                Repeat::One => Repeat::Off,
                                            };
                                            t.cmd_sender.as_ref().map(|sender| {
                                                let _ = sender.send(PlayerCommand::SetRepeat(
                                                    t.repeat.clone(),
                                                ));
                                            });
                                        })),
                                )
                                .child(
                                    Popover::new("volume_popover")
                                        .trigger(Button::new("volume").icon(Icon::Speaker2))
                                        .child(div().py_2().child(
                                            Slider::new(&self.volume_state).vertical().h_24(),
                                        )),
                                ),
                        ),
                ),
        )
    }
}
