//! Randomised bug hunt: drives a game through random sequences of player
//! actions and checks invariants after each one. Every run is reproducible
//! from its seed.
//!
//! Usage: fuzz <game_root> <seed> <steps> [out_dir]
//! `FUZZ_PREFIX` holds stress-style actions that bring the game into play
//! (title screen clicks); `FUZZ_MENU` lists `x,y` points of the in-game
//! menu's items; `FUZZ_BAR` the window's button bar.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use reallive::engine::{Engine, EngineOptions};
use reallive::input::{Button, InputEvent, Key};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

/// What a save captured, to compare against after loading it.
#[derive(Debug, Clone)]
struct Saved {
    stack: Vec<(i32, usize)>,
    bgm: String,
    step: usize,
}

struct Hunt {
    engine: Engine,
    rng: Rng,
    history: Vec<String>,
    errors_seen: usize,
    saves: std::collections::HashMap<i32, Saved>,
    out: PathBuf,
    seed: u64,
    findings: usize,
    menu: Vec<(i32, i32)>,
    bar: Vec<(i32, i32)>,
    /// Last progress signature and since when it has not changed.
    progress: (usize, i32, usize),
    stalled_for: usize,
    seen: std::collections::BTreeSet<String>,
}

/// Foreground objects that would be drawn.
fn visible_objects(engine: &Engine) -> Vec<usize> {
    engine
        .machine
        .sys
        .gfx
        .fg
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            o.as_ref()
                .is_some_and(|o| o.params.visible && o.data.is_some())
        })
        .map(|(i, _)| i)
        .collect()
}

/// The sections of a slot save that describe the game (not its date or
/// picture).
fn state_sections(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let body = &bytes[8..];
    reallive::serial::Reader::new(body)
        .sections()
        .unwrap_or_default()
        .into_iter()
        .filter(|(name, _)| !matches!(name.as_str(), "header" | "thumbnail" | "extra"))
        .map(|(name, body)| (name, body.to_vec()))
        .collect()
}

fn op_name(engine: &Engine) -> &'static str {
    engine.machine.current_long_op().map_or("-", |op| op.name())
}

fn points(var: &str, default: &str) -> Vec<(i32, i32)> {
    std::env::var(var)
        .unwrap_or_else(|_| default.to_owned())
        .split_whitespace()
        .filter_map(|p| {
            let (x, y) = p.split_once(',')?;
            Some((x.parse().ok()?, y.parse().ok()?))
        })
        .collect()
}

impl Hunt {
    fn tick(&mut self) -> bool {
        let engine = &mut self.engine;
        let r = catch_unwind(AssertUnwindSafe(|| engine.tick(16)));
        if r.is_err() {
            self.finding("panic", "the engine panicked");
            return false;
        }
        true
    }

    fn ticks(&mut self, n: usize) -> bool {
        for _ in 0..n {
            if !self.tick() {
                return false;
            }
        }
        true
    }

    fn press(&mut self, button: Button) -> bool {
        self.engine.input(InputEvent::Press(button));
        if !self.tick() {
            return false;
        }
        self.engine.input(InputEvent::Release(button));
        self.tick()
    }

    fn key(&mut self, key: Key) -> bool {
        self.engine.input(InputEvent::KeyDown(key));
        if !self.tick() {
            return false;
        }
        self.engine.input(InputEvent::KeyUp(key));
        self.tick()
    }

    fn mouse(&mut self, x: i32, y: i32) {
        self.engine.machine.sys.input.mouse = (x, y);
    }

    fn in_menu(&self) -> bool {
        let m = &self.engine.machine;
        m.sys.in_menu || m.scene_number() >= 9000
    }

