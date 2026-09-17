//! Every declared enum's **underlying integer type** and its cases' constant
//! values — the third thing this crate resolves once and publishes, alongside
//! [`crate::expr_table`] and [`crate::layout`].
//!
//! # Why this table exists at all
//!
//! `rule:enums/closed-integer-type` makes an enum a
//! closed, named **integer** type: a case is a compile-time constant, never a
//! singleton object, and "every enum has exactly one underlying integer type,
//! `int` unless `: uint` is written." Both halves of that sentence are facts
//! about the *declaration* that every later stage needs and none of them can
//! re-derive:
//!
//! * the backing type decides an enum-typed value's machine representation, so
//!   [`crate::ty::Ty::Enum`] carries it (see that variant's own doc comment for
//!   why it is part of the type's identity rather than a side lookup);
//! * a case's value is what `EnumName::CaseName` *is* — `rule:enums/no-class-machinery`'s "a case
//!   is an integer constant, inlined at every use site" — so `nvs-ir` reads it
//!   back through [`crate::expr_table::ExprInfo::EnumCase`] and emits a plain
//!   constant, with no storage, no descriptor and no allocation anywhere.
//!
//! # Auto-increment is C#'s rule, kept exactly
//!
//! A case with no explicit value takes the previous case's plus one, starting
//! at `0`; an explicit value resets the counter for whatever follows it
//! (`rule:enums/declaration`). Two cases may share a value — that ADR says so outright,
//! because equality is value equality and there is no identity to collide — so
//! this table deliberately reports nothing for a duplicate.
//!
//! # What is diagnosed here
//!
//! Only what needs the whole declaration in view: a backing type that is not
//! `int`/`uint`, a case value that is not an integer literal, and one that does
//! not fit the backing type (including an auto-increment that runs off the end
//! of the range). `enum Name: string`, `implements`, and a member other than a
//! case are all rejected by `nvs-syntax`'s parser instead — see
//! `nvs_syntax::parser::parse_enum_backing_type` and its neighbours.
//!
//! # What it costs
//!
//! One [`EnumInfo`] per declared enum, each holding one small map entry per
//! case — O(enums × cases in the compiled file), built once per compile and
//! dropped with the rest of the check run's tables. Nothing survives into the
//! compiled artifact but the constants themselves.

use nvs_diagnostics::{Diagnostic, SourceFile, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    EnumDecl, Expr, ExprKind, NamespaceDecl, Stmt, StmtKind, Type, TypeAtom, TypeKind, UnaryOp,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::span_text;

/// An enum's underlying integer type — `rule:enums/one-backing-type`: exactly one per enum,
/// `int` unless `: uint` is written. There is no third option and no unbacked
/// form.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum EnumBacking {
    /// `int` — the default when no `: Type` clause is written.
    #[default]
    Int,
    /// `uint`, written `enum Name: uint`.
    Uint,
}

/// One case's constant value, in its enum's own backing type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnumValue {
    /// A case of an `int`-backed enum.
    Int(i64),
    /// A case of a `uint`-backed enum.
    Uint(u64),
}

/// One declared enum: its backing type, and every case's value by name.
#[derive(Clone, Debug)]
pub struct EnumInfo {
    /// The underlying integer type every case of this enum is a constant of.
    pub backing: EnumBacking,
    /// Each case's constant value, keyed by the case's own name.
    pub cases: FxHashMap<String, EnumValue>,
    /// The cases whose value the declaration **wrote out**, rather than
    /// counting it on from the case before.
    ///
    /// Not a difference a value can carry: `Active = 0` and the first case of
    /// `{ Active, Banned }` are the same constant, and only one of them is a
    /// number its author stated and means to keep. `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`
    /// is the reader that needs the difference — it spells a route capture's
    /// subset by its values only where every admitted case wrote one — and the
    /// declaration is the only place it can be read, since a counted value is
    /// indistinguishable from a written one everywhere after this.
    pub written: FxHashSet<String>,
}

/// Every enum declared in the files walked, by resolved name.
#[derive(Debug, Default)]
pub struct EnumTable {
    by_name: FxHashMap<QName, EnumInfo>,
}

