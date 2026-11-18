//! The hidden `--no-in-place` of `nvs run`, `nvs test` and `nvs serve`
//! (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! It switches in-place changes off for every program the process lowers,
//! so every owning call sees a shared subject and builds its result. It is
//! for the differential checks alone: `bun nv suite --no-in-place-too` runs
//! every case both ways and fails on any difference in output. It is not in
//! `--help`.
//!
//! One process runs one command, so the switch is process-wide: [`set`] is
//! called once, before the command runs, and [`lower`] reads it around each
//! lowering, on whichever thread compiles. The IR's own switch,
//! `nvs_ir::lower::without_in_place`, is scoped to a thread and a call, so a
//! test that lowers both ways in one process needs no global state.
//!
//! A `.nvst` case runs its program in an `nvs` process of its own, so
//! `nvs test --no-in-place` hands the switch on as [`VARIABLE`], as
//! `--probes` does with its own variable. The variable is not in `--help`
//! either.

use std::sync::atomic::{AtomicBool, Ordering};

/// The variable that switches in-place changes off for every program the
/// process lowers. It works on presence.
pub(crate) const VARIABLE: &str = "NOVIS_NO_IN_PLACE";

/// Whether this process lowers with in-place changes switched off.
static OFF: AtomicBool = AtomicBool::new(false);

/// The flag, flattened into each command that compiles and runs a program.
#[derive(clap::Args, Clone, Copy, Debug, Default)]
pub(crate) struct Switch {
    /// Switch off in-place changes, for the differential checks.
    #[arg(long = "no-in-place", hide = true)]
    no_in_place: bool,
}

/// Records the command's switch, or [`VARIABLE`], for every lowering this
/// process does.
pub(crate) fn set(switch: Switch) {
    let off = switch.no_in_place || std::env::var_os(VARIABLE).is_some();
    OFF.store(off, Ordering::Relaxed);
}

/// Whether this process lowers with in-place changes switched off.
pub(crate) fn is_off() -> bool {
    OFF.load(Ordering::Relaxed)
}

/// Runs `lower` under the process's switch: inside
/// `nvs_ir::lower::without_in_place` when `--no-in-place` was given, and as
/// it is otherwise.
pub(crate) fn lower<R>(lower: impl FnOnce() -> R) -> R {
    if is_off() {
        nvs_ir::lower::without_in_place(lower)
    } else {
        lower()
    }
}