    /// Plays on: clicks whenever the game waits.
    fn advance(&mut self, pages: usize) -> bool {
        let mut left = pages;
        for _ in 0..pages * 400 + 200 {
            if self.engine.waiting_for_input() {
                if left == 0 {
                    break;
                }
                left -= 1;
                if op_name(&self.engine) == "select_objbtn" && self.rng.below(2) == 0 {
                    // Object buttons answer to the mouse, not the keys.
                    let w = self.engine.machine.sys.gfx.width as u64;
                    let h = self.engine.machine.sys.gfx.height as u64;
                    let (x, y) = (self.rng.below(w) as i32, self.rng.below(h) as i32);
                    self.mouse(x, y);
                    if !self.tick() || !self.press(Button::Left) {
                        return false;
                    }
                    continue;
                }
                if matches!(op_name(&self.engine), "select" | "select_objbtn") {
                    let k = if self.rng.below(2) == 0 {
                        Key::Down
                    } else {
                        Key::Up
                    };
                    for _ in 0..self.rng.below(3) {
                        if !self.key(k) {
                            return false;
                        }
                    }
                    if !self.key(Key::Enter) {
                        return false;
                    }
                    continue;
                }
                if !self.press(Button::Left) {
                    return false;
                }
            } else if op_name(&self.engine) == "movie" {
                // Movies decode in real time.
                std::thread::sleep(std::time::Duration::from_millis(16));
                if !self.tick() {
                    return false;
                }
            } else if (8500..9000).contains(&self.engine.machine.scene_number()) {
                // Mini-games poll the mouse instead of waiting.
                if !self.press(Button::Left) || !self.ticks(8) {
                    return false;
                }
            } else if !self.tick() {
                return false;
            }
        }
        true
    }

    fn finding(&mut self, kind: &str, detail: &str) {
        let m = &self.engine.machine;
        let key = format!("{kind}: {}", detail.chars().take(160).collect::<String>());
        let first = self.seen.insert(key.clone());
        if !first {
            return;
        }
        self.findings += 1;
        let name = format!("seed{}-{}-{}", self.seed, self.findings, kind);
        let mut report = format!(
            "# {kind}\n{detail}\nseed {} step {} SEEN{:04} line {} stack {:?} op {} in_menu {}\n\nlast actions:\n",
            self.seed,
            self.history.len(),
            m.scene_number(),
            m.line,
            m.stack
                .iter()
                .map(|f| (f.scenario.number, f.ip))
                .collect::<Vec<_>>(),
            op_name(&self.engine),
            m.sys.in_menu,
        );
        for action in self.history.iter().rev().take(25).rev() {
            report.push_str(&format!("  {action}\n"));
        }
        let _ = std::fs::write(self.out.join(format!("{name}.txt")), &report);
        // The last savepoint, to replay from (`stress <game> loadfile:...`).
        let _ = std::fs::write(
            self.out.join(format!("{name}.sav")),
            reallive::save::serialize_slot(&self.engine.machine),
        );
        let engine = &mut self.engine;
        if let Ok(frame) = catch_unwind(AssertUnwindSafe(|| engine.render())) {
            let _ = image::save_buffer(
                self.out.join(format!("{name}.png")),
                &frame.rgba,
                frame.width as u32,
                frame.height as u32,
                image::ColorType::Rgba8,
            );
        }
        eprintln!("FINDING {name}: {detail}");
    }

    fn check(&mut self) {
        // Draw like the player would: window animations end while drawing.
        let engine = &mut self.engine;
        if catch_unwind(AssertUnwindSafe(|| {
            engine.render();
        }))
        .is_err()
        {
            self.finding("panic", "render panicked");
            return;
        }
        // New runtime errors.
        let errors = self.engine.machine.diagnostics.errors[self.errors_seen..].to_vec();
        self.errors_seen = self.engine.machine.diagnostics.errors.len();
        for error in errors {
            if error.contains("is not in") || error.contains("debug message") {
                continue;
            }
            self.finding("error", &error);
        }
        let m = &self.engine.machine;
        let sys = &m.sys;
        let text = &sys.text;
        // The text window vanished while the game waits for a click.
        if op_name(&self.engine) == "pause"
            && !self.in_menu()
            && !text.hidden_temporarily
            && text.backlog_view.is_none()
            && !sys.gfx.interface_hidden
            && !text.window_shown(text.active)
        {
            let detail = format!(
                "pause without a visible window: active {} visible {} display_off {} ccom_off {}",
                text.active,
                text.states[text.active].visible,
                text.display_off[text.active],
                text.ccom_off[text.active]
            );
            self.finding("ui-lost", &detail);
        }
        let m = &self.engine.machine;
        let leak = !m.sys.gfx.ccom_hidden_objects.is_empty()
            || m.sys.gfx.ccom_hide_buttons
            || m.sys.text.ccom_off.iter().any(|&b| b);
        let halted = m.halted && !m.sys.quit_requested;
        if !self.in_menu() && leak {
            self.finding("ccom-leak", "menu-only hiding outlived the menu");
        }
        if halted {
            self.finding("halted", "the machine halted without quitting");
        }
        let m = &self.engine.machine;
        if m.long_ops.iter().any(|(depth, _)| *depth > m.stack.len()) {
            let ops: Vec<_> = m.long_ops.iter().map(|(d, o)| (*d, o.name())).collect();
            self.finding(
                "stale-longop",
                &format!("{ops:?} with stack {}", m.stack.len()),
            );
        }
    }

