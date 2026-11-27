//! *Move member*: a member of one class moved into another, with every use
//! of it written again (`rule:tooling/nvs-code-answers-about-the-project`).
//!
//! [`moved_member`] takes the member as `Class::member` and the class it
//! moves to. Two kinds of move are allowed:
//!
//! - **A static member** — a static method, a static property, a constant, or
//!   a type alias or an enum the class declares — moves to any class of the
//!   project. Each `Old::member` becomes `New::member`, with a `use` line
//!   where the file needs one. Inside the moved code, each `self::` and
//!   `static::` that reads another member of the old class becomes `Old::`.
//! - **An instance member** moves only to a class above its class (a pull
//!   up) or below it (a push down). Its uses do not change, since they are
//!   written on an object.
//!
//! The moved code keeps its `///` and `#[` lines. Each type it writes by a
//! short name gets a `use` line in the new file where it needs one.
//!
//! # Decision: the check decides what still compiles
//!
//! Most of what could break is left to `crate::refactor::checked`: a pull
//! up whose body reads what the class above does not have, a push down that
//! something still calls through the class above or a sibling, and a moved
//! static body that reads a private member of its old class. Each is an
//! error after the edit, so the edit is refused with that error. This
//! module refuses only what the checker cannot see:
//!
//! - a private or protected static member that its old class still uses,
//!   with "make it public first", since that sentence helps more than the
//!   checker's;
//! - a property whose name a property callback compares as a string
//!   (`rule:classes/a-compared-property-name-is-a-use`), since the
//!   comparison would silently stop matching;
//! - moved code that writes `parent`, or `self` or `static` other than in
//!   front of `::`, since each would then name another class;
//! - a name the new class already has, a member that shares its
//!   declaration with another, and an enum case.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. One parse of each file
//! with a use of the member, one tokenizing of the old file, then the check
//! that `crate::refactor::checked` describes. Nothing is kept.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nvs_diagnostics::{BytePos, Diagnostics, SourceId, SourceMap};
use nvs_hir::QName;
use nvs_syntax::ast::{ClassDecl, ClassMember, ClassMemberKind, Stmt, StmtKind};
use nvs_syntax::{Token, parse_file, tokenize};

use crate::document::Documents;
use crate::index::{DeclKind, Site, SymbolIndex};
use crate::rename::Rewrite;

/// Why a move is refused, written for the person who asked.
type Why = String;

/// The files the move reads, each parsed once. Extract interface reads
/// the class's file through it too (`crate::extract_interface`).
pub(crate) struct Files {
    map: SourceMap,
    loaded: BTreeMap<PathBuf, Option<(SourceId, Vec<Stmt>)>>,
}

