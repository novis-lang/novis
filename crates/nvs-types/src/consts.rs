//! Every declared class's **class constants**, folded to the compile-time
//! values [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md)
//! § 2 needs — the fourth thing this crate resolves once and publishes,
//! alongside [`crate::enums`], `crate::expr_table` and `crate::layout`.
//!
//! § 2 makes `Foo::TYPE_A` in *type* position sugar for the constant's own
//! literal type, "exactly as long as that value is a `string` or `int`
//! compile-time constant". That is a question about the *value*, so it has to
//! be answered before the first annotation is interned — which is why this is
//! a pass of its own, running where [`crate::enums::build_enum_table`] runs
//! and for the same reason: [`crate::signatures::build_signatures`] interns
//! every declared annotation in the file, and one of them may be a folded
//! constant.
//!
//! Only ADR 0047's fold is served here. A class constant's *declared type* at
//! an expression site is still unmodeled — see the crate docs' known gaps —
//! and this table deliberately does not close that: it holds values, and a
//! value is all § 2 asks for.
//!
//! The one exception is a single **bit**, and that gap is why it is here.
//! [ADR 0033](../../../docs/adr/0033-secret-qualifier-for-confidential-values.md)
//! § 4's attribute-payload sink has to know whether a constant's declared type
//! carries `secret`, and because the declared type is unmodeled the qualifier
//! is invisible at every expression site — `Class::TOKEN` infers `mixed`. So
//! [`ConstEntry::secret`] is read off the annotation *here*, in the walk that
//! already has every `ConstMember` in hand, and nowhere else. It is a bit
//! rather than the type, because a bit is all the sink asks for and modelling
//! the type is the gap above, not this one.
//!
//! **Ineligible is recorded, not dropped.** A `float`, `array` or object
//! constant is a real declaration that simply has no literal type to fold to,
//! and telling that apart from a name nothing declares is what lets
//! [`crate::lower`] report the right one of ADR 0047 § 2's two mistakes.

use nvs_diagnostics::SourceFile;
use nvs_hir::QName;
use nvs_syntax::ast::{
    ClassDecl, ClassMemberKind, ConstMember, ExprKind, NamespaceDecl, Stmt, StmtKind, Type,
    TypeAtom, TypeKind, UnaryOp,
};
use rustc_hash::FxHashMap;

use crate::span_text;

/// One class constant's folded value, as far as ADR 0047 § 2 cares.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ConstValue {
    /// A `string` compile-time constant, already cooked by
    /// [`crate::string_lit::cook_string_literal`] — the value
    /// [`crate::ty::Ty::StringLiteral`] holds.
    Str(String),
    /// An `int` compile-time constant, in `int`'s own range.
    Int(i64),
    /// Declared, but not one of the two above: `Foo::RATE = 1.5`,
    /// `Foo::ROWS = [1, 2]`, `Foo::WHEN = Core\Time\Instant::now()`, or an
    /// integer whose magnitude no `int` holds.
    ///
    /// Kept rather than dropped so a use in type position can say *which*
    /// mistake was made — see this module's own docs.
    Ineligible,
}

/// One class constant, as much of it as this table holds: ADR 0047 § 2's
/// folded value, and beside it the one bit ADR 0033 § 4's attribute-payload
/// sink needs.
///
/// The bit rides here rather than being asked of the constant's type at the
/// sink, because at the sink there is no such type to ask: `signatures.rs`'s
/// own known gap leaves a class constant's declared type unmodeled, so
/// `Class::TOKEN` infers `mixed` at every expression site and the qualifier —
/// which is a property of a *declared* type and of nothing else — is gone.
/// This walk is the last place it exists, so it is the place that reads it.
#[derive(Clone, Debug)]
struct ConstEntry {
    /// What ADR 0047 § 2 folds the declaration's right-hand side to.
    value: ConstValue,
    /// Whether the declaration's own annotation carries ADR 0033 § 1's
    /// `secret`.
    secret: bool,
}

/// Every class constant declared in the files walked, by its class's resolved
/// name.
#[derive(Debug, Default)]
pub struct ConstTable {
    by_class: FxHashMap<QName, FxHashMap<String, ConstEntry>>,
}

