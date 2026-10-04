//! `rule:programs/relative-paths-resolve-from-their-file`: a relative string
//! literal passed to a path parameter names a file beside the source file that
//! wrote it, and the compiler writes the absolute path in its place.
//!
//! # What is a path parameter
//!
//! A parameter whose [`ParamText`] is [`ParamText::Path`]. A `Core` row states
//! it with `nvs_stdlib::registry::CoreTy::Path`, and a user method states it
//! with `#[Core\Path]` on the parameter ([`declared_text`]). Both arrive on
//! [`MethodSig::param_text`], so [`resolve_args`] reads one vector whoever
//! declared the method. A field of a `Core` shape or options bag carries the
//! same mark on `crate::ty::CoreShapeField::text`, and
//! `crate::expr::args::check_options_arg` hands each written field to
//! [`resolve_literal`].
//!
//! The script an isolate runs is a path position too. `spawn script <path>` is
//! a construct rather than a call, so `crate::expr::isolate`'s `check_entry`
//! hands its operand to [`resolve_literal`] itself. `Core\Socket::upgrade` and
//! `Core\Sse::upgrade` mark their entry `nvs_stdlib::registry::CoreTy::Entry`,
//! whose [`ParamText`] is [`ParamText::Path`], so they arrive through
//! [`resolve_args`] like any other `Core` row.
//!
//! # What is a literal
//!
//! **A plain string literal, written at the argument itself** — `'data/x.json'`,
//! a double-quoted string with no interpolation, a nowdoc. Nothing else
//! counts: not a class constant, not a concatenation of literals, not a
//! variable that holds one. The boundary is the expression the reader sees at
//! the call, because the rule's whole promise is that the path a call site
//! shows is resolved from the file that shows it. A constant declared in one
//! file and used in another would make that file ambiguous, so it stays a
//! run-time value, and a relative one meets the run-time refusal in
//! `nvs_runtime::capability::require`. The message there names
//! `Core\Path::join` and `Core\Path::fromCwd`.
//!
//! A path built from a relative literal is the one run-time value whose
//! refusal is certain while compiling, so it is `E0840` instead: the left end
//! of a `.` chain is a relative literal (`'data/' . $name`), or an
//! interpolated string starts with relative text (`"data/{$name}"`). Whatever
//! the program adds, the path still starts relative, so the door throws on
//! every run. Two starts are left to run time because the added text could
//! make them absolute: a single letter, which a `:` can turn into a drive, and
//! a heredoc, whose first text still carries its indentation. The error needs
//! no folder, so a source with no file reports it as well: the throw does not
//! depend on where the file is.
//!
//! # A lexical join
//!
//! The literal is joined to the folder of the declaring file and `.` and `..`
//! are removed from the text. The folder is the canonical one the `require`
//! walk names the file by ([`base_file`]), so the entry file and a required
//! file agree on it. The join itself reads nothing, for three reasons.
//! A build has to give the same answer whether or not the file exists yet,
//! since a program may be about to create it. A bundled executable compiles
//! from its payload, where the folder is synthetic
//! ([`nvs_diagnostics::embedded::on_disk`] maps it beside the executable). And
//! the security question is not this module's: every `Core` door
//! canonicalizes the path it is handed before it compares it with a grant
//! (`rule:security/path-scope-canonicalise-then-prefix`), so a literal that
//! climbs out with `..` is judged where it lands. What a lexical `..` gives up
//! is the case where a folder on the way is a symbolic link; the path then
//! names the place the source text shows, not the place the link leads.
//!
//! An absolute literal is left as it was written. A file with no path — a
//! source handed to the compiler as text, as the editor does — leaves every
//! literal as it was written too, and the run-time refusal applies.
//!
//! **Cost:** one join per literal while compiling, and one table row per
//! resolved literal for the length of a compile. A program pays nothing at
//! run time: the instruction is the same constant string a literal always
//! lowers to.
//!
//! # The file that wrote the call
//!
//! `Core\Path::thisFile()` and `Core\Path::thisDir($join)` are replaced by
//! [`fold_this`] with the path of the file that wrote them and its folder,
//! from the same [`base_dir`] a literal is joined to, so a bundle answers the
//! folder beside the executable. The answer goes into the path-literal table
//! under the call's own span, and `nvs-ir` lowers it to a string constant and
//! no call. `$join` is a literal [`resolved`] accepts and nothing else
//! (`E0837`): the fold happens while compiling, and a value built at run time
//! already has `Core\Path::join`. A source with no file records nothing, and
//! the member's own symbol throws.