impl Files {
    /// No file read yet, over the open buffers of `documents`.
    pub(crate) fn new(documents: &Documents) -> Self {
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

    /// The tokens of `path`, which [`Self::read`] has read.
    fn tokens(&mut self, path: &Path) -> Result<Vec<Token>, Why> {
        self.read(path)?;
        match self.loaded.get(path) {
            Some(Some((id, _))) => Ok(tokenize(self.map.file(*id), &mut Diagnostics::new())),
            _ => Err(format!("`{}` cannot be read.", path.display())),
        }
    }

    /// Where a `use` line goes for a name written at `at` of `path`.
    fn import_site(&mut self, path: &Path, at: BytePos) -> Option<nvs_hir::ImportSite> {
        self.read(path).ok()?;
        let Some(Some((id, stmts))) = self.loaded.get(path) else {
            return None;
        };
        nvs_hir::import_site(stmts, self.map.file(*id), at)
    }
}

/// The edits that move the member `from`, written `Class::member`, into the
/// class `to`, and write every use of it again. Every edit is checked before
/// it is returned.
///
/// # Errors
///
/// Returns why the member is not moved.
pub fn moved_member(
    documents: &Documents,
    index: &SymbolIndex,
    from: &str,
    to: &str,
) -> Result<Vec<Rewrite>, Why> {
    let (old, name) = from
        .rsplit_once("::")
        .ok_or_else(|| format!("`{from}` is not written as `Class::member`."))?;
    let symbol = if index.declaration(from).is_some() || name.starts_with('$') {
        from.to_owned()
    } else {
        format!("{old}::${name}")
    };
    let declared = index
        .declaration(&symbol)
        .ok_or_else(|| format!("`{from}` is not a member of a class of this project."))?;
    let kind = declared.kind;
    if kind == DeclKind::EnumCase {
        return Err(format!(
            "`{from}` is an enum case. An enum case belongs to its enum and cannot be moved."
        ));
    }
    if !matches!(
        kind,
        DeclKind::Method
            | DeclKind::Property
            | DeclKind::Const
            | DeclKind::TypeAlias
            | DeclKind::Enum
    ) {
        return Err(format!(
            "`{from}` is not a member of a class of this project."
        ));
    }
    let class_of = |class: &str| {
        index
            .declaration(class)
            .filter(|declared| declared.kind == DeclKind::Class)
    };
    let old_class = class_of(old)
        .ok_or_else(|| format!("`{old}` is not a class of this project."))?
        .clone();
    let new_class = class_of(to)
        .ok_or_else(|| format!("`{to}` is not a class of this project."))?
        .clone();
    if old_class.symbol == new_class.symbol {
        return Err(format!("`{from}` is already a member of `{to}`."));
    }
    let member_name = symbol.rsplit_once("::").map_or(name, |(_, it)| it);
    let target = format!("{}::{member_name}", new_class.symbol);
    if let Some(clash) = index.declaration(&target) {
        return Err(format!(
            "`{to}` already has {} named `{member_name}`.",
            clash.kind.describe()
        ));
    }

    let mut files = Files::new(documents);
    let old_path = old_class.site.path.clone();
    let new_path = new_class.site.path.clone();
    let (text, stmts) = files.read(&old_path)?;
    let text = text.to_owned();
    let unread = || format!("`{from}` cannot be read.");
    let decl = class_named(stmts, old_class.site.start).ok_or_else(unread)?;
    let member = decl
        .members
        .iter()
        .find(|member| {
            member.span.start <= declared.site.start && declared.site.end <= member.span.end
        })
        .ok_or_else(|| {
            format!("`{from}` cannot be moved. Only a member written in the class body can move.")
        })?
        .clone();
    if let Some(other) = index.declarations_in(&old_path).iter().find(|other| {
        other.site != declared.site
            && member.span.start <= other.site.start
            && other.site.end <= member.span.end
            && !other.symbol.starts_with(&format!("{symbol}::"))
    }) {
        return Err(format!(
            "`{from}` is declared together with `{}`, so it cannot be moved alone.",
            other.symbol
        ));
    }
    let head = text
        .get(member.span.start as usize..declared.site.start as usize)
        .unwrap_or_default();
    let is_static = !matches!(kind, DeclKind::Method | DeclKind::Property)
        || head.split_whitespace().any(|word| word == "static");
    if !is_static && !related(index, &old_class.symbol, &new_class.symbol) {
        return Err(format!(
            "`{from}` is an instance member, so it can only move to a class above or below \
             `{old}`."
        ));
    }

    let (from_at, to_at) = lines_of(&text, member.span.start as usize, member.span.end as usize);
    let removed =
        crate::delete::removed(&text, member.span.start as usize, member.span.end as usize);
    let inside = |site: &Site| {
        site.path == old_path && from_at <= site.start as usize && (site.end as usize) <= to_at
    };

    let occurrences: Vec<Site> = index
        .occurrences(&symbol)
        .into_iter()
        .map(|used| used.site.clone())
        .collect();
    for site in &occurrences {
        if quoted(&mut files, site)? {
            return Err(format!(
                "A property callback compares the name of `{from}` as a string, so the property \
                 cannot be moved."
            ));
        }
    }
    if is_static
        && declared.visibility != nvs_syntax::ast::Visibility::Public
        && occurrences.iter().any(|site| !inside(site))
    {
        return Err(format!(
            "`{from}` is not public, and `{old}` still uses it. Make it public first, then move \
             it."
        ));
    }

    let insert = insertion(&mut files, &new_class.site, &member)?;
    let mut heads = Vec::new();
    let mut rewrites = Vec::new();
    // Edits inside the moved code, relative to `from_at`.
    let mut local: Vec<(usize, usize, String)> = Vec::new();

    let sites: BTreeSet<Site> = occurrences
        .iter()
        .cloned()
        .chain(
            index
                .mentions_of(&symbol)
                .into_iter()
                .map(|mentioned| mentioned.site.clone()),
        )
        .collect();
    for site in &sites {
        let (qualifier, bare_at) = qualifier_of(&mut files, site)?;
        if inside(site) {
            match qualifier {
                Some((start, end, written)) if !is_keyword(&written) && is_static => {
                    let spelled = spell(
                        &mut files,
                        index,
                        &new_path,
                        insert.at,
                        &new_class.symbol,
                        &mut heads,
                    )?;
                    local.push((start - from_at, end - from_at, spelled));
                }
                _ => {}
            }
            continue;
        }
        // An instance member's uses are written on an object and stay. Only a
        // doc comment's `Old::member` names the class.
        let mentioned = index.mentions_of(&symbol).iter().any(|it| it.site == *site);
        if !is_static && (!mentioned || qualifier.is_none()) {
            continue;
        }
        // `self::`, `static::` and `parent::` outside the moved code named the
        // old class, so each becomes the new class too.
        let at = BytePos::try_from(bare_at).unwrap_or(site.start);
        let spelled = spell(
            &mut files,
            index,
            &site.path,
            at,
            &new_class.symbol,
            &mut heads,
        )?;
        match qualifier {
            Some((start, end, _)) => rewrites.push(rewrite(&site.path, start, end, spelled)),
            None => rewrites.push(rewrite(
                &site.path,
                bare_at,
                bare_at,
                format!("{spelled}::"),
            )),
        }
    }

    // The moved code's own `self`, `static`, `parent` and short type names.
    let tokens = files.tokens(&old_path)?;
    let (_, old_stmts) = files.read(&old_path)?;
    let old_space = namespace_at(&text, old_stmts, member.span.start);
    let old_uses = uses_at(&text, old_stmts, member.span.start);
    let in_range: Vec<&Token> = tokens
        .iter()
        .filter(|token| from_at <= token.span.start as usize && (token.span.end as usize) <= to_at)
        .collect();
    for (at, token) in in_range.iter().enumerate() {
        let word = text.get(token.span.range()).unwrap_or_default();
        let next = in_range
            .get(at + 1)
            .and_then(|next| text.get(next.span.range()))
            .unwrap_or_default();
        let before = at
            .checked_sub(1)
            .and_then(|before| in_range.get(before))
            .and_then(|before| text.get(before.span.range()))
            .unwrap_or_default();
        let (start, end) = (token.span.start as usize, token.span.end as usize);
        let header = token.span.start < declared.site.start;
        if !header && word.eq_ignore_ascii_case("parent") && before != "->" && before != "?->" {
            return Err(format!(
                "The code of `{from}` writes `parent`, which would name another class after the \
                 move."
            ));
        }
        if !header && (word.eq_ignore_ascii_case("self") || word.eq_ignore_ascii_case("static")) {
            if before == "->" || before == "?->" || before == "::" {
                continue;
            }
            if next != "::" {
                return Err(format!(
                    "The code of `{from}` writes `{word}`, which would name another class after \
                     the move."
                ));
            }
            let handled = local.iter().any(|(s, _, _)| *s == start - from_at);
            let own = sites.iter().any(|site| {
                inside(site) && qualifier_end(&text, site.start as usize).is_some_and(|q| q == end)
            });
            if handled || own {
                continue;
            }
            if word.eq_ignore_ascii_case("static") && !is_static {
                continue;
            }
            let spelled = spell(
                &mut files,
                index,
                &new_path,
                insert.at,
                &old_class.symbol,
                &mut heads,
            )?;
            local.push((start - from_at, end - from_at, spelled));
            continue;
        }
        let typed = matches!(token.kind, nvs_syntax::TokenKind::Ident)
            && !matches!(before, "->" | "?->" | "::" | "\\" | "function" | "const")
            && next != "\\"
            && next != ":"
            && (next != "(" || before == "new")
            && start != declared.site.start as usize;
        if !typed || new_path == old_path && old_space == namespace_of(&new_class.symbol) {
            continue;
        }
        let Some(full) = resolved(index, &old_uses, &old_space, word) else {
            continue;
        };
        let spelled = spell(&mut files, index, &new_path, insert.at, &full, &mut heads)?;
        if spelled != word {
            local.push((start - from_at, end - from_at, spelled));
        }
    }

    let mut moved = text.get(from_at..to_at).unwrap_or_default().to_owned();
    local.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    local.dedup_by_key(|(start, _, _)| *start);
    for (start, end, spelled) in &local {
        moved.replace_range(*start..*end, spelled);
    }
    let moved = reindented(&moved, &indent_of(&text, from_at), &insert.indent);
    rewrites.push(rewrite(&old_path, removed.0, removed.1, String::new()));
    rewrites.push(rewrite(
        &new_path,
        insert.at as usize,
        insert.at as usize,
        format!("{}{moved}{}", insert.lead, insert.trail),
    ));
    rewrites.extend(crate::rename::sorted_heads(documents, heads)?);
    rewrites.sort();
    rewrites.dedup();
    crate::refactor::checked(documents, index, rewrites, &[])
        .map(|checked| checked.rewrites)
        .map_err(|broken| broken.refusal())
}

/// A rewrite of `start..end` of `path` with `text`.
fn rewrite(path: &Path, start: usize, end: usize, text: String) -> Rewrite {
    let at = |offset: usize| BytePos::try_from(offset).unwrap_or(BytePos::MAX);
    Rewrite {
        site: Site {
            path: path.to_path_buf(),
            start: at(start),
            end: at(end),
        },
        text,
    }
}

/// Whether `word` is `self`, `static` or `parent`.
fn is_keyword(word: &str) -> bool {
    ["self", "static", "parent"]
        .iter()
        .any(|it| it.eq_ignore_ascii_case(word))
}

/// Whether one of `a` and `b` is above the other.
fn related(index: &SymbolIndex, a: &str, b: &str) -> bool {
    above(index, a).contains(b) || above(index, b).contains(a)
}

/// Every type above `class` in the index, each walked once.
fn above(index: &SymbolIndex, class: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![class.to_owned()];
    while let Some(next) = todo.pop() {
        for up in index.supertypes(&next) {
            if seen.insert(up.symbol.clone()) {
                todo.push(up.symbol.clone());
            }
        }
    }
    seen
}

/// The class whose name starts at `name_at`, at the top level or in a
/// namespace block.
fn class_named(stmts: &[Stmt], name_at: BytePos) -> Option<&ClassDecl> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::ClassDecl(decl) if decl.name.span.start == name_at => Some(decl),
        StmtKind::NamespaceDecl(space) => space
            .body
            .as_ref()
            .and_then(|block| class_named(&block.stmts, name_at)),
        _ => None,
    })
}