impl ConstTable {
    /// The folded value of `qname::name`, looked up on `qname` itself and then
    /// on its ancestors, or `None` if no declaration in that chain declares it.
    ///
    /// The ancestor walk is `graph`'s `extends` **and** `implements` links,
    /// because a constant may be declared on an interface and a class reaches
    /// it by either edge. The class's own declaration wins outright — an
    /// ineligible constant that shadows an eligible parent's is still the one
    /// the name means.
    #[must_use]
    pub fn get(
        &self,
        qname: &QName,
        name: &str,
        graph: &nvs_hir::ClassGraph,
    ) -> Option<&ConstValue> {
        self.lookup(qname, name, graph).map(|entry| &entry.value)
    }

    /// Whether `qname::name`'s own declaration annotates it with ADR 0033
    /// § 1's `secret` — the question ADR 0033 § 4's attribute-payload sink
    /// asks, and the one [`crate::expr::quals`] cannot answer from an
    /// inferred type. `false` for a name no declaration in the chain has, the
    /// undeclared name being someone else's diagnostic.
    #[must_use]
    pub fn is_secret(&self, qname: &QName, name: &str, graph: &nvs_hir::ClassGraph) -> bool {
        self.lookup(qname, name, graph)
            .is_some_and(|entry| entry.secret)
    }

    /// The one ancestor walk both public questions are asked through, so they
    /// cannot disagree about *which* declaration a name means.
    fn lookup(
        &self,
        qname: &QName,
        name: &str,
        graph: &nvs_hir::ClassGraph,
    ) -> Option<&ConstEntry> {
        // Bounded by the same depth `crate::lower` bounds an `array<...>` at,
        // for the same reason: a cyclic `extends` is `nvs_hir::hierarchy`'s
        // diagnostic, and this walk must terminate whether or not it fired.
        let mut frontier = vec![qname.clone()];
        for _ in 0..MAX_ANCESTOR_DEPTH {
            let mut next = Vec::new();
            for class in &frontier {
                if let Some(entry) = self.by_class.get(class).and_then(|m| m.get(name)) {
                    return Some(entry);
                }
                if let Some(links) = graph.get(class) {
                    next.extend(links.extends.iter().cloned());
                    next.extend(links.implements.iter().cloned());
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        None
    }
}

/// How many `extends`/`implements` hops [`ConstTable::get`] walks before it
/// gives up — the same bound and the same reasoning as `crate::lower`'s
/// `MAX_ARRAY_DEPTH`.
const MAX_ANCESTOR_DEPTH: u32 = 32;

/// Folds every `class`/`interface` constant in every file of the program.
///
/// Reports nothing: an ineligible value is a legal declaration, and only a
/// *use* of it in type position is a mistake — which is where ADR 0047 § 2's
/// diagnostic belongs, since that is the span the author can act on.
///
/// One table spans the whole [`crate::ProgramFile`] slice, for the reason
/// [`crate::enums::build_enum_table`]'s docs give: a constant declared in a
/// `require`d file is named from the file that required it.
pub(crate) fn build_const_table(files: &[crate::ProgramFile<'_>]) -> ConstTable {
    let mut table = ConstTable::default();
    for file in files {
        collect(file.stmts, file.src, &[], &mut table);
    }
    table
}

fn collect(stmts: &[Stmt], src: &SourceFile, namespace: &[String], table: &mut ConstTable) {
    let mut current_ns = namespace.to_vec();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name.as_ref().map_or_else(Vec::new, |n| {
                    QName::parse(span_text(src, n.span)).segments().to_vec()
                });
                match body {
                    Some(block) => collect(&block.stmts, src, &new_ns, table),
                    None => current_ns = new_ns,
                }
            }
            StmtKind::ClassDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(src, decl.name.span));
                let consts = fold_class(decl, src);
                if !consts.is_empty() {
                    table.by_class.insert(qname, consts);
                }
            }
            _ => {}
        }
    }
}

