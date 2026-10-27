//! `rule:tooling/fmt-every-construct-has-a-broken-layout`: every syntax node
//! kind is on exactly one of `nvs fmt`'s two layout lists, so a construct the
//! grammar gains is never laid out by accident and never forgotten.
//!
//! The kinds are `nvs_syntax::walk::KINDS`, the table every node the walk
//! builds is named from, so a production added there is a kind this test sees.

use std::path::Path;

use nvs_fmt::{AUTHORS_BREAKS, LAID_OUT};
use nvs_syntax::walk::KINDS;

#[test]
fn every_node_kind_is_on_exactly_one_layout_list() {
    let laid_out = |kind: &str| LAID_OUT.iter().any(|&(listed, _)| listed == kind);
    let authors = |kind: &str| AUTHORS_BREAKS.contains(&kind);
    let missing: Vec<&str> = KINDS
        .iter()
        .copied()
        .filter(|&kind| !laid_out(kind) && !authors(kind))
        .collect();
    assert!(
        missing.is_empty(),
        "these node kinds are on neither `LAID_OUT` nor `AUTHORS_BREAKS` in \
         `crates/nvs-fmt/src/chain.rs`: {missing:?}"
    );
    let both: Vec<&str> = KINDS
        .iter()
        .copied()
        .filter(|&kind| laid_out(kind) && authors(kind))
        .collect();
    assert!(
        both.is_empty(),
        "these node kinds are on both layout lists: {both:?}"
    );
}

#[test]
fn every_listed_kind_is_a_node_kind() {
    let stale: Vec<&str> = LAID_OUT
        .iter()
        .map(|&(kind, _)| kind)
        .chain(AUTHORS_BREAKS.iter().copied())
        .filter(|kind| !KINDS.contains(kind))
        .collect();
    assert!(
        stale.is_empty(),
        "`crates/nvs-fmt/src/chain.rs` lists kinds `walk.rs` does not name: {stale:?}"
    );
}

#[test]
fn every_broken_layout_names_a_rule_that_exists() {
    let rules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/rules");
    let absent: Vec<&str> = LAID_OUT
        .iter()
        .map(|&(_, rule)| rule)
        .filter(|rule| {
            let id = rule.strip_prefix("rule:").unwrap_or(rule);
            !rules.join(format!("{id}.md")).is_file()
        })
        .collect();
    assert!(
        absent.is_empty(),
        "these rules have no file under `docs/rules/`: {absent:?}"
    );
}
