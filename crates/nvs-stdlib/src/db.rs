//! `Core\Db` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 18's entry points, over [`nvs_db`]'s wire half.
//!
//! [ADR 0067](../../../../docs/adr/0067-core-db.md) is authoritative for every
//! semantic and § 18 for every signature. What belongs here is the two
//! decisions this side of the boundary owns: what an `InList` *is* once it is a
//! value a program holds, and what `quoteIdentifier` can honestly promise from
//! a class that has no connection in front of it.
//!
//! # `inList` is a carrier, and the expansion stays on the wire
//!
//! ADR 0067 § 5 makes `Core\Db::inList($values)` the explicit marker that a
//! bound parameter expands into a parenthesised run of placeholders, because
//! automatic expansion would make the *SQL text* depend on a runtime value's
//! type. The marker has to survive as a value from the call that builds it to
//! the bind that consumes it, so it is a `Core\Db\InList` — one
//! [`crate::instance`] object over one slot holding the array itself.
//!
//! **It holds the array and computes nothing.** The rendering already exists
//! once, as `nvs_db::sql`'s `expand`, which is where it has to be: the markers
//! are `$1` on PostgreSQL and `@p1` on SQL Server, so only a driver knows what
//! this expands to, and § 1's statement cache keys on the arity that expansion
//! produces. A second rendering here would be a second answer to a question the
//! rewriter already answers, and it would have to guess the dialect.
//!
//! So the only thing this side decides is the refusal. `inList([])` throws
//! `LogicError` at the *call* rather than at the bind, which is § 5's rule and
//! is also the earlier of the two places it can be caught — the caller who
//! wrote the empty list is on the stack, and the branch § 5 asks them to write
//! is the one they are standing in.
//!
//! # `quoteIdentifier` validates, and deliberately adds no delimiter
//!
//! § 18 puts this member on `Core\Db`, which means it runs with **no
//! connection and therefore no dialect**. The five backends do not agree on how
//! an identifier is delimited — `"name"` on PostgreSQL and SQLite, backticks on
//! MySQL and MariaDB, `"name"` or `[name]` on SQL Server — so a delimiter
//! chosen here is wrong on at least two of them, and a *wrong* delimiter is not
//! a cosmetic problem: on MySQL a double-quoted identifier is a string literal,
//! so the answer would parse and mean something else.
//!
//! What laundering owes [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
//! § 4 is that the result cannot end the identifier and start something else.
//! A strict character class — a letter or `_`, then letters, digits or `_` —
//! guarantees that under *every* dialect at once, which no quoting scheme does:
//! the accepted alphabet contains no quote, no backtick, no bracket, no space,
//! no dot, no semicolon and no comment opener, so there is nothing left in the
//! answer for any parser to act on. That is why this member returns its
//! argument unchanged and refuses everything else, rather than escaping.
//!
//! The cost is stated rather than hidden. A name that genuinely needs
//! delimiting — a reserved word, a name with a space, a mixed-case name on
//! PostgreSQL — is **refused**, not mangled, and the caller sees a `LogicError`
//! naming the name. A `$` is refused too, though four of the five backends
//! accept one inside an identifier: it is the one character in the plausible
//! set that also opens PostgreSQL's dollar-quoting, and the rule is worth more
//! than the character.
//!
//! There is no dotted `schema.table` form for the same reason there is no
//! delimiter: a dot is a separator the caller writes between two calls, so the
//! member stays "one identifier in, one identifier out" and cannot be handed
//! something whose halves it did not each check.
//!
//! # Known gaps
//!
//! 1. **`connect` and `open` are not here yet**, so nothing in this module
//!    reaches a database and the class carries no capability row in
//!    [`crate::registry::CAPABILITIES`]. The two members that are here need
//!    neither: they are pure functions of their arguments, which is why they
//!    could land before anything resolves a `[db.<name>]` block.
//! 2. **Nothing consumes an [`IN_LIST`] yet.** The carrier is built and held;
//!    the bind that reads its slot back arrives with `Core\Db\Queryable`, and
//!    the arity it produces is `nvs_db::sql`'s `Binding::List`.
//! 3. **A delimiting quoter, if one is ever wanted, belongs on `Connection`**
//!    and not here — that is the only place a dialect exists. § 18 does not ask
//!    for one, and this module's second decision above is why adding it to
//!    `Core\Db` cannot be the answer.

use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// The class name, once, for the messages and the rows that all name it.
const NAME: &str = r"Core\Db";

/// The carrier class's fully-qualified name, as
/// [`CoreTy::Instance`] spells it.
///
/// `pub(crate)` for the one reader outside this module: `registry`'s
/// `a_class_with_slots_has_instance_members_and_the_reverse` names every class
/// that is a handle — slots and no members — and this is one of them.
pub(crate) const IN_LIST_NAME: &str = r"Core\Db\InList";

/// The one slot an [`IN_LIST`] holds: the array `inList` was given, kept whole
/// so the bind that expands it counts the same elements the caller passed.
const VALUES_SLOT: &str = "values";

