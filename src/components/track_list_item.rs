use gpui::{IntoElement, ParentElement, RenderOnce};

#[derive(IntoElement)]
pub struct TrackListItem {

}

impl RenderOnce for TrackListItem {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        gpui::div().child("TrackItem Component")
    }
}