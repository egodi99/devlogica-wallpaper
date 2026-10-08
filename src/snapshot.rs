//! Esportazione di immagini statiche, senza finestre:
//!   devlogica-wallpaper --snapshot <sfondo> <larghezza> <altezza> <secondi> <file.png> [--no-logo] [--tile x y w h] [--scale s]
//! Con --tile si disegna solo una porzione della tela, come fa uno schermo in modalità estesa.

use crate::render::{self, Gpu, Params};
use crate::wallpapers;

pub fn run(args: &[String]) -> i32 {
    if args.len() < 5 {
        eprintln!("uso: --snapshot <sfondo> <larghezza> <altezza> <secondi> <file.png> [--no-logo]");
        eprintln!("sfondi: {}", wallpapers::catalog().iter().map(|w| w.id.clone()).collect::<Vec<_>>().join(", "));
        return 2;
    }
    let cat = wallpapers::catalog();
    let Some(w) = cat.iter().find(|w| w.id == args[0]) else {
        eprintln!("sfondo sconosciuto: {}", args[0]);
        return 2;
    };
    let (width, height): (u32, u32) = (args[1].parse().unwrap_or(1920), args[2].parse().unwrap_or(1080));
    let t: f32 = args[3].parse().unwrap_or(20.0);
    let logo = !args.iter().any(|a| a == "--no-logo");

    let mut gpu = match Gpu::new(render::new_instance(), None) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let tile: Option<[u32; 4]> = args.iter().position(|a| a == "--tile").and_then(|i| {
        let v: Vec<u32> = args.get(i + 1..i + 5)?.iter().filter_map(|s| s.parse().ok()).collect();
        (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
    });
    let [ox, oy, tw, th] = tile.unwrap_or([0, 0, width, height]);
    let (width, height, canvas) = (tw, th, [width as f32, height as f32]);
    let params = Params {
        res: [width as f32, height as f32],
        offset: [ox as f32, oy as f32],
        canvas,
        time: t,
        since: 60.0,
        logo_on: if logo { 1.0 } else { 0.0 },
        scale: args.iter().position(|a| a == "--scale").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(1.0),
        _pad: [0.0; 2],
    };
    let Some(px) = gpu.render_offscreen(w, params) else {
        eprintln!("rendering non riuscito (errore nello shader?)");
        return 1;
    };
    let (width, height) = ((width as f32 * params.scale).round() as u32, (height as f32 * params.scale).round() as u32);
    match image::save_buffer(&args[4], &px, width, height, image::ExtendedColorType::Rgba8) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}