/// Spec § 18's `Core\Db` — its two connectionless entry points. `connect` and
/// `open` join this roster above them, in the spec's own order, once a
/// `[db.<name>]` block resolves into a target.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "inList",
            names: &["values"],
            params: &[CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Instance(IN_LIST_NAME),
            symbol: "nvs_core_db_in_list",
            doc: Some(&IN_LIST_DOC),
        },
        CoreMethod {
            name: "quoteIdentifier",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_db_quote_identifier",
            doc: Some(&QUOTE_IDENTIFIER_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// Spec § 18's `Core\Db\InList` — opaque, produced by one member and read by
/// the bind. No members at all, which is that table's own "accepted only as a
/// bound parameter".
pub(crate) const IN_LIST: CoreClass = CoreClass {
    name: IN_LIST_NAME,
    methods: &[],
    instance: &[],
    slots: &[VALUES_SLOT],
    constants: &[],
};

/// `Core\Db::inList`'s reference card — ADR 0117.
const IN_LIST_DOC: MethodDoc = MethodDoc {
    short: "Marks `$values` as a run of bound values rather than one, so the placeholder it is \
            bound to expands into a parenthesised list of that many — the `IN (?, ?, ?)` every \
            PHP program builds with `implode` and `array_fill`. Nothing else in a statement \
            expands, which is what keeps the SQL text independent of what a value turned out \
            to be.",
    params: &[ParamDoc {
        name: "values",
        desc: "The values to bind, one placeholder each, in the array's own order. Keys are not \
               read: a marker binds positions inside one placeholder, not names.",
        shape: &[],
    }],
    ret: "A `Core\\Db\\InList` to bind to a single placeholder. It has no members and is accepted \
          nowhere else; two lists of different lengths bound to the same SQL are two entries in \
          the statement cache, because expansion changes the statement's arity.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$values` is empty, which means \"match nothing\" inside `IN` and \"match \
               everything\" inside `NOT IN` — the caller branches instead.",
    }],
};

/// `Core\Db::quoteIdentifier`'s reference card — ADR 0117.
const QUOTE_IDENTIFIER_DOC: MethodDoc = MethodDoc {
    short: "Checks that `$name` is a bare SQL identifier — a letter or `_`, then letters, digits \
            or `_` — and answers it unchanged and no longer `tainted`, so it can be written into \
            the text of a statement. Replaces escaping a table or column name by hand, which is \
            what `mysqli_real_escape_string` was doing there.",
    params: &[ParamDoc {
        name: "name",
        desc: "The identifier to check. It is answered exactly as given: nothing is escaped, \
               truncated or lower-cased, because a name this accepts needs none of it.",
        shape: &[],
    }],
    ret: "The same text, without the `tainted` qualifier. It carries no delimiter — `Core\\Db` \
          has no connection and so no dialect, and the five backends disagree on what a \
          delimiter is.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$name` is empty, starts with a digit, or holds any character outside letters, \
               digits and `_` — including a name that would need delimiting to be legal.",
    }],
};

nvs_runtime::nvs_helper! {
    /// `Core\Db::inList(array<mixed> $values): Db\InList` — ADR 0067 § 5's
    /// explicit expansion marker.
    ///
    /// The body is the refusal and a carrier around the argument: what the
    /// marker expands *to* is the driver's, for the reason this module's docs
    /// give. The array is retained rather than copied, so a list of a thousand
    /// ids costs one reference and not a second array.
    fn nvs_core_db_in_list(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is `array<mixed>` in `CLASS`
        // above, so a non-container argument is refused at `E0401` before any
        // of this runs. The guard is what makes the count below sound.
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Db::inList expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for the length of this call"
        )]
        let count = unsafe { nvs_runtime::nvs_array_count(array) };
        if count == 0 {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "Core\\Db::inList() was given an empty list, and ADR 0067 § 5 refuses one: an \
                 empty list matches nothing inside `IN` and everything inside `NOT IN`, the \
                 rewriter cannot tell which it is in, and the caller has to branch"
                    .to_owned(),
            ));
        }
        let values = args[0];
        #[expect(
            unsafe_code,
            reason = "the array is owned by the caller's argument slot, which \
                      outlives this call, so the slot this builds needs a \
                      reference of its own"
        )]
        unsafe {
            values.retain();
        }
        Ok(crate::instance::build(&IN_LIST, [values]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::quoteIdentifier(tainted string $name): string` — ADR 0024
    /// § 4's launderer for the statement-text sink.
    ///
    /// It validates rather than escapes, and this module's docs are the whole
    /// argument for why: with no connection there is no dialect, and the
    /// character class below is the one answer that is safe under all five at
    /// once.
    fn nvs_core_db_quote_identifier(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is a `string` in `CLASS` above,
        // so a non-text argument is refused at `E0401` first — the same
        // judgement `Core\Arr::count`'s guard states.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Db::quoteIdentifier expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        if !is_bare_identifier(name) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "Core\\Db::quoteIdentifier() takes a bare identifier — a letter or `_`, then \
                     letters, digits or `_` — and `{name}` is not one. `Core\\Db` has no \
                     connection and so no dialect to delimit for, so a name that needs \
                     delimiting is refused rather than quoted for the wrong backend"
                ),
            ));
        }
        Ok(Value::str(NvsStr::new(name.as_bytes())))
    }
}

/// Whether `name` is an identifier every backend reads the same way, and which
/// carries nothing into the statement text it is written into.
///
/// ASCII on purpose. Every backend also accepts some set of non-ASCII letters,
/// and no two of those sets are the same — `char::is_alphabetic` would accept a
/// name PostgreSQL takes and SQL Server folds differently, which is exactly the
/// dialect dependence this member exists to avoid having.
fn is_bare_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_db_in_list" => (nvs_core_db_in_list as *const ()).cast(),
        "nvs_core_db_quote_identifier" => (nvs_core_db_quote_identifier as *const ()).cast(),
        _ => return None,
    })
}
