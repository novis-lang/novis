//! The declaration checks that need only the AST: identifier casing
//! (`rule:core-api/identifier-casing`,
//! tightened by `rule:classes/no-leading-underscore-identifiers`)
//! and the visibility
//! `rule:core-api/written-visibility`
//! requires at every member declaration. Each is checked directly off the
//! AST a declaration already produces — no name resolution needed, so this
//! lives in `nvs-syntax` rather than waiting on `nvs-hir`/`nvs-types`.
//!
//! The two rules share one walk because they ask the same question of the
//! same node: a declaration site, off the parse tree, with nothing resolved.
//! Splitting them into two passes would walk every file twice to learn what
//! one visit already knows.
//!
//! Every category uses exactly the pattern in `rule:core-api/identifier-casing`'s table — the leading
//! character's case, an alphanumeric rest — with no leading-underscore
//! carve-out and no acronym restriction of any kind. A method literally named
//! `__construct` gets [`nvs_diagnostics::code::E_LEGACY_CONSTRUCTOR_SPELLING`]
//! (naming `constructor` as the fix) instead of the generic camelCase
//! diagnostic.
//!
//! # What is, and isn't, walked
//!
//! Every category `rule:core-api/identifier-casing`'s own scope names gets checked: class/interface/
//! enum/enum-case/namespace-segment names (`PascalCase`), method names
//! (`camelCase`), property/parameter/local-variable names (`camelCase`,
//! `$`-sigil stripped before the pattern check), and class constant names
//! (`SCREAMING_SNAKE_CASE`). An anonymous function's optional self-name
//! ([`crate::ast::FnExpr::name`]) is checked the same way as a local
//! variable — it is exactly that shape of identifier, just spelled without a
//! `$` sigil.
//!
//! `rule:core-api/written-visibility`'s visibility rule reaches the member slots of a `class`, an
//! `interface` and an anonymous class — the three bodies [`check_members`]
//! walks. It deliberately does not reach a parameter (§ 2: visibility is
//! what promotes one to a property, so requiring it everywhere would delete
//! the distinction) or an `enum` body (§ 3 of
//! `rule:enums/closed-integer-type` already
//! rejects every non-`case` member there, and two diagnostics for one
//! mistake is worse than one).
//!
//! # Who calls it
//!
//! **Whoever parses a file checks that file's casing**, which is what makes
//! the check fire exactly once per file no matter which entry point compiled
//! it: `nvs-cli` calls this straight after [`crate::parse_file`] on the file
//! it was pointed at, and `nvs_hir::resolve_program` calls it on each file
//! it parses for a `require`. Nothing else parses a file, so there is no
//! third call site and no path that skips the rule `rule:core-api/identifier-casing` says has no
//! suppression.
//!
//! Only a *declaration* site is checked, never a reference: `Class::method`,
//! `$obj->prop`, and an `extends`/`implements` target all name something
//! declared elsewhere, which was (or will be) checked once, at that other
//! site.
//!
//! **A construct that already has its own diagnostic is left unchecked here.**
//! A top-level `function`/`const`
//! ([`crate::ast::StmtKind::TopLevelFunction`]/[`crate::ast::StmtKind::TopLevelConst`]),
//! a function-scope `static` local, and a non-`case` member inside an `enum`
//! body are always-rejected constructs, and the same "nothing downstream ever
//! acts on it" reasoning their own AST doc comments give for not inspecting
//! them further covers their casing too.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};

use crate::ast::{
    AnonClassDecl, Arg, ArrayItem, AttributeGroup, CallArgs, ClassMember, ClassMemberKind,
    DestructureElement, DestructureTarget, EnumDecl, Expr, ExprKind, FnBody, FnExpr, MemberName,
    MethodMember, Modifier, NamespaceDecl, NewTarget, Param, PropertyHook, PropertyHookBody, Stmt,
    StmtKind, StringPart, TestOperand, Visibility,
};

/// Checks every declaration in `stmts` against `rule:core-api/identifier-casing`/0030's casing rules
/// and `rule:core-api/written-visibility`'s required member visibility, reporting one diagnostic per
/// violation into `diags`.
///
/// Needs nothing besides the parsed AST and the source it came from — no
/// symbol table, no class graph — so it can run directly on the output of
/// [`crate::parse_file`].
pub fn check_declarations(stmts: &[Stmt], src: &SourceFile, diags: &mut Diagnostics) {
    check_stmts(stmts, src, diags);
}

fn span_text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

// ============================================================================
// Pattern checks — `rule:core-api/identifier-casing`'s table, `rule:classes/no-leading-underscore-identifiers`'s zero-exception tightening
// ============================================================================

/// Exactly `rule:core-api/casing-checks-the-leading-character`'s rule: only the first character's case is checked, and the rest need
/// only be alphanumeric — no run-length or acronym check of any kind, so
/// `HTTPClient` is accepted on equal footing with `HttpClient`.
fn is_pascal_case(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_alphanumeric())
}

fn is_camel_case(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_alphanumeric())
}

