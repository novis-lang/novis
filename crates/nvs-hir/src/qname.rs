//! Fully-qualified names: `Foo`, `App\Models\User`, `Core\Str`.

use std::fmt;

/// A fully-qualified, backslash-separated name with no leading backslash.
///
/// Every [`crate::symbol::Symbol`] is keyed by one of these rather than by the
/// short name a declaration was written with, so a class declared inside
/// `namespace App\Models;` and one declared inside `namespace App\Http;` never
/// collide even when both are named `User`.
///
/// Comparison is **case-sensitive**, deliberately: PHP resolves a class or
/// namespace segment case-insensitively, which is what lets a name work on one
/// machine and not another. This is a decided property, not a gap —
/// `rule:classes/names-resolve-case-sensitively` states it, and
/// `rule:programs/autoload` already relies on it to keep an autoloaded file's on-disk name exact.
/// Do not "fix" it back toward PHP.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct QName {
    segments: Vec<String>,
}

impl QName {
    /// Parses raw source text into its segments.
    ///
    /// Nothing arrives here with a leading separator, so nothing strips one.
    /// The parser refuses that spelling
    /// (`rule:statements/a-leading-separator-does-not-parse`,
    /// `E0240`) and, on the recovery path where it reports and keeps going,
    /// leaves it outside the [`nvs_syntax::ast::Name`]'s span — so a name's
    /// text is the name. Every other caller builds from a string this compiler
    /// wrote, and none of those carries one either.
    ///
    /// # Panics
    ///
    /// Panics if `text` is empty, which would indicate a bug in the caller —
    /// every [`nvs_syntax::ast::Name`] span covers at least one identifier
    /// character.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let segments: Vec<String> = text.split('\\').map(str::to_owned).collect();
        assert!(
            !segments.is_empty(),
            "a name must have at least one segment"
        );
        Self { segments }
    }

    /// Builds a name by appending one short (unqualified) segment onto a
    /// namespace path — the shape every declaration site produces, since
    /// [`nvs_syntax::ast::Name`] at a declaration is never itself qualified.
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

    /// The class a string literal under `as class<T>` names
    /// (`rule:types/class-reference`): its cooked text read as the class's
    /// whole name, so no `namespace` and no `use` applies to it. `None` when
    /// the text is not a name at all — empty, a leading or trailing `\`, two
    /// separators in a row, or a segment that is not an identifier.
    ///
    /// The harvest that loads the class ([`crate::requires`]) and the checker
    /// that decides the conversion both read the literal through this, so the
    /// file loaded and the class checked are always the same one.
    #[must_use]
    pub fn from_literal(text: &str) -> Option<Self> {
        let is_identifier = |segment: &str| {
            let mut chars = segment.chars();
            chars
                .next()
                .is_some_and(|first| first == '_' || first.is_alphabetic())
                && chars.all(|c| c == '_' || c.is_alphanumeric())
        };
        let segments: Vec<String> = text.split('\\').map(str::to_owned).collect();
        segments
            .iter()
            .all(|segment| is_identifier(segment))
            .then_some(Self { segments })
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

    /// Whether this name's root segment is `Core`, Novis's reserved namespace
    /// for built-ins (`rule:core-api/reserved-namespace`).
    ///
    /// `Core`'s own classes are not yet declarations `nvs-hir` can see —
    /// `nvs-stdlib` doesn't exist until a later milestone — so a `use`
    /// importing one is trusted to exist rather than checked against a real
    /// symbol table.
    #[must_use]
    pub fn is_core(&self) -> bool {
        self.segments
            .first()
            .is_some_and(|s| s.eq_ignore_ascii_case("Core"))
    }

    /// Whether this name is one of the handful of global, reserved
    /// interfaces the compiler itself knows about — the same "trusted to
    /// exist, never declared in source" treatment [`Self::is_core`] gives
    /// `Core`. [`crate::interfaces::RESERVED`] is the roster and the one
    /// place a new entry is added.
    #[must_use]
    pub fn is_reserved_global_interface(&self) -> bool {
        self.segments.len() == 1 && crate::interfaces::is_reserved_interface(&self.segments[0])
    }

    /// Whether this name is one of the global exception classes
    /// [`crate::errors::TREE`] declares — trusted to exist without a source
    /// declaration the same way [`Self::is_core`] trusts `Core`, since none
    /// of them has a spelling Novis could declare (see that module's docs).
    /// Kept separate from [`Self::is_reserved_global_interface`]: these are
    /// ordinary classes reached via `extends`/`new`, not interfaces reached
    /// via `implements`.
    ///
    /// PHP's `Exception` and `Error` are deliberately *not* among them.
    ///
    /// [`crate::errors::TREE`]'s one namespaced row, `Core\Test\Failure`, is
    /// deliberately not answered here: it is trusted to exist through
    /// [`Self::is_core`] instead, which every caller of this predicate already
    /// tests beside it.
    #[must_use]
    pub fn is_reserved_global_class(&self) -> bool {
        self.segments.len() == 1 && crate::errors::is_exception_class(&self.segments[0])
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

    #[test]
    fn is_reserved_global_interface_recognizes_every_rostered_name() {
        assert!(QName::parse("Comparable").is_reserved_global_interface());
        assert!(QName::parse("Stringable").is_reserved_global_interface());
        assert!(QName::parse("Iterable").is_reserved_global_interface());
        assert!(QName::parse("Iterator").is_reserved_global_interface());
        assert!(QName::parse("PropertyObserver").is_reserved_global_interface());
        assert!(QName::parse("Parses").is_reserved_global_interface());
        assert!(!QName::parse("Countable").is_reserved_global_interface());
        assert!(!QName::parse("App\\Comparable").is_reserved_global_interface());
    }

    #[test]
    fn is_reserved_global_class_recognizes_the_exception_tree_and_nothing_else() {
        assert!(QName::parse("Throwable").is_reserved_global_class());
        assert!(QName::parse("LogicError").is_reserved_global_class());
        assert!(QName::parse("TimeoutError").is_reserved_global_class());
        assert!(!QName::parse("Exception").is_reserved_global_class());
        assert!(!QName::parse("Error").is_reserved_global_class());
        assert!(!QName::parse("Comparable").is_reserved_global_class());
        assert!(!QName::parse("App\\LogicError").is_reserved_global_class());
    }
}
