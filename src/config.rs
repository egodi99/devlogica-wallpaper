//! Configurazione persistente (JSON nella cartella di configurazione dell'utente).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Lo stesso sfondo, ripetuto su ogni schermo.
    Same,
    /// Uno sfondo diverso per ogni schermo.
    PerMonitor,
    /// Un unico sfondo esteso su tutti gli schermi.
    Span,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mode: Mode,
    /// Sfondo usato nelle modalità `Same` e `Span` (e come predefinito per gli schermi nuovi).
    pub wallpaper: String,
    /// Sfondo per schermo, indicizzato dalla chiave dello schermo (nome + risoluzione).
    pub per_monitor: HashMap<String, String>,
    pub fps: u32,
    pub show_logo: bool,
    pub paused: bool,
    /// Mette in pausa quando un'applicazione è a schermo intero (solo Windows).
    pub pause_on_fullscreen: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Same,
            wallpaper: "lame".into(),
            per_monitor: HashMap::new(),
            fps: 30,
            show_logo: true,
            paused: false,
            pause_on_fullscreen: true,
        }
    }
}

pub fn app_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
    base.join("DevLogica Wallpaper")
}

pub fn custom_wallpapers_dir() -> PathBuf {
    app_dir().join("sfondi")
}

fn config_path() -> PathBuf {
    app_dir().join("config.json")
}

impl Config {
    pub fn load() -> Self {
        std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(app_dir());
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(config_path(), s);
        }
    }

    pub fn fps(&self) -> u32 {
        self.fps.clamp(5, 60)
    }
}

/// Piccolo log su file, utile per capire cosa succede su un PC dell'ufficio.
pub fn log(msg: impl AsRef<str>) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(app_dir());
    let path = app_dir().join("log.txt");
    // Evita che il file cresca all'infinito.
    if std::fs::metadata(&path).map(|m| m.len() > 512 * 1024).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{secs}] {}", msg.as_ref());
    }
    eprintln!("{}", msg.as_ref());
}
