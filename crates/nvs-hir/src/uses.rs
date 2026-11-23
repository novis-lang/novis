//! What one file's check looked up: each declaration it found, and each name
//! it looked for and did not find.
//!
//! The language server reads these two sets to decide which files an edit
//! can reach (`rule:ide/the-index-is-incremental-and-persistent`). A file
//! whose sets name nothing an edit changed keeps its checked output.
//!
//! # The names
//!
//! A name is written the way the server's index writes a symbol: `App\User`
//! for a class, interface, enum or alias, `App\User::greet` for a method,
//! `App\User::$name` for a property, `App\User::LIMIT` for a constant and
//! `App\Status::Draft` for an enum case.
//!
//! - **A use** names the declaration a lookup found. A member found on a
//!   parent is the parent's member, `App\Base::greet`.
//! - **A miss** names what a lookup looked for. A member is missed on the
//!   class it was looked up on, `App\User::greet`, and on no other class: a
//!   member added to any class above it reaches the miss through the class
//!   graph. A member found on a parent of the class it was looked up on is
//!   also a miss on that class, because a member added between the two would
//!   be found first. A member looked up on a value of unknown type is
//!   `::greet`, and a class name that resolves to nothing is the name the
//!   lookup resolved it to.
//!
//! # What it costs
//!
//! Nothing for a check that does not record: [`Uses::off`] stores nothing,
//! and every caller checks [`Uses::is_on`] before it builds a name. `nvs check`
//! and every compile record nothing. A recording check keeps one string per
//! distinct name per file, for as long as the check's output is kept.

use rustc_hash::FxHashSet;

use crate::QName;

/// The names one file's check found and failed to find.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Uses {
    on: bool,
    used: FxHashSet<Box<str>>,
    missed: FxHashSet<Box<str>>,
}

impl Uses {
    /// A set that records nothing. Every `record` call on it returns at once.
    #[must_use]
    pub fn off() -> Self {
        Self::default()
    }

    /// An empty set that records.
    #[must_use]
    pub fn recording() -> Self {
        Self {
            on: true,
            ..Self::default()
        }
    }

    /// Whether this set records.
    #[must_use]
    pub const fn is_on(&self) -> bool {
        self.on
    }

    /// Records a use of the class, interface, enum or alias `name`.
    pub fn class(&mut self, name: &QName) {
        if self.on {
            self.used.insert(name.to_string().into());
        }
    }

    /// Records a use of `owner`'s member `member`, written as its source
    /// writes it: `greet`, `$name`, `LIMIT` or `Draft`.
    pub fn member(&mut self, owner: &QName, member: &str) {
        if self.on {
            self.used.insert(format!("{owner}::{member}").into());
        }
    }

    /// Records a member found on `owner` and looked up on `receiver`. When
    /// the two differ, the lookup also missed the member on `receiver`.
    pub fn member_through(&mut self, receiver: &QName, owner: &QName, member: &str) {
        if self.on {
            self.member(owner, member);
            if receiver != owner {
                self.missed_member(Some(receiver), member);
            }
        }
    }

    /// Records a class name that resolved to `name` and to nothing declared.
    pub fn missed_class(&mut self, name: &QName) {
        if self.on {
            self.missed.insert(name.to_string().into());
        }
    }

    /// Records a member looked up on `receiver` and not found, or on a value
    /// of unknown type when `receiver` is `None`.
    pub fn missed_member(&mut self, receiver: Option<&QName>, member: &str) {
        if self.on {
            let name = match receiver {
                Some(owner) => format!("{owner}::{member}"),
                None => format!("::{member}"),
            };
            self.missed.insert(name.into());
        }
    }

    /// Adds every name of `other` to this set, whether or not this set records.
    pub fn absorb(&mut self, other: Self) {
        self.used.extend(other.used);
        self.missed.extend(other.missed);
    }

    /// Every name a lookup found, in no order.
    pub fn used(&self) -> impl Iterator<Item = &str> {
        self.used.iter().map(AsRef::as_ref)
    }

    /// Every name a lookup looked for and did not find, in no order.
    pub fn missed(&self) -> impl Iterator<Item = &str> {
        self.missed.iter().map(AsRef::as_ref)
    }

    /// The names a lookup found, sorted, for a test or a key.
    #[must_use]
    pub fn used_sorted(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.used().collect();
        out.sort_unstable();
        out
    }

    /// The names a lookup did not find, sorted, for a test or a key.
    #[must_use]
    pub fn missed_sorted(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.missed().collect();
        out.sort_unstable();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_that_is_off_records_nothing() {
        let mut uses = Uses::off();
        uses.class(&QName::parse("App\\User"));
        uses.missed_member(None, "greet");
        assert!(uses.used_sorted().is_empty());
        assert!(uses.missed_sorted().is_empty());
    }

    #[test]
    fn a_member_found_on_a_parent_is_a_miss_on_the_receiver() {
        let mut uses = Uses::recording();
        let (child, base) = (QName::parse("App\\Child"), QName::parse("App\\Base"));
        uses.member_through(&child, &base, "greet");
        uses.member_through(&base, &base, "$name");
        uses.missed_member(None, "LIMIT");
        assert_eq!(uses.used_sorted(), ["App\\Base::$name", "App\\Base::greet"]);
        assert_eq!(uses.missed_sorted(), ["::LIMIT", "App\\Child::greet"]);
    }
}
