//! The worker pool the corpus sweeps share: `lossless.rs` and `prefixes.rs`.
//!
//! libtest runs each test on one thread, and a sweep over every corpus file is
//! one test, so the files are shared out over every core here instead.

use std::panic::resume_unwind;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// `work` applied to every item of `items`, its results in `items`' order.
///
/// The workers share nothing but the index of the next item, and sorting by
/// that index gives back the order a single thread would report in. A panic in
/// `work` is raised again on the calling thread, so it fails the test as it
/// would have serially.
pub(crate) fn map_ordered<I: Sync, T: Send>(items: &[I], work: impl Fn(&I) -> T + Sync) -> Vec<T> {
    let workers = thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(items.len())
        .max(1);
    let next = AtomicUsize::new(0);
    let mut done: Vec<(usize, T)> = thread::scope(|scope| {
        // Collected before any is joined, or each worker would be spawned and
        // joined in turn -- the serial loop again.
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(index) else {
                            break;
                        };
                        mine.push((index, work(item)));
                    }
                    mine
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap_or_else(|panic| resume_unwind(panic)))
            .collect()
    });
    done.sort_unstable_by_key(|(index, _)| *index);
    done.into_iter().map(|(_, result)| result).collect()
}
