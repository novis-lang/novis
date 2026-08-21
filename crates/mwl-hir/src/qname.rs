//! Fully-qualified names: `Foo`, `App\Models\User`, `Core\Str`.

use std::fmt;

/// A fully-qualified, backslash-separated name with no leading backslash.
///
/// Every [`crate::symbol::Symbol`] is keyed by one of these rather than by the
/// short name a declaration was written with, so a class declared inside
/// `namespace App\Models;` and one declared inside `namespace App\Http;` never
/// collide even when both are named `User`.
///
/// **Known gap:** comparison is case-sensitive. PHP treats a class/namespace
/// segment case-insensitively; MWL does not do that yet, since no ADR has
/// decided whether to keep that divergence-prone PHP behaviour. Revisit before
/// this leaves M2.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct QName {
    segments: Vec<String>,
}

impl QName {
    /// Parses raw source text — `\`-qualified or not — into its segments. A
    /// leading `\` (PHP's fully-qualified-name syntax) is stripped, since this
    /// type always stores the fully-resolved form with no ambiguity left to
    /// carry.
    ///
    /// # Panics
    ///
    /// Panics if `text` is empty, which would indicate a bug in the caller —
    /// every [`mwl_syntax::ast::Name`] span covers at least one identifier
    /// character.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let segments: Vec<String> = text
            .trim_start_matches('\\')
            .split('\\')
            .map(str::to_owned)
            .collect();
        assert!(
            !segments.is_empty(),
            "a name must have at least one segment"
        );
        Self { segments }
    }

    /// Builds a name by appending one short (unqualified) segment onto a
    /// namespace path — the shape every declaration site produces, since
    /// [`mwl_syntax::ast::Name`] at a declaration is never itself qualified.
    #[must_use]
    pub fn join(namespace: &[String], short_name: &str) -> Self {
        let mut segments = namespace.to_vec();
        segments.push(short_name.to_owned());
        Self { segments }
    }

    /// Builds a name directly from already-split segments — for a resolver
    /// that has computed the segments itself (e.g. an import's target with a
    /// qualified remainder appended) rather than starting from raw source
    /// text.
    ///
    /// # Panics
    ///
    /// Panics if `segments` is empty, same as [`Self::parse`].
    #[must_use]
    pub fn from_segments(segments: Vec<String>) -> Self {
        assert!(
            !segments.is_empty(),
            "a name must have at least one segment"
        );
        Self { segments }
    }

    /// The path's segments, root to leaf.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// The last segment — the name a bare, unqualified reference resolves
    /// under once imported.
    #[must_use]
    pub fn short_name(&self) -> &str {
        self.segments
            .last()
            .expect("a QName always has at least one segment")
    }

    /// Whether this name's root segment is `Core`, MWL's reserved namespace
    /// for built-ins ([ADR 0011](../../../docs/adr/0011-functions-and-constants-are-class-members.md)
    /// § 2).
    ///
    /// `Core`'s own classes are not yet declarations `mwl-hir` can see —
    /// `mwl-stdlib` doesn't exist until a later milestone — so a `use`
    /// importing one is trusted to exist rather than checked against a real
    /// symbol table.
    #[must_use]
    pub fn is_core(&self) -> bool {
        self.segments
            .first()
            .is_some_and(|s| s.eq_ignore_ascii_case("Core"))
    }
}

impl fmt::Display for QName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segments.join("\\"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_strips_a_leading_backslash() {
        assert_eq!(QName::parse("\\App\\User"), QName::parse("App\\User"));
    }

    #[test]
    fn parse_splits_on_backslash() {
        let q = QName::parse("App\\Models\\User");
        assert_eq!(q.segments(), ["App", "Models", "User"]);
        assert_eq!(q.short_name(), "User");
    }

    #[test]
    fn join_appends_a_short_name_to_a_namespace() {
        let ns = vec!["App".to_owned(), "Models".to_owned()];
        let q = QName::join(&ns, "User");
        assert_eq!(q, QName::parse("App\\Models\\User"));
    }

    #[test]
    fn join_onto_the_global_namespace_is_just_the_short_name() {
        let q = QName::join(&[], "Foo");
        assert_eq!(q, QName::parse("Foo"));
    }

    #[test]
    fn display_joins_segments_with_backslashes() {
        assert_eq!(QName::parse("App\\User").to_string(), "App\\User");
    }

    #[test]
    fn is_core_matches_case_insensitively_on_the_root_segment_only() {
        assert!(QName::parse("Core\\Str").is_core());
        assert!(QName::parse("core\\Str").is_core());
        assert!(!QName::parse("App\\Core").is_core());
    }
}