impl EnumTable {
    /// The entry for `qname`, or `None` if it names no declared enum.
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&EnumInfo> {
        self.by_name.get(qname)
    }

    /// `qname`'s backing type, defaulting to [`EnumBacking::Int`] for a name
    /// this table has no entry for.
    ///
    /// The default is not a guess: it is the same one `rule:enums/one-backing-type` gives a
    /// declaration that omits its `: Type` clause, and the only way to reach
    /// it here is a program that already failed to resolve the name (which is
    /// diagnosed where the reference is, not here).
    #[must_use]
    pub fn backing_of(&self, qname: &QName) -> EnumBacking {
        self.by_name
            .get(qname)
            .map_or(EnumBacking::Int, |info| info.backing)
    }

    /// The constant value of `qname::case`, or `None` if either names nothing.
    #[must_use]
    pub fn case(&self, qname: &QName, case: &str) -> Option<EnumValue> {
        self.by_name.get(qname)?.cases.get(case).copied()
    }

    /// Every entry, in no particular order — what `nvs_ir::lower` copies down
    /// so `rule:enums/reflection` has a case list to report at run time.
    ///
    /// Unordered because the map is: a declaration's own order survives nothing
    /// below the parser, so the order a reader sees is settled once, by
    /// `nvs_runtime::ClassTable::define_enum`, rather than half here.
    pub fn iter(&self) -> impl Iterator<Item = (&QName, &EnumInfo)> {
        self.by_name.iter()
    }
}

/// Resolves every `enum` declaration in every file of the program, reporting
/// `rule:enums/declaration`/§ 2's declaration-level errors.
///
/// Runs *before* [`crate::signatures::build_signatures`], because interning an
/// enum-typed annotation needs the backing type this produces — see
/// [`crate::ty::Ty::Enum`]. One table spans the whole [`crate::ProgramFile`]
/// slice: an enum declared in a `require`d file is named from the file that
/// required it, so a per-file table would answer `None` there.
pub(crate) fn build_enum_table(
    files: &[crate::ProgramFile<'_>],
    diags: &mut nvs_diagnostics::Diagnostics,
) -> EnumTable {
    let mut table = EnumTable::default();
    seed_core(&mut table);
    for file in files {
        collect(file.stmts, file.src, &[], &mut table, diags);
    }
    table
}

/// Adds every `nvs_stdlib::registry::ENUMS` entry, before any declaration is
/// walked.
///
/// The same "seed a table rather than special-case `Core`" arrangement
/// [`crate::core_lib`] uses for members, and it buys the same thing: nothing
/// downstream — not [`EnumTable::case`], not `crate::expr`'s
/// `ClassConstAccess` arm, not `nvs-ir` — learns that a `Core` enum is
/// different from a declared one.
///
/// Seeded first so a *declared* `Core\Order` would overwrite it rather than
/// the other way round. That cannot happen today: `Core` is the reserved
/// namespace (`rule:classes/no-free-functions-or-constants`),
/// and a program declaring into it is a question for `nvs-hir`'s resolver,
/// not something this table should answer by silently winning.
fn seed_core(table: &mut EnumTable) {
    for declared in nvs_stdlib::registry::ENUMS {
        let cases = declared
            .cases
            .iter()
            .map(|(name, value)| ((*name).to_owned(), EnumValue::Int(*value)))
            .collect();
        table.by_name.insert(
            QName::parse(declared.name),
            EnumInfo {
                backing: EnumBacking::Int,
                cases,
                written: core_written(declared.cases),
            },
        );
    }
}

/// Which of a `Core` enum's cases stated a value, read back out of the run of
/// values `nvs_stdlib::registry::CoreEnum::cases` states for every one of them.
///
/// That table writes each constant out because a table the compiler reads has
/// nothing to gain from re-deriving what it could state, so the source form it
/// stands for — which cases carried a written value and which counted on — is
/// recoverable and is recovered here: a case whose value is the one
/// auto-increment would have handed it counted, and any other wrote its own.
/// The distinction is [`EnumInfo::written`]'s, and a `Core` enum reaching a
/// route capture is answered by the same rule a declared one is.
fn core_written(cases: &[(&str, i64)]) -> FxHashSet<String> {
    let mut written = FxHashSet::default();
    let mut next = 0;
    for (name, value) in cases {
        if *value != next {
            written.insert((*name).to_owned());
        }
        next = value.saturating_add(1);
    }
    written
}

fn collect(
    stmts: &[Stmt],
    src: &SourceFile,
    namespace: &[String],
    table: &mut EnumTable,
    diags: &mut nvs_diagnostics::Diagnostics,
) {
    let mut current_ns = namespace.to_vec();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name.as_ref().map_or_else(Vec::new, |n| {
                    QName::parse(span_text(src, n.span)).segments().to_vec()
                });
                match body {
                    Some(block) => collect(&block.stmts, src, &new_ns, table, diags),
                    None => current_ns = new_ns,
                }
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(src, decl.name.span));
                let info = resolve_enum(decl, src, diags);
                table.by_name.insert(qname, info);
            }
            _ => {}
        }
    }
}

