//! Headless auto-player: clicks through a game and reports unimplemented
//! opcodes, runtime errors and panics.
//! Usage: autoplay <game_root> [frames] [scene]

use std::panic::{AssertUnwindSafe, catch_unwind};

use reallive::engine::{Engine, EngineOptions};
use reallive::input::{Button, InputEvent, Key};

fn main() {
    let mut args = std::env::args().skip(1);
    let root = args.next().expect("game root");
    let frames: usize = args.next().map_or(200_000, |s| s.parse().unwrap());
    let scene: Option<i32> = args.next().map(|s| s.parse().unwrap());
    let mut options = EngineOptions::headless(root);
    options.fonts = std::env::var_os("AUTOPLAY_FONTS").is_some();
    options.start_scene = scene;
    let mut engine = Engine::open(options).expect("open");
    let render_every: usize = std::env::var("AUTOPLAY_RENDER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut last_scene = -1;
    let mut errors_seen = 0;
    let mut rng: u32 = 12345;
    let mut waits = 0usize;
    for frame in 0..frames {
        let scene = engine.machine.scene_number();
        if scene != last_scene {
            eprintln!("frame {frame}: SEEN{scene:04}");
            last_scene = scene;
        }
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        let op = engine.machine.current_long_op().map(|op| op.name());
        if engine.waiting_for_input() || frame % 97 == 0 {
            waits += 1;
            if op == Some("select") || op == Some("select_objbtn") || waits % 50 == 0 {
                let w = engine.machine.sys.gfx.width as u32;
                let h = engine.machine.sys.gfx.height as u32;
                let x = (rng % w) as i32;
                let y = ((rng >> 10) % h) as i32;
                engine.machine.sys.input.mouse = (x, y);
                engine.input(InputEvent::KeyDown(Key::Down));
                engine.input(InputEvent::KeyUp(Key::Down));
                engine.input(InputEvent::KeyDown(Key::Enter));
                engine.input(InputEvent::KeyUp(Key::Enter));
            }
            if scene == 9012 {
                engine.machine.sys.input.mouse = (670, 479);
            }
            engine.input(InputEvent::Press(Button::Left));
            let r = catch_unwind(AssertUnwindSafe(|| engine.tick(16)));
            if r.is_err() {
                report(&mut engine);
                eprintln!(
                    "PANIC at frame {frame} SEEN{:04} line {}",
                    engine.machine.scene_number(),
                    engine.machine.line
                );
                std::process::exit(2);
            }
            engine.input(InputEvent::Release(Button::Left));
        }
        let r = catch_unwind(AssertUnwindSafe(|| {
            engine.tick(16);
            if render_every > 0 && frame % render_every == 0 {
                engine.render();
            }
        }));
        if r.is_err() {
            report(&mut engine);
            eprintln!(
                "PANIC at frame {frame} SEEN{:04} line {} op {:?}",
                engine.machine.scene_number(),
                engine.machine.line,
                op
            );
            std::process::exit(2);
        }
        let errors = &engine.machine.diagnostics.errors;
        for e in &errors[errors_seen..] {
            eprintln!("ERR frame {frame}: {e}");
        }
        errors_seen = errors.len();
        if engine.finished() {
            eprintln!("finished at frame {frame}");
            break;
        }
    }
    report(&mut engine);
}

fn report(engine: &mut Engine) {
    if let Ok(path) = std::env::var("AUTOPLAY_SHOT") {
        let frame = engine.render();
        let _ = image::save_buffer(
            path,
            &frame.rgba,
            frame.width as u32,
            frame.height as u32,
            image::ColorType::Rgba8,
        );
    }
    eprintln!(
        "long ops: {:?}",
        engine
            .machine
            .long_ops
            .iter()
            .map(|(d, op)| (*d, op.name()))
            .collect::<Vec<_>>()
    );
    eprintln!("--- unimplemented ---");
    for (k, v) in &engine.machine.diagnostics.unimplemented {
        eprintln!("{v:8} {k}");
    }
    eprintln!("errors: {}", engine.machine.diagnostics.errors.len());
    eprintln!(
        "final: SEEN{:04} line {} stack {}",
        engine.machine.scene_number(),
        engine.machine.line,
        engine.machine.stack.len()
    );
}