fn is_screaming_snake_case(s: &str) -> bool {
    let mut groups = s.split('_');
    let Some(first) = groups.next() else {
        return false;
    };
    let mut first_chars = first.chars();
    if !matches!(first_chars.next(), Some(c) if c.is_ascii_uppercase()) {
        return false;
    }
    if !first_chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return false;
    }
    groups.all(|g| {
        !g.is_empty()
            && g.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    })
}

// ============================================================================
// Mechanical rename suggestions — split on existing case/underscore
// boundaries, re-join in the target convention (`rule:core-api/identifier-casing`'s *Diagnostics*
// section promises every message names one of these).
// ============================================================================

fn split_words(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        if c.is_ascii_uppercase() && !current.is_empty() {
            let prev_lower_or_digit =
                chars[i - 1].is_ascii_lowercase() || chars[i - 1].is_ascii_digit();
            let acronym_then_word = chars[i - 1].is_ascii_uppercase()
                && chars.get(i + 1).is_some_and(char::is_ascii_lowercase);
            if prev_lower_or_digit || acronym_then_word {
                words.push(std::mem::take(&mut current));
            }
        }
        current.push(c);
    }
    if !current.is_empty() {
        words.push(current);
    }
    if words.is_empty() {
        // Only reachable for an identifier made up entirely of underscores
        // (e.g. PHP's conventional `$_`) — there is no case/underscore
        // boundary to recover a word from, so fall back to a placeholder
        // rather than suggesting an empty name.
        words.push("value".to_owned());
    }
    words
}

fn cap_first_lower_rest(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_ascii_uppercase().to_string();
            out.extend(chars.map(|c| c.to_ascii_lowercase()));
            out
        }
        None => String::new(),
    }
}

fn suggest_pascal(name: &str) -> String {
    split_words(name)
        .iter()
        .map(|w| cap_first_lower_rest(w))
        .collect()
}

fn suggest_camel(name: &str) -> String {
    let words = split_words(name);
    let mut out = String::new();
    for (i, word) in words.iter().enumerate() {
        if i == 0 {
            out.extend(word.chars().map(|c| c.to_ascii_lowercase()));
        } else {
            out.push_str(&cap_first_lower_rest(word));
        }
    }
    out
}

fn suggest_screaming_snake(name: &str) -> String {
    split_words(name)
        .iter()
        .map(|w| w.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

fn strip_sigil(text: &str) -> &str {
    text.strip_prefix('$').unwrap_or(text)
}

// ============================================================================
// Per-category diagnostics
// ============================================================================

fn check_type_name(span: Span, src: &SourceFile, category: &str, diags: &mut Diagnostics) {
    let text = span_text(src, span);
    if text.is_empty() || is_pascal_case(text) {
        return;
    }
    let suggested = suggest_pascal(text);
    diags.report(
        Diagnostic::error(
            code::E_BAD_TYPE_CASING,
            format!("`{text}` must be PascalCase, e.g. `{suggested}`"),
        )
        .with_primary(span, format!("{category} name must be PascalCase"))
        .with_fix(span, suggested.clone(), format!("rename to `{suggested}`")),
    );
}

fn check_namespace_segments(span: Span, src: &SourceFile, diags: &mut Diagnostics) {
    let text = span_text(src, span);
    let mut offset: u32 = 0;
    for segment in text.split('\\') {
        if !segment.is_empty() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a segment's length is bounded by the enclosing span's own u32 length"
            )]
            let seg_span = Span::new(
                span.file,
                span.start + offset,
                span.start + offset + segment.len() as u32,
            );
            check_type_name(seg_span, src, "namespace segment", diags);
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a segment's length is bounded by the enclosing span's own u32 length"
        )]
        {
            offset += segment.len() as u32 + 1;
        }
    }
}

fn check_method_name(span: Span, src: &SourceFile, diags: &mut Diagnostics) {
    let text = span_text(src, span);
    if text.is_empty() {
        return;
    }
    if text == "__construct" {
        diags.report(
            Diagnostic::error(
                code::E_LEGACY_CONSTRUCTOR_SPELLING,
                "Novis's constructor is spelled `constructor`, not `__construct`",
            )
            .with_primary(span, "rename to `constructor`")
            .with_fix(span, "constructor", "rename to `constructor`"),
        );
        return;
    }
    if is_camel_case(text) {
        return;
    }
    let suggested = suggest_camel(text);
    diags.report(
        Diagnostic::error(
            code::E_BAD_METHOD_CASING,
            format!("method names must be camelCase, e.g. `{suggested}`"),
        )
        .with_primary(span, "method name must be camelCase")
        .with_fix(span, suggested.clone(), format!("rename to `{suggested}`")),
    );
}