use std::path::{Component, Path, PathBuf, Prefix};

use nvs_diagnostics::{Diagnostic, SourceFile, code};
use nvs_hir::QName;
use nvs_stdlib::registry::ParamText;
use nvs_syntax::ast::{
    Arg, AttributeGroup, BinaryOp, CallArgs, ClassMember, ClassMemberKind, EnumCase, Expr,
    ExprKind, MethodMember, Param, StringPart, Type, TypeAtom, TypeKind,
};

use crate::defaults::ConstArg;
use crate::expr_table::ArgSlot;
use crate::signatures::MethodSig;
use crate::{Ctx, Env, span_text, strip_sigil};

/// Each parameter's [`ParamText`] as `#[Core\Path]` declares it, or an empty
/// list when no parameter carries the marker — [`MethodSig::param_text`]'s
/// "nothing is marked".
pub(crate) fn declared_text(params: &[Param], ctx: &Ctx<'_>, env: &Env<'_>) -> Vec<ParamText> {
    let text: Vec<ParamText> = params
        .iter()
        .map(|param| {
            if crate::testing::attribute_named(&param.attributes, crate::derive::PATH, ctx, env)
                .is_some()
            {
                ParamText::Path
            } else {
                ParamText::Plain
            }
        })
        .collect();
    if text.iter().all(|one| *one == ParamText::Plain) {
        Vec::new()
    } else {
        text
    }
}

/// A `#[Core\Path]` parameter's default, resolved the way an argument written
/// at the call would be: the default is written in the declaring file, so a
/// relative literal there names a file beside that file.
pub(crate) fn resolve_defaults(
    params: &[Param],
    text: &[ParamText],
    defaults: &mut [Option<ConstArg>],
    env: &Env<'_>,
) {
    for ((param, mark), default) in params.iter().zip(text).zip(defaults.iter_mut()) {
        if *mark != ParamText::Path {
            continue;
        }
        let Some(written) = &param.default else {
            continue;
        };
        if !matches!(written.unparenthesized().kind, ExprKind::Str(_)) {
            continue;
        }
        if let Some(ConstArg::Str(value)) = default
            && let Some(joined) = resolved(env.src, value)
        {
            *value = joined;
        }
    }
}

/// Every written argument that fills a [`ParamText::Path`] parameter, offered
/// to [`resolve_literal`]. A `...` spread fills the parameter with an array's
/// entries, so there is no literal to resolve there.
pub(crate) fn resolve_args(list: &[Arg], slots: &[ArgSlot], sig: &MethodSig, env: &mut Env<'_>) {
    if sig.param_text.is_empty() {
        return;
    }
    for (arg, slot) in list.iter().zip(slots) {
        if let ArgSlot::Param(index) = *slot
            && sig.text_at(index) == ParamText::Path
        {
            resolve_literal(&arg.value, env);
        }
    }
}

/// Records the absolute path a relative string literal at a path position
/// names, under the literal's own span, where `nvs-ir` lowers it
/// (`crate::ExprTypeTable::path_literal`). A literal that is already absolute
/// records nothing. A path built from a relative literal is `E0840`
/// ([`report_built`]), and anything else records nothing.
pub(crate) fn resolve_literal(value: &Expr, env: &mut Env<'_>) {
    let value = value.unparenthesized();
    let ExprKind::Str(span) = value.kind else {
        report_built(value, env);
        return;
    };
    let text = nvs_syntax::string_lit::cook_string_literal(env.src, span);
    if let Some(joined) = resolved(env.src, &text) {
        env.exprs.record_path_literal(span, joined);
    }
}

