//! Ciclo principale: una finestra per schermo, ritmo dei fotogrammi, gestione del menu.

use crate::config::{log, Config, Mode};
use crate::platform;
use crate::render::{self, Gpu, Params, Target};
use crate::tray::{self, Action, MonitorInfo};
use crate::wallpapers::{self, Wallpaper};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tray_icon::menu::MenuId;
use tray_icon::{TrayIcon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::dpi::{Position, Size};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::monitor::MonitorHandle;
use winit::window::{Window, WindowId};

#[derive(Debug)]
pub enum UserEvent {
    Menu(MenuId),
}

struct Screen {
    target: Target,
    wallpaper: String,
    offset: [f32; 2],
    canvas: [f32; 2],
    occluded: bool,
}

pub struct App {
    cfg: Config,
    catalog: Vec<Wallpaper>,
    gpu: Option<Gpu>,
    screens: HashMap<WindowId, Screen>,
    monitors: Vec<MonitorInfo>,
    monitor_sig: String,
    tray: Option<TrayIcon>,
    actions: HashMap<MenuId, Action>,
    start: Instant,
    /// Momento dell'ultimo cambio di sfondo: fa ripartire l'animazione di apertura.
    changed: Instant,
    next_frame: Instant,
    next_check: Instant,
    fullscreen: bool,
}

fn monitor_key(m: &MonitorHandle) -> String {
    let s = m.size();
    format!("{}|{}x{}", m.name().unwrap_or_else(|| "Schermo".into()), s.width, s.height)
}

fn monitor_list(el: &ActiveEventLoop) -> Vec<MonitorHandle> {
    let mut v: Vec<MonitorHandle> = el.available_monitors().collect();
    v.sort_by_key(|m| (m.position().x, m.position().y));
    v
}

fn signature(mons: &[MonitorHandle]) -> String {
    mons.iter()
        .map(|m| format!("{}@{},{}x{}", monitor_key(m), m.position().x, m.position().y, m.scale_factor()))
        .collect::<Vec<_>>()
        .join(";")
}

impl App {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            cfg: Config::load(),
            catalog: wallpapers::catalog(),
            gpu: None,
            screens: HashMap::new(),
            monitors: Vec::new(),
            monitor_sig: String::new(),
            tray: None,
            actions: HashMap::new(),
            start: now,
            changed: now,
            next_frame: now,
            next_check: now + Duration::from_secs(2),
            fullscreen: false,
        }
    }

    /// (Ri)crea tutte le finestre in base agli schermi presenti e alla modalità scelta.
    fn rebuild(&mut self, el: &ActiveEventLoop) {
        self.screens.clear();
        let mons = monitor_list(el);
        self.monitor_sig = signature(&mons);
        self.monitors = mons
            .iter()
            .map(|m| {
                let s = m.size();
                MonitorInfo { key: monitor_key(m), label: format!("{} ({}×{})", m.name().unwrap_or_default(), s.width, s.height) }
            })
            .collect();
        if mons.is_empty() {
            return;
        }

        // Rettangolo che contiene tutti gli schermi (serve alla modalità estesa).
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for m in &mons {
            let (p, s) = (m.position(), m.size());
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x + s.width as i32);
            y1 = y1.max(p.y + s.height as i32);
        }

        let instance = self.gpu.as_ref().map(|g| g.instance.clone()).unwrap_or_else(render::new_instance);
        let mut created: Vec<(Arc<Window>, wgpu::Surface<'static>, String, [f32; 2], [f32; 2])> = Vec::new();
        for m in &mons {
            let (pos, size) = (m.position(), m.size());
            let window = match create_window(el, m) {
                Some(w) => w,
                None => continue,
            };
            if !platform::attach(&window, pos, size) {
                log(format!("Aggancio al desktop non riuscito per {}", monitor_key(m)));
            }
            window.set_visible(true);
            let surface = match instance.create_surface(window.clone()) {
                Ok(s) => s,
                Err(e) => {
                    log(format!("Superficie non creata: {e}"));
                    continue;
                }
            };
            let key = monitor_key(m);
            let wallpaper = match self.cfg.mode {
                Mode::PerMonitor => self.cfg.per_monitor.get(&key).cloned().unwrap_or_else(|| self.cfg.wallpaper.clone()),
                _ => self.cfg.wallpaper.clone(),
            };
            let (offset, canvas) = match self.cfg.mode {
                Mode::Span => ([(pos.x - x0) as f32, (pos.y - y0) as f32], [(x1 - x0) as f32, (y1 - y0) as f32]),
                _ => ([0.0, 0.0], [size.width as f32, size.height as f32]),
            };
            created.push((window, surface, wallpaper, offset, canvas));
        }

        if self.gpu.is_none() {
            match Gpu::new(instance, created.first().map(|c| &c.1)) {
                Ok(g) => self.gpu = Some(g),
                Err(e) => {
                    log(e);
                    el.exit();
                    return;
                }
            }
        }
        let gpu = self.gpu.as_ref().unwrap();
        for (window, surface, wallpaper, offset, canvas) in created {
            let id = window.id();
            let target = gpu.make_target(window, surface);
            self.screens.insert(id, Screen { target, wallpaper, offset, canvas, occluded: false });
        }
        self.changed = Instant::now();
        log(format!("{} schermi, modalità {:?}", self.screens.len(), self.cfg.mode));
    }

    fn refresh_menu(&mut self) {
        let built = tray::build(&self.cfg, &self.catalog, &self.monitors, platform::autostart_enabled());
        self.actions = built.actions;
        match &self.tray {
            Some(t) => t.set_menu(Some(Box::new(built.menu))),
            None => {
                match TrayIconBuilder::new()
                    .with_menu(Box::new(built.menu))
                    .with_icon(tray::icon())
                    .with_tooltip("DevLogica Wallpaper")
                    .build()
                {
                    Ok(t) => self.tray = Some(t),
                    Err(e) => log(format!("Icona di sistema non creata: {e}")),
                }
            }
        }
    }

    fn render_all(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else { return };
        // Il tempo riparte ogni 6 ore per non perdere precisione nei calcoli in virgola mobile.
        let time = (self.start.elapsed().as_secs_f64() % 21_600.0) as f32;
        let since = self.changed.elapsed().as_secs_f32().min(1_000.0);
        let logo = if self.cfg.show_logo { 1.0 } else { 0.0 };
        for screen in self.screens.values_mut() {
            if screen.occluded {
                continue;
            }
            let size = screen.target.window.inner_size();
            let params = Params {
                res: [size.width as f32, size.height as f32],
                offset: screen.offset,
                canvas: screen.canvas,
                time,
                since,
                logo_on: logo,
                _pad: [0.0; 3],
            };
            let w = self.catalog.iter().find(|w| w.id == screen.wallpaper).unwrap_or(&self.catalog[0]);
            gpu.draw(&mut screen.target, w, params);
        }
    }

    fn handle(&mut self, el: &ActiveEventLoop, action: Action) {
        let mut rebuild = false;
        match action {
            Action::SetMode(m) => {
                self.cfg.mode = m;
                rebuild = true;
            }
            Action::SetWallpaper(id) => {
                self.cfg.wallpaper = id;
                rebuild = true;
            }
            Action::SetMonitorWallpaper { monitor, wallpaper } => {
                self.cfg.per_monitor.insert(monitor, wallpaper);
                rebuild = true;
            }
            Action::SetFps(n) => self.cfg.fps = n,
            Action::ToggleLogo => self.cfg.show_logo = !self.cfg.show_logo,
            Action::TogglePause => self.cfg.paused = !self.cfg.paused,
            Action::ToggleFullscreenPause => self.cfg.pause_on_fullscreen = !self.cfg.pause_on_fullscreen,
            Action::ToggleAutostart => {
                if let Err(e) = platform::set_autostart(!platform::autostart_enabled()) {
                    log(format!("Avvio automatico: {e}"));
                }
            }
            Action::OpenFolder => platform::open_folder(&crate::config::custom_wallpapers_dir()),
            Action::Reload => {
                self.catalog = wallpapers::catalog();
                if let Some(g) = self.gpu.as_mut() {
                    g.clear_pipelines();
                }
                rebuild = true;
            }
            Action::Quit => {
                self.screens.clear();
                el.exit();
                return;
            }
        }
        self.cfg.save();
        if rebuild {
            self.rebuild(el);
        }
        self.refresh_menu();
        // Un fotogramma subito, anche se in pausa, così il cambio si vede.
        self.render_all();
    }
}

