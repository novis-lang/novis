//! Times each job of `tests/image_guest.rs` on both arms, the image component in the guest and the
//! same codec core built for the host, and prints the median of each arm and their ratio
//! (`rule:packaging/the-boundary-is-the-cost`). The guest arm's time is one host-to-guest call
//! carrying the whole input, so it includes the copy into guest memory and back.
//!
//! `cargo bench -p nvs-ext --bench image_guest -- --record` writes the figures to
//! `benches/results/image-guest.json`, the file the test reads.

#[path = "../tests/support/image_arms.rs"]
mod arms;

use std::time::Instant;

use arms::{Guest, Inputs, JOBS};

/// Untimed runs of each arm before the timed ones.
const WARMUP: usize = 3;
/// Timed runs of each arm. The median is the figure.
const RUNS: usize = 21;

/// The median, in nanoseconds, of `RUNS` timed calls of `work`.
fn median(mut work: impl FnMut() -> Vec<u8>) -> u64 {
    for _ in 0..WARMUP {
        std::hint::black_box(work());
    }
    let mut times: Vec<u64> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            std::hint::black_box(work());
            u64::try_from(start.elapsed().as_nanos()).expect("a run fits in u64 nanoseconds")
        })
        .collect();
    times.sort_unstable();
    times[RUNS / 2]
}

#[allow(clippy::print_stdout, reason = "a bench reports on stdout")]
fn main() {
    let record = std::env::args().any(|arg| arg == "--record");
    let inputs = Inputs::new();
    let guest = Guest::new();
    let mut jobs = serde_json::Map::new();
    for job in JOBS {
        let guest_ns = median(|| guest.run(job, &inputs));
        let guest_size_ns = median(|| guest.run_size(job, &inputs));
        let native_ns = median(|| arms::native(job, &inputs));
        #[allow(clippy::cast_precision_loss, reason = "a ratio of two medians")]
        let ratio = (guest_ns as f64 / native_ns as f64 * 100.0).round() / 100.0;
        #[allow(
            clippy::cast_precision_loss,
            reason = "milliseconds for a person to read"
        )]
        let ms = |ns: u64| ns as f64 / 1e6;
        println!(
            "{:<12} guest {:>9.3} ms  guest, size only {:>9.3} ms  native {:>9.3} ms  guest/native {ratio:.2}",
            job.key(),
            ms(guest_ns),
            ms(guest_size_ns),
            ms(native_ns),
        );
        jobs.insert(
            job.key().to_owned(),
            serde_json::json!({
                "what": job.what(),
                "guest_median_ns": guest_ns,
                "guest_size_only_median_ns": guest_size_ns,
                "native_median_ns": native_ns,
                "ratio": ratio,
            }),
        );
    }
    guest.end();
    if record {
        let figures = serde_json::json!({
            "how": format!(
                "`cargo bench -p nvs-ext --bench image_guest -- --record`: the median of {RUNS} runs of each arm after {WARMUP} untimed ones, in a release build. The guest arm is one host-to-guest call through nvs-ext's call bridge, so it includes copying the input in and the output out, and a list of bytes crosses as one wasmtime `Val` per byte. `guest_size_only_median_ns` is the same call returning the size header alone, so no pixel crosses back. The native arm is the same crate built for the host, with the SIMD the host's compiler picks."
            ),
            "host": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
            "jobs": jobs,
        });
        let path = nvs_repo::path("benches/results/image-guest.json");
        let mut text = serde_json::to_string_pretty(&figures).expect("the figures serialize");
        text.push('\n');
        std::fs::write(&path, text).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        println!("wrote {}", path.display());
    }
}