/// `E0840`, at the relative literal that starts a path built while the
/// program runs — the module doc's § *What is a literal*.
///
/// The leftmost operand of a `.` chain is followed down, and an interpolated
/// string is judged by the text before its first value. The error does not
/// need the file's folder, so a source with no file reports it too.
fn report_built(value: &Expr, env: &mut Env<'_>) {
    let mut first = value;
    while let ExprKind::Binary {
        op: BinaryOp::Concat,
        lhs,
        ..
    } = &first.kind
    {
        first = lhs.unparenthesized();
    }
    // A plain literal here is the left end of a `.` chain: [`resolve_literal`]
    // has already taken a literal that is the whole argument.
    let (span, text) = match &first.kind {
        ExprKind::Str(span) => (
            *span,
            nvs_syntax::string_lit::cook_string_literal(env.src, *span),
        ),
        ExprKind::Interpolated(parts) if !span_text(env.src, first.span).starts_with("<<<") => {
            let Some(StringPart::Text(span)) = parts.first() else {
                return;
            };
            (
                *span,
                nvs_syntax::string_lit::cook_double_quoted_text(env.src, *span).0,
            )
        }
        _ => return,
    };
    if !is_relative(&text) || may_become_a_drive(&text) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_PATH_BUILT_FROM_A_RELATIVE_LITERAL,
            "this path is relative when the program runs, so the call always throws an error",
        )
        .with_primary(
            span,
            "a relative path, with more added while the program runs",
        )
        .with_help(
            "start the path with the folder of this file: \
             `Core\\Path::thisDir('data') . '/' . $name`, or \
             `Core\\Path::join(Core\\Path::thisDir('data'), $name)`",
        ),
    );
}

/// Whether text the program adds to `text` could turn it into a drive, so the
/// whole path could be absolute: `'C' . $rest` names `C:\data` when `$rest` is
/// `':\data'`. Only one letter with no separator after it can.
fn may_become_a_drive(text: &str) -> bool {
    let mut chars = text.chars();
    matches!((chars.next(), chars.next()), (Some(letter), None) if letter.is_ascii_alphabetic())
}

/// Whether `member` of `owner` is `Core\Path::thisFile` or `thisDir`, the two
/// calls [`fold_this`] replaces with a path.
pub(crate) fn is_this(owner: &QName, member: &str) -> bool {
    owner.to_string() == r"Core\Path" && matches!(member, "thisFile" | "thisDir")
}

/// Records the path `Core\Path::thisFile()` or `Core\Path::thisDir($join)`
/// names under the call's own span, in the table a path literal is recorded
/// in, where `nvs-ir` lowers it to a string constant — the module doc's
/// § *The file that wrote the call*.
///
/// A `$join` that is not a relative string literal is `E0837`. A source with
/// no folder records nothing, so the call stays a call and its symbol throws.
pub(crate) fn fold_this(call: &Expr, member: &str, args: &CallArgs, env: &mut Env<'_>) {
    let CallArgs::List(list) = args else {
        return;
    };
    let join = list
        .first()
        .filter(|arg| !matches!(arg.value.unparenthesized().kind, ExprKind::Null));
    let path = match (member, join) {
        ("thisFile", _) => base_file(env.src).map(|file| file.to_string_lossy().into_owned()),
        (_, None) => base_folder(env.src),
        (_, Some(arg)) => {
            let text = match arg.value.unparenthesized().kind {
                ExprKind::Str(span) if !arg.spread => {
                    Some(nvs_syntax::string_lit::cook_string_literal(env.src, span))
                }
                _ => None,
            };
            let Some(text) = text.filter(|text| is_relative(text)) else {
                report_join(&arg.value, env);
                return;
            };
            resolved(env.src, &text)
        }
    };
    if let Some(path) = path {
        env.exprs.record_path_literal(call.span, path);
    }
}

/// `E0837`, at a `$join` [`fold_this`] cannot add to the folder.
fn report_join(join: &Expr, env: &mut Env<'_>) {
    let written = span_text(env.src, join.span).to_owned();
    env.diags.report(
        Diagnostic::error(
            code::E_PATH_THIS_DIR_JOIN_NOT_A_RELATIVE_LITERAL,
            format!("`Core\\Path::thisDir({written})` needs a relative path written as a literal"),
        )
        .with_primary(join.span, "not a relative string literal")
        .with_help(
            "the folder is joined while compiling \
             (`rule:programs/relative-paths-resolve-from-their-file`): write a relative literal \
             such as `'data'`, or `Core\\Path::join(Core\\Path::thisDir(), $part)` for a path \
             the program builds",
        ),
    );
}