fn resolve_enum(
    decl: &EnumDecl,
    src: &SourceFile,
    diags: &mut nvs_diagnostics::Diagnostics,
) -> EnumInfo {
    let backing = backing_of(decl.backing.as_ref(), diags);
    let mut cases = FxHashMap::default();
    let mut written = FxHashSet::default();
    // `rule:enums/declaration`: "a case with no explicit literal takes the previous case's
    // value plus one, starting at `0` ... including that an explicit value
    // resets the counter for whatever follows it." Held as the *next* value to
    // hand out, so both halves are the same assignment, and as an `EnumValue`
    // rather than a raw `u64` so the increment runs in the backing type's own
    // arithmetic: `-2`'s successor is `-1`, not a `u64` bit pattern no `int`
    // can hold.
    let mut next = Some(zero_of(backing));
    for case in &decl.cases {
        let value = match &case.value {
            Some(expr) => literal_value(expr, backing, src, diags),
            None => next.or_else(|| {
                diags.report(
                    Diagnostic::error(
                        code::E_ENUM_CASE_VALUE_OUT_OF_RANGE,
                        "this case's auto-incremented value runs past its enum's backing type",
                    )
                    .with_primary(case.span, "no value left to auto-increment into")
                    .with_help("give this case an explicit value (`rule:enums/declaration`)"),
                );
                None
            }),
        };
        let Some(value) = value else { continue };
        next = successor_of(value);
        let name = span_text(src, case.name.span).to_owned();
        if case.value.is_some() {
            written.insert(name.clone());
        }
        cases.insert(name, value);
    }
    EnumInfo {
        backing,
        cases,
        written,
    }
}

/// `rule:enums/one-backing-type`: `int` unless `: uint` is written.
///
/// `enum Name: string` is already reported by `nvs-syntax`'s parser, so this
/// only has to catch every *other* non-integer spelling — which the parser
/// deliberately leaves to "a later check" (see
/// `nvs_syntax::parser::parse_enum_backing_type`). A `string` backing falls
/// through to `int` here with no second diagnostic.
fn backing_of(ty: Option<&Type>, diags: &mut nvs_diagnostics::Diagnostics) -> EnumBacking {
    let Some(ty) = ty else {
        return EnumBacking::Int;
    };
    match &ty.kind {
        TypeKind::Atom(TypeAtom::Int) => EnumBacking::Int,
        TypeKind::Atom(TypeAtom::Uint) => EnumBacking::Uint,
        // Already diagnosed by the parser — see this function's own doc.
        TypeKind::Atom(TypeAtom::String) => EnumBacking::Int,
        _ => {
            diags.report(
                Diagnostic::error(
                    code::E_ENUM_BACKING_NOT_INTEGER,
                    "an enum's backing type must be `int` or `uint`",
                )
                .with_primary(ty.span, "not an integer type")
                .with_help(
                    "use `: int` or `: uint`, or omit the clause (`rule:enums/one-backing-type`)",
                ),
            );
            EnumBacking::Int
        }
    }
}

