//! Dumps scenarios: one element per line with its index.
//! Usage: dump <game_root> <scene>...
use reallive::bytecode::Element;
fn main() {
    let mut args = std::env::args().skip(1);
    let root: std::path::PathBuf = args.next().unwrap().into();
    let engine =
        reallive::engine::Engine::open(reallive::engine::EngineOptions::headless(root)).unwrap();
    let nls = engine.machine.archive.nls;
    for scene in args {
        let scenario = engine
            .machine
            .archive
            .scenario(scene.parse().unwrap())
            .unwrap();
        for (i, e) in scenario.script.elements.iter().enumerate() {
            match e {
                Element::Textout(raw) => {
                    println!("{scene}:{i} TEXT {}", nls.decode(&Element::text(raw, nls)))
                }
                Element::Command(c) => println!(
                    "{scene}:{i} {} {:?}",
                    reallive::opcodes::name(c.op).unwrap_or("?"),
                    c
                ),
                other => println!("{scene}:{i} {other:?}"),
            }
        }
    }
}
