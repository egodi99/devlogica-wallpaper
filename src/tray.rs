//! Icona nella barra di sistema (Windows) / barra dei menu (macOS) con tutte le impostazioni.

use crate::config::{Config, Mode};
use crate::wallpapers::Wallpaper;
use std::collections::HashMap;
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

#[derive(Clone, Debug)]
pub enum Action {
    SetMode(Mode),
    SetWallpaper(String),
    SetMonitorWallpaper { monitor: String, wallpaper: String },
    SetFps(u32),
    ToggleLogo,
    TogglePause,
    #[cfg_attr(not(windows), allow(dead_code))]
    ToggleFullscreenPause,
    ToggleAutostart,
    OpenFolder,
    Reload,
    Quit,
}

pub struct MonitorInfo {
    pub key: String,
    pub label: String,
}

pub struct Built {
    pub menu: Menu,
    pub actions: HashMap<MenuId, Action>,
}

pub fn build(cfg: &Config, catalog: &[Wallpaper], monitors: &[MonitorInfo], autostart: bool) -> Built {
    let mut actions = HashMap::new();
    let menu = Menu::new();
    let mut check = |text: &str, on: bool, action: Action| {
        let item = CheckMenuItem::new(text, true, on, None);
        actions.insert(item.id().clone(), action);
        item
    };

    let title = MenuItem::new("DevLogica Wallpaper", false, None);
    let _ = menu.append(&title);
    let _ = menu.append(&PredefinedMenuItem::separator());

    // Modalità multimonitor
    let modes = Submenu::new("Schermi", true);
    for (m, label) in [
        (Mode::Same, "Stesso sfondo su ogni schermo"),
        (Mode::PerMonitor, "Uno sfondo diverso per schermo"),
        (Mode::Span, "Un unico sfondo esteso su tutti"),
    ] {
        let _ = modes.append(&check(label, cfg.mode == m, Action::SetMode(m)));
    }
    let _ = menu.append(&modes);

    // Scelta dello sfondo
    if cfg.mode == Mode::PerMonitor {
        for (i, mon) in monitors.iter().enumerate() {
            let current = cfg.per_monitor.get(&mon.key).unwrap_or(&cfg.wallpaper);
            let sub = Submenu::new(format!("Schermo {} — {}", i + 1, mon.label), true);
            for w in catalog {
                let _ = sub.append(&check(
                    &w.name,
                    *current == w.id,
                    Action::SetMonitorWallpaper { monitor: mon.key.clone(), wallpaper: w.id.clone() },
                ));
            }
            let _ = menu.append(&sub);
        }
    } else {
        let sub = Submenu::new("Sfondo", true);
        for w in catalog {
            let _ = sub.append(&check(&w.name, cfg.wallpaper == w.id, Action::SetWallpaper(w.id.clone())));
        }
        let _ = menu.append(&sub);
    }

    let fps = Submenu::new("Fluidità", true);
    for (n, label) in [(15, "15 fps — consumo minimo"), (24, "24 fps"), (30, "30 fps — consigliato"), (60, "60 fps")] {
        let _ = fps.append(&check(label, cfg.fps() == n, Action::SetFps(n)));
    }
    let _ = menu.append(&fps);

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&check("Mostra il logo", cfg.show_logo, Action::ToggleLogo));
    let _ = menu.append(&check("In pausa", cfg.paused, Action::TogglePause));
    #[cfg(windows)]
    let _ = menu.append(&check("Pausa con app a schermo intero", cfg.pause_on_fullscreen, Action::ToggleFullscreenPause));
    let _ = menu.append(&check("Avvia all'accesso", autostart, Action::ToggleAutostart));
    let _ = menu.append(&PredefinedMenuItem::separator());

    let folder = MenuItem::new("Apri cartella sfondi personalizzati", true, None);
    actions.insert(folder.id().clone(), Action::OpenFolder);
    let _ = menu.append(&folder);
    let reload = MenuItem::new("Ricarica sfondi", true, None);
    actions.insert(reload.id().clone(), Action::Reload);
    let _ = menu.append(&reload);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let quit = MenuItem::new("Esci", true, None);
    actions.insert(quit.id().clone(), Action::Quit);
    let _ = menu.append(&quit);

    Built { menu, actions }
}

pub fn icon() -> tray_icon::Icon {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png")).expect("icona").to_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("icona")
}
