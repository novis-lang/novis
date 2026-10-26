//! The callers and the callees of one method, read off the call graph.
//!
//! `textDocument/callHierarchy` is `rule:ide/the-request-set-is-closed`'s
//! twenty-second standard request, and it reads the edges [`crate::reach`]
//! walks (`rule:ide/five-features-are-one-reference-index`): every
//! [`Occurrence`] that resolved to a method and names its [`Occurrence::caller`]
//! is an edge from that caller, and a `Core\Test::request` the route table
//! matched is an edge from the method that sends it to the route's handler.
//! Nothing here walks the front end.
//!
//! [`incoming`] reads one method's uses and the requests its route answers, and
//! groups them by the method each is written in. [`outgoing`] reads the uses
//! and requests written in the method's own file whose caller is the method. A
//! method's body is in the file that declares it, so that one file holds every
//! edge out of it.
//!
//! **A call the checker resolved to no single method is not shown.** A method
//! taken as a callable value is an occurrence with no caller, so it is no
//! incoming edge, and a call through a `callable` value or by name on a
//! `mixed` receiver resolved to nothing, so it is no occurrence at all. A
//! closure's calls are the calls of the method that writes it, as the lens
//! counts them.
//!
//! **What it costs:** one ordered map of callers or callees per question, freed
//! with the answer. Each answer is one lookup in the index's postings, or one
//! file's occurrences read through.

use std::collections::BTreeMap;

use crate::index::{DeclKind, Occurrence, Site, SymbolIndex};

/// One method at the other end of a call, and every place the call is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calls<'a> {
    /// The method's symbol, spelled as a [`crate::index::Declaration`] spells
    /// one.
    pub method: String,
    /// Every call site, in file and then source order. All of them are in one
    /// file: the caller's, for an incoming call, and `method`'s caller's, for
    /// an outgoing one.
    pub sites: Vec<&'a Site>,
}

/// Every method that calls `method` or sends a request its route answers, by
/// symbol, each with the places it does so.
#[must_use]
pub fn incoming<'a>(index: &'a SymbolIndex, method: &str) -> Vec<Calls<'a>> {
    grouped(
        index
            .occurrences(method)
            .into_iter()
            .chain(index.requests(method))
            .filter_map(|edge| Some((edge.caller.clone()?, &edge.site))),
    )
}

/// Every method `method` calls, or reaches through a request it sends, by
/// symbol, each with the places it does so. A use of anything but a method,
/// such as a property or a class, is no call.
#[must_use]
pub fn outgoing<'a>(index: &'a SymbolIndex, method: &str) -> Vec<Calls<'a>> {
    let Some(declared) = index.declaration(method) else {
        return Vec::new();
    };
    let path = declared.site.path.as_path();
    let written_in = |edge: &&'a Occurrence| edge.caller.as_deref() == Some(method);
    let calls = index
        .occurrences_in(path)
        .iter()
        .filter(written_in)
        .filter(|edge| {
            index
                .declaration(&edge.symbol)
                .is_some_and(|callee| callee.kind == DeclKind::Method)
        });
    let sent = index.requests_in(path).iter().filter(written_in);
    grouped(
        calls
            .chain(sent)
            .map(|edge| (edge.symbol.clone(), &edge.site)),
    )
}

/// `edges` grouped by the method at their other end, in symbol order.
fn grouped<'a>(edges: impl Iterator<Item = (String, &'a Site)>) -> Vec<Calls<'a>> {
    let mut by_method: BTreeMap<String, Vec<&'a Site>> = BTreeMap::new();
    for (method, site) in edges {
        by_method.entry(method).or_default().push(site);
    }
    by_method
        .into_iter()
        .map(|(method, mut sites)| {
            sites.sort();
            Calls { method, sites }
        })
        .collect()
}
