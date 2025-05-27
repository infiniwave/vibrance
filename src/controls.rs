use std::{ffi::c_void};

use anyhow::Result;
use souvlaki::{MediaControlEvent, MediaControls, PlatformConfig};

use crate::get_mainwindow_hwnd;

pub fn initialize() -> Result<MediaControls> {
    let hwnd = try_get_hwnd()?;
    let mut controls = MediaControls::new(PlatformConfig {
        dbus_name: "vibrance",
        display_name: "Vibrance",
        hwnd,
    }).map_err(|e| anyhow::anyhow!("Failed to create MediaControls: {:?}", e))?;
    controls.attach(|e| {
        match e {
            MediaControlEvent::Pause => {
                crate::pause();
            }
            MediaControlEvent::Play => {
                crate::pause();
            }
            _ => {
                // Handle other events if needed
            }
        }
    }).map_err(|e| anyhow::anyhow!("Failed to attach MediaControls: {:?}", e))?;

    Ok(controls)
}


pub fn try_get_hwnd() -> Result<Option<*mut c_void>> {

    #[cfg(not(target_os = "windows"))]
    return Ok(None);

    #[cfg(target_os = "windows")]
    return unsafe {
        let hwnd = get_mainwindow_hwnd();       
        println!("HWND: {:?}", hwnd); 
        if hwnd.is_null() {
            Err(anyhow::anyhow!("Failed to get main window handle"))
        } else {
            Ok(Some(hwnd))
        }
    };
}