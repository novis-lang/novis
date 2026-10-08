//! The closed exception tree the compiler declares for every program.
//!
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) § 10
//! is authoritative for the tree's shape and its members; this module is the
//! one place that shape becomes data the rest of the compiler can read.
//! `rule:errors/on-limit` points
//! at § 10 rather than restating it, so nothing else here needs to.
//!
//! # Why a table rather than a written declaration
//!
//! `Throwable` cannot be written in Novis: its `backtrace` is grown by the
//! runtime as a throw propagates (`nvs_runtime::throwable`), and a source
//! declaration would need a body that has no legal spelling. Every consumer
//! that needs a class to exist therefore seeds itself from [`TREE`] —
//! `crate::hierarchy` for the `extends` links, `nvs_types::error_lib` for the
//! property/constructor signatures, `nvs_types::layout` for the slot order,
//! and `nvs_ir::lower` for the one synthesized constructor body.
//!
//! # There is no `Exception` and no `Error`
//!
//! Neither exists, deliberately: § 10 makes `Throwable` the root that user
//! classes extend directly, so a second root-shaped name would be a second
//! way to spell the same thing (`rule:core-api/shape-rules`
//! R20). A program naming either gets an ordinary undeclared-class
//! diagnostic. [`FINISH_MARKER`] is not that second name: no row extends it
//! and no `catch` arm matches it, and [`TREE`] says what it is for.

