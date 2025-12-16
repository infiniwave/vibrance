use gpui::{IntoElement, ParentElement, RenderOnce};

#[derive(IntoElement)]
pub struct SidebarItem {}

impl RenderOnce for SidebarItem {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        gpui::div().child("SidebarItem Component")
    }
}
