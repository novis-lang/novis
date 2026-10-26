//! The routes and the tests that reach a method, read off the call graph.
//!
//! `rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method` is the
//! specification. The call graph is the one reference index read one more way
//! (`rule:ide/five-features-are-one-reference-index`): every
//! [`crate::index::Occurrence`] that resolved to a method names the method it is
//! written in, so it is an edge from that method to the one it calls, and a
//! `Core\Test::request` the route table matched is an edge from the method that
//! sends it to the route's handler ([`SymbolIndex::requests`]). Nothing here
//! walks the front end.
//!
//! [`reach`] walks those edges backwards from one method, each method once, so
//! a recursive chain ends. A method it reaches that handles a route adds the
//! route, and a `#[Test]` method it reaches adds the test.
//!
//! **A count is never too high, and "at least" says when it may be too low.**
//! Only a call the checker resolved to one method is an edge. A call through a
//! `callable` value and a call by name on a `mixed` receiver are not, and
//! neither is a call to a method that a reached method overrides, because
//! which of the two such a call runs is decided when the program runs. Where
//! one of those could reach a method the walk visited, [`Reach::at_least`] is
//! set.
//!
//! **What it costs:** one visited set and one queue of method names per
//! question, freed with the answer. Each step is one lookup in the index's
//! postings, so a walk costs what it visits.

use rustc_hash::FxHashSet;

use crate::index::SymbolIndex;

/// What reaches one method.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reach {
    /// Every route whose handler reaches the method, as its verb and path
    /// (`GET /users/{id}`), sorted.
    pub routes: Vec<String>,
    /// Every `#[Test]` method that reaches it, by symbol, sorted.
    pub tests: Vec<String>,
    /// Whether a call the graph does not follow could reach a method the walk
    /// visited, so either count may be too low.
    pub at_least: bool,
}

/// The routes and the tests that reach `method`, a method's symbol spelled as
/// a [`crate::index::Declaration`] spells one.
///
/// `method` itself is the first link: a route handler reaches itself, and so
/// does a test.
#[must_use]
pub fn reach(index: &SymbolIndex, method: &str) -> Reach {
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut pending = vec![method.to_owned()];
    let mut found = Reach::default();
    while let Some(callee) = pending.pop() {
        if !seen.insert(callee.clone()) {
            continue;
        }
        if let Some(declared) = index.declaration(&callee) {
            found.routes.extend(declared.routes.iter().cloned());
            if declared.test {
                found.tests.push(callee.clone());
            }
        }
        if index.unfollowed(&callee)
            || index
                .overridden(&callee)
                .iter()
                .any(|above| !index.occurrences(&above.symbol).is_empty())
        {
            found.at_least = true;
        }
        let calls = index.occurrences(&callee);
        let sent = index.requests(&callee);
        pending.extend(
            calls
                .into_iter()
                .chain(sent)
                .filter_map(|edge| edge.caller.clone())
                .filter(|caller| !seen.contains(caller)),
        );
    }
    found.routes.sort();
    found.routes.dedup();
    found.tests.sort();
    found
}
