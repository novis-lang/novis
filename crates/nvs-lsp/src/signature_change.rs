//! *Change parameters*: a method's parameter list rewritten in every method
//! of its override family, with the arguments of every call rewritten to
//! match (`rule:tooling/nvs-code-answers-about-the-project`).
//!
//! [`signature`] takes the whole new list, as it is written in source.
//! Parameters match the old list by name, so a kept parameter may change its
//! type, its default or its place. An added one needs a default or a value,
//! given per name, that is written at every call; with both, a call keeps
//! the default. A removed one deletes its argument at every call. Renaming a
//! parameter is `rename`'s.
//!
//! # Decision: a change is refused wherever it could change what runs
//!
//! - **A removed argument that may run code** (a call, `new`, an assignment,
//!   a property read, [`crate::inline::runs_code`]) is refused, since the code
//!   would no longer run. A removed parameter that a body of the family still
//!   reads is refused too.
//! - **A reorder** keeps every argument that may run code in the order it
//!   was written: if two of them would swap, the change is refused.
//! - **Every call must be known.** A method of the family that is used as a
//!   value, a type above the family that is declared outside the workspace,
//!   a call that spreads its arguments, a call nested in the arguments of
//!   another call of the family and a call the parse cannot find each refuse
//!   the change. So do a variadic parameter and two methods of the family
//!   with a different number of parameters.
//!
//! Each method of the family keeps its own parameter names, and the
//! modifiers, attributes and spelling of the type and default of each kept
//! parameter that the new list does not change. A changed or added type is
//! written as the new list writes it. Where a file of the family needs
//! another spelling, `crate::refactor::checked` refuses the edit with the
//! checker's error. A positional argument after the first one a call leaves
//! to its default is written with its name. Each removed parameter's
//! `@param` line goes; an added one gets none.
//!
//! A constructor is refused for now: its calls are `new` expressions, which
//! the index records as uses of the class, not of the constructor.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. One parse of each file
//! that declares a method of the family or calls one, then the check that
//! `crate::refactor::checked` describes. Nothing is kept.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nvs_diagnostics::{BytePos, Diagnostics, SourceId, SourceMap, Span};
use nvs_syntax::ast::{CallArgs, ClassMemberKind, MethodMember, Param, Stmt, StmtKind};
use nvs_syntax::parse_file;
use nvs_syntax::walk::{self, Node};

use crate::document::Documents;
use crate::index::{DeclKind, Site, SymbolIndex};
use crate::inline::{line_of, method_member, path_to, runs_code};
use crate::rename::{Rewrite, call_in_stmts, core_has, family, outside_above};

/// Why a change is refused, written for the person who asked.
type Why = String;

/// The name a class gives its constructor.
const CONSTRUCTOR: &str = "constructor";

/// One parameter of a list, read as text.
#[derive(Clone, Debug)]
struct Written {
    /// The name with its `$`.
    name: String,
    /// The text before the type, or before the name where there is no type:
    /// attributes, modifiers and `inout`.
    prefix: String,
    /// The type's text.
    ty: Option<String>,
    /// The default's text.
    default: Option<String>,
    /// The whole parameter.
    whole: String,
    /// Whether it is written `inout`.
    inout: bool,
    /// Whether it is variadic.
    variadic: bool,
}

impl Written {
    /// `param` of `source`.
    fn of(source: &str, param: &Param) -> Self {
        let text = |span: Span| source.get(span.range()).unwrap_or_default().to_owned();
        let head = param
            .ty
            .as_ref()
            .map_or(param.name.start, |ty| ty.span.start);
        Self {
            name: text(param.name),
            prefix: source
                .get(param.span.start as usize..head as usize)
                .unwrap_or_default()
                .to_owned(),
            ty: param.ty.as_ref().map(|ty| text(ty.span)),
            default: param.default.as_ref().map(|default| text(default.span)),
            whole: text(param.span),
            inout: param.inout,
            variadic: param.variadic,
        }
    }