fn check_const_name(span: Span, src: &SourceFile, diags: &mut Diagnostics) {
    let text = span_text(src, span);
    if text.is_empty() || is_screaming_snake_case(text) {
        return;
    }
    let suggested = suggest_screaming_snake(text);
    diags.report(
        Diagnostic::error(
            code::E_BAD_CONST_CASING,
            format!("class constants must be SCREAMING_SNAKE_CASE, e.g. `{suggested}`"),
        )
        .with_primary(span, "class constant must be SCREAMING_SNAKE_CASE")
        .with_fix(span, suggested.clone(), format!("rename to `{suggested}`")),
    );
}

/// Property/parameter/local-variable/anonymous-function-name check. `strip_dollar`
/// distinguishes the three sigil-carrying categories from an anonymous function's
/// self-name, which is a plain identifier — see [`crate::ast::FnExpr::name`].
fn check_member_casing(
    span: Span,
    src: &SourceFile,
    category: &str,
    strip_dollar: bool,
    diags: &mut Diagnostics,
) {
    let raw = span_text(src, span);
    let text = if strip_dollar { strip_sigil(raw) } else { raw };
    if text.is_empty() || is_camel_case(text) {
        return;
    }
    let suggested = suggest_camel(text);
    let replacement = if strip_dollar {
        format!("${suggested}")
    } else {
        suggested.clone()
    };
    diags.report(
        Diagnostic::error(
            code::E_BAD_MEMBER_CASING,
            format!("{category} names must be camelCase, e.g. `{suggested}`"),
        )
        .with_primary(span, format!("{category} name must be camelCase"))
        .with_fix(span, replacement, format!("rename to `{suggested}`")),
    );
}

fn check_property_name(span: Span, src: &SourceFile, diags: &mut Diagnostics) {
    check_member_casing(span, src, "property", true, diags);
}

fn check_param_name(span: Span, src: &SourceFile, diags: &mut Diagnostics) {
    check_member_casing(span, src, "parameter", true, diags);
}

fn check_local_name(span: Span, src: &SourceFile, diags: &mut Diagnostics) {
    check_member_casing(span, src, "local variable", true, diags);
}

fn check_anon_fn_name(span: Span, src: &SourceFile, diags: &mut Diagnostics) {
    check_member_casing(span, src, "anonymous function", false, diags);
}

// ============================================================================
// The walk
// ============================================================================

fn check_stmts(stmts: &[Stmt], src: &SourceFile, diags: &mut Diagnostics) {
    for stmt in stmts {
        check_stmt(stmt, src, diags);
    }
}

fn check_stmt(stmt: &Stmt, src: &SourceFile, diags: &mut Diagnostics) {
    match &stmt.kind {
        StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
            if let Some(name) = name {
                check_namespace_segments(name.span, src, diags);
            }
            if let Some(block) = body {
                check_stmts(&block.stmts, src, diags);
            }
        }
        StmtKind::ClassDecl(decl) => {
            check_type_name(decl.name.span, src, "class", diags);
            check_members(&decl.members, src, diags);
        }
        StmtKind::TypeAliasDecl(decl) => {
            check_type_name(decl.name.span, src, "type alias", diags);
        }
        StmtKind::InterfaceDecl(decl) => {
            check_type_name(decl.name.span, src, "interface", diags);
            check_members(&decl.members, src, diags);
        }
        StmtKind::EnumDecl(decl) => check_enum_decl(decl, src, diags),
        StmtKind::Block(b) => check_stmts(&b.stmts, src, diags),
        StmtKind::If { arms, else_ } => {
            for arm in arms {
                check_expr(&arm.cond, src, diags);
                check_stmt(&arm.then, src, diags);
            }
            if let Some(else_) = else_ {
                check_stmt(else_, src, diags);
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            check_expr(cond, src, diags);
            check_stmt(body, src, diags);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            // `rule:iteration/for-init-clause`: the counter a declaration form binds is an
            // ordinary local, so `rule:core-api/identifier-casing`'s `camelCase` rule reaches it
            // through the same arm a declaration above the loop takes.
            if let Some(decl) = init.decl() {
                check_stmt(decl, src, diags);
            }
            for e in init.exprs().iter().chain(cond).chain(step) {
                check_expr(e, src, diags);
            }
            check_stmt(body, src, diags);
        }
        StmtKind::Foreach {
            subject,
            key,
            value,
            body,
            ..
        } => {
            check_expr(subject, src, diags);
            if let Some(key) = key {
                check_local_name(key.name, src, diags);
            }
            check_local_name(value.name, src, diags);
            check_stmt(body, src, diags);
        }
        StmtKind::Switch { subject, cases } => {
            check_expr(subject, src, diags);
            for case in cases {
                if let Some(cond) = &case.cond {
                    check_expr(cond, src, diags);
                }
                check_stmts(&case.body, src, diags);
            }
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            check_stmts(&body.stmts, src, diags);
            for catch in catches {
                if let Some(var) = catch.var {
                    check_local_name(var, src, diags);
                }
                check_stmts(&catch.body.stmts, src, diags);
            }
            if let Some(finally) = finally {
                check_stmts(&finally.stmts, src, diags);
            }
        }
        StmtKind::Echo(xs) | StmtKind::Unset(xs) => {
            for x in xs {
                check_expr(x, src, diags);
            }
        }
        StmtKind::LocalDecl { name, value, .. } => {
            check_local_name(*name, src, diags);
            if let Some(value) = value {
                check_expr(value, src, diags);
            }
        }
        StmtKind::Destructure { target, value } => {
            check_destructure_target(target, src, diags);
            check_expr(value, src, diags);
        }
        StmtKind::Expr(x) => check_expr(x, src, diags),
        StmtKind::Return(Some(x)) | StmtKind::Break(Some(x)) | StmtKind::Continue(Some(x)) => {
            check_expr(x, src, diags);
        }
        // Rejected constructs whose name nothing downstream ever acts on
        // (see the module docs), plus every remaining no-nested-identifier
        // statement kind (`Return`/`Break`/`Continue` with no expression,
        // `Empty`, `InlineHtml`, `Global`, `Goto`, `UseDecl`, `Error`).
        _ => {}
    }
}

