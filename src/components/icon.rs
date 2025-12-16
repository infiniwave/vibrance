use gpui_component::IconNamed;

pub enum Icon {
    Play,
    Pause,
    Next,
    Previous,
    Search,
    Settings,
    Home,
    FolderList,
    FolderOpen,
    Navigation,
    Speaker2,
}

impl IconNamed for Icon {
    fn path(self) -> gpui::SharedString {
        match self {
            Icon::Play => "svg/play.svg",
            Icon::Pause => "svg/pause.svg",
            Icon::Next => "svg/next.svg",
            Icon::Previous => "svg/previous.svg",
            Icon::Search => "svg/search.svg",
            Icon::Settings => "svg/settings.svg",
            Icon::Home => "svg/home.svg",
            Icon::FolderList => "svg/folder_list.svg",
            Icon::FolderOpen => "svg/folder_open.svg",
            Icon::Navigation => "svg/navigation.svg",
            Icon::Speaker2 => "svg/speaker_2.svg",
        }
        .into()
    }
}

// impl Sizeable for Icon {

// }
