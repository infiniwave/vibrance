use std::sync::Arc;

use gpui::{
    AbsoluteLength, AppContext, Entity, ImageSource, IntoElement, ParentElement, Render,
    SharedString, Styled, div, img, rgba,
};
use gpui_component::{
    IndexPath, StyledExt,
    button::Button,
    list::{List, ListDelegate, ListItem, ListState},
};

use crate::{
    components::{icon::Icon, render_image},
    player::Track,
};

pub type OnPlayCallback = Arc<dyn Fn(Track) + Send + Sync>;

pub struct TrackListDelegate {
    items: Vec<Track>,
    selected_index: Option<IndexPath>,
    on_play: Option<OnPlayCallback>,
}

impl TrackListDelegate {
    pub fn new(items: Vec<Track>) -> Self {
        Self {
            items,
            selected_index: None,
            on_play: None,
        }
    }

    pub fn with_on_play(mut self, callback: OnPlayCallback) -> Self {
        self.on_play = Some(callback);
        self
    }
}

impl From<Vec<Track>> for TrackListDelegate {
    fn from(items: Vec<Track>) -> Self {
        Self::new(items)
    }
}

impl ListDelegate for TrackListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &gpui::App) -> usize {
        self.items.len()
    }

    fn render_item(
        &mut self,
        ix: gpui_component::IndexPath,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.items.get(ix.row).map(|track| {
            let aa = track.album_art.clone();
            let track_for_click = track.clone();
            let on_play = self.on_play.clone();
            //println!("Rendering track at index {}: {:?}", ix.row, track);
            ListItem::new(ix)
                .child(
                    div()
                        .h_flex()
                        .justify_between()
                        .w_full()
                        .child(
                            div()
                                .h_flex()
                                .gap_4()
                                .child(
                                    img(ImageSource::Custom(Arc::new(move |w, a| {
                                        // album_art is base64
                                        if let Some(album_art) = &aa {
                                            Some(render_image(w, a, album_art))
                                        } else {
                                            None
                                        }
                                    })))
                                    .rounded_md()
                                    .h_16(),
                                )
                                .child(
                                    div()
                                        .v_flex()
                                        .child(
                                            div().child(
                                                track
                                                    .title
                                                    .clone()
                                                    .unwrap_or("Unknown Title".to_string()),
                                            ).text_ellipsis(),
                                        )
                                        .child(div().child(track.artists.clone().join(", ")).text_sm().text_ellipsis()),
                                ),
                        )
                        .child(
                            Button::new(SharedString::new(format!("play_{}", track.id)))
                                .icon(Icon::Play)
                                .on_click(move |_event, _window, _cx| {
                                    if let Some(ref callback) = on_play {
                                        callback(track_for_click.clone());
                                    }
                                }),
                        )
                        .p_1(),
                )
                .bg(if Some(ix) == self.selected_index {
                    gpui::rgb(0x444444)
                } else {
                    gpui::rgb(0x222222)
                })
                .rounded(AbsoluteLength::Pixels(5_f32.into()))
        })
    }

    fn set_selected_index(
        &mut self,
        ix: Option<gpui_component::IndexPath>,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }
}

pub struct TrackList {
    pub list_state: Entity<ListState<TrackListDelegate>>,
}
impl TrackList {
    pub fn new(
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
        delegate: TrackListDelegate,
    ) -> Self {
        let list_state = cx.new(|cx| ListState::new(delegate, window, cx));
        Self { list_state }
    }
    pub fn update_delegate(
        &self,
        cx: &mut gpui::Context<'_, Self>,
        new_delegate: TrackListDelegate,
    ) {
        self.list_state.update(cx, |t, cx| {
            println!(
                "TrackList delegate updated with {:?} items",
                new_delegate.items
            );
            *t.delegate_mut() = new_delegate;
            cx.notify();
        });
    }
}

impl Render for TrackList {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        gpui::div()
            .bg(rgba(0x00000000)) // 9f
            .h_full()
            .child(List::new(&self.list_state))
    }
}
