//! `rule:testing/tags-and-shards-select-what-runs`'s `--shard N/M`: which part of a selection one
//! machine runs.
//!
//! A test's part is a hash of its name, so the parts stay put when a test is added or removed, and
//! M machines that each run one part run every selected test exactly once. The hash is FNV-1a over
//! the name's bytes, written here rather than taken from `std`, because `std` promises no hash that
//! two builds of the compiler agree on and every machine of one CI run has to compute the same parts.
//! A path is hashed with `/` between its parts on every platform, for the same reason.
//!
//! `nvs test` over a program hashes `Class::method`, and over a `.nvst` tree the case's path. Both
//! read this one type, so the two suites split the same way.
//!
//! **What it spends:** nothing per test. The hash is computed once, when the selection is made.

use std::fmt;
use std::str::FromStr;

/// One part of M: `--shard 2/4` is the second of four.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shard {
    /// Which part, from 1.
    index: u64,
    /// How many parts, at least 1.
    count: u64,
}

impl Shard {
    /// Whether the test named `name` is in this part.
    #[must_use]
    pub fn holds(self, name: &str) -> bool {
        fnv1a(name) % self.count == self.index - 1
    }

    /// Whether the `.nvst` case at `path`, as the report names it, is in this part. Each `\` is
    /// read as `/`, so a tree splits the same way on every platform.
    #[must_use]
    pub fn holds_path(self, path: &str) -> bool {
        self.holds(&path.replace('\\', "/"))
    }
}

impl FromStr for Shard {
    type Err = String;

    fn from_str(written: &str) -> Result<Self, String> {
        let wrong = || {
            format!(
                "`{written}` is not a part. Write the part and the number of parts, such as `2/4`"
            )
        };
        let (index, count) = written.split_once('/').ok_or_else(wrong)?;
        let index: u64 = index.trim().parse().map_err(|_| wrong())?;
        let count: u64 = count.trim().parse().map_err(|_| wrong())?;
        if count == 0 {
            return Err(format!(
                "`{written}` has no parts. The number after `/` is 1 or more"
            ));
        }
        if index == 0 || index > count {
            return Err(format!(
                "`{written}` names part {index} of {count}. The parts are numbered from 1 to {count}"
            ));
        }
        Ok(Self { index, count })
    }
}

impl fmt::Display for Shard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.index, self.count)
    }
}

/// 64-bit FNV-1a: the module doc owns why it is this hash.
fn fnv1a(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shard(written: &str) -> Shard {
        written.parse().expect("a well-formed shard")
    }

    /// Every name is in exactly one of the parts, whatever else the selection holds.
    #[test]
    fn the_parts_are_disjoint_and_cover_every_name() {
        let names: Vec<String> = (0..200).map(|n| format!("Shop{n}Test::itWorks")).collect();
        let parts: Vec<Shard> = (1..=4).map(|n| shard(&format!("{n}/4"))).collect();
        for name in &names {
            let holding = parts.iter().filter(|part| part.holds(name)).count();
            assert_eq!(holding, 1, "{name} is in {holding} parts");
        }
        for part in &parts {
            assert!(names.iter().any(|name| part.holds(name)), "{part} is empty");
        }
    }

    /// The part a name is in is a function of the name alone, pinned so that a change of hash,
    /// which would move tests between machines, fails here first.
    #[test]
    fn a_name_keeps_its_part() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
        assert!(shard("1/1").holds("BlogTest::itPublishes"));
        assert!(
            shard("2/3").holds_path("tests\\a.nvst") == shard("2/3").holds_path("tests/a.nvst")
        );
    }

    #[test]
    fn a_part_outside_its_count_is_an_error() {
        for written in ["0/4", "5/4", "1/0", "2", "a/b", "-1/4"] {
            assert!(written.parse::<Shard>().is_err(), "{written} parsed");
        }
        assert_eq!(shard("4/4").to_string(), "4/4");
    }
}
