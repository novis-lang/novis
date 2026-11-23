//! The key a file's checked output is valid for: its own text and the
//! surface rows of what its check used.
//!
//! [`crate::session`] checks a file's bodies again when its key differs from
//! the key its output was stored under. [`Surfaces`] is what one analysis's
//! graph declares, cut into the rows `crate::surface` writes, and
//! [`Surfaces::key`] hashes, for one file, the part of it that the file's
//! last check read (`rule:ide/the-index-is-incremental-and-persistent`):
//!
//! - **What every file shares**: the extension set and the restrictions the
//!   caller hashed, and the whole text of each file the parse cache does not
//!   hold, which no walk leaves behind.
//! - **The file's own id and text.** Its `use`, `require` and `autoload`
//!   lines are in its text.
//! - **Each name the check used**, with the rows that have it: each row's
//!   file, text and attributes. A member brings its class's header row with
//!   it. A class brings its header and cases rows. `Owner::$` is the class's
//!   property list, which a derived codec reads, so it brings the properties
//!   row.
//! - **Each name the check failed to find**, and whether the graph has it
//!   now. A member `R::m` is found when R or any class above it declares `m`.
//!   A member looked up on an unknown type, `::m`, hashes every class that
//!   declares `m`.
//! - **Every row of every class above a class the file declares.** The
//!   override, abstract, interface, finality and constructor checks of a
//!   subclass read its parents' members, private ones included.
//!
//! Which other files are in the graph is not in the key. A file's check reads
//! another file only through a name it looks up, so a file checked from two
//! entries' graphs has one key, and an output stored by one is current for the
//! other. The tables over the whole graph, the routes, commands and links, are
//! built again on every analysis from each file's own rows.
//!
//! An output that recorded nothing, which only a check outside the server
//! makes, hashes every row of the graph in place of the last three, so any
//! surface edit moves its key.
//!
//! # What it costs
//!
//! [`Surfaces::new`] is one pass over every row of the graph per analysis,
//! and three maps of borrowed rows that are freed with it. A key is one hash
//! over a file's uses, and one walk of the class graph per class the file
//! declares and per member it failed to find.

use nvs_diagnostics::{SourceFile, SourceId};
use nvs_hir::{ClassGraph, QName, Uses};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::parse_cache::ParseCache;
use crate::surface::{Kind, Row};

/// One row and the file that declares it.
type Declared<'a> = (SourceId, &'a Row);

/// What one analysis's graph declares, row by row, for the keys of its files.
pub(crate) struct Surfaces<'a> {
    /// The part of every key that every file of the graph shares.
    shared: blake3::Hash,
    /// Every row of every file, for an output that recorded no uses.
    whole: blake3::Hash,
    /// The hash of each file's text.
    texts: FxHashMap<SourceId, [u8; 32]>,
    /// The classes, interfaces, enums and aliases each file declares at file
    /// scope.
    declares: FxHashMap<SourceId, Vec<&'a str>>,
    /// Every row, by its symbol.
    by_symbol: FxHashMap<&'a str, Vec<Declared<'a>>>,
    /// Every row of a class and its members, by the class.
    by_owner: FxHashMap<&'a str, Vec<Declared<'a>>>,
    /// The classes that declare a member, by the member's name, sorted.
    owners: FxHashMap<&'a str, Vec<&'a str>>,
    graph: &'a ClassGraph,
}

impl<'a> Surfaces<'a> {
    /// The rows of `files`, in graph order, as `parses` holds them. `base` is
    /// the hash of the inputs every file shares that are not a file: the
    /// extension set and the restrictions.
    pub(crate) fn new(
        parses: &'a ParseCache,
        files: impl Iterator<Item = &'a SourceFile>,
        base: &blake3::Hash,
        graph: &'a ClassGraph,
    ) -> Self {
        let mut shared = blake3::Hasher::new();
        let mut whole = blake3::Hasher::new();
        shared.update(base.as_bytes());
        let mut surfaces = Surfaces {
            shared: blake3::Hash::from_bytes([0; 32]),
            whole: blake3::Hash::from_bytes([0; 32]),
            texts: FxHashMap::default(),
            declares: FxHashMap::default(),
            by_symbol: FxHashMap::default(),
            by_owner: FxHashMap::default(),
            owners: FxHashMap::default(),
            graph,
        };
        for src in files {
            let id = src.id();
            let Some((text, rows)) = parses.hashes(src) else {
                let text = *blake3::hash(src.text().as_bytes()).as_bytes();
                shared.update(&id.index().to_le_bytes());
                shared.update(&text);
                surfaces.texts.insert(id, text);
                continue;
            };
            surfaces.texts.insert(id, *text.as_bytes());
            whole.update(&id.index().to_le_bytes());
            whole.update(rows.digest().as_bytes());
            let declares = surfaces.declares.entry(id).or_default();
            for row in rows.iter().filter(|row| row.kind != Kind::Graph) {
                let symbol = row.symbol.as_str();
                surfaces.by_symbol.entry(symbol).or_default().push((id, row));
                match symbol.split_once("::") {
                    Some((owner, member)) => {
                        surfaces.by_owner.entry(owner).or_default().push((id, row));
                        surfaces.owners.entry(member).or_default().push(owner);
                    }
                    None => {
                        surfaces.by_owner.entry(symbol).or_default().push((id, row));
                        if row.kind == Kind::Header {
                            declares.push(symbol);
                        }
                    }
                }
            }
        }
        for owners in surfaces.owners.values_mut() {
            owners.sort_unstable();
            owners.dedup();
        }
        surfaces.shared = shared.finalize();
        whole.update(surfaces.shared.as_bytes());
        surfaces.whole = whole.finalize();
        surfaces
    }