    /// The parameter as it is written with `ty` and `default`.
    fn rendered(&self, ty: Option<&str>, default: Option<&str>) -> String {
        let mut out = self.prefix.clone();
        if let Some(ty) = ty {
            out.push_str(ty);
            out.push(' ');
        }
        out.push_str(&self.name);
        if let Some(default) = default {
            out.push_str(" = ");
            out.push_str(default);
        }
        out
    }
}

/// Where a parameter of the new list comes from.
#[derive(Clone, Copy, Debug)]
enum Source {
    /// The old parameter at this place.
    Kept(usize),
    /// A new parameter.
    Added,
}

/// One method of the family, read from its file.
struct Member {
    /// Its symbol.
    symbol: String,
    /// Its file.
    path: PathBuf,
    /// Its parameters.
    params: Vec<Written>,
    /// The bytes between its parameter list's parentheses.
    inside: (BytePos, BytePos),
    /// The variables its body writes.
    reads: BTreeSet<String>,
    /// The line of its name, counted from 0, where its `///` run ends.
    name_line: usize,
}

/// The files the change reads, each parsed once.
struct Files {
    map: SourceMap,
    loaded: BTreeMap<PathBuf, Option<(SourceId, Vec<Stmt>)>>,
}

impl Files {
    fn new(documents: &Documents) -> Self {
        let mut map = SourceMap::new();
        documents.overlay(&mut map);
        Self {
            map,
            loaded: BTreeMap::new(),
        }
    }

    /// The text and the statements of `path`, or why it cannot be read.
    fn read(&mut self, path: &Path) -> Result<(&str, &[Stmt]), Why> {
        if !self.loaded.contains_key(path) {
            let parsed = self.map.load(path).ok().map(|id| {
                let stmts = parse_file(self.map.file(id), &mut Diagnostics::new());
                (id, stmts)
            });
            self.loaded.insert(path.to_path_buf(), parsed);
        }
        match self.loaded.get(path) {
            Some(Some((id, stmts))) => Ok((self.map.file(*id).text(), stmts.as_slice())),
            _ => Err(format!("`{}` cannot be read.", path.display())),
        }
    }
}

/// The edits that give the method `symbol` and every method of its override
/// family the parameter list `list`, and every call the arguments it then
/// needs. `values` gives, per new parameter's name with its `$`, the
/// expression written at every call. Every edit is checked before it is
/// returned.
///
/// # Errors
///
/// Returns why the parameters are not changed.
pub fn signature(
    documents: &Documents,
    index: &SymbolIndex,
    symbol: &str,
    list: &str,
    values: &[(String, String)],
) -> Result<Vec<Rewrite>, Why> {
    let declared = index
        .declaration(symbol)
        .filter(|declared| declared.kind == DeclKind::Method)
        .ok_or_else(|| format!("`{symbol}` is not a method of this project."))?;
    if symbol.rsplit_once("::").map(|(_, name)| name) == Some(CONSTRUCTOR) {
        return Err(format!(
            "`{symbol}` is a constructor. Its parameters cannot be changed with this command yet."
        ));
    }
    let new = parsed_list(list)?;
    let methods = family(index, symbol);
    for method in &methods {
        outside(index, method)?;
        if index.unfollowed(method) {
            return Err(format!(
                "`{method}` is used as a value, or called where its class is not known, so not \
                 every call can be changed."
            ));
        }
    }

    let mut files = Files::new(documents);
    let mut members = Vec::new();
    for method in &methods {
        let site = &index
            .declaration(method)
            .ok_or_else(|| format!("`{method}` is not a method of this project."))?
            .site;
        members.push(member(&mut files, method, &site.path, site.start)?);
    }
    let target = members
        .iter()
        .position(|member| member.symbol == declared.symbol)
        .ok_or_else(|| format!("`{symbol}` cannot be read."))?;
    let old = members[target].params.clone();
    if members
        .iter()
        .any(|member| member.params.len() != old.len())
    {
        return Err(format!(
            "The methods that override `{symbol}`, or that it overrides, have a different number \
             of parameters."
        ));
    }
    if old.iter().chain(&new).any(|param| param.variadic) {
        return Err(format!(
            "`{symbol}` has a variadic parameter, so its calls cannot be matched to the new list."
        ));
    }

    let sources = matched(&old, &new, values)?;
    let removed: Vec<usize> = (0..old.len())
        .filter(|at| {
            !sources
                .iter()
                .any(|source| matches!(source, Source::Kept(kept) if kept == at))
        })
        .collect();

    let mut rewrites = Vec::new();
    for member in &members {
        for at in &removed {
            let name = &member.params[*at].name;
            if member.reads.contains(name) {
                return Err(format!(
                    "The body of `{}` reads `{name}`, so the parameter cannot be removed.",
                    member.symbol
                ));
            }
        }
        rewrites.extend(declaration(
            &mut files, member, &old, &new, &sources, &removed,
        )?);
    }
    let value_of: BTreeMap<&str, &str> = values
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    for member in &members {
        let names: Vec<&str> = member
            .params
            .iter()
            .map(|param| param.name.trim_start_matches('$'))
            .collect();
        for used in index.occurrences(&member.symbol) {
            if let Some(rewrite) = call(
                &mut files,
                &used.site,
                &member.symbol,
                &names,
                &new,
                &sources,
                &removed,
                &value_of,
            )? {
                rewrites.push(rewrite);
            }
        }
    }
    nested(&mut files, &rewrites, symbol)?;
    crate::refactor::checked(documents, index, rewrites, &[])
        .map(|checked| checked.rewrites)
        .map_err(|broken| broken.refusal())
}

