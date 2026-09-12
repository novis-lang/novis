//! Modifier order: one canonical sequence, whichever one the author wrote.
//!
//! `rule:tooling/fmt-base-style-is-per` fixes that sequence —
//! `abstract`/`final`, then visibility, then `static`, then `readonly`, then
//! `lateinit` — and says in the same breath why a formatter has to impose it:
//! the parser takes these in any order, so `static public $x;` and `public
//! static $x;` both compile and two files would otherwise never converge on one
//! spelling.
//!
//! This is the first rule here that writes a byte somewhere other than where
//! its author wrote it, and it stays the smallest such rule there is. A list is
//! a permutation of itself: each keyword goes out at a place one of that same
//! list's keywords was written, so the spacing, the line breaks and any comment
//! between two modifiers stay exactly where they were put, and a list already
//! in order produces nothing at all. A missing modifier is never supplied —
//! that would change what the declaration means, which
//! `rule:core-api/written-visibility` leaves to the author and to a diagnostic.
//!
//! Where each modifier was written is
//! [`nvs_syntax::Parsed`]'s to say: the parse collects one entry per list, in
//! source order, from the single loop every declaration's modifiers go through.
//! Nothing here walks the grammar, which is what keeps a production added later
//! from quietly formatting as though it had no modifiers.

use nvs_syntax::ast::{Modifier, WrittenModifier};
use nvs_syntax::{Parsed, Trivia};

use crate::print::{self, Rewrite};

/// Every keyword `parsed` writes somewhere other than where it was written, in
/// source order and covering no byte twice.
///
/// `text` must be `parsed`'s own file: every span is an offset into it.
pub(crate) fn rewrites<'t>(parsed: &Parsed, text: &'t str) -> Vec<Rewrite<'t>> {
    let mut out = Vec::new();
    for list in &parsed.modifiers {
        push_list(&mut out, list, &parsed.trivia, text);
    }
    out
}

/// Adds what `list` owes, which is nothing when it is already in order.
fn push_list<'t>(
    out: &mut Vec<Rewrite<'t>>,
    list: &[WrittenModifier],
    trivia: &[Trivia],
    text: &'t str,
) {
    // `private (set)` is one modifier written across a skipped run, and a list
    // holding one is left exactly as its author wrote it rather than moved a
    // piece at a time.
    if !list
        .iter()
        .all(|written| print::one_code_run(trivia, written.span))
    {
        return;
    }
    let mut canonical: Vec<&WrittenModifier> = list.iter().collect();
    canonical.sort_by_key(|written| rank(written.modifier));
    for (place, belongs) in list.iter().zip(canonical) {
        if belongs.span.start == place.span.start {
            continue;
        }
        out.push(Rewrite {
            start: place.span.start as usize,
            end: place.span.end as usize,
            written: &text[belongs.span.start as usize..belongs.span.end as usize],
        });
    }
}

/// Where a modifier belongs, as `rule:tooling/fmt-base-style-is-per` orders
/// them.
///
/// A plain visibility and an asymmetric `private(set)` share a place, so a list
/// writing both keeps the order its author chose between those two: the rule
/// bands them together and decides nothing finer, and a stable sort is what
/// leaves that decision where the rule left it.
const fn rank(modifier: Modifier) -> u8 {
    match modifier {
        Modifier::Abstract | Modifier::Final => 0,
        Modifier::Public | Modifier::Protected | Modifier::Private | Modifier::SetVisibility(_) => {
            1
        }
        Modifier::Static => 2,
        Modifier::Readonly => 3,
        Modifier::Lateinit => 4,
    }
}
