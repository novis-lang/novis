//! The decompression bound: Novis's patch, absent from the published crate.
//!
//! A caller [`set`]s a ceiling and a ratio on its thread before it opens or reads a document, and
//! [`take`]s the result after. While one is set, every stream filter that decompresses stops once
//! its output passes `min(input × ratio, ceiling)`, as the output grows, and fails as a broken
//! stream does. The first such stop is recorded, so the caller can tell a bomb from a stream
//! that was only broken. Without `std` no bound is ever set.

/// Which half of the bound stopped a decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breach {
    /// The output passed the ceiling.
    Ceiling,
    /// The output passed the ratio times its input.
    Ratio,
}

#[cfg(feature = "std")]
std::thread_local! {
    static BOUND: core::cell::Cell<Option<(u64, u64)>> = const { core::cell::Cell::new(None) };
    static BREACH: core::cell::Cell<Option<Breach>> = const { core::cell::Cell::new(None) };
}

/// Bounds every decode on this thread to at most `ceiling` bytes and `ratio` bytes per byte of
/// input, until [`take`].
#[cfg(feature = "std")]
pub fn set(ceiling: u64, ratio: u64) {
    BOUND.set(Some((ceiling, ratio)));
    BREACH.set(None);
}

/// Removes this thread's bound, and returns the first breach since [`set`].
#[cfg(feature = "std")]
pub fn take() -> Option<Breach> {
    BOUND.set(None);
    BREACH.take()
}

/// The most bytes a decode of `input` bytes may produce, and which half gives that number.
pub(crate) fn cap(input: usize) -> (usize, Breach) {
    #[cfg(feature = "std")]
    if let Some((ceiling, ratio)) = BOUND.get() {
        let by_ratio = (input as u64).saturating_mul(ratio);
        let (bytes, half) = if by_ratio < ceiling {
            (by_ratio, Breach::Ratio)
        } else {
            (ceiling, Breach::Ceiling)
        };
        return (usize::try_from(bytes).unwrap_or(usize::MAX), half);
    }
    let _ = input;
    (usize::MAX, Breach::Ceiling)
}

/// Whether a decode passed the bound since [`set`].
pub(crate) fn breached() -> bool {
    #[cfg(feature = "std")]
    return BREACH.get().is_some();
    #[cfg(not(feature = "std"))]
    false
}

/// Whether `output` bytes from a decode of `input` bytes pass the bound. The first that does is
/// recorded for [`take`].
pub(crate) fn passed(output: usize, input: usize) -> bool {
    let (bytes, half) = cap(input);
    if output <= bytes {
        return false;
    }
    #[cfg(feature = "std")]
    if BREACH.get().is_none() {
        BREACH.set(Some(half));
    }
    let _ = half;
    true
}
