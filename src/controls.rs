use std::ffi::c_void;

use anyhow::Result;
use gpui::Window;
use souvlaki::{MediaControlEvent, MediaControls, PlatformConfig};

pub fn initialize(window: &mut Window) -> Result<MediaControls> {
    let hwnd = try_get_hwnd(window)?;
    let mut controls = MediaControls::new(PlatformConfig {
        dbus_name: "vibrance",
        display_name: "Vibrance",
        hwnd,
    })
    .map_err(|e| anyhow::anyhow!("Failed to create MediaControls: {:?}", e))?;
    controls
        .attach(|e| {
            match e {
                MediaControlEvent::Pause => {
                    // TODO: pause the player
                }
                MediaControlEvent::Play => {
                    // TODO: play the player
                }
                _ => {
                    // Handle other events if needed
                }
            }
        })
        .map_err(|e| anyhow::anyhow!("Failed to attach MediaControls: {:?}", e))?;

    Ok(controls)
}

pub fn try_get_hwnd(window: &mut Window) -> Result<Option<*mut c_void>> {
    #[cfg(not(target_os = "windows"))]
    return Ok(None);

    #[cfg(target_os = "windows")]
    return {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let handle = window.window_handle();
        if let Ok(hwnd) = handle {
            let handle = hwnd.as_raw();
            let RawWindowHandle::Win32(handle) = handle else {
                return Err(anyhow::anyhow!("Failed to get Win32 window handle"));
            };
            println!("Handle: {:?}", handle);
            Ok(Some(handle.hwnd.get() as *mut c_void))
        } else {
            Err(anyhow::anyhow!("Failed to get main window handle"))
        }
    };
}
