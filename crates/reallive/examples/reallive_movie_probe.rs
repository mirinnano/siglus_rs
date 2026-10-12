//! Measures how long a movie's audio takes to become ready.
use reallive::movie::Movie;
fn main() {
    let path = std::env::args().nth(1).expect("movie path");
    let t = std::time::Instant::now();
    let mut movie = Movie::open(path.into(), None, false, 0);
    loop {
        let now = t.elapsed().as_millis() as u64;
        movie.update(now);
        if movie.started() {
            println!(
                "started after {} ms, audio {}",
                now,
                movie.audio_ready.is_some()
            );
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