/// The whole lines of a member written from `start` to `end`, with the `///`
/// and `#[` lines above it.
fn lines_of(text: &str, start: usize, end: usize) -> (usize, usize) {
    let line_start = |at: usize| text[..at].rfind('\n').map_or(0, |found| found + 1);
    let mut from = line_start(start);
    while from > 0 {
        let up = line_start(from - 1);
        let line = text[up..from].trim();
        if line.starts_with("///") || line.starts_with("#[") {
            from = up;
        } else {
            break;
        }
    }
    let to = text[end..]
        .find('\n')
        .map_or(text.len(), |found| end + found + 1);
    (from, to)
}

/// The spaces and tabs that start the line `at` is on.
fn indent_of(text: &str, at: usize) -> String {
    let start = text[..at].rfind('\n').map_or(0, |found| found + 1);
    text[start..]
        .chars()
        .take_while(|ch| *ch == ' ' || *ch == '\t')
        .collect()
}

/// `text` with `old` replaced by `new` at the start of each line.
fn reindented(text: &str, old: &str, new: &str) -> String {
    if old == new {
        return text.to_owned();
    }
    text.split_inclusive('\n')
        .map(|line| {
            line.strip_prefix(old)
                .map_or_else(|| line.to_owned(), |rest| format!("{new}{rest}"))
        })
        .collect()
}