fn check_enum_decl(decl: &EnumDecl, src: &SourceFile, diags: &mut Diagnostics) {
    check_type_name(decl.name.span, src, "enum", diags);
    for case in &decl.cases {
        check_type_name(case.name.span, src, "enum case", diags);
        if let Some(value) = &case.value {
            check_expr(value, src, diags);
        }
    }
    // `decl.members` (anything other than a case) is always rejected per
    // `rule:enums/no-class-machinery` — left unchecked, same as the module docs' "already
    // rejected" gap.
}

fn check_members(members: &[ClassMember], src: &SourceFile, diags: &mut Diagnostics) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                check_visibility(
                    &p.attributes,
                    &p.modifiers,
                    member.span,
                    p.name,
                    "property",
                    diags,
                );
                check_property_name(p.name, src, diags);
                if let Some(default) = &p.default {
                    check_expr(default, src, diags);
                }
                if let Some(hooks) = &p.hooks {
                    for hook in hooks {
                        check_property_hook(hook, src, diags);
                    }
                }
            }
            ClassMemberKind::Const(c) => {
                check_visibility(
                    &c.attributes,
                    &c.modifiers,
                    member.span,
                    c.name,
                    "class constant",
                    diags,
                );
                check_const_name(c.name, src, diags);
                check_expr(&c.value, src, diags);
            }
            ClassMemberKind::Method(m) => {
                check_visibility(
                    &m.attributes,
                    &m.modifiers,
                    member.span,
                    m.name,
                    "method",
                    diags,
                );
                check_method(m, src, diags);
            }
            // An alias names a type, so it is PascalCase like the class whose
            // place it can stand in. The file-scope form is checked in
            // [`check_stmt`] with the same category: the two are one
            // production, so they get one answer.
            ClassMemberKind::TypeAlias(alias) => {
                check_type_name(alias.name.span, src, "type alias", diags);
            }
            ClassMemberKind::Error => {}
        }
    }
}

/// `rule:core-api/written-visibility`: a member declaration carrying none of `public`/`protected`/`private`
/// is [`code::E_MISSING_VISIBILITY`], because there is no default for it to
/// have meant. § 3 makes PHP 8.4's bare `private(set)` the same error rather
/// than a member whose read side is inferred, so a `(set)` modifier does not
/// answer this question — it is why the visibility test below asks only about
/// the three plain modifiers.
///
/// `member` is the declaration's full span, which starts *before* any
/// `#[...]` groups; the fix therefore inserts after the last of them, since a
/// modifier written ahead of an attribute would not parse.
fn check_visibility(
    attributes: &[AttributeGroup],
    modifiers: &[Modifier],
    member: Span,
    name: Span,
    category: &str,
    diags: &mut Diagnostics,
) {
    if modifiers.iter().any(|m| {
        matches!(
            m,
            Modifier::Public | Modifier::Protected | Modifier::Private
        )
    }) {
        return;
    }
    let message = match modifiers.iter().find_map(set_visibility) {
        Some(v) => format!(
            "`{v}(set)` is only the write half; a {category} writes its read visibility too, \
             e.g. `public {v}(set)`"
        ),
        None => format!("a {category} must declare `public`, `protected` or `private`"),
    };
    let (at, keyword) = match attributes.last() {
        Some(group) => (group.span.shrink_to_end(), " public"),
        None => (member.shrink_to_start(), "public "),
    };
    diags.report(
        Diagnostic::error(code::E_MISSING_VISIBILITY, message)
            .with_primary(name, "no visibility written here")
            .with_fix(at, keyword, "write `public`"),
    );
}

/// The keyword spelling of a `(set)` modifier's visibility, or `None` for
/// every other modifier.
fn set_visibility(m: &Modifier) -> Option<&'static str> {
    match m {
        Modifier::SetVisibility(Visibility::Public) => Some("public"),
        Modifier::SetVisibility(Visibility::Protected) => Some("protected"),
        Modifier::SetVisibility(Visibility::Private) => Some("private"),
        _ => None,
    }
}

