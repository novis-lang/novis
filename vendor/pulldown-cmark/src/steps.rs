//! Novis: a count of the parser's work, for the test that holds it linear.
//!
//! `rule:core-classes/markdown-cost` says the work grows linearly with the
//! input, and `work_grows_linearly_over_every_catalogued_slow_input` in
//! `nvs-stdlib` checks it by counting steps rather than timing. A step is one
//! pass of a loop in the block pass, the inline pass, the scanners, the link
//! label reader or the tree, plus one per byte a scanning primitive reads.
//!
//! The count exists only in a build with debug assertions, which is every
//! test build. A release build compiles [`add`] to nothing, so the parser on
//! the request path does no extra work. Each thread counts alone, so tests
//! that run side by side do not mix their counts.

#[cfg(debug_assertions)]
thread_local! {
    static STEPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Adds `n` steps to this thread's count.
#[inline(always)]
pub(crate) fn add(n: usize) {
    #[cfg(debug_assertions)]
    STEPS.with(|steps| steps.set(steps.get().saturating_add(n as u64)));
    #[cfg(not(debug_assertions))]
    let _ = n;
}

/// The steps this thread has counted so far, or `None` in a build without
/// debug assertions, which counts nothing.
#[doc(hidden)]
#[must_use]
pub fn taken() -> Option<u64> {
    #[cfg(debug_assertions)]
    return Some(STEPS.with(std::cell::Cell::get));
    #[cfg(not(debug_assertions))]
    None
}
