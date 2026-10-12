//! Scripted stress run: plays to a text page, then performs a list of
//! actions given on the command line and checks invariants.
//! Usage: stress <game_root> <action>...
//! Actions: pages:N  rclick  click  key:enter|esc|up|down|pgup|pgdn|space
//!          mouse:X,Y  tick:N  ff:N (fast-forward N frames)  save:S  load:S
//!          prev  shot:PATH  skipall  state

use reallive::engine::{Engine, EngineOptions};
use reallive::input::{Button, InputEvent, Key};

fn click(engine: &mut Engine, b: Button) {
    let render = std::env::var_os("STRESS_RENDER").is_some();
    engine.input(InputEvent::Press(b));
    engine.tick(16);
    if render {
        engine.render();
    }
    engine.input(InputEvent::Release(b));
    engine.tick(16);
    if render {
        engine.render();
    }
}

fn state(engine: &Engine) -> String {
    let m = &engine.machine;
    let t = &m.sys.text;
    format!(
        "SEEN{:04} line {} stack {} backlog {:?}/{} ops {:?} active {} vis {:?} disp_off {:?} hidden_tmp {} iface_hidden {} in_menu {} skip {} ccom_obj {:?} errors {}",
        m.scene_number(),
        m.line,
        m.stack.len(),
        t.backlog_view,
        t.backlog.len(),
        m.long_ops
            .iter()
            .map(|(d, o)| (*d, o.name()))
            .collect::<Vec<_>>(),
        t.active,
        t.states
            .iter()
            .take(4)
            .map(|s| s.visible as u8)
            .collect::<Vec<_>>(),
        t.display_off
            .iter()
            .take(4)
            .map(|&b| b as u8)
            .collect::<Vec<_>>(),
        t.hidden_temporarily,
        m.sys.gfx.interface_hidden,
        m.sys.in_menu,
        m.sys.syscom.skip_mode,
        m.sys.gfx.ccom_hidden_objects,
        m.diagnostics.errors.len(),
    )
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = args.next().expect("game root");
    let mut options = EngineOptions::headless(root);
    options.fonts = true;
    if let Ok(dir) = std::env::var("STRESS_SAVE_DIR") {
        options.persist = true;
        options.save_dir = Some(dir.into());
    }
    let real = std::env::var_os("STRESS_REAL").is_some();
    let render_all = std::env::var_os("STRESS_RENDER").is_some();
    if real {
        options.audio = true;
        options.virtual_clock = false;
    }
    let mut engine = Engine::open(options).expect("open");
    for _ in 0..1200 {
        engine.tick(16);
    }
    let mut errors_seen = 0;
    for action in args {
        let (name, arg) = action.split_once(':').unwrap_or((&action, ""));
        match name {
            "pages" => {
                let mut pages: usize = arg.parse().unwrap();
                for _ in 0..200_000 {
                    if engine.waiting_for_input() {
                        if pages == 0 {
                            break;
                        }
                        pages -= 1;
                        if engine.machine.scene_number() == 9012 {
                            engine.machine.sys.input.mouse = (670, 479);
                        }
                        if engine.machine.current_long_op().map(|o| o.name()) == Some("select") {
                            engine.input(InputEvent::KeyDown(Key::Down));
                            engine.input(InputEvent::KeyDown(Key::Enter));
                        }
                        click(&mut engine, Button::Left);
                    } else {
                        engine.tick(16);
                    }
                }
            }
            // Plays until a selection is up (or N frames pass).
            "toselect" => {
                for _ in 0..arg.parse::<usize>().unwrap_or(200_000) {
                    let op = engine.machine.current_long_op().map(|o| o.name());
                    if matches!(op, Some("select" | "select_objbtn"))
                        && engine.machine.scene_number() < 9000
                    {
                        break;
                    }
                    if engine.waiting_for_input() && op == Some("pause") {
                        click(&mut engine, Button::Left);
                    } else if op == Some("movie") {
                        std::thread::sleep(std::time::Duration::from_millis(16));
                        engine.tick(16);
                    } else if (8500..9000).contains(&engine.machine.scene_number()) {
                        click(&mut engine, Button::Left);
                        for _ in 0..8 {
                            engine.tick(16);
                        }
                    } else {
                        engine.tick(16);
                    }
                }
            }
            "hit" => {
                for x in [525, 555, 587, 617, 648, 679, 710, 741] {
                    engine.machine.sys.input.mouse = (x, 585);
                    eprintln!(
                        "    hit {x}: {:?}",
                        reallive::window_buttons::hit(&mut engine.machine.sys)
                    );
                }
            }
            "title" => {
                reallive::modules::menu::return_to_menu(&mut engine.machine, true).unwrap();
                engine.tick(16);
            }
            "ips" => {
                for _ in 0..arg.parse::<usize>().unwrap() {
                    engine.tick(16);
                    let m = &engine.machine;
                    eprintln!(
                        "    now {} ip {:?} yield {} ops {}",
                        m.sys.now(),
                        m.stack
                            .iter()
                            .map(|f| (f.scenario.number, f.ip))
                            .collect::<Vec<_>>(),
                        m.yield_frame,
                        m.long_ops.len()
                    );
                }
            }
            "objs" => {
                let g = &engine.machine.sys.gfx;
                let v: Vec<_> =
                    g.fg.iter()
                        .enumerate()
                        .filter(|(_, o)| {
                            o.as_ref()
                                .is_some_and(|o| o.params.visible && o.data.is_some())
                        })
                        .map(|(i, _)| i)
                        .collect();
                eprintln!("    visible objects {v:?}");
            }
            "rclick" => click(&mut engine, Button::Right),
            "click" => click(&mut engine, Button::Left),
            "press" => {
                engine.input(InputEvent::Press(Button::Left));
                engine.tick(16);
            }
            "release" => {
                engine.input(InputEvent::Release(Button::Left));
                engine.tick(16);
            }
            "wheelup" => click(&mut engine, Button::WheelUp),
            "wheeldown" => click(&mut engine, Button::WheelDown),
            "key" => {
                let key = match arg {
                    "enter" => Key::Enter,
                    "esc" => Key::Escape,
                    "up" => Key::Up,
                    "down" => Key::Down,
                    "pgup" => Key::PageUp,
                    "pgdn" => Key::PageDown,
                    "space" => Key::Space,
                    "ctrl" => Key::Ctrl,
                    other => Key::Char(other.chars().next().unwrap()),
                };
                engine.input(InputEvent::KeyDown(key));
                engine.tick(16);
                engine.input(InputEvent::KeyUp(key));
                engine.tick(16);
            }
            "mouse" => {
                let (x, y) = arg.split_once(',').unwrap();
                engine.machine.sys.input.mouse = (x.parse().unwrap(), y.parse().unwrap());
                engine.tick(16);
            }
            "tick" => {
                let t = std::time::Instant::now();
                let n = arg.parse::<usize>().unwrap();
                for _ in 0..n {
                    let frame = std::time::Instant::now();
                    engine.tick(16);
                    if render_all {
                        engine.render();
                    }
                    if real {
                        std::thread::sleep(std::time::Duration::from_millis(16));
                    }
                    if frame.elapsed() > std::time::Duration::from_millis(200) {
                        eprintln!(
                            "    SLOW TICK {:?} at SEEN{:04} line {}",
                            frame.elapsed(),
                            engine.machine.scene_number(),
                            engine.machine.line
                        );
                    }
                }
                eprintln!(
                    "    {n} ticks took {:?} ({:?}/tick)",
                    t.elapsed(),
                    t.elapsed() / n.max(1) as u32
                );
            }
            "ff" => {
                engine.machine.sys.fast_forward = true;
                for _ in 0..arg.parse::<usize>().unwrap() {
                    if engine.machine.current_long_op().map(|o| o.name()) == Some("select") {
                        engine.input(InputEvent::KeyDown(Key::Enter));
                    }
                    engine.tick(16);
                }
                engine.machine.sys.fast_forward = false;
            }
            "skipall" => engine.machine.memory.global.all_read = true,
            "save" => reallive::save::save_slot(&mut engine.machine, arg.parse().unwrap()).unwrap(),
            "load" => {
                reallive::save::load_slot(&mut engine.machine, arg.parse().unwrap()).unwrap();
                engine.tick(16);
            }
            "loadfile" => {
                let bytes = std::fs::read(arg).unwrap();
                reallive::save::restore(&mut engine.machine, &bytes).unwrap();
                engine.tick(16);
            }
            "prev" => {
                let r = reallive::save::return_to_previous_selection(&mut engine.machine).unwrap();
                eprintln!("prev -> {r}");
                engine.tick(16);
            }
            "shot" => {
                let frame = engine.render();
                image::save_buffer(
                    arg,
                    &frame.rgba,
                    frame.width as u32,
                    frame.height as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            "trace" => unsafe { std::env::set_var("REALLIVE_TRACE", arg) },
            "state" => {}
            other => panic!("unknown action {other}"),
        }
        let errors = &engine.machine.diagnostics.errors;
        for e in &errors[errors_seen..] {
            eprintln!("  ERR {e}");
        }
        errors_seen = errors.len();
        eprintln!(
            "[{action}] {} halted {} quit {} polled {} interrupt {:?} mouse_area {:?} log {}",
            state(&engine),
            engine.machine.halted,
            engine.machine.sys.quit_requested,
            engine.machine.sys.polled_buttons.is_some(),
            engine.machine.interrupt,
            engine.machine.sys.mouse_area,
            engine.machine.sys.text.log.len()
        );
        let g = &engine.machine.sys.gfx;
        eprintln!(
            "    hide_buttons {} fg_visible {}",
            g.ccom_hide_buttons,
            g.fg.iter()
                .filter(|o| o.as_ref().is_some_and(|o| o.params.visible))
                .count()
        );
    }
    eprintln!(
        "unimplemented: {:?}",
        engine.machine.diagnostics.unimplemented
    );
}