fn create_window(el: &ActiveEventLoop, m: &MonitorHandle) -> Option<Arc<Window>> {
    // Su macOS lo spazio delle coordinate è in punti: con schermi a scala diversa
    // (Retina + monitor esterno) bisogna convertire con la scala di *quello* schermo.
    #[cfg(target_os = "macos")]
    let (pos, size): (Position, Size) = {
        let s = m.scale_factor();
        (m.position().to_logical::<f64>(s).into(), m.size().to_logical::<f64>(s).into())
    };
    #[cfg(not(target_os = "macos"))]
    let (pos, size): (Position, Size) = (m.position().into(), m.size().into());

    #[allow(unused_mut)]
    let mut attrs = Window::default_attributes()
        .with_title("DevLogica Wallpaper")
        .with_decorations(false)
        .with_resizable(false)
        .with_visible(false)
        .with_position(pos)
        .with_inner_size(size);
    #[cfg(windows)]
    {
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs = attrs.with_skip_taskbar(true);
    }
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::WindowAttributesExtMacOS;
        attrs = attrs.with_has_shadow(false);
    }
    match el.create_window(attrs) {
        Ok(w) => Some(Arc::new(w)),
        Err(e) => {
            log(format!("Finestra non creata: {e}"));
            None
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gpu.is_none() {
            log(format!("Avvio, {} sfondi disponibili", self.catalog.len()));
            if !self.catalog.iter().any(|w| w.id == self.cfg.wallpaper) {
                self.cfg.wallpaper = self.catalog[0].id.clone();
            }
            self.rebuild(el);
            self.refresh_menu();
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, event: UserEvent) {
        let UserEvent::Menu(id) = event;
        if let Some(action) = self.actions.get(&id).cloned() {
            self.handle(el, action);
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Occluded(o) => {
                if let Some(s) = self.screens.get_mut(&id) {
                    s.occluded = o;
                }
            }
            WindowEvent::Resized(size) => {
                if let (Some(gpu), Some(s)) = (self.gpu.as_ref(), self.screens.get_mut(&id)) {
                    gpu.resize(&mut s.target, size.width, size.height);
                    if self.cfg.mode != Mode::Span {
                        s.canvas = [size.width as f32, size.height as f32];
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Il ritmo dei fotogrammi è gestito in `about_to_wait`.
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();

        // Controlli periodici: schermi collegati/scollegati, Explorer riavviato, app a schermo intero.
        if now >= self.next_check {
            self.next_check = now + Duration::from_secs(2);
            let sig = signature(&monitor_list(el));
            if sig != self.monitor_sig || platform::desktop_lost() {
                log("Configurazione schermi cambiata: ricreo le finestre");
                self.rebuild(el);
                self.refresh_menu();
            }
            self.fullscreen = self.cfg.pause_on_fullscreen && platform::fullscreen_busy();
        }

        let running = !self.cfg.paused && !self.fullscreen;
        if running && now >= self.next_frame {
            self.render_all();
            let step = Duration::from_secs_f64(1.0 / self.cfg.fps() as f64);
            self.next_frame += step;
            if self.next_frame < now {
                self.next_frame = now + step;
            }
        }
        let wake = if running { self.next_frame.min(self.next_check) } else { self.next_check };
        el.set_control_flow(ControlFlow::WaitUntil(wake));
    }
}