/// Whether `text` is a path [`resolved`] joins to a folder: not empty, not
/// starting at a root, and with no `:` in its first segment.
///
/// "Starts at a root" is [`Path::has_root`], the same test the run-time
/// refusal makes, so a literal this leaves alone is one that check accepts.
///
/// A first segment that holds a `:` is a drive (`C:data`), a URI scheme
/// (`file:app.db?mode=memory`) or a name an engine gives something that is not
/// a file (SQLite's `:memory:`), and joining a folder in front of any of them
/// makes a name nothing can open.
fn is_relative(text: &str) -> bool {
    let first = text.split(['/', '\\']).next().unwrap_or_default();
    !text.is_empty() && !first.contains(':') && !Path::new(text).has_root()
}

/// The absolute path `text` names when it is written in `src`: joined to the
/// folder that holds `src`, with `.` and `..` removed. `None` for a `text`
/// that is not [`is_relative`], and for a source with no folder to join to.
#[must_use]
pub fn resolved(src: &SourceFile, text: &str) -> Option<String> {
    if !is_relative(text) {
        return None;
    }
    let joined = normalize(&base_dir(src)?.join(text));
    joined
        .has_root()
        .then(|| joined.to_string_lossy().into_owned())
}

/// The folder every relative literal in `src` is joined to, as text, or `None`
/// for a source with no folder. A compiled program embeds paths built from
/// it, so a cache of compiled programs keys on it beside the file's text: the
/// same file in another folder compiles to other paths.
#[must_use]
pub fn base_folder(src: &SourceFile) -> Option<String> {
    base_dir(src).map(|dir| dir.to_string_lossy().into_owned())
}

/// The folder a relative literal in `src` is joined to: the one that holds
/// [`base_file`].
fn base_dir(src: &SourceFile) -> Option<PathBuf> {
    base_file(src)?.parent().map(Path::to_path_buf)
}

/// The path of the file `src` was read from, as the `require` walk names a
/// file: canonical, so every link is followed, a Windows short name such as
/// `RUNNER~1` is written out in full, and the letters have their on-disk case.
/// The entry file arrives as it was typed and a required file arrives
/// canonical, so this is what makes `thisDir()` and a literal give the same
/// folder in both. Inside a bundled executable the folder is synthetic, and
/// [`nvs_diagnostics::embedded::on_disk`] answers where it is on disk.
///
/// A file that cannot be canonicalized, such as an editor buffer not yet
/// saved, is made absolute instead. A file named relative to the shell's
/// working directory — `nvs run tool.nvs` — is made absolute against that
/// directory here, while compiling. That is the one time the working
/// directory is read, and it is what the person who typed the command meant
/// by the name they typed.
///
/// **Cost:** one `canonicalize` call per literal and per `thisFile` or
/// `thisDir` call while compiling, on a file the compiler has just read.
fn base_file(src: &SourceFile) -> Option<PathBuf> {
    let path = src.path()?;
    let name = path.file_name()?;
    let dir = path.parent()?;
    if let Some(on_disk) = nvs_diagnostics::embedded::on_disk(dir) {
        return Some(on_disk.join(name));
    }
    nvs_footprint::exists(path);
    if let Ok(real) = path.canonicalize() {
        return Some(without_verbatim(real));
    }
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    std::path::absolute(dir)
        .ok()
        .map(|dir| without_verbatim(dir).join(name))
}

