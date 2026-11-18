//! The copies the ownership pass keeps, and where each is written
//! (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! [`crate::ownership::run`] records a [`KeptCopy`] at each place it leaves a
//! value shared that a change could otherwise have made in place: a separated
//! change in a lent body, a local whose move it refused, a slot it did not
//! lend, and a call whose callee is not known while compiling. Each row has
//! the statement's span, a [`Reason`], and a [`Fix`] where moving one
//! statement removes the copy.
//!
//! The rows are part of the [`crate::ir::Program`], so a unit loaded from the
//! compile cache has them too: a warm hit still lowers the program and runs
//! the pass, and skips only code generation. [`table`] adds what needs the
//! source: the file's name, the line, the file's BLAKE3 and the fix as a text
//! edit.
//!
//! **A row is a place a copy may happen, not one that does.** A callee that
//! never changes its parameter copies nothing. Only [`Reason::warns`] rows
//! are warnings, because each has one edit that removes its copy. The others
//! are shown only where a recorded request counted a copy at them.
//!
//! **What it spends:** one row per kept copy per compiled program, and while
//! compiling one walk back to the statement marker per row.

use nvs_diagnostics::{Diagnostic, SourceMap, Span, code};

/// Why the pass keeps a copy at a statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reason {
    /// A function changes a value it was lent, and then reads the slot the
    /// value came from. The read must see the old value, so the change copies.
    ReadAfterChange,
    /// A function changes a value it was lent, and then something can throw.
    /// The caller must keep the old value when it does, so the change copies.
    FailureAfterChange,
    /// The value is still used after the call, so two values are alive.
    StillUsed,
    /// The property is not lent to the call: it has a hook, a guard or an
    /// observer, or code between its read and its store may run user code or
    /// touch it, or a callback or the callee may write it.
    SlotKept,
    /// The call's target is not known while compiling: a callable, or a
    /// method that a class overrides.
    CalleeUnknown,
}

impl Reason {
    /// Whether a row with this reason is a warning. Only the two reasons a
    /// lent body copies for are, and only where the row has a [`Fix`].
    #[must_use]
    pub const fn warns(self) -> bool {
        matches!(self, Self::ReadAfterChange | Self::FailureAfterChange)
    }

    /// The name a `copies` record and the debug index give this reason.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ReadAfterChange => "read_after_change",
            Self::FailureAfterChange => "failure_after_change",
            Self::StillUsed => "still_used",
            Self::SlotKept => "slot_kept",
            Self::CalleeUnknown => "callee_unknown",
        }
    }

    /// The reason whose [`Self::name`] is `name`, or `None` for a name no
    /// reason has, such as one a newer build recorded.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        [
            Self::ReadAfterChange,
            Self::FailureAfterChange,
            Self::StillUsed,
            Self::SlotKept,
            Self::CalleeUnknown,
        ]
        .into_iter()
        .find(|reason| reason.name() == name)
    }

    /// The row's help, in end-user prose.
    #[must_use]
    pub const fn help(self) -> &'static str {
        match self {
            Self::ReadAfterChange => {
                "The function reads the property after it changes the value. \
                 Read it before the change, and the value is changed in place."
            }
            Self::FailureAfterChange => {
                "Something after the change can throw an error, so the old value is kept. \
                 Move that statement above the change, and the value is changed in place."
            }
            Self::StillUsed => {
                "The variable is used again after this call, so the call changes a copy."
            }
            Self::SlotKept => {
                "The property is copied here. It has a hook or a guard, \
                 or other code here can read or change it."
            }
            Self::CalleeUnknown => {
                "The function that runs here is only known when the program runs, \
                 so the value is copied."
            }
        }
    }
}

/// The edit that removes a copy: move the statement at `moved` to just
/// before the statement at `before`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fix {
    /// The statement to move.
    pub moved: Span,
    /// The statement it goes in front of: the change.
    pub before: Span,
}

/// One copy the pass keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeptCopy {
    /// The statement the copy is made in.
    pub span: Span,
    /// Why the copy is kept.
    pub reason: Reason,
    /// The statement move that removes it, where one exists.
    pub fix: Option<Fix>,
}

/// A [`Fix`] as text: delete `delete`, then insert `text` at `insert_at`.
/// Both are byte offsets into the file's text as it was compiled, and
/// `insert_at` is before `delete`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// The bytes to remove: the moved statement's whole lines.
    pub delete: (u32, u32),
    /// Where the moved statement goes: the start of the change's line.
    pub insert_at: u32,
    /// The moved statement's lines, as written.
    pub text: String,
}

