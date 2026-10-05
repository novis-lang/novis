//! The set of locals definitely assigned on the path being checked, which
//! [`crate::locals`]' walk threads through a function body.
//!
//! A branch is checked on the same [`Live`] as the code around it, never on
//! a copy: [`Live::mark`] records where the branch starts, and
//! [`Live::rewind`] takes back exactly the names assigned since then and
//! returns them. A join then puts back the names every way out assigned. So
//! a branch costs what it assigns, and a function of `n` statements checks in
//! time linear in `n` however many locals are live at each branch.
//!
//! That rests on one invariant: between a mark and its rewind, the set only
//! grows, and every name it gains is in the log. [`Live::insert`] is the only
//! way in, and a nested rewind takes back only names gained after its own
//! mark, so the invariant holds at every depth. There is no `remove`: a name
//! that must not survive a branch, such as a `catch` binding, is dropped from
//! the list the rewind returns.
//!
//! The memory spent is one extra copy of each name assigned, per function
//! body being checked, freed when the body is done.

use rustc_hash::FxHashSet;

/// Where a branch starts, from [`Live::mark`].
#[derive(Clone, Copy)]
pub(crate) struct Mark(usize);

/// The locals definitely assigned so far, and the order they were assigned
/// in, so a branch can be taken back. See the module doc.
#[derive(Default)]
pub(crate) struct Live {
    set: FxHashSet<String>,
    log: Vec<String>,
}

impl Live {
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.set.contains(name)
    }

    /// Marks `name` assigned. A name already assigned is left as it is.
    pub(crate) fn insert(&mut self, name: String) {
        if !self.set.contains(&name) {
            self.set.insert(name.clone());
            self.log.push(name);
        }
    }

    pub(crate) fn mark(&self) -> Mark {
        Mark(self.log.len())
    }

    /// Takes back every name assigned since `mark` and returns them.
    pub(crate) fn rewind(&mut self, mark: Mark) -> Vec<String> {
        let added = self.log.split_off(mark.0);
        for name in &added {
            self.set.remove(name);
        }
        added
    }

    /// Marks every name in `names` assigned.
    pub(crate) fn extend(&mut self, names: Vec<String>) {
        for name in names {
            self.insert(name);
        }
    }

    /// Keeps, of the names assigned since `mark`, only the ones `other` also
    /// assigned: the join of two ways out of a branch that both start at
    /// `mark`.
    pub(crate) fn intersect_since(&mut self, mark: Mark, other: &[String]) {
        let other: FxHashSet<&str> = other.iter().map(String::as_str).collect();
        let mut kept = Vec::new();
        for name in self.log.split_off(mark.0) {
            if other.contains(name.as_str()) {
                kept.push(name);
            } else {
                self.set.remove(&name);
            }
        }
        self.log.extend(kept);
    }
}

/// The names every list in `ways` assigned, or `None` when there are no
/// ways at all. Each list is what one way out of a branch assigned past the
/// same mark.
pub(crate) fn assigned_on_every_way(ways: Vec<Vec<String>>) -> Option<Vec<String>> {
    let mut ways = ways.into_iter();
    let mut common = ways.next()?;
    for way in ways {
        let way: FxHashSet<&str> = way.iter().map(String::as_str).collect();
        common.retain(|name| way.contains(name.as_str()));
    }
    Some(common)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live(names: &[&str]) -> Live {
        let mut live = Live::default();
        for name in names {
            live.insert((*name).to_owned());
        }
        live
    }

    #[test]
    fn a_rewind_returns_what_the_branch_assigned_and_leaves_the_rest() {
        let mut l = live(&["a"]);
        let m = l.mark();
        l.insert("a".to_owned());
        l.insert("b".to_owned());
        assert_eq!(l.rewind(m), vec!["b".to_owned()]);
        assert!(l.contains("a"));
        assert!(!l.contains("b"));
    }

    #[test]
    fn a_nested_rewind_takes_back_only_its_own_names() {
        let mut l = live(&[]);
        let outer = l.mark();
        l.insert("a".to_owned());
        let inner = l.mark();
        l.insert("b".to_owned());
        l.rewind(inner);
        assert!(l.contains("a"));
        assert_eq!(l.rewind(outer), vec!["a".to_owned()]);
    }

    #[test]
    fn a_join_keeps_the_names_both_ways_assigned() {
        let mut l = live(&["p"]);
        let m = l.mark();
        l.insert("a".to_owned());
        l.insert("b".to_owned());
        l.intersect_since(m, &["b".to_owned(), "c".to_owned()]);
        assert!(l.contains("p") && l.contains("b"));
        assert!(!l.contains("a") && !l.contains("c"));
        assert_eq!(l.rewind(m), vec!["b".to_owned()]);
    }

    #[test]
    fn every_way_must_assign_a_name_for_it_to_be_kept() {
        let ways = vec![
            vec!["a".to_owned(), "b".to_owned()],
            vec!["b".to_owned()],
            vec!["b".to_owned(), "c".to_owned()],
        ];
        assert_eq!(assigned_on_every_way(ways), Some(vec!["b".to_owned()]));
        assert_eq!(assigned_on_every_way(Vec::new()), None);
    }
}
