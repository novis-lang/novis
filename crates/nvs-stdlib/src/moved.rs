//! The `Core` names that moved (`rule:core-api/a-moved-name-says-where-it-went`).
//!
//! [`MOVES`] is one row per move: the old name, the new name and the release
//! that moved it. A row names a type, such as `Core\Order`, or a static member
//! or a class-scoped type as `Class::member`, such as `Core\Time::fromEpoch`. A
//! member whose arguments changed as it moved also carries the call to write
//! instead ([`Move::call`]). A member removed with no one replacement carries
//! the sentence that says what to write ([`Move::help`]).
//! The table is data beside the registry, and the registry is the source of
//! truth: a test holds every row to it, so the new name exists and the old one
//! does not. `nvs_types::moved` is the pass that reads the table and adds the
//! note and the fix to a failed compile.
//!
//! The table is empty while Novis is Pre-Alpha, and the changelog is the only
//! record of a move. The mechanism stays for the first release after it, and
//! `nvs-types`' tests feed the pass made-up rows. A row is a few static bytes
//! and is read only on a failed compile.

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
    /// For a member removed with no one replacement, the whole help line,
    /// which names every member to choose from. Such a row has no fix, and
    /// [`Self::new`] is the first member the line names.
    pub help: Option<&'static str>,
}

impl Move {
    /// A rename: the arguments stay, so the fix writes the new name.
    #[must_use]
    pub const fn renamed(old: &'static str, new: &'static str) -> Self {
        Self {
            old,
            new,
            since: None,
            call: None,
            help: None,
        }
    }

    /// A member removed with no one replacement: the help line is `help`,
    /// and no fix is offered.
    #[must_use]
    pub const fn removed(old: &'static str, new: &'static str, help: &'static str) -> Self {
        Self {
            old,
            new,
            since: None,
            call: None,
            help: Some(help),
        }
    }

    /// The class and the member of a member row, or `None` for a type row.
    #[must_use]
    pub fn member(&self) -> Option<(&'static str, &'static str)> {
        self.old.split_once("::")
    }
}

/// Every `Core` name that moved, oldest first. Empty while Novis is
/// Pre-Alpha: an old name gets the plain unknown-name error, and the
/// changelog says what to write instead
/// (`rule:core-api/no-deprecation-window-before-a-release`). The first release
/// after Pre-Alpha adds its moves here.
pub const MOVES: &[Move] = &[];

/// The row for `name`, a type or a `Class::member`, matched without regard to
/// ASCII case as `Core` names are matched everywhere else.
#[must_use]
pub fn moved(name: &str) -> Option<&'static Move> {
    moved_in(MOVES, name)
}

/// The row of `table` for `name`, as [`moved`] reads [`MOVES`]. A test passes
/// its own made-up rows.
#[must_use]
pub fn moved_in(table: &'static [Move], name: &str) -> Option<&'static Move> {
    table.iter().find(|row| row.old.eq_ignore_ascii_case(name))
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
                // A class-scoped type moved out of its class names a type, and
                // its old name is gone from the types. It is not compared with
                // the class's members: `DateTime::Weekday` and the method
                // `weekday` differ only in case, and the checker reads a row
                // only for a name that did not resolve.
                match row.new.split_once("::") {
                    Some((new_class, new_member)) => {
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
                    }
                    None => {
                        assert!(has(row.new), "`{}` is not a `Core` type", row.new);
                        assert!(!has(row.old), "`{}` is still a `Core` type", row.old);
                    }
                }
                if let Some(call) = row.call {
                    assert!(
                        call.starts_with(row.new),
                        "`{call}` does not call `{}`",
                        row.new
                    );
                }
                if let Some(help) = row.help {
                    assert!(
                        help.contains(&format!("`{}`", row.new)),
                        "`{help}` does not name `{}`",
                        row.new
                    );
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