/// One explicit `= expr` case value.
///
/// `rule:enums/declaration` gives a case an integer *literal*, so exactly two expression
/// shapes are accepted: a bare integer literal, and a negated one (which the
/// parser produces as a `-` unary over the literal, never as part of its
/// digits). Anything else is refused rather than const-evaluated: an enum case
/// is not a general constant-expression position, and admitting one would put
/// a second evaluator in the language for no requirement.
fn literal_value(
    expr: &Expr,
    backing: EnumBacking,
    src: &SourceFile,
    diags: &mut nvs_diagnostics::Diagnostics,
) -> Option<EnumValue> {
    let (negated, span) = match &expr.kind {
        ExprKind::Int(span) => (false, *span),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            expr: inner,
        } => match &inner.kind {
            ExprKind::Int(span) => (true, *span),
            _ => return not_a_literal(expr.span, diags),
        },
        _ => return not_a_literal(expr.span, diags),
    };
    let (radix, digits) = int_literal_digits(src, span);
    let value = u64::from_str_radix(&digits, radix)
        .ok()
        .and_then(|magnitude| match (negated, backing) {
            (false, _) => in_range(magnitude, backing),
            (true, EnumBacking::Int) => i64::try_from(magnitude)
                .ok()
                .and_then(i64::checked_neg)
                .map(EnumValue::Int),
            (true, EnumBacking::Uint) => None,
        });
    if value.is_none() {
        out_of_range(expr.span, backing, diags);
    }
    value
}

fn not_a_literal(span: Span, diags: &mut nvs_diagnostics::Diagnostics) -> Option<EnumValue> {
    diags.report(
        Diagnostic::error(
            code::E_ENUM_CASE_VALUE_NOT_LITERAL,
            "an enum case's value must be an integer literal",
        )
        .with_primary(span, "not an integer literal")
        .with_help("`rule:enums/declaration`: a case is a compile-time integer constant"),
    );
    None
}

fn out_of_range(span: Span, backing: EnumBacking, diags: &mut nvs_diagnostics::Diagnostics) {
    let name = match backing {
        EnumBacking::Int => "int",
        EnumBacking::Uint => "uint",
    };
    diags.report(
        Diagnostic::error(
            code::E_ENUM_CASE_VALUE_OUT_OF_RANGE,
            format!("this case's value does not fit its enum's backing type `{name}`"),
        )
        .with_primary(span, format!("outside `{name}`")),
    );
}

/// `raw` reinterpreted in `backing`, or `None` if it does not fit.
fn in_range(raw: u64, backing: EnumBacking) -> Option<EnumValue> {
    match backing {
        EnumBacking::Int => i64::try_from(raw).ok().map(EnumValue::Int),
        EnumBacking::Uint => Some(EnumValue::Uint(raw)),
    }
}

/// Where an enum with no explicit first value starts counting.
fn zero_of(backing: EnumBacking) -> EnumValue {
    match backing {
        EnumBacking::Int => EnumValue::Int(0),
        EnumBacking::Uint => EnumValue::Uint(0),
    }
}

/// `value + 1` in the backing type's own arithmetic, or `None` at the top of
/// its range — which is the one place the auto-increment counter can run out,
/// and is reported against the case that would have received it.
///
/// Deliberately not a `u64` step shared by both backings: `i64`'s successor
/// has to run over `i64`, or a negative explicit value's successor comes back
/// as a bit pattern no `int` can hold.
fn successor_of(value: EnumValue) -> Option<EnumValue> {
    match value {
        EnumValue::Int(v) => v.checked_add(1).map(EnumValue::Int),
        EnumValue::Uint(v) => v.checked_add(1).map(EnumValue::Uint),
    }
}

