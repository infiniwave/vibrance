use std::ffi::c_void;

use anyhow::Result;
use gpui::Window;
use souvlaki::{MediaControlEvent, OsMediaControls};

use crate::player::PLAYER;

pub fn initialize(window: &mut Window) -> Result<OsMediaControls> {
    #[cfg(target_os = "windows")]
    let config = {
        use souvlaki::platform::windows::WindowsConfig;

        let hwnd = try_get_hwnd(window)?;
        WindowsConfig {
            hwnd,
        }
    };
    #[cfg(target_os = "linux")]
    let config = {
        use souvlaki::platform::mpris::MprisConfig;
        MprisConfig {
            dbus_name: "vibrance",
            display_name: "Vibrance",
        }
    };
    let mut controls = OsMediaControls::new(config)
    .map_err(|e| anyhow::anyhow!("Failed to create MediaControls: {:?}", e))?;
    controls
        .attach(|e| {
            match e {
                MediaControlEvent::Pause => {
                    PLAYER.get().map(|p| p.pause());
                }
                MediaControlEvent::Play => {
                    PLAYER.get().map(|p| p.pause());
                }
                _ => {
                    // Handle other events if needed
                }
            }
        })
        .map_err(|e| anyhow::anyhow!("Failed to attach MediaControls: {:?}", e))?;

    Ok(controls)
}

#[cfg(target_os = "windows")]
pub fn try_get_hwnd(window: &mut Window) -> Result<*mut c_void> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle();
    if let Ok(hwnd) = handle {
        let handle = hwnd.as_raw();
        let RawWindowHandle::Win32(handle) = handle else {
            return Err(anyhow::anyhow!("Failed to get Win32 window handle"));
        };
        println!("Handle: {:?}", handle);
        Ok(handle.hwnd.get() as *mut c_void)
    } else {
        Err(anyhow::anyhow!("Failed to get main window handle"))
    }
}