    fn progress_check(&mut self, advancing: bool) {
        let m = &self.engine.machine;
        let signature = (
            m.sys.text.log.len(),
            m.scene_number(),
            m.stack.last().map_or(0, |f| f.ip),
        );
        if signature != self.progress {
            self.progress = signature;
            self.stalled_for = 0;
        } else if advancing && !self.in_menu() {
            self.stalled_for += 1;
            if self.stalled_for >= 6 {
                let op = op_name(&self.engine);
                self.finding("stuck", &format!("no progress through 6 advances; op {op}"));
                self.stalled_for = 0;
            }
        }
    }

    fn save(&mut self, slot: i32) {
        let m = &mut self.engine.machine;
        match reallive::save::save_slot(m, slot) {
            Ok(()) => {
                let saved = Saved {
                    stack: m
                        .savepoint_stack
                        .iter()
                        .map(|f| (f.scenario.number, f.ip))
                        .collect(),
                    bgm: m
                        .sys
                        .savepoint_bgm
                        .clone()
                        .map(|(n, _)| n)
                        .unwrap_or_default(),
                    step: self.history.len(),
                };
                self.saves.insert(slot, saved);
            }
            Err(e) => self.finding("save-failed", &format!("{e:#}")),
        }
    }

    fn load(&mut self, slot: i32) {
        let Some(saved) = self.saves.get(&slot).cloned() else {
            return;
        };
        let m = &mut self.engine.machine;
        if let Err(e) = reallive::save::load_slot(m, slot) {
            self.finding("load-failed", &format!("{e:#}"));
            return;
        }
        let stack: Vec<_> = m.stack.iter().map(|f| (f.scenario.number, f.ip)).collect();
        if stack != saved.stack {
            self.finding(
                "load-stack",
                &format!("loaded {stack:?}, saved {:?}", saved.stack),
            );
        }
        let now = self.engine.machine.sys.now();
        let bgm = self
            .engine
            .machine
            .sys
            .sound
            .current_track(now)
            .map(|(n, _)| n)
            .unwrap_or_default();
        if bgm != saved.bgm {
            self.finding(
                "load-bgm",
                &format!(
                    "bgm after load {bgm:?}, at save {:?} (saved at step {})",
                    saved.bgm, saved.step
                ),
            );
        }
        if !self.ticks(30) {
            return;
        }
        // The game must go on after a load.
        if !self.advance(2) {
            return;
        }
        if self.engine.machine.halted {
            self.finding("load-halt", "halted after load");
        }
    }

