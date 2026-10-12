//! Robustness hunt for the X file parser: parses the sample files and
//! mutated copies of them and reports panics.
//!
//! Usage: fuzz_x <dir> <out_dir> [mutations] [seed]

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

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

fn mutate(rng: &mut Rng, input: &[u8]) -> Vec<u8> {
    let mut b = input.to_vec();
    if b.is_empty() {
        return b;
    }
    for _ in 0..1 + rng.below(6) {
        if b.is_empty() {
            break;
        }
        let i = rng.below(b.len());
        match rng.below(6) {
            0 => b[i] ^= 1 << rng.below(8),
            1 => b[i] = b"0123456789-.{};, \n\"eE"[rng.below(21)],
            2 => b.truncate(i + 1),
            3 => {
                let digits = ["999999999", "-1", "0", "4294967295", "1e308", "nan"][rng.below(6)];
                b.splice(i..i, digits.bytes());
            }
            4 => {
                let j = (i + rng.below(16)).min(b.len());
                b.drain(i..j);
            }
            _ => b[i] = rng.next() as u8,
        }
    }
    b
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().expect("dir"));
    let out = PathBuf::from(args.next().expect("out dir"));
    let count: usize = args.next().map_or(20000, |s| s.parse().unwrap());
    let seed: u64 = args.next().map_or(1, |s| s.parse().unwrap());
    std::fs::create_dir_all(&out).unwrap();
    if std::env::var_os("FUZZ_QUIET").is_some() {
        std::panic::set_hook(Box::new(|_| {}));
    }
    let mut corpus = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let bytes = std::fs::read(entry.path()).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            shion_xfile::parse_x(&bytes).map(|_| ())
        }));
        eprintln!(
            "{}: {}",
            entry.path().display(),
            match &result {
                Ok(Ok(())) => "ok".to_owned(),
                Ok(Err(e)) => format!("error: {e:#}"),
                Err(_) => "PANIC".to_owned(),
            }
        );
        corpus.push(bytes);
    }
    let mut rng = Rng(seed);
    let mut panics: BTreeMap<String, usize> = BTreeMap::new();
    for _ in 0..count {
        let base = &corpus[rng.below(corpus.len())];
        let input = mutate(&mut rng, base);
        if let Err(p) = catch_unwind(AssertUnwindSafe(|| {
            shion_xfile::parse_x(&input).map(|_| ())
        })) {
            let msg = p
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            let distinct = panics.len();
            let n = panics.entry(msg.clone()).or_default();
            if *n == 0 {
                std::fs::write(out.join(format!("x-{}.bin", distinct + 1)), &input).unwrap();
                eprintln!("MUTATION PANIC {msg}");
            }
            *n += 1;
        }
    }
    eprintln!(
        "--- {count} mutated inputs, {} distinct panics ---",
        panics.len()
    );
    for (msg, n) in &panics {
        eprintln!("{n:6} {msg}");
    }
}