/// Whether the text before `site` is a quote, which is how a compared
/// property name is written.
fn quoted(files: &mut Files, site: &Site) -> Result<bool, Why> {
    let (text, _) = files.read(&site.path)?;
    Ok(site
        .start
        .checked_sub(1)
        .and_then(|at| text.as_bytes().get(at as usize))
        .is_some_and(|byte| matches!(byte, b'\'' | b'"')))
}

/// The end of the class written before `::` in front of a member name that
/// starts at `start`: the name may have a `$`.
fn qualifier_end(text: &str, start: usize) -> Option<usize> {
    let mut at = start;
    if text.as_bytes().get(at) != Some(&b'$') && at > 0 && text.as_bytes()[at - 1] == b'$' {
        at -= 1;
    }
    text[..at].ends_with("::").then(|| at - 2)
}

/// The class written before `::` in front of a member name: its first byte,
/// its end and its text.
type Qualifier = (usize, usize, String);

/// The class written before `::` at `site`, and where the site starts with
/// that class.
fn qualifier_of(files: &mut Files, site: &Site) -> Result<(Option<Qualifier>, usize), Why> {
    let (text, _) = files.read(&site.path)?;
    let start = site.start as usize;
    let Some(end) = qualifier_end(text, start) else {
        return Ok((None, start));
    };
    let bytes = text.as_bytes();
    let mut first = end;
    while first > 0
        && bytes
            .get(first - 1)
            .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'\\'))
    {
        first -= 1;
    }
    if first == end {
        return Err(format!(
            "A use of the member at byte {start} of `{}` is not written on a class name.",
            site.path.display()
        ));
    }
    Ok((Some((first, end, text[first..end].to_owned())), first))
}

