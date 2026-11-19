//! The `Core` names that moved (`rule:core-api/a-moved-name-says-where-it-went`).
//!
//! [`MOVES`] is one row per move: the old name, the new name and the release
//! that moved it. A row names a type, such as `Core\Order`, or a static member
//! as `Class::member`, such as `Core\Time::fromEpoch`. A member whose arguments
//! changed as it moved also carries the call to write instead ([`Move::call`]).
//! The table is data beside the registry, and the registry is the source of
//! truth: a test holds every row to it, so the new name exists and the old one
//! does not. `nvs_types::moved` is the pass that reads the table and adds the
//! note and the fix to a failed compile.
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
    /// For a member whose arguments changed, the call to write in its place,
    /// as code. `None` for a rename, whose fix writes [`Self::new`] over the
    /// old name and keeps the arguments.
    pub call: Option<&'static str>,
}

impl Move {
    /// A rename: the arguments stay, so the fix writes the new name.
    const fn renamed(old: &'static str, new: &'static str) -> Self {
        Self {
            old,
            new,
            since: None,
            call: None,
        }
    }

    /// The class and the member of a member row, or `None` for a type row.
    #[must_use]
    pub fn member(&self) -> Option<(&'static str, &'static str)> {
        self.old.split_once("::")
    }
}

/// Every `Core` name that moved, oldest first.
pub const MOVES: &[Move] = &[
    Move::renamed(r"Core\Order", r"Core\Arr::Order"),
    // rule:core-classes/a-time-class-builds-its-own-values
    Move::renamed(r"Core\Time::fromEpoch", r"Core\Time\Instant::fromEpoch"),
    Move::renamed(r"Core\Time::fromIso", r"Core\Time\Instant::fromIso"),
    Move::renamed(r"Core\Time::parse", r"Core\Time\DateTime::parse"),
    Move {
        old: r"Core\Time::at",
        new: r"Core\Time\DateTime::at",
        since: None,
        call: Some(
            r"Core\Time\DateTime::at(Core\Time\Date::at(…), Core\Time\TimeOfDay::at(…), $zone)",
        ),
    },
    Move {
        old: r"Core\Time\Date::atStartOfDay",
        new: r"Core\Time\DateTime::at",
        since: None,
        call: Some(r"Core\Time\DateTime::at($date, Core\Time\TimeOfDay::at(0, 0), $zone)"),
    },
];

/// The row for `name`, a type or a `Class::member`, matched without regard to
/// ASCII case as `Core` names are matched everywhere else.
#[must_use]
pub fn moved(name: &str) -> Option<&'static Move> {
    MOVES.iter().find(|row| row.old.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether the registry's class `class` has a member named `member`.
    fn has_member(class: &str, member: &str) -> bool {
        crate::registry::class(class)
            .is_some_and(|class| class.members().any(|m| m.name.eq_ignore_ascii_case(member)))
    }

    /// A row's new name is a `Core` type or member the registry has, and its
    /// old name is not, so the note can never point at a place that is not
    /// there.
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
            if let Some((class, member)) = row.member() {
                let (new_class, new_member) = row
                    .new
                    .split_once("::")
                    .unwrap_or_else(|| panic!("`{}` names no member", row.new));
                assert!(
                    has_member(new_class, new_member),
                    "`{}` is not a `Core` member",
                    row.new
                );
                assert!(
                    !has_member(class, member),
                    "`{}` is still a `Core` member",
                    row.old
                );
                if let Some(call) = row.call {
                    assert!(call.starts_with(row.new), "`{call}` does not call `{}`", row.new);
                }
                continue;
            }
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