/// An error when a type above the class of `method` is declared outside the
/// workspace and has, or may have, a method of the same name.
fn outside(index: &SymbolIndex, method: &str) -> Result<(), Why> {
    let Some((owner, name)) = method.rsplit_once("::") else {
        return Ok(());
    };
    for above in outside_above(index, owner) {
        let known = nvs_stdlib::registry::class(&above).is_some_and(|core| !core_has(core, name));
        if !known {
            return Err(format!(
                "`{method}` may override a method of `{above}`, which is declared outside this \
                 workspace, so its parameters cannot be changed."
            ));
        }
    }
    Ok(())
}

/// The parameters `list` writes, or why it is not a parameter list.
fn parsed_list(list: &str) -> Result<Vec<Written>, Why> {
    let source = format!("<?nvs\nclass Signature {{\n    public function of({list}) {{}}\n}}\n");
    let mut map = SourceMap::new();
    let id = map.add("signature.nvs", source.clone());
    let mut diagnostics = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diagnostics);
    let invalid = || format!("`{list}` is not a parameter list.");
    if diagnostics.error_count() > 0 {
        return Err(invalid());
    }
    let Some(StmtKind::ClassDecl(decl)) = stmts.first().map(|stmt| &stmt.kind) else {
        return Err(invalid());
    };
    let Some(ClassMemberKind::Method(method)) = decl.members.first().map(|member| &member.kind)
    else {
        return Err(invalid());
    };
    let params: Vec<Written> = method
        .params
        .iter()
        .map(|param| Written::of(&source, param))
        .collect();
    for (at, param) in params.iter().enumerate() {
        if params[..at].iter().any(|before| before.name == param.name) {
            return Err(format!("The new list names `{}` twice.", param.name));
        }
    }
    Ok(params)
}