    fn step(&mut self) -> bool {
        let choice = self.rng.below(100);
        let action: String;
        let ok = match choice {
            0..=34 => {
                let n = 1 + self.rng.below(6) as usize;
                action = format!("advance {n}");
                self.history.push(action.clone());
                let ok = self.advance(n);
                self.progress_check(true);
                ok
            }
            35..=42 => {
                let n = 60 + self.rng.below(900) as usize;
                action = format!("fast-forward {n}");
                self.history.push(action.clone());
                self.engine.machine.sys.fast_forward = true;
                let mut ok = true;
                for _ in 0..n {
                    if matches!(op_name(&self.engine), "select" | "select_objbtn") {
                        self.engine.input(InputEvent::KeyDown(Key::Enter));
                    }
                    if !self.tick() {
                        ok = false;
                        break;
                    }
                }
                self.engine.machine.sys.fast_forward = false;
                ok
            }
            43..=46 => {
                let on = !self.engine.machine.sys.syscom.skip_mode;
                action = format!("skip-mode {on} (all read)");
                self.history.push(action.clone());
                self.engine.machine.memory.global.all_read = true;
                self.engine.machine.sys.syscom.skip_mode = on;
                {
                    let n = 1 + self.rng.below(300) as usize;
                    self.ticks(n)
                }
            }
            47..=58 => {
                // The in-game menu, then a few clicks in it.
                let before = (
                    visible_objects(&self.engine),
                    self.engine
                        .machine
                        .stack
                        .iter()
                        .map(|f| (f.scenario.number, f.ip))
                        .collect::<Vec<_>>(),
                    self.in_menu(),
                );
                let mut ok = self.press(Button::Right) && self.ticks(40);
                let mut parts = vec!["rclick".to_owned()];
                for _ in 0..1 + self.rng.below(4) {
                    if !ok {
                        break;
                    }
                    let (x, y) = if self.rng.below(3) > 0 && !self.menu.is_empty() {
                        *self.rng.pick(&self.menu.clone())
                    } else {
                        let w = self.engine.machine.sys.gfx.width as u64;
                        let h = self.engine.machine.sys.gfx.height as u64;
                        (self.rng.below(w) as i32, self.rng.below(h) as i32)
                    };
                    parts.push(format!("click {x},{y}"));
                    self.mouse(x, y);
                    let n = 30 + self.rng.below(90) as usize;
                    ok = self.press(Button::Left) && self.ticks(n);
                }
                // Leave with right clicks.
                for _ in 0..self.rng.below(4) {
                    if !ok {
                        break;
                    }
                    parts.push("rclick".into());
                    ok = self.press(Button::Right) && self.ticks(40);
                }
                action = format!("menu [{}]", parts.join(", "));
                self.history.push(action.clone());
                let after_stack: Vec<_> = self
                    .engine
                    .machine
                    .stack
                    .iter()
                    .map(|f| (f.scenario.number, f.ip))
                    .collect();
                if ok && !before.2 && !self.in_menu() && after_stack == before.1 {
                    let after = visible_objects(&self.engine);
                    if after != before.0 {
                        let lost: Vec<_> = before.0.iter().filter(|i| !after.contains(i)).collect();
                        let gained: Vec<_> =
                            after.iter().filter(|i| !before.0.contains(i)).collect();
                        self.finding(
                            "menu-objects",
                            &format!(
                                "objects changed across the menu: lost {lost:?} gained {gained:?}"
                            ),
                        );
                    }
                }
                ok
            }
            59..=64 => {
                let (x, y) = *self.rng.pick(&self.bar.clone());
                action = format!("bar {x},{y}");
                self.history.push(action.clone());
                self.mouse(x, y);
                self.press(Button::Left) && self.ticks(30)
            }
            65..=69 => {
                let n = 1 + self.rng.below(5);
                action = format!(
                    "backlog up {n} then {}",
                    ["down", "click", "rclick", "esc"][(n % 4) as usize]
                );
                self.history.push(action.clone());
                let mut ok = true;
                for _ in 0..n {
                    ok = ok && self.press(Button::WheelUp);
                }
                ok = ok && self.ticks(10);
                ok && match n % 4 {
                    0 => (0..n + 1).all(|_| self.press(Button::WheelDown)),
                    1 => self.press(Button::Left),
                    2 => self.press(Button::Right),
                    _ => self.key(Key::Escape),
                }
            }
            70..=72 => {
                action = "space-hide then click".into();
                self.history.push(action.clone());
                self.key(Key::Space) && self.ticks(10) && self.press(Button::Left)
            }
            73..=79 => {
                let slot = self.rng.below(3) as i32;
                action = format!("api-save {slot}");
                self.history.push(action.clone());
                self.save(slot);
                if self.rng.below(2) == 0 {
                    // Loading a save and saving again must give the same game.
                    let m = &mut self.engine.machine;
                    let first = reallive::save::serialize_slot(m);
                    if let Err(e) = reallive::save::restore(m, &first) {
                        self.finding("restore-failed", &format!("{e:#}"));
                    } else {
                        let second = reallive::save::serialize_slot(&self.engine.machine);
                        let (a, b) = (state_sections(&first), state_sections(&second));
                        for ((name, x), (_, y)) in a.iter().zip(&b) {
                            if x != y {
                                self.finding(
                                    "save-roundtrip",
                                    &format!("section {name} differs after load+save"),
                                );
                            }
                        }
                        if a.len() != b.len() {
                            self.finding("save-roundtrip", "section count differs");
                        }
                    }
                    self.history.push("  (immediate restore)".into());
                }
                true
            }
            80..=86 => {
                let slot = self.rng.below(3) as i32;
                action = format!("api-load {slot}");
                self.history.push(action.clone());
                self.load(slot);
                true
            }
            87..=90 => {
                action = "api-return-prev-select".into();
                self.history.push(action.clone());
                let m = &mut self.engine.machine;
                match reallive::save::return_to_previous_selection(m) {
                    Ok(jumped) => {
                        let ok = self.ticks(30);
                        if jumped && ok {
                            // Should land at a selection again.
                            let mut found = false;
                            for _ in 0..3000 {
                                let op = op_name(&self.engine);
                                if matches!(op, "select" | "select_objbtn") {
                                    found = true;
                                    break;
                                }
                                if self.engine.waiting_for_input() && op == "pause" {
                                    self.press(Button::Left);
                                } else if !self.tick() {
                                    return false;
                                }
                            }
                            if !found {
                                self.finding(
                                    "prev-select",
                                    "ReturnPrevSelect did not lead back to a selection",
                                );
                            }
                        }
                        ok
                    }
                    Err(e) => {
                        self.finding("prev-select-err", &format!("{e:#}"));
                        true
                    }
                }
            }
            91..=94 => {
                let w = self.engine.machine.sys.gfx.width as u64;
                let h = self.engine.machine.sys.gfx.height as u64;
                let (x, y) = (self.rng.below(w) as i32, self.rng.below(h) as i32);
                action = format!("click {x},{y}");
                self.history.push(action.clone());
                self.mouse(x, y);
                self.press(Button::Left) && self.ticks(5)
            }
            95..=97 => {
                let n = 30 + self.rng.below(400) as usize;
                action = format!("ctrl-held {n}");
                self.history.push(action.clone());
                self.engine.machine.sys.input.ctrl = true;
                let ok = self.ticks(n);
                self.engine.machine.sys.input.ctrl = false;
                ok
            }
            _ => {
                let k = *self.rng.pick(&[
                    Key::Escape,
                    Key::Enter,
                    Key::Up,
                    Key::Down,
                    Key::PageUp,
                    Key::PageDown,
                ]);
                action = format!("key {k:?}");
                self.history.push(action.clone());
                self.key(k)
            }
        };
        if ok {
            self.check();
        }
        ok
    }
}