/// One row of a program's copy table: [`KeptCopy`] with what needs the source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopySite {
    /// The file, as the program named it.
    pub file: String,
    /// The 1-based line the statement starts on.
    pub line: u32,
    /// The BLAKE3 of the file's text when the program was compiled, in
    /// lowercase hex.
    pub text_hash: String,
    /// The row's span.
    pub span: Span,
    /// Why the copy is kept.
    pub reason: Reason,
    /// The fix as a text edit, where one exists.
    pub edit: Option<TextEdit>,
}

/// The copy table of `copies`, in their order, with each row's file, line,
/// hash and edit read from `map`. A row whose file is not in `map` is left
/// out. Made when asked for, never on the request path.
#[must_use]
pub fn table(copies: &[KeptCopy], map: &SourceMap) -> Vec<CopySite> {
    let mut hashes = rustc_hash::FxHashMap::default();
    copies
        .iter()
        .filter_map(|copy| {
            let file = map.get(copy.span.file)?;
            let (line, _) = file.line_col(copy.span.start);
            let text_hash = hashes
                .entry(copy.span.file)
                .or_insert_with(|| blake3::hash(file.text().as_bytes()).to_hex().to_string())
                .clone();
            Some(CopySite {
                file: file.name().to_owned(),
                line: u32::try_from(line).map_or(u32::MAX, |line| line.saturating_add(1)),
                text_hash,
                span: copy.span,
                reason: copy.reason,
                edit: copy.fix.and_then(|fix| edit(file.text(), fix)),
            })
        })
        .collect()
}

/// The warnings of `copies`: one for each row whose [`Reason::warns`] and
/// whose fix is a whole-line [`TextEdit`], in the rows' order, with each
/// row once. Its span is the change, its second label the statement to
/// move, and its fix one replacement from the change's line to the end of
/// the moved lines, so the edit is one [`nvs_diagnostics::Suggestion`]. The
/// fix moves code, so a person reviews it and `nvs check --fix` does not
/// apply it. `nvs check` and the language server both call this, so they
/// show the same warnings.
#[must_use]
pub fn warnings(copies: &[KeptCopy], map: &SourceMap) -> Vec<Diagnostic> {
    let mut seen = rustc_hash::FxHashSet::default();
    copies
        .iter()
        .filter(|copy| copy.reason.warns() && seen.insert((copy.span, copy.reason)))
        .filter_map(|copy| {
            let fix = copy.fix?;
            let file = map.get(copy.span.file)?;
            let text = file.text();
            let edit = edit(text, fix)?;
            let (code, message, moved) = match copy.reason {
                Reason::ReadAfterChange => (
                    code::W_COPY_READ_AFTER_CHANGE,
                    "this change copies the value, because the property is read after it",
                    "this reads the property after the change",
                ),
                _ => (
                    code::W_COPY_FAILURE_AFTER_CHANGE,
                    "this change copies the value, because a statement after it can throw an error",
                    "this can throw an error after the change",
                ),
            };
            let (start, end) = (edit.insert_at, edit.delete.1);
            let replacement = format!(
                "{}{}",
                edit.text,
                text.get(start as usize..edit.delete.0 as usize)?
            );
            let whole = Span {
                file: copy.span.file,
                start,
                end,
            };
            Some(
                Diagnostic::warning(code, message)
                    .with_primary(copy.span, "the value is copied here")
                    .with_secondary(fix.moved, moved)
                    .with_help(copy.reason.help())
                    .with_unsafe_fix(whole, replacement, "move this statement above the change"),
            )
        })
        .collect()
}

/// `fix` as a text edit over `text`: the moved statement's whole lines are
/// deleted and inserted at the start of the change's line. `None` when the
/// moved statement shares a line with other code, or does not come after
/// the change.
fn edit(text: &str, fix: Fix) -> Option<TextEdit> {
    let line_start = |at: usize| text[..at].rfind('\n').map_or(0, |nl| nl + 1);
    let start = line_start(fix.moved.start as usize);
    let end = fix.moved.end as usize;
    let after = text.get(end..)?;
    let end = end + after.find('\n').map_or(after.len(), |nl| nl + 1);
    let insert_at = line_start(fix.before.start as usize);
    let whole_lines = text.get(start..fix.moved.start as usize)?.trim().is_empty()
        && text.get(fix.moved.end as usize..end)?.trim().is_empty();
    if !whole_lines || insert_at >= start {
        return None;
    }
    let mut moved = text[start..end].to_owned();
    if !moved.ends_with('\n') {
        moved.push('\n');
    }
    Some(TextEdit {
        delete: (u32::try_from(start).ok()?, u32::try_from(end).ok()?),
        insert_at: u32::try_from(insert_at).ok()?,
        text: moved,
    })
}
