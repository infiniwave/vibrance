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
    library::Track,
    providers::youtube::YtTrack,
};

pub type OnPlayCallback<T> = Arc<dyn Fn(T) + Send + Sync>;

/// This is necessary because we want to be able to use full `Track`s as well as
/// other types that represent tracks (e.g. search results) in the TrackListDelegate.
pub trait RenderedTrack: Clone + 'static {
    fn artists_string(&self) -> String;
    fn id(&self) -> String;
    fn title(&self) -> String;
    fn album_art(&self) -> Option<Vec<u8>>;
}

pub struct TrackListDelegate<T: RenderedTrack> {
    items: Vec<T>,
    selected_index: Option<IndexPath>,
    on_play: Option<OnPlayCallback<T>>,
}

impl<T: RenderedTrack> TrackListDelegate<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            selected_index: None,
            on_play: None,
        }
    }

    pub fn with_on_play(mut self, callback: OnPlayCallback<T>) -> Self {
        self.on_play = Some(callback);
        self
    }
}

impl<T: RenderedTrack> From<Vec<T>> for TrackListDelegate<T> {
    fn from(items: Vec<T>) -> Self {
        Self::new(items)
    }
}

impl<T: RenderedTrack> ListDelegate for TrackListDelegate<T> {
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
            let aa = track.album_art();
            let track_for_click = track.clone();
            let on_play = self.on_play.clone();
            let title = track.title();
            let artists = track.artists_string();
            let track_id = track.id();
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
                                            Some(render_image(w, a, album_art.clone()))
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
                                        .child(div().child(title.to_string()).text_ellipsis())
                                        .child(div().child(artists).text_sm().text_ellipsis()),
                                ),
                        )
                        .child(
                            Button::new(SharedString::new(format!("play_{}", track_id)))
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

pub struct TrackList<T: RenderedTrack> {
    pub list_state: Entity<ListState<TrackListDelegate<T>>>,
}
impl<T: RenderedTrack> TrackList<T> {
    pub fn new(
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
        delegate: TrackListDelegate<T>,
    ) -> Self {
        let list_state = cx.new(|cx| ListState::new(delegate, window, cx));
        Self { list_state }
    }
    pub fn update_delegate(
        &self,
        cx: &mut gpui::Context<'_, Self>,
        new_delegate: TrackListDelegate<T>,
    ) {
        self.list_state.update(cx, |t, cx| {
            *t.delegate_mut() = new_delegate;
            cx.notify();
        });
    }
}

impl<T: RenderedTrack> Render for TrackList<T> {
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

impl RenderedTrack for Track {
    fn artists_string(&self) -> String {
        self.artists_string()
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn album_art(&self) -> Option<Vec<u8>> {
        self.album.album_art.clone()
    }
}

impl RenderedTrack for YtTrack {
    fn artists_string(&self) -> String {
        self.artist.clone()
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn album_art(&self) -> Option<Vec<u8>> {
        self.album_art.clone()
    }
}