/// Every class the compiler declares with no source declaration to collect it
/// from, as `(name, parent)`: spec § 10's exception tree from [`ROOT`] down,
/// and after it [`FINISH_MARKER`], which is a root of its own and not an
/// exception at all.
///
/// Ordered parent-before-child so a consumer building a flattened supertype
/// set can walk it in one pass.
///
/// # The namespaced entries are here rather than in the registry
///
/// `Core\Test\Failure` is `rule:testing/failure-ledger`'s assertion failure, and that section makes it "an ordinary
/// `Throwable`" — so it is a *class in this tree* rather than a
/// `nvs_stdlib::registry` row, which is what buys it every property the root
/// declares, the inherited constructor, a slot layout, and a `catch` clause
/// that matches it by name with no case anywhere above this line. Its parent
/// is the root directly: a failed assertion is neither "the world said no"
/// nor one of `RuntimeError`'s narrower readings.
///
/// `Core\Cli\NotInteractive` is here for exactly that reason rather than by
/// analogy:
/// `rule:tooling/a-prompt-is-a-core-member` makes
/// it what a prompt throws when the process has no controlling terminal and
/// the call named no default, so a program that wants to fall back writes a
/// `catch` — and a `catch` matches a name in this tree and nothing else. Its
/// parent is `RuntimeError`, because "there is nobody to ask" is the world
/// saying no rather than a bug in the program: the same code is correct when
/// it is run from a terminal.
///
/// `Core\Db\RolledBack` is here for the same reason once
/// more: `rule:core-classes/db-transactions` makes
/// `Transaction::rollBack` throw it and
/// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) § 18
/// puts it *in this tree*, extending `RuntimeError` — a deliberate rollback is
/// the database saying no rather than a bug in the program, and the callable
/// that owns the transaction propagates it to a `catch` written by name. Like
/// `ParseError`, it declares a property of its own; see [`OWN_PROPERTIES`].
///
/// `Core\Db\DbError` is § 18's other half: everything the
/// server itself refused, which `rule:core-classes/db-error` makes **one** class carrying a normalised `kind`
/// rather than a class per condition, whose boundaries would differ per
/// driver. It sits beside `Core\Db\RolledBack`
/// under `RuntimeError` deliberately — a `catch` that has to tell "I gave up"
/// from "the database said no" is the whole reason § 18 spells two names. It
/// declares every one of § 8's properties in [`OWN_PROPERTIES`] — the normalised
/// `kind`, the raw `sqlState`, `driverCode` and `constraint` the server worded,
/// and the `sql` the program wrote — so the class carries the root's own set,
/// those, and a message that is what the server said.
///
/// `Core\Ldap\LdapError` is the same shape for a directory: every failure
/// `Core\Ldap` reports, as one class with a `kind` a program branches on and
/// the LDAP result `code` beside it (ADR 0278 § 10). Its `kind` is a different
/// enum from `Core\Db\DbError`'s, which is why `nvs_types::error_lib` types a
/// property by its class and not by its name alone.
///
/// `Core\DeprecatedError` is what a use of deprecated code throws under
/// `[errors] deprecated = "throw"`
/// (`rule:errors/a-use-of-deprecated-code-may-log-or-throw`). Its parent is
/// `LogicError`, because calling code its author retired is a fault in the
/// program and not the world saying no. It declares nothing of its own: the
/// message is `W1003`'s text.
///
/// Those are the entries whose names have more than one segment, which is
/// why every consumer here goes through `QName::parse` rather than treating a
/// row as a bare global segment. `QName::is_reserved_global_class`
/// deliberately still answers only for the single-segment rows: what makes
/// these trusted-to-exist is `QName::is_core`, the reserved `Core` namespace
/// (`rule:core-api/reserved-namespace`), which every site pairs with that predicate already.
///
/// # [`FINISH_MARKER`] is a root of its own
///
/// A request that ends through `Core\Script::finish()` still unwinds every
/// `finally` between the call and the request root, and the throw path is the
/// only unwind that runs them (`rule:errors/propagation`). So the value that
/// travels out is an ordinary raised object, and what keeps it out of every
/// `catch` is where it sits rather than a case anywhere: a class with no
/// parent conforms to nothing but itself — `nvs_runtime::object`'s
/// `ClassDesc::conforms_to_name` is self-or-ancestor by name — so an arm
/// naming anything under [`ROOT`] is false against it, and so is an `is` test
/// against any of those names.
/// It declares no properties and no constructor: there is nothing on it to
/// read and no spelling that builds one.
pub const TREE: &[(&str, Option<&str>)] = &[
    ("Throwable", None),
    ("LogicError", Some("Throwable")),
    ("RuntimeError", Some("Throwable")),
    ("IOError", Some("RuntimeError")),
    ("ParseError", Some("RuntimeError")),
    ("TimeoutError", Some("RuntimeError")),
    ("RecursionError", Some("RuntimeError")),
    ("ExtensionError", Some("RuntimeError")),
    ("ArithmeticError", Some("Throwable")),
    ("Core\\Test\\Failure", Some("Throwable")),
    ("Core\\Cli\\NotInteractive", Some("RuntimeError")),
    ("Core\\Db\\DbError", Some("RuntimeError")),
    ("Core\\Db\\RolledBack", Some("RuntimeError")),
    ("Core\\Ldap\\LdapError", Some("RuntimeError")),
    ("Core\\DeprecatedError", Some("LogicError")),
    (FINISH_MARKER, None),
];

/// The root of spec § 10's exception tree: every [`TREE`] entry but
/// [`FINISH_MARKER`] descends from it, and it is the one name a `catch` clause
/// can use to mean "anything at all".
pub const ROOT: &str = "Throwable";

/// [`TREE`]'s other root: the class `Core\Script::finish()` raises so that the
/// unwind out of a finished request runs every `finally` on its way to the
/// request root, and which no `catch` arm admits because it descends from
/// [`ROOT`] not at all. [`TREE`]'s own docs are why that is the whole
/// mechanism.
pub const FINISH_MARKER: &str = "Core\\Script\\Finished";

/// `Throwable`'s own instance properties, in slot order.
///
/// Slot order is load-bearing: `nvs_runtime::object` lays a
/// subclass's slots out *after* its parent's, so these indices are the
/// same for every exception class in existence — which is what lets
/// `nvs_runtime::throwable` reach `backtrace` on a value it knows nothing
/// else about.
pub const PROPERTIES: &[&str] = &["message", "previous", "backtrace", "location"];