fn run_prefix(engine: &mut Engine, prefix: &str) {
    for _ in 0..1200 {
        engine.tick(16);
    }
    for action in prefix.split_whitespace() {
        let (name, arg) = action.split_once(':').unwrap_or((action, ""));
        match name {
            "mouse" => {
                let (x, y) = arg.split_once(',').unwrap();
                engine.machine.sys.input.mouse = (x.parse().unwrap(), y.parse().unwrap());
                engine.tick(16);
            }
            "click" => {
                engine.input(InputEvent::Press(Button::Left));
                engine.tick(16);
                engine.input(InputEvent::Release(Button::Left));
                engine.tick(16);
            }
            "tick" => {
                for _ in 0..arg.parse::<usize>().unwrap() {
                    engine.tick(16);
                }
            }
            other => panic!("prefix action {other}"),
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = args.next().expect("game root");
    let seed: u64 = args.next().map_or(1, |s| s.parse().unwrap());
    let steps: usize = args.next().map_or(300, |s| s.parse().unwrap());
    let out = PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    std::fs::create_dir_all(&out).unwrap();
    let mut options = EngineOptions::headless(root);
    options.fonts = std::env::var_os("FUZZ_FONTS").is_some();
    let mut engine = Engine::open(options).expect("open");
    run_prefix(
        &mut engine,
        &std::env::var("FUZZ_PREFIX").unwrap_or_default(),
    );
    let mut hunt = Hunt {
        engine,
        rng: Rng(seed),
        history: Vec::new(),
        errors_seen: 0,
        saves: Default::default(),
        out,
        seed,
        findings: 0,
        menu: points("FUZZ_MENU", ""),
        bar: points("FUZZ_BAR", ""),
        progress: (0, 0, 0),
        stalled_for: 0,
        seen: Default::default(),
    };
    for step in 0..steps {
        if !hunt.step() {
            eprintln!("seed {seed}: stopped at step {step} (panic)");
            break;
        }
        if step % 25 == 0 {
            let m = &hunt.engine.machine;
            eprintln!(
                "seed {seed} step {step}: SEEN{:04} line {} op {} stack {:?}",
                m.scene_number(),
                m.line,
                op_name(&hunt.engine),
                m.stack
                    .iter()
                    .map(|f| f.scenario.number)
                    .collect::<Vec<_>>()
            );
        }
        if hunt.engine.finished() {
            eprintln!("seed {seed}: game finished at step {step}");
            break;
        }
    }
    let frame = hunt.engine.render();
    let _ = image::save_buffer(
        hunt.out.join(format!("seed{seed}-final.png")),
        &frame.rgba,
        frame.width as u32,
        frame.height as u32,
        image::ColorType::Rgba8,
    );
    let m = &hunt.engine.machine;
    eprintln!(
        "seed {seed}: done, {} findings, SEEN{:04}, unimplemented {:?}",
        hunt.findings,
        m.scene_number(),
        m.diagnostics.unimplemented
    );
}
