//! Robustness hunt for the asset decoders: decodes every real file under a
//! directory, then mutated copies of them, and reports panics (a damaged
//! file must give an error, not take the player down).
//!
//! Usage: fuzz_assets <dir> <out_dir> [mutations_per_file] [seed]

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use siglus_assets::g00::G00;
use siglus_assets::nwa::NwaReader;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    G00,
    Nwa,
}

/// Refuses inputs whose declared size would need an absurd allocation, so
/// an allocation failure (an abort, not a panic) does not end the run.
const MAX_PIXELS: u64 = 16384 * 16384;

fn decode(kind: Kind, bytes: &[u8]) -> Result<(), String> {
    match kind {
        Kind::G00 => {
            let g = G00::parse(bytes).map_err(|e| format!("{e:#}"))?;
            for cut in &g.cuts {
                for chip in &cut.chips {
                    if u64::from(chip.width) * u64::from(chip.height) > MAX_PIXELS {
                        return Err("huge chip (skipped)".into());
                    }
                    chip.decode_rgba().map_err(|e| format!("{e:#}"))?;
                }
            }
        }
        Kind::Nwa => {
            let mut r = NwaReader::open_from_bytes(bytes.to_vec()).map_err(|e| format!("{e:#}"))?;
            let h = r.header();
            if u64::from(h.frame_count()) > 48_000 * 60 * 30 {
                return Err("implausible frame count (skipped)".into());
            }
            r.to_wav_bytes().map_err(|e| format!("{e:#}"))?;
        }
    }
    Ok(())
}

fn kind_of(path: &Path) -> Option<Kind> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "g00" => Some(Kind::G00),
        "nwa" => Some(Kind::Nwa),
        _ => None,
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Game folders can link to themselves.
        if entry.file_type().is_ok_and(|t| t.is_symlink()) {
            continue;
        }
        if path.is_dir() {
            walk(&path, out);
        } else if kind_of(&path).is_some() {
            out.push(path);
        }
    }
}

fn mutate(rng: &mut Rng, input: &[u8], other: &[u8]) -> Vec<u8> {
    let mut b = input.to_vec();
    if b.is_empty() {
        return b;
    }
    for _ in 0..1 + rng.below(4) {
        match rng.below(7) {
            0 => {
                let i = rng.below(b.len());
                b[i] ^= 1 << rng.below(8);
            }
            1 => {
                let i = rng.below(b.len());
                b[i] = rng.next() as u8;
            }
            2 => {
                let len = rng.below(b.len()) + 1;
                b.truncate(len);
            }
            3 => {
                // Header fields: overwrite a little-endian word near the start.
                let i = rng.below(b.len().min(64));
                let v = [0u32, 1, 0xffff, 0xffff_ffff, 0x7fff_ffff, 0x8000_0000][rng.below(6)];
                for (k, byte) in v.to_le_bytes().iter().enumerate() {
                    if let Some(slot) = b.get_mut(i + k) {
                        *slot = *byte;
                    }
                }
            }
            4 => {
                let i = rng.below(b.len());
                let n = rng.below(64);
                b.splice(i..i, std::iter::repeat_n(rng.next() as u8, n));
            }
            5 if !other.is_empty() => {
                let i = rng.below(b.len());
                let j = rng.below(other.len());
                b.truncate(i);
                b.extend_from_slice(&other[j..]);
            }
            _ => {
                let i = rng.below(b.len());
                let j = (i + rng.below(32)).min(b.len());
                b.drain(i..j);
            }
        }
    }
    b
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "<non-string panic>".into())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().expect("dir"));
    let out = PathBuf::from(args.next().expect("out dir"));
    let per_file: usize = args.next().map_or(20, |s| s.parse().unwrap());
    let seed: u64 = args.next().map_or(1, |s| s.parse().unwrap());
    std::fs::create_dir_all(&out).unwrap();
    std::panic::set_hook(Box::new(|_| {}));

    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort();
    eprintln!("{} files", files.len());

    // Phase 1: the real files.
    let mut real: BTreeMap<(Kind, String), usize> = BTreeMap::new();
    let mut corpus: Vec<(Kind, Vec<u8>)> = Vec::new();
    for path in &files {
        let kind = kind_of(path).unwrap();
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let result = catch_unwind(AssertUnwindSafe(|| decode(kind, &bytes)));
        let key = match result {
            Ok(Ok(())) => "ok".to_owned(),
            Ok(Err(e)) => {
                if real.get(&(kind, format!("err: {e}"))).is_none() {
                    eprintln!("real {kind:?} error: {} → {e}", path.display());
                }
                format!("err: {e}")
            }
            Err(p) => {
                let msg = panic_message(&p);
                eprintln!("REAL PANIC {kind:?} {}: {msg}", path.display());
                format!("PANIC: {msg}")
            }
        };
        *real
            .entry((kind, key.chars().take(120).collect()))
            .or_default() += 1;
        corpus.push((kind, bytes));
    }
    eprintln!("--- real files ---");
    for ((kind, key), n) in &real {
        eprintln!("{n:6} {kind:?} {key}");
    }

    // Phase 2: mutations.
    let mut rng = Rng(seed);
    let mut panics: BTreeMap<String, usize> = BTreeMap::new();
    let mut runs = 0usize;
    let picks = corpus.len().min(600);
    for _ in 0..picks {
        let (kind, base) = &corpus[rng.below(corpus.len())];
        let other = &corpus[rng.below(corpus.len())].1;
        for _ in 0..per_file {
            let input = mutate(&mut rng, base, other);
            runs += 1;
            if let Err(p) = catch_unwind(AssertUnwindSafe(|| decode(*kind, &input))) {
                let msg = panic_message(&p);
                let key = format!("{kind:?}: {msg}");
                let distinct = panics.len();
                let count = panics.entry(key.clone()).or_default();
                if *count == 0 {
                    let name = format!("{kind:?}-{}.bin", distinct + 1);
                    std::fs::write(out.join(&name), &input).unwrap();
                    eprintln!("MUTATION PANIC {key} (saved {name})");
                }
                *count += 1;
            }
        }
    }
    eprintln!(
        "--- {runs} mutated inputs, {} distinct panics ---",
        panics.len()
    );
    for (key, n) in &panics {
        eprintln!("{n:6} {key}");
    }
}
