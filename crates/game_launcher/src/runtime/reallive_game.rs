use std::path::Path;

use anyhow::Result;
use reallive::engine::{Engine, EngineOptions};
use reallive::input::{Button, InputEvent, Key};

use super::{FramebufferGame, GameKey, PointerButton};
use crate::nls::Nls;

pub struct RealLiveGame {
    engine: Engine,
    frame: Vec<u8>,
    size: (u32, u32),
    ended: bool,
    /// Debug state dump: last signature written and when.
    last_state: (String, Option<std::time::Instant>),
}

impl RealLiveGame {
    pub fn open(root: &Path, nls: Nls) -> Result<Self> {
        let mut options = EngineOptions::new(root);
        options.nls = nls.to_reallive();
        let engine = Engine::open(options)?;
        let size = (
            engine.machine.sys.gfx.width.max(1) as u32,
            engine.machine.sys.gfx.height.max(1) as u32,
        );
        Ok(Self {
            engine,
            frame: vec![0; (size.0 * size.1 * 4) as usize],
            size,
            ended: false,
            last_state: (String::new(), None),
        })
    }
}

fn map_key(key: GameKey) -> Key {
    match key {
        GameKey::Enter => Key::Enter,
        GameKey::Escape => Key::Escape,
        GameKey::Space => Key::Space,
        GameKey::Up => Key::Up,
        GameKey::Down => Key::Down,
        GameKey::Left => Key::Left,
        GameKey::Right => Key::Right,
        GameKey::PageUp => Key::PageUp,
        GameKey::PageDown => Key::PageDown,
        GameKey::Home => Key::Home,
        GameKey::End => Key::End,
        GameKey::Backspace => Key::Backspace,
        GameKey::Tab => Key::Tab,
        GameKey::Ctrl => Key::Ctrl,
        GameKey::Shift => Key::Shift,
        GameKey::F(n) => Key::F(n),
        GameKey::Char(c) => Key::Char(c),
    }
}

impl RealLiveGame {
    /// Appends a line to `reallive_debug.log` next to the saves.
    fn debug_log(&self, line: &str) {
        use std::io::Write;
        let dir = reallive::save::save_dir(&self.engine.machine);
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("reallive_debug.log"))
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0.0, |d| d.as_secs_f64());
            let _ = writeln!(file, "{now:.3} {line}");
        }
    }
}

impl FramebufferGame for RealLiveGame {
    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn step(&mut self, dt_ms: u32) -> bool {
        if self.ended {
            return false;
        }
        let started = std::time::Instant::now();
        self.engine.tick(u64::from(dt_ms));
        let ticked = started.elapsed();
        let errors: Vec<String> = self.engine.machine.diagnostics.errors.drain(..).collect();
        for error in &errors {
            log::warn!("reallive: {error}");
            self.debug_log(&format!("error: {error}"));
        }
        if self.engine.finished() {
            self.shutdown();
            return false;
        }
        let surface = self.engine.render();
        let total = started.elapsed();
        // `REALLIVE_DEBUG_STATE`: what the engine is doing, every 2 s.
        if std::env::var_os("REALLIVE_DEBUG_STATE").is_some()
            && self
                .last_state
                .1
                .is_none_or(|at| at.elapsed() >= std::time::Duration::from_secs(2))
        {
            let m = &self.engine.machine;
            let t = &m.sys.text;
            let state = format!(
                "state: SEEN{:04} line {} stack {:?} long ops {:?} in_menu {} halted {} window {} visible {} hidden_tmp {} backlog {:?} skip {} auto {} movie {} polled {} selection {} events {:?} left_held {} transition {}",
                m.scene_number(),
                m.line,
                m.stack
                    .iter()
                    .map(|f| (f.scenario.number, f.ip))
                    .collect::<Vec<_>>(),
                m.long_ops
                    .iter()
                    .map(|(d, op)| (*d, op.name()))
                    .collect::<Vec<_>>(),
                m.sys.in_menu,
                m.halted,
                t.active,
                t.window_shown(t.active),
                t.hidden_temporarily,
                t.backlog_view,
                m.sys.syscom.skip_mode,
                m.sys.settings.auto_mode,
                m.sys.movie.is_some(),
                m.sys.polled_buttons.is_some(),
                m.sys.selection.is_some(),
                m.sys.input.events,
                m.sys.input.left_held,
                m.sys.gfx.transition_frame.is_some(),
            );
            if state != self.last_state.0 {
                self.debug_log(&state);
                self.last_state.0 = state;
            }
            self.last_state.1 = Some(std::time::Instant::now());
        }
        if total > std::time::Duration::from_millis(500) {
            let m = &self.engine.machine;
            let note = format!(
                "slow frame: tick {:?}, render {:?}; SEEN{:04} line {} stack {:?} long ops {:?} movie {}",
                ticked,
                total - ticked,
                m.scene_number(),
                m.line,
                m.stack
                    .iter()
                    .map(|f| (f.scenario.number, f.ip))
                    .collect::<Vec<_>>(),
                m.long_ops
                    .iter()
                    .map(|(d, op)| (*d, op.name()))
                    .collect::<Vec<_>>(),
                m.sys.movie.is_some(),
            );
            self.debug_log(&note);
        }
        self.size = (surface.width.max(1) as u32, surface.height.max(1) as u32);
        self.frame = surface.rgba;
        true
    }

    fn frame(&self) -> &[u8] {
        &self.frame
    }

    fn pointer_move(&mut self, x: i32, y: i32) {
        self.engine.machine.sys.input.mouse = (x, y);
    }

    fn pointer_button(&mut self, button: PointerButton, pressed: bool) {
        let button = match button {
            PointerButton::Left => Button::Left,
            PointerButton::Right => Button::Right,
        };
        self.engine.input(if pressed {
            InputEvent::Press(button)
        } else {
            InputEvent::Release(button)
        });
    }

    fn wheel(&mut self, up: bool) {
        self.engine.input(InputEvent::Press(if up {
            Button::WheelUp
        } else {
            Button::WheelDown
        }));
    }

    fn key(&mut self, key: GameKey, pressed: bool) {
        let key = map_key(key);
        self.engine.input(if pressed {
            InputEvent::KeyDown(key)
        } else {
            InputEvent::KeyUp(key)
        });
    }

    fn text(&mut self, text: &str) {
        self.engine.input(InputEvent::Text(text.to_owned()));
    }

    fn cursor_visible(&self) -> bool {
        self.engine.machine.sys.cursor_visible
    }

    fn title(&self) -> String {
        let sys = &self.engine.machine.sys;
        if sys.title.is_empty() {
            sys.gameexe.str("CAPTION").unwrap_or("RealLive").to_owned()
        } else {
            sys.title.clone()
        }
    }

    fn shutdown(&mut self) {
        if !self.ended {
            reallive::save::save_global(&self.engine.machine).ok();
        }
        self.ended = true;
    }
}