/// The slot [`PROPERTIES`] puts `message` in.
pub const MESSAGE_SLOT: usize = 0;

/// The slot [`PROPERTIES`] puts `backtrace` in — the one the runtime appends
/// a frame label to as a throw propagates.
pub const BACKTRACE_SLOT: usize = 2;

/// Every [`TREE`] entry that declares instance properties **of its own**, in
/// slot order, keyed by class name.
///
/// [`PROPERTIES`] is the root's row; most of the tree inherits it and adds
/// nothing. `ParseError` is one exception, which
/// `rule:core-classes/derive-reports-every-field` gives an
/// `issues` list so that a decode reports **every** bad field from one throw
/// rather than the first. `Core\Db\RolledBack` is another, which spec § 18
/// gives a `reason` — the string
/// `rule:core-classes/db-transactions`'s `Transaction::rollBack`
/// was called with, readable from the `catch` outside the transaction callable
/// that the throw unwound. `Core\Db\DbError` is another, which
/// `rule:core-classes/db-error` gives a normalised `kind`
/// so that an application branches on the condition rather than on a vendor
/// code, and beside it the raw `sqlState`, `driverCode` and `constraint` it was
/// read off plus the `sql` that was refused. `Core\Ldap\LdapError` is the
/// last, with ADR 0278 § 10's `kind` and `code`. Every one of spec § 18's is here,
/// and none of them could have been added without a type in
/// `nvs_types::error_lib::own_properties`, which `panic!`s at seed time on a
/// property it cannot type. That is a *narrowing* rule and not a queue: a row
/// added here without the arm there fails the very first seed.
///
/// Each is declared on its own class rather than on the root deliberately: the
/// root is allocated by every `throw` in every program, and another slot there
/// would cost sixteen bytes plus one empty-array allocation on a path that
/// PHP-shaped code takes for ordinary control flow
/// (`rule:errors/propagation`'s measured cost).
pub const OWN_PROPERTIES: &[(&str, &[&str])] = &[
    (ROOT, PROPERTIES),
    ("ParseError", ISSUES),
    ("Core\\Db\\DbError", KIND),
    ("Core\\Db\\RolledBack", REASON),
    ("Core\\Ldap\\LdapError", LDAP_KIND),
];

/// `ParseError`'s own row of [`OWN_PROPERTIES`].
const ISSUES: &[&str] = &["issues"];

/// `Core\Db\DbError`'s own row of [`OWN_PROPERTIES`] — `rule:core-classes/db-error`'s
/// properties, all of them, in § 8's own order. The order is the rule and not
/// an accident of how they arrived: a further value § 8 names is appended here
/// too, so that no slot already compiled into a program moves.
const KIND: &[&str] = &["kind", "sqlState", "driverCode", "constraint", "sql"];

/// `Core\Db\RolledBack`'s own row of [`OWN_PROPERTIES`].
const REASON: &[&str] = &["reason"];

/// `Core\Ldap\LdapError`'s own row of [`OWN_PROPERTIES`]: the kind, then the
/// result code. A property added later is appended, for [`KIND`]'s reason.
const LDAP_KIND: &[&str] = &["kind", "code"];

/// The slot `ParseError::$issues` occupies.
///
/// `ParseError` descends from the root through `RuntimeError`, and neither
/// declares anything of its own, so its first own slot sits immediately after
/// [`PROPERTIES`] — `parse_error_s_own_slots_start_after_the_root_s` is what
/// holds that rather than a comment.
pub const ISSUES_SLOT: usize = PROPERTIES.len();

/// The slot `Core\Db\RolledBack::$reason` occupies.
///
/// The same arithmetic as [`ISSUES_SLOT`] and for the same reason: nothing
/// between `Core\Db\RolledBack` and the root declares a slot, so its one own
/// property sits immediately after [`PROPERTIES`]. It is
/// `rolled_back_s_own_slot_starts_after_the_root_s` that holds that, not this
/// sentence — the two classes are siblings under `RuntimeError` and neither
/// index is derived from the other.
pub const REASON_SLOT: usize = PROPERTIES.len();