/// Where each parameter of `new` comes from, checked against `values`.
fn matched(
    old: &[Written],
    new: &[Written],
    values: &[(String, String)],
) -> Result<Vec<Source>, Why> {
    let mut sources = Vec::new();
    for param in new {
        match old.iter().position(|kept| kept.name == param.name) {
            Some(at) => {
                if old[at].inout != param.inout {
                    return Err(format!(
                        "`{}` changes whether it is `inout`, which changes what every call does.",
                        param.name
                    ));
                }
                sources.push(Source::Kept(at));
            }
            None => {
                if param.inout {
                    return Err(format!(
                        "The new parameter `{}` is `inout`, so every call would need a variable \
                         for it.",
                        param.name
                    ));
                }
                let valued = values.iter().any(|(name, _)| *name == param.name);
                if param.default.is_none() && !valued {
                    return Err(format!(
                        "The new parameter `{}` needs a default or a `--value`.",
                        param.name
                    ));
                }
                sources.push(Source::Added);
            }
        }
    }
    for (name, _) in values {
        let added = new
            .iter()
            .zip(&sources)
            .any(|(param, source)| param.name == *name && matches!(source, Source::Added));
        if !added {
            return Err(format!(
                "`{name}` is not a new parameter, so it takes no `--value`."
            ));
        }
    }
    Ok(sources)
}

/// The method `symbol` whose name starts at `name_at` of `path`.
fn member(files: &mut Files, symbol: &str, path: &Path, name_at: BytePos) -> Result<Member, Why> {
    let (source, stmts) = files.read(path)?;
    let unread = || format!("`{symbol}` cannot be read.");
    let (_, method) = method_member(stmts, name_at).ok_or_else(unread)?;
    let params = method
        .params
        .iter()
        .map(|param| Written::of(source, param))
        .collect();
    let inside = parameter_parens(source, method).ok_or_else(unread)?;
    let mut reads = BTreeSet::new();
    if let Some(body) = &method.body {
        for root in walk::of_stmts(&body.stmts) {
            for node in std::iter::once(&root).chain(root.descendants()) {
                if node.kind == "Variable"
                    && let Some(name) = source.get(node.name.unwrap_or(node.span).range())
                {
                    reads.insert(name.to_owned());
                }
            }
        }
    }
    Ok(Member {
        symbol: symbol.to_owned(),
        path: path.to_path_buf(),
        params,
        inside,
        reads,
        name_line: line_of(source, method.name.start) - 1,
    })
}

/// The bytes between the parentheses of `method`'s parameter list.
fn parameter_parens(source: &str, method: &MethodMember) -> Option<(BytePos, BytePos)> {
    let after = method
        .params
        .last()
        .map_or(method.name.end, |last| last.span.end);
    parens(source, method.name.end, after)
}

/// The bytes between the first `(` after `name_end` and the first `)` after
/// `after`.
fn parens(source: &str, name_end: BytePos, after: BytePos) -> Option<(BytePos, BytePos)> {
    let open = name_end as usize + source.get(name_end as usize..)?.find('(')? + 1;
    let from = (after as usize).max(open);
    let close = from + source.get(from..)?.find(')')?;
    Some((byte(open), byte(close)))
}

/// The edits to `member`'s declaration: its parameter list and the `@param`
/// lines of the parameters it loses.
fn declaration(
    files: &mut Files,
    member: &Member,
    old: &[Written],
    new: &[Written],
    sources: &[Source],
    removed: &[usize],
) -> Result<Vec<Rewrite>, Why> {
    let (source, _) = files.read(&member.path)?;
    let items: Vec<String> = new
        .iter()
        .zip(sources)
        .map(|(param, origin)| match origin {
            Source::Added => param.whole.clone(),
            Source::Kept(at) => {
                let own = &member.params[*at];
                let ty = if param.ty == old[*at].ty {
                    own.ty.as_deref()
                } else {
                    param.ty.as_deref()
                };
                let default = if param.default == old[*at].default {
                    own.default.as_deref()
                } else {
                    param.default.as_deref()
                };
                if param.ty == old[*at].ty && param.default == old[*at].default {
                    own.whole.clone()
                } else {
                    own.rendered(ty, default)
                }
            }
        })
        .collect();
    let (start, end) = member.inside;
    let original = source.get(start as usize..end as usize).unwrap_or_default();
    let mut rewrites = Vec::new();
    let text = listed(original, &items);
    if text != original {
        rewrites.push(rewrite(&member.path, start, end, text));
    }
    let gone: Vec<&str> = removed
        .iter()
        .map(|at| member.params[*at].name.as_str())
        .collect();
    for (from, to) in param_lines(source, member.name_line, &gone) {
        rewrites.push(rewrite(&member.path, byte(from), byte(to), String::new()));
    }
    Ok(rewrites)
}

