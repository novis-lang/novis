//! Stable diagnostic codes.
//!
//! A code is a promise: once released, its meaning never changes. Retire a code
//! rather than repurposing it, because users write suppressions against them and
//! tooling matches on them.
//!
//! Each area of the compiler has a range of its own, so a code alone tells you
//! where it came from, and a file of its own, so two changes that add codes in
//! two areas never take the same number:
//!
//! | range | area | file |
//! |---|---|---|
//! | `E00xx` | lexer | `lexer.rs` |
//! | `E01xx` | parser | `parser.rs` |
//! | `E02xx` | rejected PHP constructs (the "pragmatic superset" exclusions) | `rejected_php.rs` |
//! | `E03xx` | name resolution | `names.rs` |
//! | `E04xx` | types | `types.rs` |
//! | `E05xx` | IR and codegen | `ir.rs` |
//! | `E06xx` | configuration and capabilities | `config.rs` |
//! | `E07xx` | types, continued: the `E04xx` range filled at `E0499` | `types_e07.rs` |
//! | `E08xx` | types, continued again: the `E07xx` range filled at `E0799` | `types_e08.rs` |
//! | `E09xx` | internal compiler errors | `internal.rs` |
//! | `E10xx` | reserve: whichever range fills next continues here, in a new file | none yet |
//! | `W1xxx` | warnings | `warnings.rs` |
//!
//! A new code takes the highest number in its area's file plus one, and a
//! test fails on a code outside its file's range or a number declared twice.
//! A range is two digits wide, and a full one continues in a new range and a
//! new file instead of running past its end. Max-plus-one past `E0499` would
//! give `E0500`, whose digits read as the IR-and-codegen range, so `E0500` is
//! never issued and each continuation is one more row in the table above. The
//! two alternatives cost more than a second range: widening every range to
//! three digits renumbers every released code and every `.nvst` case that
//! names one, and filling the lowest hole inside a range reuses a retired
//! number, which the promise above forbids.

use crate::diagnostic::Code;

mod config;
mod internal;
mod ir;
mod lexer;
mod names;
mod parser;
mod rejected_php;
mod types;
mod types_e07;
mod types_e08;
mod warnings;

pub use config::declared::*;
pub use internal::declared::*;
#[expect(
    unused_imports,
    unreachable_pub,
    reason = "the IR area declares no code yet; its first code makes this expectation fail, and the attribute goes"
)]
pub use ir::declared::*;
pub use lexer::declared::*;
pub use names::declared::*;
pub use parser::declared::*;
pub use rejected_php::declared::*;
pub use types::declared::*;
pub use types_e07::declared::*;
pub use types_e08::declared::*;
pub use warnings::declared::*;

/// One area of the table: the prefix every code in it starts with, and its
/// codes with their cards, in the order its file declares them.
struct Area {
    range: &'static str,
    codes: &'static [(Code, Option<&'static str>)],
}

/// Every area, in the order of its range.
const AREAS: &[Area] = &[
    lexer::AREA,
    parser::AREA,
    rejected_php::AREA,
    names::AREA,
    types::AREA,
    ir::AREA,
    config::AREA,
    types_e07::AREA,
    types_e08::AREA,
    internal::AREA,
    warnings::AREA,
];

/// How many codes the areas declare between them.
const COUNT: usize = {
    let mut count = 0;
    let mut area = 0;
    while area < AREAS.len() {
        count += AREAS[area].codes.len();
        area += 1;
    }
    count
};

/// The areas' codes, one after another.
const EVERY: [Code; COUNT] = {
    let mut out = [Code::new(""); COUNT];
    let mut at = 0;
    let mut area = 0;
    while area < AREAS.len() {
        let codes = AREAS[area].codes;
        let mut i = 0;
        while i < codes.len() {
            out[at] = codes[i].0;
            at += 1;
            i += 1;
        }
        area += 1;
    }
    out
};

/// Every code, area by area in the order of their ranges, and within an area
/// in the order its file declares them.
pub const ALL: &[Code] = &EVERY;

/// The card `nvs agent show <code>` prints: what the error means and how to
/// fix it, written for the person who met it. `None` for a code that still
/// owes its card, and for a code no area declares.
#[must_use]
pub fn card(code: Code) -> Option<&'static str> {
    AREAS
        .iter()
        .filter(|area| code.as_str().starts_with(area.range))
        .flat_map(|area| area.codes)
        .find(|(declared, _)| *declared == code)
        .and_then(|(_, card)| *card)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{ALL, AREAS};

    /// Every code is its area's prefix and then digits, five characters in
    /// all, so a code taken in one area's file can never be a number of
    /// another's.
    #[test]
    fn every_code_is_in_its_areas_range() {
        let mut wrong = Vec::new();
        for area in AREAS {
            for (code, _) in area.codes {
                let text = code.as_str();
                let digits = text.strip_prefix(area.range);
                if text.len() != 5 || !digits.is_some_and(|d| d.bytes().all(|b| b.is_ascii_digit()))
                {
                    wrong.push(format!(
                        "{text} is outside its area's range `{}`",
                        area.range
                    ));
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// No two areas share a range, and no range is the start of another, so a
    /// code belongs to one area alone.
    #[test]
    fn no_two_areas_overlap() {
        for (i, a) in AREAS.iter().enumerate() {
            for b in &AREAS[i + 1..] {
                assert!(
                    !a.range.starts_with(b.range) && !b.range.starts_with(a.range),
                    "the ranges `{}` and `{}` overlap",
                    a.range,
                    b.range,
                );
            }
        }
    }

    /// No number is declared twice, in one area or across two.
    #[test]
    fn no_code_is_declared_twice() {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for code in ALL {
            *seen.entry(code.as_str()).or_default() += 1;
        }
        let mut twice: Vec<&str> = seen
            .into_iter()
            .filter(|&(_, n)| n > 1)
            .map(|(c, _)| c)
            .collect();
        twice.sort_unstable();
        assert!(
            twice.is_empty(),
            "declared more than once: {}",
            twice.join(", ")
        );
    }
}
