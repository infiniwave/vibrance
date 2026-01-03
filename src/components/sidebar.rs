use gpui::{
    AppContext, ClickEvent, Entity, IntoElement, PathPromptOptions, Render, SharedString, Styled, Window, rgba,
};
use gpui_component::{
    Side,
    sidebar::{Sidebar as GpuiSidebar, SidebarMenu, SidebarMenuItem, SidebarToggleButton},
};
use tokio::task;
use walkdir::WalkDir;

use crate::{
    components::icon::Icon,
    library::LIBRARY,
    providers::local,
};

pub struct Sidebar {
    pub navigation_state: Entity<NavigationState>,
    pub collapsed: Entity<bool>,
}

pub enum NavigationState {
    Home,
    Search,
    Lyrics,
}

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac"];

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

    pub fn item_lyrics(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut gpui::Context<'_, Self>,
    ) {
        self.navigation_state.update(cx, |state, _| {
            *state = NavigationState::Lyrics;
        });
    }

    pub fn load_media_file(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut gpui::Context<'_, Self>,
    ) {
        let path_future = cx.prompt_for_paths(PathPromptOptions {
            directories: false,
            files: true,
            multiple: true,
            prompt: Some(SharedString::new("Select media file(s)")),
        });
        cx.spawn(async move |_, _| {
            let paths = path_future.await.ok().and_then(|r| r.ok()).and_then(|p| p);
            if let Some(paths) = paths {
                task::spawn(async move {
                    let library = LIBRARY.get().expect("Library not initialized");
                    for path in paths {
                        match local::resolve_track(path.to_str().unwrap_or("")) {
                            Ok(track) => {
                                if let Err(e) = library.add_track(&track).await {
                                    eprintln!("Failed to add track to library: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to resolve track: {}", e);
                            }
                        }
                    }
                })
                .await
                .ok();
            }
        })
        .detach();
    }

    pub fn load_media_directory(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut gpui::Context<'_, Self>,
    ) {
        let path_future = cx.prompt_for_paths(PathPromptOptions {
            directories: true,
            files: false,
            multiple: false,
            prompt: Some(SharedString::new("Select media directory")),
        });
        cx.spawn(async move |_, _| {
            let paths = path_future.await.ok().and_then(|r| r.ok()).and_then(|p| p);
            if let Some(paths) = paths 
            && let Some(dir) = paths.first() {
                let dir = dir.clone();
                task::spawn(async move {
                    let library = LIBRARY.get().expect("Library not initialized");
                    for entry in WalkDir::new(&dir)
                        .follow_links(true)
                        .into_iter()
                        .filter_map(|e| e.ok()) {
                        if entry.file_type().is_file() {
                            if let Some(ext) = entry.path().extension() 
                            && SUPPORTED_EXTENSIONS
                                .iter()
                                .any(|&e| e.eq_ignore_ascii_case(ext.to_str().unwrap_or("")))
                            {
                                match local::resolve_track(entry.path().to_str().unwrap_or("")) {
                                    Ok(track) => {
                                        if let Err(e) = library.add_track(&track).await {
                                            eprintln!("Failed to add track to library: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to resolve track {}: {}",entry.path().display(),e);
                                    }
                                }
                            }
                        }
                    }
                })
                .await
                .ok();
            }
        })
        .detach();
    }
}

impl Render for Sidebar {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        GpuiSidebar::new(Side::Left)
            .bg(rgba(0x0000007a))
            .collapsible(true)
            .collapsed(*self.collapsed.read(cx))
            .header(
                // div()
                //     .size_full()
                //     .h_flex()
                //     .justify_center()
                //     .child(
                SidebarToggleButton::left()
                    .collapsed(*self.collapsed.read(cx))
                    .on_click(cx.listener(|t, _, _, cx| {
                        t.collapsed.update(cx, |c, _| {
                            *c = !*c;
                        });
                    })),
                // )
                // .child("Vibrance"),
            )
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Load media")
                            .icon(Icon::FolderOpen)
                            .on_click(cx.listener(Self::load_media_file)),
                    )
                    .child(
                        SidebarMenuItem::new("Load media directory")
                            .icon(Icon::FolderList)
                            .on_click(cx.listener(Self::load_media_directory)),
                    ),
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
                    )
                    .child(
                        SidebarMenuItem::new("Lyrics")
                            .icon(Icon::Play)
                            .on_click(cx.listener(Self::item_lyrics))
                            .active(matches!(
                                self.navigation_state.read(cx),
                                NavigationState::Lyrics
                            )),
                    ),
            )
    }
}