/// Splits an integer-literal span into its radix and digit run — the third
/// copy of `nvs_ir::lower::int_literal_digits`, for the reason
/// `crate::expr`'s own copy already states: the magnitude has to be known
/// here, at check time, and sharing across the crate boundary would invert
/// this crate's dependency direction.
fn int_literal_digits(src: &SourceFile, span: Span) -> (u32, String) {
    let cleaned: String = span_text(src, span).chars().filter(|&c| c != '_').collect();
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(rest) = cleaned.strip_prefix(prefix) {
            return (radix, rest.to_owned());
        }
    }
    (10, cleaned)
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};

    use super::{EnumBacking, EnumTable, EnumValue, build_enum_table};

    fn table(source: &str) -> (EnumTable, Diagnostics) {
        let mut map = SourceMap::new();
        let id = map.add("test.nvs", source);
        let mut diags = Diagnostics::new();
        let stmts = nvs_syntax::parse_file(map.file(id), &mut diags);
        let table = build_enum_table(
            &[crate::ProgramFile {
                src: map.file(id),
                stmts: &stmts,
            }],
            &mut diags,
        );
        (table, diags)
    }

    fn case(table: &EnumTable, name: &str, case: &str) -> Option<EnumValue> {
        table.case(&nvs_hir::QName::parse(name), case)
    }

    #[test]
    fn cases_auto_increment_from_zero() {
        let (table, diags) = table("<?nvs\nenum Rank { Bronze, Silver, Gold }\n");
        assert!(!diags.has_errors());
        assert_eq!(case(&table, "Rank", "Bronze"), Some(EnumValue::Int(0)));
        assert_eq!(case(&table, "Rank", "Silver"), Some(EnumValue::Int(1)));
        assert_eq!(case(&table, "Rank", "Gold"), Some(EnumValue::Int(2)));
    }

    #[test]
    fn an_explicit_value_resets_the_counter() {
        let (table, diags) = table("<?nvs\nenum E { A, B = 10, C }\n");
        assert!(!diags.has_errors());
        assert_eq!(case(&table, "E", "A"), Some(EnumValue::Int(0)));
        assert_eq!(case(&table, "E", "B"), Some(EnumValue::Int(10)));
        assert_eq!(case(&table, "E", "C"), Some(EnumValue::Int(11)));
    }

    #[test]
    fn a_uint_backing_is_recorded_and_its_cases_are_uint() {
        let (table, diags) = table("<?nvs\nenum P: uint { Read = 0b001, Write = 0b010 }\n");
        assert!(!diags.has_errors());
        assert_eq!(
            table.backing_of(&nvs_hir::QName::parse("P")),
            EnumBacking::Uint
        );
        assert_eq!(case(&table, "P", "Read"), Some(EnumValue::Uint(1)));
        assert_eq!(case(&table, "P", "Write"), Some(EnumValue::Uint(2)));
    }

    #[test]
    fn a_negative_case_value_is_accepted_for_an_int_backing() {
        let (table, diags) = table("<?nvs\nenum E { A = -1, B }\n");
        assert!(!diags.has_errors());
        assert_eq!(case(&table, "E", "A"), Some(EnumValue::Int(-1)));
        assert_eq!(case(&table, "E", "B"), Some(EnumValue::Int(0)));
    }

    #[test]
    fn a_negative_case_value_is_refused_for_a_uint_backing() {
        let (_, diags) = table("<?nvs\nenum E: uint { A = -1 }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(nvs_diagnostics::code::E_ENUM_CASE_VALUE_OUT_OF_RANGE))
        );
    }

    #[test]
    fn a_non_literal_case_value_is_refused() {
        let (_, diags) = table("<?nvs\nenum E { A = 1 + 1 }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(nvs_diagnostics::code::E_ENUM_CASE_VALUE_NOT_LITERAL))
        );
    }

    #[test]
    fn a_non_integer_backing_type_is_refused() {
        let (_, diags) = table("<?nvs\nenum E: float { A }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(nvs_diagnostics::code::E_ENUM_BACKING_NOT_INTEGER))
        );
    }

    #[test]
    fn an_enum_inside_a_namespace_is_keyed_by_its_resolved_name() {
        // The statement form, which is the only one: the braced
        // `namespace App { … }` this fixture used to write is refused by the
        // parser (`E0243`) and never reaches this table.
        let (table, diags) = table("<?nvs\nnamespace App;\nenum Rank { Bronze }\n");
        assert!(!diags.has_errors());
        assert_eq!(case(&table, "App\\Rank", "Bronze"), Some(EnumValue::Int(0)));
    }
}