fn check_property_hook(hook: &PropertyHook, src: &SourceFile, diags: &mut Diagnostics) {
    if let Some(param) = &hook.param {
        check_param_name(param.name, src, diags);
        if let Some(default) = &param.default {
            check_expr(default, src, diags);
        }
    }
    match &hook.body {
        Some(PropertyHookBody::Expr(e)) => check_expr(e, src, diags),
        Some(PropertyHookBody::Block(b)) => check_stmts(&b.stmts, src, diags),
        None => {}
    }
}

fn check_method(m: &MethodMember, src: &SourceFile, diags: &mut Diagnostics) {
    check_method_name(m.name, src, diags);
    check_params(&m.params, src, diags);
    if let Some(body) = &m.body {
        check_stmts(&body.stmts, src, diags);
    }
}

fn check_params(params: &[Param], src: &SourceFile, diags: &mut Diagnostics) {
    for param in params {
        check_param_name(param.name, src, diags);
        if let Some(default) = &param.default {
            check_expr(default, src, diags);
        }
    }
}

fn check_destructure_target(target: &DestructureTarget, src: &SourceFile, diags: &mut Diagnostics) {
    for element in &target.elements {
        match element {
            DestructureElement::Skip => {}
            DestructureElement::Leaf { key, name, .. } => {
                if let Some(key) = key {
                    check_expr(key, src, diags);
                }
                check_local_name(*name, src, diags);
            }
            DestructureElement::Nested { key, target, .. } => {
                if let Some(key) = key {
                    check_expr(key, src, diags);
                }
                check_destructure_target(target, src, diags);
            }
        }
    }
}

fn check_anon_class(anon: &AnonClassDecl, src: &SourceFile, diags: &mut Diagnostics) {
    check_members(&anon.members, src, diags);
}

fn check_fn_expr(fn_expr: &FnExpr, src: &SourceFile, diags: &mut Diagnostics) {
    if let Some(name) = fn_expr.name {
        check_anon_fn_name(name, src, diags);
    }
    check_params(&fn_expr.params, src, diags);
    match &fn_expr.body {
        FnBody::Expr(e) => check_expr(e, src, diags),
        FnBody::Block(b) => check_stmts(&b.stmts, src, diags),
    }
}

fn check_member_name_expr(member: &MemberName, src: &SourceFile, diags: &mut Diagnostics) {
    if let MemberName::Variable(e) | MemberName::Expr(e) = member {
        check_expr(e, src, diags);
    }
}

fn check_call_args(args: &CallArgs, src: &SourceFile, diags: &mut Diagnostics) {
    let CallArgs::List(list) = args else {
        return;
    };
    for Arg { value, .. } in list {
        check_expr(value, src, diags);
    }
}