/// The slot `Core\Db\DbError::$kind` occupies.
///
/// The same arithmetic as [`REASON_SLOT`], and the two are equal for the same
/// reason they are separate constants: `Core\Db\DbError` and
/// `Core\Db\RolledBack` are siblings under `RuntimeError`, so neither index is
/// derived from the other and a property added to either must not move the
/// other's. `db_error_s_own_slot_starts_after_the_root_s` holds it.
///
/// It is 0-relative-to-the-root and stays there: § 8's others are appended
/// to [`KIND`] after it rather than inserted before it, so `kind` keeps this
/// index. They are [`SQL_STATE_SLOT`], [`DRIVER_CODE_SLOT`],
/// [`CONSTRAINT_SLOT`] and [`SQL_SLOT`].
pub const KIND_SLOT: usize = PROPERTIES.len();

/// The slot `Core\Db\DbError::$sqlState` occupies — `rule:core-classes/db-error`'s raw
/// five-character code, beside the kind normalised from it.
///
/// Derived from [`KIND_SLOT`] rather than from [`PROPERTIES`], which is the
/// opposite of how that constant relates to its siblings: two properties of one
/// class are in declaration order by rule, where two classes sharing an index
/// only coincide. `nvs_runtime::SQL_STATE_SLOT` is the runtime's copy.
pub const SQL_STATE_SLOT: usize = KIND_SLOT + 1;

/// The slot `Core\Db\DbError::$driverCode` occupies — `rule:core-classes/db-error`'s vendor
/// integer, which is `null` wherever the driver has no code the `SQLSTATE` does
/// not already carry. `nvs_runtime::DRIVER_CODE_SLOT` is the runtime's copy.
pub const DRIVER_CODE_SLOT: usize = KIND_SLOT + 2;

/// The slot `Core\Db\DbError::$constraint` occupies — `rule:core-classes/db-error`'s name of
/// the constraint the condition violated, where the condition names one.
///
/// Derived from [`KIND_SLOT`] like its siblings above, and `?string` for a
/// reason of its own rather than theirs: most of § 8's kinds name no
/// constraint at all — a syntax error, a permission, a timeout — so the absent
/// case here is the common one and not a driver's gap.
/// `nvs_runtime::CONSTRAINT_SLOT` is the runtime's copy.
pub const CONSTRAINT_SLOT: usize = KIND_SLOT + 3;

/// The slot `Core\Db\DbError::$sql` occupies — `rule:core-classes/db-error`'s statement text,
/// the last of the row and the only one the client rather than the server
/// worded.
///
/// Derived from [`KIND_SLOT`] like its siblings above, and `?string` for
/// a reason of its own again: a refusal is not always *of* a statement the caller
/// spelled, since § 7's `BEGIN`, `COMMIT` and `SAVEPOINT` are the runtime's own
/// text. `nvs_runtime::SQL_SLOT` is the runtime's copy.
pub const SQL_SLOT: usize = KIND_SLOT + 4;

/// The slot `Core\Ldap\LdapError::$kind` occupies.
///
/// Equal to [`KIND_SLOT`] for that constant's reason and not derived from it:
/// `Core\Ldap\LdapError` is another sibling under `RuntimeError`, so its first
/// own slot lands right after [`PROPERTIES`] too.
/// `ldap_error_s_own_slots_start_after_the_root_s` holds it, and
/// `nvs_runtime::LDAP_KIND_SLOT` is the runtime's copy.
pub const LDAP_KIND_SLOT: usize = PROPERTIES.len();

/// The slot `Core\Ldap\LdapError::$code` occupies — the LDAP result code, or
/// `null` where no server sent one. `nvs_runtime::LDAP_CODE_SLOT` is the
/// runtime's copy.
pub const LDAP_CODE_SLOT: usize = LDAP_KIND_SLOT + 1;

