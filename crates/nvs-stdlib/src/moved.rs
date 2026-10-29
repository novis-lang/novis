//! The `Core` names that moved (`rule:core-api/a-moved-name-says-where-it-went`).
//!
//! [`MOVES`] is one row per move: the old name, the new name and the release
//! that moved it. It is data beside the registry, and the registry is the
//! source of truth: a test holds every row to it, so the new name exists and
//! the old one does not. `nvs_types::moved` is the pass that reads the table
//! and adds the note and the fix to a failed compile.
//!
//! A row stays until 1.0. Before then any release may move a `Core` name, and
//! a project that skips releases meets every move at once. A row is a few
//! static bytes and is read only on a failed compile.

/// One `Core` name that moved.
#[derive(Clone, Copy, Debug)]
pub struct Move {
    /// The name code wrote before the move, as source writes it.
    pub old: &'static str,
    /// The name that replaced it, as source writes it.
    pub new: &'static str,
    /// The release that moved it, or `None` for a move made before the first
    /// release.
    pub since: Option<&'static str>,
}

/// Every `Core` name that moved, oldest first.
pub const MOVES: &[Move] = &[Move {
    old: r"Core\Order",
    new: r"Core\Arr::Order",
    since: None,
}];

/// The row for `name`, matched without regard to ASCII case as `Core` names
/// are matched everywhere else.
#[must_use]
pub fn moved(name: &str) -> Option<&'static Move> {
    MOVES.iter().find(|row| row.old.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row's new name is a `Core` type the registry has, and its old name is
    /// not, so the note can never point at a place that is not there.
    #[test]
    fn every_move_points_from_a_missing_name_to_a_registered_one() {
        let names = crate::registry::type_names();
        let has = |name: &str| names.iter().any(|known| known.eq_ignore_ascii_case(name));
        for row in MOVES {
            assert!(
                row.old.starts_with(r"Core\"),
                "`{}` is not under `Core`",
                row.old
            );
            assert!(has(row.new), "`{}` is not a `Core` type", row.new);
            assert!(!has(row.old), "`{}` is still a `Core` type", row.old);
            assert!(
                crate::registry::class(row.old).is_none(),
                "`{}` is still a `Core` class",
                row.old
            );
        }
    }
}
