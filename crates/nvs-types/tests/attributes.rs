//! `rule:attributes/structural-retrieval` and `rule:attributes/retrieval-folds-while-checking`'s retrieval, asserted where it is *decided*: an attached
//! attribute list is fixed by its source, so `Core\Attributes::get<T>` is
//! answered while checking and the call is replaced by that answer.
//!
//! What each fixture below asks is therefore not "what did the program print"
//! but "what is recorded at the call's own span" — an
//! [`ExprInfo::CoreConst`], which is the same entry a class constant leaves
//! and the whole of what `nvs-ir` finds there. A retrieval that reached a
//! runtime lookup would record an `ExprInfo::Call` instead, and
//! `nvs_stdlib::attributes`' two symbols name a body that aborts if it is ever
//! reached, so the absence of that entry is the no-runtime-lookup half of § 5.
//!
//! The refusals share `tests/conformance/reject/`'s
//! `an-attribute-retrieval-is-refused-where-it-cannot-be-folded.nvst`, which
//! pins their rendered text; what is added here is the *fold*, which prints
//! nothing and so cannot be seen from a conformance case at all.

mod common;

use common::check_src_declared;
use nvs_diagnostics::{Code, Diagnostics, code};
use nvs_types::defaults::ConstArg;
use nvs_types::expr_table::ExprInfo;

/// Whether `diags` reported `want`. By code rather than by `has_errors`, for
/// `derive.rs`'s reason: a fixture written to trip one rule routinely trips a
/// second, and "some error was reported" is satisfied by the wrong one.
fn reported(diags: &Diagnostics, want: Code) -> bool {
    diags.iter().any(|d| d.code == Some(want))
}

#[test]
fn get_with_no_matching_attribute_folds_to_a_constant_null() {
    // Something *is* attached to `$id` and it does not satisfy the retrieved
    // shape — the case a table keyed by "does this declaration carry
    // attributes" would get wrong, since § 4 matches on shape and never on
    // presence.
    let (diags, run) = check_src_declared(
        "<?nvs
class Row {
    #[{table: \"rows\"}]
    public string $id = \"\";
}
?{column: string} $absent = Core\\Attributes::get<{column: string}>(Row::constructor(...), \"id\");
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let folded =
        run.folded_at("Core\\Attributes::get<{column: string}>(Row::constructor(...), \"id\")");
    assert!(
        matches!(
            folded,
            Some(ExprInfo::CoreConst {
                value: ConstArg::Null
            })
        ),
        "§ 5 compiles a `null` in where nothing matches, and this recorded {folded:?}"
    );
}

// covers: Core\Attributes::get
#[test]
fn get_with_one_match_folds_to_that_constant_with_no_runtime_lookup() {
    let (diags, run) = check_src_declared(
        "<?nvs
class Row {
    #[{column: \"name\", unique: true}]
    public string $name = \"\";
}
?{column: string} $found = Core\\Attributes::get<{column: string}>(Row::constructor(...), \"name\");
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let folded =
        run.folded_at("Core\\Attributes::get<{column: string}>(Row::constructor(...), \"name\")");
    let Some(ExprInfo::CoreConst {
        value: ConstArg::Shape(fields),
    }) = folded
    else {
        panic!("§ 5 replaces the call with the payload itself, and this recorded {folded:?}");
    };
    // The payload *whole*, not the fields the shape asked for: § 5 compiles in
    // the attribute's payload object, and `rule:types/shape-type`'s width subtyping is what let it
    // match while carrying `unique` as well.
    let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["column", "unique"], "{fields:?}");
    assert!(
        matches!(&fields[0].1, ConstArg::Str(value) if value == "name"),
        "{fields:?}"
    );
    assert!(matches!(fields[1].1, ConstArg::Bool(true)), "{fields:?}");
}

// covers: Core\Attributes::all
#[test]
fn get_with_two_matches_is_a_diagnostic_naming_all() {
    let two_columns = "<?nvs
class Row {
    #[{column: \"name\"}]
    #[{column: \"title\"}]
    public string $name = \"\";
}
";
    let (diags, _) = check_src_declared(&format!(
        "{two_columns}?{{column: string}} $both = \
         Core\\Attributes::get<{{column: string}}>(Row::constructor(...), \"name\");\n"
    ));
    assert!(
        reported(&diags, code::E_ATTRIBUTE_RETRIEVAL_AMBIGUOUS),
        "{diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| d.notes.iter().any(|note| note.contains("::all<T>"))),
        "§ 5's refusal names the member that answers every match: {diags:?}"
    );

    // The other side of the same bound, over the identical declarations: what
    // `get` refuses is exactly what `all` is for, so a rule that refused both
    // — or neither — would still satisfy the half above.
    let (diags, run) = check_src_declared(&format!(
        "{two_columns}array<{{column: string}}> $both = \
         Core\\Attributes::all<{{column: string}}>(Row::constructor(...), \"name\");\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let folded =
        run.folded_at("Core\\Attributes::all<{column: string}>(Row::constructor(...), \"name\")");
    let Some(ExprInfo::CoreConst {
        value: ConstArg::Array(entries),
    }) = folded
    else {
        panic!("`all` answers every match as an array constant, and this recorded {folded:?}");
    };
    assert_eq!(entries.len(), 2, "{entries:?}");
}

#[test]
fn a_written_member_name_that_names_no_real_member_is_a_diagnostic() {
    let row = "<?nvs
class Row {
    #[{column: \"name\"}]
    public string $name = \"\";
}
";
    // A misspelling of a real property. Left unchecked this folds to the very
    // `null` a correct retrieval of an absent attribute folds to, which is why
    // § 4 checks the written name here rather than leaving it to a test run.
    let (diags, _) = check_src_declared(&format!(
        "{row}?{{column: string}} $typo = \
         Core\\Attributes::get<{{column: string}}>(Row::constructor(...), \"nmae\");\n"
    ));
    assert!(
        reported(&diags, code::E_ATTRIBUTE_MEMBER_NOT_DECLARED),
        "{diags:?}"
    );

    // § 4's *Consequences* is the other side of that bound: a member name that
    // is not a literal has nothing to check, so it is an empty result and not
    // a diagnostic — the same misspelling, one concatenation apart.
    let (diags, run) = check_src_declared(&format!(
        "{row}string $computed = \"nm\" . \"ae\";
?{{column: string}} $dynamic = \
         Core\\Attributes::get<{{column: string}}>(Row::constructor(...), $computed);\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let folded =
        run.folded_at("Core\\Attributes::get<{column: string}>(Row::constructor(...), $computed)");
    assert!(
        matches!(
            folded,
            Some(ExprInfo::CoreConst {
                value: ConstArg::Null
            })
        ),
        "a computed member name resolves nothing, and this recorded {folded:?}"
    );
}