    /// The key of the file `id`, whose last check recorded `uses`.
    pub(crate) fn key(&self, id: SourceId, uses: &Uses) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.shared.as_bytes());
        hasher.update(&id.index().to_le_bytes());
        hasher.update(self.texts.get(&id).unwrap_or(&[0; 32]));
        if !uses.is_on() {
            hasher.update(self.whole.as_bytes());
            return hasher.finalize();
        }
        for name in uses.used_sorted() {
            hasher.update(name.as_bytes());
            hasher.update(&[0]);
            self.used(&mut hasher, name);
        }
        hasher.update(&[1]);
        for name in uses.missed_sorted() {
            hasher.update(name.as_bytes());
            hasher.update(&[0]);
            self.missed(&mut hasher, name);
        }
        hasher.update(&[1]);
        for class in self.declares.get(&id).into_iter().flatten() {
            for ancestor in self.ancestors(class) {
                hasher.update(ancestor.as_bytes());
                hasher.update(&[0]);
                rows(
                    &mut hasher,
                    self.by_owner
                        .get(ancestor.as_str())
                        .into_iter()
                        .flatten()
                        .copied(),
                );
            }
        }
        hasher.finalize()
    }

    /// Hashes the rows a use of `name` read.
    fn used(&self, hasher: &mut blake3::Hasher, name: &str) {
        let of = |symbol: &str| self.by_symbol.get(symbol).into_iter().flatten().copied();
        match name.split_once("::") {
            Some((owner, "$")) => rows(
                hasher,
                of(owner).filter(|(_, row)| row.kind == Kind::Properties),
            ),
            Some((owner, _)) => {
                rows(hasher, of(name));
                rows(hasher, of(owner).filter(|(_, row)| row.kind == Kind::Header));
            }
            None => rows(
                hasher,
                of(name).filter(|(_, row)| matches!(row.kind, Kind::Header | Kind::Cases)),
            ),
        }
    }

    /// Hashes whether the graph has the name a lookup failed to find.
    fn missed(&self, hasher: &mut blake3::Hasher, name: &str) {
        match name.split_once("::") {
            Some(("", member)) => {
                for owner in self.owners.get(member).into_iter().flatten() {
                    hasher.update(owner.as_bytes());
                    hasher.update(&[0]);
                }
            }
            Some((receiver, member)) => {
                let declares = |class: &str| {
                    self.owners
                        .get(member)
                        .is_some_and(|owners| owners.binary_search(&class).is_ok())
                };
                let found = declares(receiver)
                    || self
                        .ancestors(receiver)
                        .iter()
                        .any(|class| declares(class));
                hasher.update(&[u8::from(found)]);
            }
            None => {
                hasher.update(&[u8::from(self.by_symbol.contains_key(name))]);
            }
        }
    }

    /// Every class and interface above `class`, each once, sorted.
    fn ancestors(&self, class: &str) -> Vec<String> {
        let mut seen: FxHashSet<String> = FxHashSet::default();
        let mut out = Vec::new();
        let mut queue = vec![QName::parse(class)];
        while let Some(name) = queue.pop() {
            let Some(links) = self.graph.get(&name) else {
                continue;
            };
            for parent in links.extends.iter().chain(&links.implements) {
                let written = parent.to_string();
                if seen.insert(written.clone()) {
                    out.push(written);
                    queue.push(parent.clone());
                }
            }
        }
        out.sort_unstable();
        out
    }
}

/// Hashes each of `rows`: its file, symbol, kind, text and attributes, and
/// then how many there were.
fn rows<'r>(hasher: &mut blake3::Hasher, rows: impl Iterator<Item = Declared<'r>>) {
    let mut count: u32 = 0;
    for (file, row) in rows {
        hasher.update(&file.index().to_le_bytes());
        hasher.update(row.symbol.as_bytes());
        hasher.update(&[0, row.kind as u8]);
        hasher.update(row.text.as_bytes());
        hasher.update(row.attributes.as_bytes());
        count += 1;
    }
    hasher.update(&count.to_le_bytes());
}
