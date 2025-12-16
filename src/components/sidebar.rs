use gpui::{
    AppContext, ClickEvent, Entity, IntoElement, ParentElement, Render, Styled, Window, div, rgba,
};
use gpui_component::{
    Side,
    sidebar::{Sidebar as GpuiSidebar, SidebarMenu, SidebarMenuItem, SidebarToggleButton},
};

use crate::components::icon::Icon;

pub struct Sidebar {
    pub navigation_state: Entity<NavigationState>,
    pub collapsed: Entity<bool>,
}

pub enum NavigationState {
    Home,
    Search,
}

impl Sidebar {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        let current_state = cx.new(|_| NavigationState::Home);
        let collapsed = cx.new(|_| false);
        Self {
            navigation_state: current_state,
            collapsed,
        }
    }

    pub fn item_home(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut gpui::Context<'_, Self>,
    ) {
        self.navigation_state.update(cx, |state, _| {
            *state = NavigationState::Home;
        });
    }

    pub fn item_search(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut gpui::Context<'_, Self>,
    ) {
        self.navigation_state.update(cx, |state, _| {
            *state = NavigationState::Search;
        });
    }
}

impl Render for Sidebar {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        // gpui::div()
        //     .w_full()
        //     .h_full()
        //     .max_w_64()
        //     .border(AbsoluteLength::Pixels(1_f32.into()))
        //     .border_color(rgb(0))
        //     .v_flex()
        //     .px_2()
        //     .py_4()
        //     .gap_2()
        //     .child(div().child("Vibrance").text_xl().font_bold().text_center())
        //     .child(div()
        //         .v_flex()
        //         .gap_1()
        //         .child(Button::new("load_single").primary().label("Load media"))
        //         .child(Button::new("load").primary().label("Load media directory"))
        //     )
        //     .child(div().v_flex())
        GpuiSidebar::new(Side::Left)
            .bg(rgba(0x0000007a))
            .collapsible(true)
            .collapsed(*self.collapsed.read(cx))
            .header(
                div()
                    .child(
                        SidebarToggleButton::left()
                            .collapsed(*self.collapsed.read(cx))
                            .on_click(cx.listener(|t, _, _, cx| {
                                t.collapsed.update(cx, |c, _| {
                                    *c = !*c;
                                });
                            })),
                    )
                    .child("Vibrance"),
            )
            .child(
                SidebarMenu::new()
                    .child(SidebarMenuItem::new("Load media").icon(Icon::FolderOpen))
                    .child(SidebarMenuItem::new("Load media directory").icon(Icon::FolderList)),
            )
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Home")
                            .icon(Icon::Home)
                            .on_click(cx.listener(Self::item_home))
                            .active(matches!(
                                self.navigation_state.read(cx),
                                NavigationState::Home
                            )),
                    )
                    .child(
                        SidebarMenuItem::new("Search")
                            .icon(Icon::Search)
                            .on_click(cx.listener(Self::item_search))
                            .active(matches!(
                                self.navigation_state.read(cx),
                                NavigationState::Search
                            )),
                    ),
            )
    }
}