/// A Windows verbatim path — `\\?\C:\app`, which is how a canonicalized
/// `require` target is spelled — written the ordinary way, `C:\app`.
///
/// Both name the same file. The ordinary spelling is the one a program prints
/// and the one `Core\Path`'s grammar reads, so a file reached through `require`
/// gives its literals the same text as the entry file would. Every other path
/// is returned unchanged.
fn without_verbatim(path: PathBuf) -> PathBuf {
    let mut parts = path.components();
    let Some(Component::Prefix(prefix)) = parts.next() else {
        return path;
    };
    let head = match prefix.kind() {
        Prefix::VerbatimDisk(letter) => format!("{}:", char::from(letter)),
        Prefix::VerbatimUNC(server, share) => format!(
            r"\\{}\{}",
            server.to_string_lossy(),
            share.to_string_lossy()
        ),
        _ => return path,
    };
    let mut out = PathBuf::from(head);
    out.extend(parts);
    out
}

/// `.` dropped and `..` taking away the folder before it, with no filesystem
/// access. A `..` at a root is dropped, because a root has no parent.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// `#[Core\Path]` held to the one place it means something: a method
/// parameter whose declared type is a `string`. Asked from
/// [`crate::attributes::check_declaration`]'s walk, which visits every attach
/// site of one declaration, for [`crate::routes::check_stray_query`]'s reason.
pub(crate) fn check_marker_sites(
    groups: &[AttributeGroup],
    members: &[ClassMember],
    cases: &[EnumCase],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    report_stray(groups, "a declaration", ctx, env);
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                report_stray(&p.attributes, "a property", ctx, env);
                for hook in p.hooks.iter().flatten() {
                    report_stray(&hook.attributes, "a property hook", ctx, env);
                }
            }
            ClassMemberKind::Const(c) => report_stray(&c.attributes, "a constant", ctx, env),
            ClassMemberKind::Method(m) => check_method(m, ctx, env),
            _ => {}
        }
    }
    for case in cases {
        report_stray(&case.attributes, "an enum case", ctx, env);
    }
}

/// One method: the marker on the method itself, and on each parameter whose
/// type is not a `string`.
fn check_method(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    report_stray(&m.attributes, "a method", ctx, env);
    for param in &m.params {
        let Some(attr) =
            crate::testing::attribute_named(&param.attributes, crate::derive::PATH, ctx, env)
        else {
            continue;
        };
        if param.ty.as_ref().is_some_and(is_string_type) {
            continue;
        }
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        let written = param
            .ty
            .as_ref()
            .map_or("no type", |ty| span_text(env.src, ty.span))
            .to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_PATH_MARKER_NOT_ON_A_STRING,
                format!("`#[Core\\Path] ${name}` is on a parameter of type `{written}`"),
            )
            .with_primary(attr.span, "a path is a `string`")
            .with_help(
                "`#[Core\\Path]` says a string literal passed here is a file path \
                 (`rule:programs/relative-paths-resolve-from-their-file`): declare the parameter \
                 `string` or `?string`, or delete the marker",
            ),
        );
    }
}

/// Each `#[Core\Path]` among `groups`, which sit on `site` — somewhere no
/// argument is ever passed.
fn report_stray(groups: &[AttributeGroup], site: &str, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for attr in groups.iter().flat_map(|group| &group.attributes) {
        if !crate::derive::attribute_is(attr, crate::derive::PATH, ctx, env) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_PATH_MARKER_NOT_ON_A_STRING,
                format!("`#[Core\\Path]` is on {site}"),
            )
            .with_primary(attr.span, "nothing is passed here")
            .with_help(
                "`#[Core\\Path]` marks a method parameter that takes a file path \
                 (`rule:programs/relative-paths-resolve-from-their-file`): move it to that \
                 parameter, or delete it",
            ),
        );
    }
}

/// Whether a written type is `string` — plain or qualified — alone or with
/// `null`: `string`, `?string`, `string|null`, `tainted string`.
fn is_string_type(ty: &Type) -> bool {
    match &ty.kind {
        TypeKind::Atom(atom) => matches!(
            atom,
            TypeAtom::String
                | TypeAtom::TaintedString
                | TypeAtom::SecretString
                | TypeAtom::SecretTaintedString
        ),
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => is_string_type(inner),
        TypeKind::Union(members) => {
            members.iter().any(is_string_type)
                && members.iter().all(|member| {
                    is_string_type(member) || matches!(member.kind, TypeKind::Atom(TypeAtom::Null))
                })
        }
        _ => false,
    }
}
