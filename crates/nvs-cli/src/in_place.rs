//! The hidden `--no-in-place` and `--no-cheaper-forms` of `nvs run`,
//! `nvs test` and `nvs serve` (`rule:types/a-replaced-value-is-changed-in-place`,
//! `rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`).
//!
//! `--no-in-place` switches in-place changes off for every program the
//! process lowers, so every owning call sees a shared subject and builds its
//! result. `--no-cheaper-forms` compiles every form as written, so no row of
//! the table of cheaper forms is applied. Both are for the differential checks
//! alone: `bun nv suite --no-in-place-too` and `--no-cheaper-forms-too` run
//! every case both ways and fail on any difference in output. Neither is in
//! `--help`.
//!
//! One process runs one command, so each switch is process-wide: [`set`] is
//! called once, before the command runs, and [`lower`] reads both around each
//! lowering, on whichever thread compiles. The IR's own switches,
//! `nvs_ir::lower::without_in_place` and `without_cheaper_forms`, are scoped to
//! a thread and a call, so a test that lowers both ways in one process needs
//! no global state.
//!
//! A `.nvst` case runs its program in an `nvs` process of its own, so
//! `nvs test` hands each switch on as [`VARIABLE`] and [`FORMS_VARIABLE`], as
//! `--probes` does with its own variable. The variables are not in `--help`
//! either.

use std::sync::atomic::{AtomicBool, Ordering};

/// The variable that switches in-place changes off for every program the
/// process lowers. It works on presence.
pub(crate) const VARIABLE: &str = "NOVIS_NO_IN_PLACE";

/// The variable that switches the cheaper forms off for every program the
/// process lowers. It works on presence.
pub(crate) const FORMS_VARIABLE: &str = "NOVIS_NO_CHEAPER_FORMS";

/// Whether this process lowers with in-place changes switched off.
static OFF: AtomicBool = AtomicBool::new(false);

/// Whether this process lowers with the cheaper forms switched off.
static FORMS_OFF: AtomicBool = AtomicBool::new(false);

/// The flags, flattened into each command that compiles and runs a program.
#[derive(clap::Args, Clone, Copy, Debug, Default)]
pub(crate) struct Switch {
    /// Switch off in-place changes, for the differential checks.
    #[arg(long = "no-in-place", hide = true)]
    no_in_place: bool,
    /// Compile every form as written, for the differential checks.
    #[arg(long = "no-cheaper-forms", hide = true)]
    no_cheaper_forms: bool,
}

/// Records the command's switches, or their variables, for every lowering
/// this process does.
pub(crate) fn set(switch: Switch) {
    let off = switch.no_in_place || std::env::var_os(VARIABLE).is_some();
    OFF.store(off, Ordering::Relaxed);
    let forms_off = switch.no_cheaper_forms || std::env::var_os(FORMS_VARIABLE).is_some();
    FORMS_OFF.store(forms_off, Ordering::Relaxed);
}

/// Whether this process lowers with in-place changes switched off.
pub(crate) fn is_off() -> bool {
    OFF.load(Ordering::Relaxed)
}

/// Whether this process lowers with the cheaper forms switched off.
pub(crate) fn forms_off() -> bool {
    FORMS_OFF.load(Ordering::Relaxed)
}

/// Runs `lower` under the process's switches: inside
/// `nvs_ir::lower::without_in_place` when `--no-in-place` was given, inside
/// `nvs_ir::lower::without_cheaper_forms` when `--no-cheaper-forms` was, and
/// as it is otherwise.
pub(crate) fn lower<R>(lower: impl FnOnce() -> R) -> R {
    let in_place = move || {
        if is_off() {
            nvs_ir::lower::without_in_place(lower)
        } else {
            lower()
        }
    };
    if forms_off() {
        nvs_ir::lower::without_cheaper_forms(in_place)
    } else {
        in_place()
    }
}