/// Where the moved member goes in the class declared at `class`.
struct Insertion {
    /// The offset it is inserted at.
    at: BytePos,
    /// What is written before it.
    lead: String,
    /// What is written after it.
    trail: String,
    /// The indentation of the class's members.
    indent: String,
}

/// The place in the class declared at `class` for `moved`: after the last
/// member of its group, constants first, then properties, then methods.
fn insertion(files: &mut Files, class: &Site, moved: &ClassMember) -> Result<Insertion, Why> {
    let (text, stmts) = files.read(&class.path)?;
    let decl = class_named(stmts, class.start)
        .ok_or_else(|| format!("The class at `{}` cannot be read.", class.path.display()))?;
    let rank = |member: &ClassMember| match member.kind {
        ClassMemberKind::Property(_) => 1,
        ClassMemberKind::Method(_) | ClassMemberKind::Error => 2,
        _ => 0,
    };
    let at = |offset: usize| BytePos::try_from(offset).unwrap_or(BytePos::MAX);
    let wanted = rank(moved);
    if let Some(last) = decl
        .members
        .iter()
        .rev()
        .find(|member| rank(member) <= wanted)
    {
        let (_, end) = lines_of(text, last.span.start as usize, last.span.end as usize);
        let more = decl
            .members
            .iter()
            .any(|member| member.span.start > last.span.end);
        return Ok(Insertion {
            at: at(end),
            lead: "\n".to_owned(),
            trail: if more { "\n".to_owned() } else { String::new() },
            indent: indent_of(text, last.span.start as usize),
        });
    }
    if let Some(first) = decl.members.first() {
        let (start, _) = lines_of(text, first.span.start as usize, first.span.end as usize);
        return Ok(Insertion {
            at: at(start),
            lead: String::new(),
            trail: "\n".to_owned(),
            indent: indent_of(text, first.span.start as usize),
        });
    }
    let close = (decl.span.end as usize).saturating_sub(1);
    let open = text[decl.name.span.end as usize..close]
        .find('{')
        .map(|found| decl.name.span.end as usize + found + 1)
        .ok_or_else(|| format!("The class at `{}` cannot be read.", class.path.display()))?;
    let indent = indent_of(text, decl.span.start as usize);
    let inner = format!("{indent}    ");
    if text[open..close].trim().is_empty() && text[open..close].contains('\n') {
        let line = text[..close].rfind('\n').map_or(open, |found| found + 1);
        return Ok(Insertion {
            at: at(line),
            lead: String::new(),
            trail: String::new(),
            indent: inner,
        });
    }
    Ok(Insertion {
        at: at(open),
        lead: "\n".to_owned(),
        trail: indent,
        indent: inner,
    })
}

/// The namespace of `symbol`, a fully-qualified name, and `""` for the
/// global namespace.
fn namespace_of(symbol: &str) -> String {
    symbol
        .rsplit_once('\\')
        .map_or_else(String::new, |(space, _)| space.to_owned())
}

/// The namespace in force at `at`, and `""` for the global namespace.
fn namespace_at(text: &str, stmts: &[Stmt], at: BytePos) -> String {
    let mut found = String::new();
    for stmt in stmts {
        if let StmtKind::NamespaceDecl(space) = &stmt.kind {
            let name = space
                .name
                .as_ref()
                .and_then(|name| text.get(name.span.range()))
                .unwrap_or_default()
                .trim_start_matches('\\')
                .to_owned();
            match &space.body {
                Some(block) if block.span.start <= at && at < block.span.end => return name,
                Some(_) => {}
                None if stmt.span.start <= at => found = name,
                None => {}
            }
        }
    }
    found
}