fn check_new_target(target: &NewTarget, src: &SourceFile, diags: &mut Diagnostics) {
    match target {
        NewTarget::Expr(e) => check_expr(e, src, diags),
        NewTarget::AnonClass(anon) => check_anon_class(anon, src, diags),
        NewTarget::Name(_) | NewTarget::SelfTy | NewTarget::StaticTy | NewTarget::ParentTy => {}
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST expression variant, each a couple of lines"
)]
fn check_expr(expr: &Expr, src: &SourceFile, diags: &mut Diagnostics) {
    match &expr.kind {
        ExprKind::Interpolated(parts) | ExprKind::HtmlTemplate(parts) => {
            for part in parts {
                if let StringPart::Expr(x) = part {
                    check_expr(x, src, diags);
                }
            }
        }
        ExprKind::ArrayLiteral(items) => {
            for ArrayItem { key, value, .. } in items {
                if let Some(key) = key {
                    check_expr(key, src, diags);
                }
                check_expr(value, src, diags);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::PreIncDec { expr, .. }
        | ExprKind::PostIncDec { expr, .. }
        | ExprKind::Clone(expr)
        | ExprKind::YieldFrom(expr)
        | ExprKind::Print(expr)
        | ExprKind::Throw(expr)
        | ExprKind::Empty(expr)
        | ExprKind::Paren(expr) => check_expr(expr, src, diags),
        ExprKind::Binary { lhs, rhs, .. } => {
            check_expr(lhs, src, diags);
            check_expr(rhs, src, diags);
        }
        ExprKind::Assign { target, value, .. } => {
            check_expr(target, src, diags);
            check_expr(value, src, diags);
        }
        ExprKind::Ternary { cond, then, else_ } => {
            check_expr(cond, src, diags);
            if let Some(then) = then {
                check_expr(then, src, diags);
            }
            check_expr(else_, src, diags);
        }
        ExprKind::Conversion { expr, .. } => {
            check_expr(expr, src, diags);
        }
        ExprKind::TypeTest { expr, against } => {
            check_expr(expr, src, diags);
            if let TestOperand::Value(operand) = against {
                check_expr(operand, src, diags);
            }
        }
        ExprKind::Call { callee, args } => {
            check_expr(callee, src, diags);
            check_call_args(args, src, diags);
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            check_expr(object, src, diags);
            check_member_name_expr(method, src, diags);
            check_call_args(args, src, diags);
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
            ..
        } => {
            check_expr(class, src, diags);
            check_member_name_expr(method, src, diags);
            check_call_args(args, src, diags);
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => {
            check_expr(object, src, diags);
            check_member_name_expr(property, src, diags);
        }
        ExprKind::StaticPropertyAccess { class, .. } | ExprKind::ClassConstAccess { class, .. } => {
            check_expr(class, src, diags);
        }
        ExprKind::ClassNameConst { class } => check_expr(class, src, diags),
        ExprKind::Index { base, index } => {
            check_expr(base, src, diags);
            if let Some(index) = index {
                check_expr(index, src, diags);
            }
        }
        ExprKind::New { target, args, .. } => {
            check_new_target(target, src, diags);
            check_call_args(args, src, diags);
        }
        ExprKind::Fn(fn_expr) => check_fn_expr(fn_expr, src, diags),
        ExprKind::Match { subject, arms } => {
            check_expr(subject, src, diags);
            for arm in arms {
                if let Some(conds) = &arm.conditions {
                    for cond in conds {
                        check_expr(cond, src, diags);
                    }
                }
                check_expr(&arm.body, src, diags);
            }
        }
        ExprKind::Catch { guarded, arms } => {
            check_expr(guarded, src, diags);
            for arm in arms {
                if let Some(var) = arm.var {
                    check_local_name(var, src, diags);
                }
                check_expr(&arm.body, src, diags);
            }
        }
        ExprKind::Yield { key, value } => {
            if let Some(key) = key {
                check_expr(key, src, diags);
            }
            if let Some(value) = value {
                check_expr(value, src, diags);
            }
        }
        ExprKind::Exit(Some(x)) => check_expr(x, src, diags),
        ExprKind::Isset(xs) => {
            for x in xs {
                check_expr(x, src, diags);
            }
        }
        ExprKind::SpawnScript { path, options } => {
            check_expr(path, src, diags);
            for option in options {
                check_expr(&option.value, src, diags);
            }
        }
        ExprKind::Await(inner) => check_expr(inner, src, diags),
        ExprKind::Require { path } => check_expr(path, src, diags),
        ExprKind::AnonObject(fields) => {
            // `rule:types/anonymous-object`: a literal's field names are ordinary property
            // names, so `rule:core-api/identifier-casing`'s camelCase rule applies unchanged — reuse
            // the same check an ordinary class property declaration gets,
            // even though this field carries no `$` sigil to strip.
            for field in fields {
                check_member_casing(field.name, src, "field", false, diags);
                check_expr(&field.value, src, diags);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::SourceMap;

    use super::*;
    use crate::parse_file;

    fn check(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        check_declarations(&stmts, map.file(file), &mut diags);
        diags
    }

    /// [`check`] for a fixture the parser itself reports on — the `var` and
    /// enum-body shapes, which never reach a well-formed member, and the
    /// anonymous class, which is refused outright and parsed whole anyway — so
    /// it collects both halves rather than asserting the parse was clean.
    fn parse_and_check(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        check_declarations(&stmts, map.file(file), &mut diags);
        diags
    }

    fn only_code(diags: &Diagnostics) -> nvs_diagnostics::Code {
        assert_eq!(diags.len(), 1, "{diags:?}");
        diags.iter().next().unwrap().code.unwrap()
    }

    #[test]
    fn a_correctly_cased_class_is_clean() {
        let diags = check("<?nvs\nclass HttpClient {}\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_mis_cased_class_is_diagnosed() {
        let diags = check("<?nvs\nclass http_client {}\n");
        assert_eq!(only_code(&diags), code::E_BAD_TYPE_CASING);
        assert!(diags.iter().next().unwrap().message.contains("HttpClient"));
    }

    #[test]
    fn an_all_caps_acronym_is_accepted() {
        // `rule:core-api/casing-checks-the-leading-character`: only the leading character's case is checked, so a
        // kept-all-caps acronym is not flagged.
        let diags = check("<?nvs\nclass HTTPClient {}\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn an_all_caps_acronym_in_a_camel_case_name_is_accepted() {
        let diags = check("<?nvs\nclass Foo { public function parseHTTPRequest(): void {} }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_correctly_cased_interface_is_clean() {
        let diags = check("<?nvs\ninterface Comparable {}\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_mis_cased_interface_is_diagnosed() {
        let diags = check("<?nvs\ninterface comparable {}\n");
        assert_eq!(only_code(&diags), code::E_BAD_TYPE_CASING);
    }

    #[test]
    fn a_correctly_cased_enum_and_case_is_clean() {
        let diags = check("<?nvs\nenum Status { Active, Banned }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_mis_cased_enum_is_diagnosed() {
        let diags = check("<?nvs\nenum status { Active }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_TYPE_CASING))
        );
    }

    #[test]
    fn a_mis_cased_enum_case_is_diagnosed() {
        let diags = check("<?nvs\nenum Status { active }\n");
        assert_eq!(only_code(&diags), code::E_BAD_TYPE_CASING);
    }

    #[test]
    fn a_correctly_cased_namespace_segment_is_clean() {
        let diags = check("<?nvs\nnamespace App\\Http;\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_mis_cased_namespace_segment_is_diagnosed() {
        let diags = check("<?nvs\nnamespace app\\http;\n");
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == Some(code::E_BAD_TYPE_CASING))
                .count(),
            2
        );
    }

    #[test]
    fn a_correctly_cased_method_is_clean() {
        let diags = check("<?nvs\nclass Foo { public function getName(): void {} }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_mis_cased_method_is_diagnosed() {
        let diags = check("<?nvs\nclass Foo { public function get_name(): void {} }\n");
        assert_eq!(only_code(&diags), code::E_BAD_METHOD_CASING);
        assert!(diags.iter().next().unwrap().message.contains("getName"));
    }

    #[test]
    fn dunder_construct_gets_the_targeted_diagnostic() {
        let diags = check("<?nvs\nclass Foo { public function __construct(): void {} }\n");
        assert_eq!(only_code(&diags), code::E_LEGACY_CONSTRUCTOR_SPELLING);
        assert!(diags.iter().next().unwrap().message.contains("constructor"));
    }

    #[test]
    fn plain_constructor_is_not_diagnosed() {
        let diags = check("<?nvs\nclass Foo { public function constructor(): void {} }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_correctly_cased_property_is_clean() {
        let diags = check("<?nvs\nclass Foo { public int $userId; }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_mis_cased_property_is_diagnosed() {
        let diags = check("<?nvs\nclass Foo { public int $user_id; }\n");
        assert_eq!(only_code(&diags), code::E_BAD_MEMBER_CASING);
        assert!(diags.iter().next().unwrap().message.contains("userId"));
    }

    #[test]
    fn a_leading_underscore_property_is_rejected_with_no_allowance() {
        let diags = check("<?nvs\nclass Foo { public int $_cache; }\n");
        assert_eq!(only_code(&diags), code::E_BAD_MEMBER_CASING);
        assert!(diags.iter().next().unwrap().message.contains("cache"));
    }

    #[test]
    fn a_correctly_cased_anon_object_field_is_clean() {
        // `rule:types/anonymous-object`: field names are ordinary property names.
        let diags = check("<?nvs\n$o = {userId: 1};\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_mis_cased_anon_object_field_is_diagnosed() {
        let diags = check("<?nvs\n$o = {user_id: 1};\n");
        assert_eq!(only_code(&diags), code::E_BAD_MEMBER_CASING);
        assert!(diags.iter().next().unwrap().message.contains("userId"));
    }

    #[test]
    fn a_leading_underscore_anon_object_field_is_rejected() {
        let diags = check("<?nvs\n$o = {_cache: 1};\n");
        assert_eq!(only_code(&diags), code::E_BAD_MEMBER_CASING);
    }

    #[test]
    fn a_correctly_cased_parameter_is_clean() {
        let diags = check("<?nvs\nclass Foo { public function a(int $userId): void {} }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_leading_underscore_parameter_is_rejected() {
        let diags = check("<?nvs\nclass Foo { public function a(int $_unused): void {} }\n");
        assert_eq!(only_code(&diags), code::E_BAD_MEMBER_CASING);
    }

    #[test]
    fn a_correctly_cased_local_is_clean() {
        let diags =
            check("<?nvs\nclass Foo { public function a(): void { int $rowCount = 1; } }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_leading_underscore_local_is_rejected() {
        let diags = check("<?nvs\nclass Foo { public function a(): void { int $_tmp = 1; } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_MEMBER_CASING))
        );
    }

    #[test]
    fn a_foreach_binding_is_checked() {
        let diags = check(
            "<?nvs\nclass Foo { public function a(): void { foreach ($xs as int $_bad) {} } }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_MEMBER_CASING))
        );
    }

    #[test]
    fn a_catch_binding_is_checked() {
        let diags = check(
            "<?nvs\nclass Foo { public function a(): void { try {} catch (Exception $_e) {} } }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_MEMBER_CASING))
        );
    }

    #[test]
    fn a_destructure_leaf_is_checked() {
        let diags = check("<?nvs\nclass Foo { public function a(): void { [int $_x] = $xs; } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_MEMBER_CASING))
        );
    }

    #[test]
    fn an_anon_fn_self_name_is_checked_like_a_local() {
        let diags = check(
            "<?nvs\nclass Foo { public function a(): void { $f = fn bad_name(int $n) => $n; } }\n",
        );
        assert_eq!(only_code(&diags), code::E_BAD_MEMBER_CASING);
    }

    #[test]
    fn a_correctly_named_anon_fn_self_name_is_clean() {
        let diags = check(
            "<?nvs\nclass Foo { public function a(): void { $f = fn factorial(int $n) => $n; } }\n",
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_correctly_cased_constant_is_clean() {
        let diags = check("<?nvs\nclass Foo { public const int MAX_RETRIES = 3; }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_mis_cased_constant_is_diagnosed() {
        let diags = check("<?nvs\nclass Foo { public const int maxRetries = 3; }\n");
        assert_eq!(only_code(&diags), code::E_BAD_CONST_CASING);
        assert!(diags.iter().next().unwrap().message.contains("MAX_RETRIES"));
    }

    #[test]
    fn a_nested_class_declaration_is_still_checked() {
        let diags = check("<?nvs\nclass Outer { public function a(): void { class inner {} } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_TYPE_CASING))
        );
    }

    #[test]
    fn an_anonymous_class_bodys_members_are_checked() {
        let diags = parse_and_check(
            "<?nvs\nclass Foo { public function a(): void { $x = new class { public int $bad_name = 1; }; } }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_MEMBER_CASING))
        );
    }

    #[test]
    fn a_property_hook_parameter_is_checked() {
        let diags = check("<?nvs\nclass Foo { public int $bar { set(int $bad_value) { } } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_MEMBER_CASING))
        );
    }

    #[test]
    fn a_reference_to_a_mis_cased_method_is_not_double_checked() {
        // `Foo::bar()` is a reference, not a declaration — only `bar`'s own
        // declaration (absent here) would ever be checked, so a call site
        // naming a mis-cased method produces nothing on its own.
        let diags =
            check("<?nvs\nclass Foo { public function a(): void { self::snake_case_call(); } }\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    // ------------------------------------------------------------------
    // `rule:core-api/written-visibility` — a member declaration writes its visibility
    // ------------------------------------------------------------------

    #[test]
    fn a_member_without_visibility_is_a_compile_error() {
        // `rule:core-api/written-visibility`: all three member slots, in all three bodies that
        // have one. There is no default for any of them to have meant.
        for src in [
            "<?nvs\nclass Foo { int $count = 0; }\n",
            "<?nvs\nclass Foo { const int MAX = 1; }\n",
            "<?nvs\nclass Foo { function run(): void {} }\n",
            "<?nvs\nclass Foo { static function run(): void {} }\n",
            "<?nvs\ninterface Runner { function run(): void; }\n",
        ] {
            let diags = check(src);
            assert_eq!(only_code(&diags), code::E_MISSING_VISIBILITY, "{src}");
        }

        // The sixth body: an anonymous class, which the parser refuses and
        // still builds, so this pass reaches its members exactly as it reaches
        // a named class's.
        let diags = parse_and_check(
            "<?nvs\nclass Foo { public function a(): void { $x = new class { int $n = 1; }; } }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_MISSING_VISIBILITY)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_written_visibility_is_clean() {
        for src in [
            "<?nvs\nclass Foo { public int $count = 0; }\n",
            "<?nvs\nclass Foo { protected const int MAX = 1; }\n",
            "<?nvs\nclass Foo { private static function run(): void {} }\n",
            "<?nvs\nclass Foo { public private(set) string $name = \"a\"; }\n",
        ] {
            let diags = check(src);
            assert!(diags.is_empty(), "{src}: {diags:?}");
        }
    }

    #[test]
    fn a_bare_set_visibility_is_a_compile_error() {
        // `rule:core-api/asymmetric-visibility-is-a-pair`: PHP 8.4 infers a `public` read side here, which is
        // the same implicit `public` this rule removes — so the message
        // names the pair rather than a bare keyword.
        let diags = check("<?nvs\nclass Foo { private(set) string $name = \"a\"; }\n");
        assert_eq!(only_code(&diags), code::E_MISSING_VISIBILITY);
        let message = &diags.iter().next().unwrap().message;
        assert!(message.contains("public private(set)"), "{message}");
    }

    #[test]
    fn a_plain_constructor_parameter_needs_no_visibility() {
        // `rule:core-api/a-parameter-is-not-a-member`: visibility is what promotes a parameter to a
        // property, so requiring it on every parameter would delete the
        // distinction. Only the promoted one is a member.
        let diags = check(
            "<?nvs\nclass Foo { public function constructor(int $n, public int $kept) {} }\n",
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn a_class_body_var_names_the_missing_visibility() {
        // `rule:core-api/legacy-property-shapes-name-the-visibility`: `var` is `rule:types/var-inference`'s local-inference keyword, so
        // without its own arm this would report something about the
        // statement grammar to an author writing PHP's property form.
        let diags = parse_and_check("<?nvs\nclass Foo { var $name; }\n");
        assert_eq!(only_code(&diags), code::E_MISSING_VISIBILITY);
    }

    #[test]
    fn an_enum_body_reports_only_that_it_has_no_members() {
        // `rule:core-api/written-visibility`'s scope stops at a body with a member slot; `rule:enums/no-class-machinery`
        // already rejects everything in an enum that is not a case, and two
        // diagnostics for one mistake is worse than one.
        let diags = parse_and_check(
            "<?nvs\nenum Color { Red = 1, }\nenum Sized { function run(): void {} }\n",
        );
        assert_eq!(only_code(&diags), code::E_ENUM_MEMBER_UNSUPPORTED);
    }
}