/// `name`'s own instance properties, in slot order — empty for a class that
/// declares none, and for a name that is not in [`TREE`] at all.
///
/// The one reader that matters is `nvs_types::layout`, which appends these
/// after every ancestor's; `nvs_types::error_lib` seeds the same list as
/// signatures.
#[must_use]
pub fn own_properties(name: &str) -> &'static [&'static str] {
    OWN_PROPERTIES
        .iter()
        .find(|(entry, _)| *entry == name)
        .map_or(&[], |(_, properties)| *properties)
}

/// Whether `name` declares a synthesized constructor of its own.
///
/// Exactly the classes with own properties: a constructor exists to assign
/// them (`rule:classes/definite-property-initialization`),
/// so a class that adds none inherits its parent's and needs no second one.
/// `nvs_ir::lower::exception` is what actually builds each body.
#[must_use]
pub fn declares_constructor(name: &str) -> bool {
    !own_properties(name).is_empty()
}

/// Whether `name` is one of [`TREE`]'s entries, spelled exactly as that table
/// spells it — a bare global segment for all but the namespaced rows.
#[must_use]
pub fn is_exception_class(name: &str) -> bool {
    TREE.iter().any(|(entry, _)| *entry == name)
}

/// Every class `name` also *is*, transitively, excluding itself — `None` if
/// `name` is not in [`TREE`] at all.
#[must_use]
pub fn conforms_to(name: &str) -> Option<Vec<&'static str>> {
    let mut current = TREE.iter().find(|(entry, _)| *entry == name)?.1;
    let mut out = Vec::new();
    while let Some(parent) = current {
        out.push(parent);
        current = TREE
            .iter()
            .find(|(entry, _)| *entry == parent)
            .and_then(|(_, up)| *up);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_but_a_root_has_a_parent_that_is_itself_an_entry() {
        for (name, parent) in TREE {
            match parent {
                None => assert!(*name == ROOT || *name == FINISH_MARKER, "{name}"),
                Some(parent) => assert!(is_exception_class(parent), "{parent} is not in the tree"),
            }
        }
    }

    /// The whole of what keeps a finished request out of every `catch`: the
    /// marker is in the table, so a `catch` resolves the name and the runtime
    /// finds the descriptor, and it is above nothing, so the self-or-ancestor
    /// walk every arm goes through is false against it.
    #[test]
    fn the_finish_marker_is_a_root_of_its_own_and_is_above_and_below_nothing() {
        assert!(is_exception_class(FINISH_MARKER));
        assert_eq!(conforms_to(FINISH_MARKER), Some(Vec::new()));
        assert!(own_properties(FINISH_MARKER).is_empty());
        assert!(!declares_constructor(FINISH_MARKER));
        assert!(
            TREE.iter()
                .all(|(_, parent)| *parent != Some(FINISH_MARKER))
        );
    }

    #[test]
    fn a_leaf_conforms_to_every_class_above_it() {
        assert_eq!(
            conforms_to("TimeoutError"),
            Some(vec!["RuntimeError", "Throwable"])
        );
        assert_eq!(conforms_to("LogicError"), Some(vec!["Throwable"]));
        assert_eq!(conforms_to("Throwable"), Some(Vec::new()));
        assert_eq!(conforms_to("Animal"), None);
    }

    #[test]
    fn parse_error_s_own_slots_start_after_the_root_s() {
        // `ISSUES_SLOT` is a constant other crates restate, so what it depends
        // on is checked rather than remembered: nothing between `ParseError`
        // and the root contributes a slot.
        let above = conforms_to("ParseError").expect("ParseError is in the tree");
        let inherited: usize = above.iter().map(|name| own_properties(name).len()).sum();
        assert_eq!(inherited, ISSUES_SLOT);
        assert_eq!(own_properties("ParseError"), &["issues"]);
    }

    #[test]
    fn rolled_back_s_own_slot_starts_after_the_root_s() {
        // The same claim `parse_error_s_own_slots_start_after_the_root_s`
        // makes, asserted separately because the two are siblings: a slot
        // added to either one must not move the other, and a single test
        // written over one of them would not notice if it did.
        let above = conforms_to("Core\\Db\\RolledBack").expect("RolledBack is in the tree");
        let inherited: usize = above.iter().map(|name| own_properties(name).len()).sum();
        assert_eq!(inherited, REASON_SLOT);
        assert_eq!(own_properties("Core\\Db\\RolledBack"), &["reason"]);
        assert_eq!(above, vec!["RuntimeError", "Throwable"]);
    }

    #[test]
    fn db_error_s_own_slot_starts_after_the_root_s() {
        // The same claim once more, asserted separately for the reason
        // `rolled_back_s_own_slot_starts_after_the_root_s` gives: these two are
        // siblings, and one growing a property must not move the other's slot.
        let above = conforms_to("Core\\Db\\DbError").expect("DbError is in the tree");
        let inherited: usize = above.iter().map(|name| own_properties(name).len()).sum();
        assert_eq!(inherited, KIND_SLOT);
        // The one class in the tree declaring more than one property, so it is
        // also the only place the *order* of a row is load-bearing: § 8's
        // others are appended, which is what keeps `kind` at slot 0 relative to
        // the root however many of them there are.
        assert_eq!(
            own_properties("Core\\Db\\DbError"),
            &["kind", "sqlState", "driverCode", "constraint", "sql"]
        );
        assert_eq!(inherited + 1, SQL_STATE_SLOT);
        assert_eq!(inherited + 2, DRIVER_CODE_SLOT);
        assert_eq!(inherited + 3, CONSTRAINT_SLOT);
        assert_eq!(inherited + 4, SQL_SLOT);
        assert_eq!(above, vec!["RuntimeError", "Throwable"]);
    }

    #[test]
    fn ldap_error_s_own_slots_start_after_the_root_s() {
        // The sibling claim once more, for `rolled_back_s_own_slot_starts_after_the_root_s`'s reason.
        let above = conforms_to("Core\\Ldap\\LdapError").expect("LdapError is in the tree");
        let inherited: usize = above.iter().map(|name| own_properties(name).len()).sum();
        assert_eq!(inherited, LDAP_KIND_SLOT);
        assert_eq!(own_properties("Core\\Ldap\\LdapError"), &["kind", "code"]);
        assert_eq!(inherited + 1, LDAP_CODE_SLOT);
        assert_eq!(above, vec!["RuntimeError", "Throwable"]);
    }

    #[test]
    fn only_the_root_parse_error_and_the_db_and_ldap_errors_declare_anything_of_their_own() {
        for (name, _) in TREE {
            let expected = *name == ROOT
                || *name == "ParseError"
                || *name == "Core\\Db\\DbError"
                || *name == "Core\\Db\\RolledBack"
                || *name == "Core\\Ldap\\LdapError";
            assert_eq!(declares_constructor(name), expected, "{name}");
        }
        for (name, _) in OWN_PROPERTIES {
            assert!(is_exception_class(name), "{name} is not in the tree");
        }
    }

    /// `rule:packaging/a-guest-crash-throws`: a handler that catches runtime
    /// errors catches a guest's trap, and the message is all it carries.
    #[test]
    fn extension_error_is_a_runtime_error_with_no_property_of_its_own() {
        assert!(is_exception_class("ExtensionError"));
        assert_eq!(
            conforms_to("ExtensionError"),
            Some(vec!["RuntimeError", "Throwable"])
        );
        assert!(own_properties("ExtensionError").is_empty());
        assert!(!declares_constructor("ExtensionError"));
    }

    #[test]
    fn php_s_exception_and_error_are_not_in_the_tree() {
        assert!(!is_exception_class("Exception"));
        assert!(!is_exception_class("Error"));
    }
}