/// The byte ranges of the `@param` lines in the `///` run above the line
/// `name_line` of `source` that name one of `gone`, each with its line break.
fn param_lines(source: &str, name_line: usize, gone: &[&str]) -> Vec<(usize, usize)> {
    let mut starts = vec![0];
    starts.extend(source.match_indices('\n').map(|(at, _)| at + 1));
    let mut found = Vec::new();
    let mut line = name_line;
    while line > 0 {
        line -= 1;
        let (Some(&from), Some(&to)) = (starts.get(line), starts.get(line + 1)) else {
            break;
        };
        let text = source.get(from..to).unwrap_or_default().trim();
        if text.starts_with("#[") {
            continue;
        }
        let Some(doc) = text.strip_prefix("///") else {
            break;
        };
        let mut words = doc.split_whitespace();
        if words.next() == Some("@param") && words.take(2).any(|word| gone.contains(&word)) {
            found.push((from, to));
        }
    }
    found
}

/// `items` joined the way `original`, the text between a list's parentheses,
/// was laid out: one per line with its indentation and trailing comma where
/// it spanned lines, and with `, ` otherwise.
fn listed(original: &str, items: &[String]) -> String {
    if !original.contains('\n') || items.is_empty() {
        return items.join(", ");
    }
    let indent: String = original
        .trim_start_matches(['\r', '\n'])
        .chars()
        .take_while(|ch| *ch == ' ' || *ch == '\t')
        .collect();
    let closing: String = original
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .take_while(|ch| *ch == ' ' || *ch == '\t')
        .collect();
    let trailing = original.trim_end().ends_with(',');
    let mut out = String::from("\n");
    for (at, item) in items.iter().enumerate() {
        out.push_str(&indent);
        out.push_str(item);
        if at + 1 < items.len() || trailing {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&closing);
    out
}

/// One argument of a call.
struct Given {
    /// What is written, `inout` included and the name left out.
    text: String,
    /// Where it was written among the call's arguments.
    order: usize,
    /// Whether evaluating it may run code.
    runs: bool,
}

/// The edit to the arguments of the call of `symbol` whose name is `site`,
/// or `None` where they do not change. `names` are the called method's
/// parameter names, without `$`.
#[allow(clippy::too_many_arguments)]
fn call(
    files: &mut Files,
    site: &Site,
    symbol: &str,
    names: &[&str],
    new: &[Written],
    sources: &[Source],
    removed: &[usize],
    values: &BTreeMap<&str, &str>,
) -> Result<Option<Rewrite>, Why> {
    let (source, stmts) = files.read(&site.path)?;
    let line = line_of(source, site.start);
    let file = site.path.display();
    let unread = || format!("The call of `{symbol}` on line {line} of `{file}` cannot be read.");
    let Some(CallArgs::List(args)) = call_in_stmts(stmts, site.start, site.end) else {
        return Err(unread());
    };
    let after = args.last().map_or(site.end, |last| last.span.end);
    let (start, end) = parens(source, site.end, after).ok_or_else(unread)?;
    let roots = walk::of_stmts(stmts);
    let mut given: Vec<Option<Given>> = (0..names.len()).map(|_| None).collect();
    let mut position = 0;
    for (order, arg) in args.iter().enumerate() {
        if arg.spread {
            return Err(format!(
                "The call of `{symbol}` on line {line} of `{file}` spreads its arguments."
            ));
        }
        let at = match arg.name {
            Some(name) => {
                let label = source.get(name.range()).unwrap_or_default();
                names
                    .iter()
                    .position(|name| *name == label)
                    .ok_or_else(unread)?
            }
            None => {
                position += 1;
                position - 1
            }
        };
        // A named argument's text starts after its `name:`, so `inout` stays.
        let from = match arg.name {
            Some(name) => source
                .get(name.end as usize..)
                .and_then(|rest| rest.find(':'))
                .map_or(arg.value.span.start as usize, |colon| {
                    name.end as usize + colon + 1
                }),
            None => arg.span.start as usize,
        };
        let text = source
            .get(from..arg.span.end as usize)
            .unwrap_or_default()
            .trim_start()
            .to_owned();
        let runs = path_to(&roots, &|node: &Node| node.span == arg.value.span)
            .and_then(|path| path.last().copied())
            .is_none_or(runs_code);
        let slot = given.get_mut(at).ok_or_else(unread)?;
        *slot = Some(Given { text, order, runs });
    }
    for at in removed {
        if let Some(gone) = &given[*at]
            && gone.runs
        {
            return Err(format!(
                "The argument `{}` on line {line} of `{file}` may run code, so it cannot be \
                 removed.",
                gone.text
            ));
        }
    }

    let mut items = Vec::new();
    let mut gap = false;
    let mut ran: Vec<(usize, &str)> = Vec::new();
    for (param, origin) in new.iter().zip(sources) {
        let (name, value) = match origin {
            Source::Kept(at) => (names[*at], given[*at].as_ref()),
            Source::Added => {
                let name = param.name.trim_start_matches('$');
                let value = if param.default.is_some() {
                    None
                } else {
                    values.get(param.name.as_str()).copied()
                };
                if let Some(value) = value {
                    if gap {
                        items.push(format!("{name}: {value}"));
                    } else {
                        items.push(value.to_owned());
                    }
                } else {
                    gap = true;
                }
                continue;
            }
        };
        let Some(value) = value else {
            gap = true;
            continue;
        };
        if value.runs {
            ran.push((value.order, &value.text));
        }
        if gap || args[value.order].name.is_some() {
            gap = true;
            items.push(format!("{name}: {}", value.text));
        } else {
            items.push(value.text.clone());
        }
    }
    for pair in ran.windows(2) {
        if pair[0].0 > pair[1].0 {
            return Err(format!(
                "The arguments `{}` and `{}` on line {line} of `{file}` may both run code, and \
                 the new order would run them in another order.",
                pair[1].1, pair[0].1
            ));
        }
    }
    let original = source.get(start as usize..end as usize).unwrap_or_default();
    let text = listed(original, &items);
    Ok((text != original).then(|| rewrite(&site.path, start, end, text)))
}

/// An error when one edit of `rewrites` lies inside another, as a call does
/// in the arguments of another call of the family.
fn nested(files: &mut Files, rewrites: &[Rewrite], symbol: &str) -> Result<(), Why> {
    for outer in rewrites {
        for inner in rewrites {
            if !std::ptr::eq(outer, inner)
                && outer.site.path == inner.site.path
                && outer.site.start <= inner.site.start
                && inner.site.end <= outer.site.end
                && outer.site.start < outer.site.end
            {
                let (source, _) = files.read(&inner.site.path)?;
                return Err(format!(
                    "Two calls of `{symbol}` are nested on line {} of `{}`. Change the inner \
                     call first.",
                    line_of(source, inner.site.start),
                    inner.site.path.display()
                ));
            }
        }
    }
    Ok(())
}

/// The edit that replaces `start..end` of `path` with `text`.
fn rewrite(path: &Path, start: BytePos, end: BytePos, text: String) -> Rewrite {
    Rewrite {
        site: Site {
            path: path.to_path_buf(),
            start,
            end,
        },
        text,
    }
}

/// A byte offset as a position.
fn byte(offset: usize) -> BytePos {
    BytePos::try_from(offset).unwrap_or(BytePos::MAX)
}