/// The names the `use` lines in force at `at` import, without a leading `\`.
fn uses_at(text: &str, stmts: &[Stmt], at: BytePos) -> Vec<String> {
    let mut found = Vec::new();
    let mut read = |stmts: &[Stmt]| {
        for stmt in stmts {
            if let StmtKind::UseDecl(used) = &stmt.kind {
                found.push(
                    text.get(used.path.span.range())
                        .unwrap_or_default()
                        .trim_start_matches('\\')
                        .to_owned(),
                );
            }
        }
    };
    read(stmts);
    for stmt in stmts {
        if let StmtKind::NamespaceDecl(space) = &stmt.kind
            && let Some(block) = &space.body
            && block.span.start <= at
            && at < block.span.end
        {
            read(&block.stmts);
        }
    }
    found
}

/// The last segment of a qualified name.
fn short_of(full: &str) -> &str {
    full.rsplit_once('\\').map_or(full, |(_, short)| short)
}

/// The type `word` names where `uses` are in force in `space`, when it is a
/// `use` line's name or a type the index declares in `space`.
fn resolved(index: &SymbolIndex, uses: &[String], space: &str, word: &str) -> Option<String> {
    if let Some(used) = uses
        .iter()
        .find(|used| short_of(used).eq_ignore_ascii_case(word))
    {
        return Some(used.clone());
    }
    let full = if space.is_empty() {
        word.to_owned()
    } else {
        format!("{space}\\{word}")
    };
    index
        .declaration(&full)
        .filter(|declared| {
            matches!(
                declared.kind,
                DeclKind::Class
                    | DeclKind::Interface
                    | DeclKind::Enum
                    | DeclKind::TypeAlias
                    | DeclKind::Attribute
            )
        })
        .map(|declared| declared.symbol.clone())
}

/// How the file `path` writes the type `full` at `at`: its short name where
/// its namespace or a `use` line reaches it, else its short name with a new
/// `use` line pushed to `heads`, else its qualified name.
pub(crate) fn spell(
    files: &mut Files,
    index: &SymbolIndex,
    path: &Path,
    at: BytePos,
    full: &str,
    heads: &mut Vec<Rewrite>,
) -> Result<String, Why> {
    let (text, stmts) = files.read(path)?;
    let space = namespace_at(text, stmts, at);
    let uses = uses_at(text, stmts, at);
    let short = short_of(full).to_owned();
    if space.eq_ignore_ascii_case(&namespace_of(full))
        || uses.iter().any(|used| used.eq_ignore_ascii_case(full))
    {
        return Ok(short);
    }
    let local = if space.is_empty() {
        short.clone()
    } else {
        format!("{space}\\{short}")
    };
    let taken = uses
        .iter()
        .any(|used| short_of(used).eq_ignore_ascii_case(&short))
        || index.declaration(&local).is_some();
    if !taken && let Some(site) = files.import_site(path, at) {
        heads.push(rewrite(
            path,
            site.at as usize,
            site.at as usize,
            site.use_line(&QName::parse(full)),
        ));
        return Ok(short);
    }
    if full.contains('\\') {
        Ok(full.to_owned())
    } else {
        Err(format!(
            "`{}` cannot name `{full}` without a second name for it.",
            path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_of_takes_the_doc_comment_and_attributes() {
        let text = "class A {\n    /// Doc.\n    #[Mark]\n    public const int X = 1;\n}\n";
        let start = text.find("public").expect("a member");
        let end = text.find(";").expect("an end") + 1;
        let (from, to) = lines_of(text, start, end);
        assert_eq!(
            &text[from..to],
            "    /// Doc.\n    #[Mark]\n    public const int X = 1;\n"
        );
    }

    #[test]
    fn reindented_replaces_the_indentation_of_each_line() {
        assert_eq!(reindented("  a\n    b\n", "  ", "    "), "    a\n      b\n");
    }

    #[test]
    fn qualifier_end_reads_a_static_property() {
        let text = "Cart::$count";
        assert_eq!(qualifier_end(text, 7), Some(4));
        assert_eq!(qualifier_end(text, 6), Some(4));
        assert_eq!(qualifier_end("count", 0), None);
    }
}
