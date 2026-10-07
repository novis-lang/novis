//! The image component in the guest against the same codec core built for the host
//! (`rule:packaging/the-boundary-is-the-cost`): a resize and a JPEG decode give the same bytes on
//! both arms, so the bench in `benches/image_guest.rs` times the same work, and the figures it
//! committed to `benches/results/image-guest.json` name both arms of every job.

#[path = "support/image_arms.rs"]
#[allow(
    dead_code,
    reason = "the bench reads the job descriptions this test does not"
)]
mod arms;

use arms::{Guest, Inputs, JOBS, Job};

/// `job`'s output on each arm, which are the same bytes.
fn agree(job: Job) {
    let inputs = Inputs::new();
    let guest = Guest::new();
    let from_guest = guest.run(job, &inputs);
    guest.end();
    let from_native = arms::native(job, &inputs);
    assert_eq!(from_guest.len(), from_native.len(), "{job:?}");
    let first = from_guest
        .iter()
        .zip(&from_native)
        .position(|(a, b)| a != b);
    assert_eq!(
        first, None,
        "{job:?}: the arms first differ at byte {first:?}"
    );
}

#[test]
fn the_guest_and_native_resize_agree_byte_for_byte() {
    agree(Job::Resize);
}

#[test]
fn the_guest_and_native_jpeg_decode_agree_byte_for_byte() {
    agree(Job::JpegDecode);
}

#[test]
fn the_committed_guest_figures_name_every_arm() {
    let path = nvs_repo::path("benches/results/image-guest.json");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let figures: serde_json::Value = serde_json::from_str(&text).expect("the figures are JSON");
    for job in JOBS {
        let entry = &figures["jobs"][job.key()];
        let median = |arm: &str| {
            entry[arm]
                .as_u64()
                .filter(|ns| *ns > 0)
                .unwrap_or_else(|| panic!("`{}` has no `{arm}`", job.key()))
        };
        let (guest, native) = (median("guest_median_ns"), median("native_median_ns"));
        let ratio = entry["ratio"]
            .as_f64()
            .unwrap_or_else(|| panic!("`{}` has no `ratio`", job.key()));
        #[allow(clippy::cast_precision_loss, reason = "a ratio of two medians")]
        let measured = guest as f64 / native as f64;
        assert!(
            (ratio - measured).abs() < 0.01,
            "`{}`'s ratio {ratio} is not its medians' {measured:.2}",
            job.key()
        );
    }
}
