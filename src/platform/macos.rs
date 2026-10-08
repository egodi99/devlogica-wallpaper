//! macOS: la finestra viene portata al livello "desktop" (sotto le icone),
//! resa visibile in tutti gli Spaces e trasparente ai clic.

use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_foundation::NSRect;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowLevelForKey(key: i32) -> i32;
}
const K_CG_DESKTOP_WINDOW_LEVEL_KEY: i32 = 2;

// NSWindowCollectionBehavior
const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
const STATIONARY: usize = 1 << 4;
const IGNORES_CYCLE: usize = 1 << 6;
const FULL_SCREEN_NONE: usize = 1 << 9;

pub fn attach(window: &Window, _pos: PhysicalPosition<i32>, _size: PhysicalSize<u32>) -> bool {
    let Ok(handle) = window.window_handle() else { return false };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else { return false };
    unsafe {
        let view: &AnyObject = h.ns_view.cast::<AnyObject>().as_ref();
        let ns_window: *mut AnyObject = msg_send![view, window];
        let Some(ns_window) = ns_window.as_ref() else { return false };
        let level = CGWindowLevelForKey(K_CG_DESKTOP_WINDOW_LEVEL_KEY) as isize;
        let _: () = msg_send![ns_window, setLevel: level];
        let _: () = msg_send![ns_window, setCollectionBehavior: CAN_JOIN_ALL_SPACES | STATIONARY | IGNORES_CYCLE | FULL_SCREEN_NONE];
        let _: () = msg_send![ns_window, setIgnoresMouseEvents: true];
        let _: () = msg_send![ns_window, setHasShadow: false];
        // Occupa tutto lo schermo su cui è stata creata, compresa la zona sotto la barra dei menu.
        let screen: *mut AnyObject = msg_send![ns_window, screen];
        if let Some(screen) = screen.as_ref() {
            let frame: NSRect = msg_send![screen, frame];
            let _: () = msg_send![ns_window, setFrame: frame, display: true];
        }
    }
    true
}

pub fn desktop_lost() -> bool {
    false
}

/// Su macOS le app a schermo intero stanno in uno Space separato: la finestra
/// risulta "coperta" (evento Occluded) e il rendering si ferma da solo.
pub fn fullscreen_busy() -> bool {
    false
}

pub fn restore_desktop() {}

fn agent_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join("Library/LaunchAgents/com.devlogica.wallpaper.plist"))
}

pub fn autostart_enabled() -> bool {
    agent_path().map(|p| p.exists()).unwrap_or(false)
}

pub fn set_autostart(on: bool) -> Result<(), String> {
    let path = agent_path().ok_or("cartella utente non trovata")?;
    if !on {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.devlogica.wallpaper</string>
  <key>ProgramArguments</key><array><string>{}</string></array>
  <key>RunAtLoad</key><true/>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#,
        exe.display()
    );
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, plist).map_err(|e| e.to_string())
}