fn fold_class(decl: &ClassDecl, src: &SourceFile) -> FxHashMap<String, ConstEntry> {
    let mut out = FxHashMap::default();
    for member in &decl.members {
        if let ClassMemberKind::Const(c) = &member.kind {
            out.insert(
                span_text(src, c.name).to_owned(),
                ConstEntry {
                    value: fold_const(c, src),
                    secret: c.ty.as_ref().is_some_and(type_carries_secret),
                },
            );
        }
    }
    out
}

/// Whether a written annotation carries ADR 0033 § 1's `secret` anywhere in
/// it.
///
/// Read off the **syntax** rather than off an interned type, because this pass
/// runs before the first annotation is interned — that is the whole reason it
/// is a pass of its own, and giving it an interner would put it after
/// [`crate::signatures::build_signatures`], which needs it. The qualifier is a
/// property of a type *atom* ([`nvs_syntax::ast::TypeAtom`]'s four secret
/// rows), so a composite carries it exactly when one of its members does —
/// `crate::expr::quals::is_secret` reaches the same answer one representation
/// down, and a union answering `secret` for one member is the safe direction
/// for a sink either way.
fn type_carries_secret(ty: &Type) -> bool {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => type_carries_secret(inner),
        TypeKind::Union(members) | TypeKind::Intersection(members) => {
            members.iter().any(type_carries_secret)
        }
        TypeKind::Atom(atom) => matches!(
            atom,
            TypeAtom::SecretString
                | TypeAtom::SecretBytes
                | TypeAtom::SecretTaintedString
                | TypeAtom::SecretTaintedBytes
        ),
        // [`TypeKind`] is `#[non_exhaustive]` in a crate below this one, so
        // the arm is required rather than chosen. A kind this does not name
        // carries no atom of its own — the four rows above are the only place
        // the qualifier is spellable — so a new composite reaches this walk
        // through its members or not at all.
        _ => false,
    }
}

/// One `const NAME = expr;`, folded.
///
/// The accepted shapes are exactly [`crate::enums::literal_value`]'s, plus a
/// string literal: a bare `int` literal, a negated one (the parser produces
/// `-1` as a unary over the literal, never as part of its digits), and a
/// `string` literal. Anything else is [`ConstValue::Ineligible`] rather than
/// const-evaluated — ADR 0047 § 2 folds a constant that *is* a literal, and a
/// general constant-expression evaluator is a second evaluator in the language
/// for no requirement.
fn fold_const(c: &ConstMember, src: &SourceFile) -> ConstValue {
    let (negated, inner) = match &c.value.kind {
        ExprKind::Unary {
            op: UnaryOp::Neg,
            expr: inner,
        } => (true, &**inner),
        _ => (false, &c.value),
    };
    match &inner.kind {
        ExprKind::Str(span) if !negated => {
            ConstValue::Str(crate::string_lit::cook_string_literal(src, *span))
        }
        ExprKind::Int(span) => {
            int_value(*span, negated, src).map_or(ConstValue::Ineligible, ConstValue::Int)
        }
        _ => ConstValue::Ineligible,
    }
}

/// An integer literal's value as an `int`, in whatever radix it was written —
/// [`crate::expr::int_literal_digits`]'s job, reused so this never grows a
/// second integer grammar. `None` for a magnitude no `int` holds, which the
/// caller records as ineligible: ADR 0047 § 1's atom is an `int` literal, so a
/// value outside `int` has no literal type to be.
fn int_value(span: nvs_diagnostics::Span, negated: bool, src: &SourceFile) -> Option<i64> {
    let (radix, digits) = crate::expr::int_literal_digits(src, span);
    let magnitude = u64::from_str_radix(&digits, radix).ok()?;
    if negated {
        negate(magnitude)
    } else {
        i64::try_from(magnitude).ok()
    }
}

/// `-magnitude` as an `i64`, `i64::MIN`'s own magnitude included — which
/// `i64::try_from` alone would refuse. The same edge
/// [`crate::defaults::negate_int`] answers, at the one other place a written
/// magnitude is negated.
fn negate(magnitude: u64) -> Option<i64> {
    if magnitude == (i64::MAX as u64) + 1 {
        return Some(i64::MIN);
    }
    i64::try_from(magnitude).ok().and_then(i64::checked_neg)
}
