//! `nv-scan --items <root>`: every item of each Rust file, so a coverage record maps to the item it ran
//! in and a change maps to the items it touched.
//!
//! Input on stdin: one path per line, relative to `<root>` and `/`-separated. Output on stdout: one
//! JSON object per path, in input order:
//!
//! ```text
//! {"file": "crates/a/src/b.rs", "parsed": true, "raw": "<digest of the bytes>", "items": [
//!   {"id": "Handle::wait", "kind": "fn", "start": 120, "end": 161, "test": false,
//!    "digest": "<32 hex>", "bare": "<32 hex>", "cardTypes": ["MethodDoc"],
//!    "refs": ["Fault", "child"], "defines": ["wait"],
//!    "parent": "Handle::{impl}", "includes": ["crates/a/src/data.txt"],
//!    "class": ["Core\\Process"], "rows": [{"classes": ["Core\\Math"], "digest": "<32 hex>"}],
//!    "cards": ["Core\\Math"]}]}
//! ```
//!
//! - `id` is stable while the file's lines move: the inline module path, then the impl's self type
//!   (`<Type as Trait>` for a trait impl), then the name. An impl's or trait's own tokens, less its
//!   methods, are the item `<base>::{impl}` or `<base>::{trait}`. A `macro_rules!` definition is
//!   `name!`. A macro invocation is named after the first `fn` its body spells, else the first
//!   identifier it defines, as `macro!{name}`. A `use` is `use:` and the names it binds. An id that
//!   is still not unique gets `#2`, `#3` and so on, in source order.
//! - `kind` is `fn`, `const`, `static`, `struct`, `enum`, `union`, `type`, `trait-rest`,
//!   `impl-rest`, `macro-rules`, `macro`, `use`, `extern` or `verbatim`. A `fn` is anything that
//!   runs: a free function, a method, a trait's default method, or a `fn` spelled inside a macro
//!   invocation's body, which is then an item of its own with the invocation as its `parent`.
//! - `start` and `end` are the item's first and last line, doc comments included.
//! - `test` says the item is compiled only with `cfg(test)`, or is a `#[test]`.
//! - `digest` covers the item's tokens with every doc attribute taken out, so editing, adding or
//!   removing a doc comment is no change, and neither is layout or a plain comment. Each file an
//!   `include_str!`, `include_bytes!` or `include!` with a literal path embeds is folded in by its
//!   bytes, and is listed in `includes` relative to `<root>`; a missing one folds in as missing.
//! - `bare` is the same digest with the item's links to reference cards taken out, and is set only
//!   when it differs from `digest`. For a `const` or `static` that is not itself a card, a
//!   `doc: Some(&CARD)` field naming a card of this file reads as `doc: None`; for a `use`, the card
//!   types it imports are taken out of it, and `cardTypes` lists the names only those bind. Two
//!   versions of an item whose `bare` (or, lacking one, `digest`) is the same differ in their cards
//!   alone, which no program reads.
//! - `refs` is every identifier the item's tokens name, keywords and primitive types left out, and
//!   `defines` every name the item binds for others to name: a function's, a type's, a const's, the
//!   names a `use` binds (`*` for a glob), and for a macro invocation the identifiers its body spells
//!   in a defining position (`nvs_core_math_sqrt` in `unary_float! { nvs_core_math_sqrt, .. }`),
//!   which is how a function a macro generated maps back to the invocation that named it.
//! - `scope` says who may name a free item of the file (a `fn`, `const`, `static`, `struct`, `enum`,
//!   `union`, `type` or `trait`) when that is narrower than `pub`: `private` for no visibility or
//!   `pub(self)`, which is its own module and the modules inside it, and `crate` for `pub(crate)`,
//!   `pub(super)` or `pub(in ..)`, which is at most its own package. It is left out for a `pub` item
//!   and for every other kind of item, and a reader takes a missing `scope` as `pub`.
//! - `class` is set on a `const` or `static` whose value names a `Core` class: a struct literal whose
//!   `name` field is a class name, or a class name itself, directly or through another const, in
//!   this file or another file of the batch. `rows` is set on a `const` or `static` whose value is
//!   an array literal with at least one element that names a class, and lists every element in
//!   order with the classes it names and its own digest. `cards` is set on a const of a reference
//!   card type and lists the classes whose class items in the same file reach it, directly or
//!   through another card.
//!
//! A file that is missing or does not parse has `"parsed": false` and no items, and its reader then
//! takes it as a whole.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::{self, BufRead, Write};
use std::path::{Component, Path};

use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream, TokenTree};
use quote::ToTokens;
use sha2::Digest;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::visit_mut::VisitMut;
use syn::{Attribute, Expr, ImplItem, Item, TraitItem, UseTree};

use crate::{
    Card, CardNames, digest, doc_attr, feed, is_card_type, quote_json, test_only, without_cards,
};

/// Identifiers that never name an item: the keywords and the primitive types.
const NOT_NAMES: &[&str] = &[
    "Self", "abstract", "as", "async", "await", "become", "bool", "box", "break", "char", "const",
    "continue", "crate", "do", "dyn", "else", "enum", "extern", "f32", "f64", "false", "final",
    "fn", "for", "gen", "i128", "i16", "i32", "i64", "i8", "if", "impl", "in", "isize", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "self", "static", "str", "struct", "super", "trait", "true", "try", "type", "typeof", "u128",
    "u16", "u32", "u64", "u8", "unsafe", "unsized", "use", "usize", "virtual", "where", "while",
    "yield",
];

/// The keywords after which a macro body spells the name of the item it defines.
const DEFINING: &[&str] = &[
    "fn", "static", "const", "struct", "enum", "type", "mod", "trait",
];

/// Runs the item scan over the paths on stdin.
pub fn run(root: &Path) -> io::Result<()> {
    let mut files = Vec::new();
    for line in io::stdin().lock().lines() {
        let line = line?;
        let line = line.trim();
        if !line.is_empty() {
            files.push(line.to_string());
        }
    }
    let inputs: Vec<(String, Option<Vec<u8>>)> = files
        .into_iter()
        .map(|file| {
            let bytes = std::fs::read(root.join(&file)).ok();
            (file, bytes)
        })
        .collect();
    let read = |path: &str| std::fs::read(root.join(path)).ok();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in scan(&inputs, &read) {
        writeln!(out, "{line}")?;
    }
    out.flush()
}

/// Each file's JSON line, in input order. `read` returns an embedded file's bytes by its path
/// relative to the root, or `None` when there is nothing there.
pub fn scan(
    inputs: &[(String, Option<Vec<u8>>)],
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Vec<String> {
    analyse(inputs, read).iter().map(FileScan::json).collect()
}

/// Each file's items, with every class reference of the batch resolved.
fn analyse(
    inputs: &[(String, Option<Vec<u8>>)],
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Vec<FileScan> {
    let mut world = World::default();
    let mut files: Vec<FileScan> = inputs
        .iter()
        .map(|(path, bytes)| scan_file(path, bytes.as_deref(), read, &mut world))
        .collect();
    for file in &mut files {
        world.attribute(file);
    }
    files
}

// ---- one file ----------------------------------------------------------------------------------

/// Where an item sits: the crate its file belongs to and its module path inside that crate.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Ctx {
    krate: String,
    module: Vec<String>,
}

/// A value that may name a class, not yet resolved.
#[derive(Clone, Debug)]
enum Ref {
    /// A string literal.
    Lit(String),
    /// A path, as written, and where it was written.
    Path(Ctx, Vec<String>),
}

struct Row {
    refs: Vec<Ref>,
    digest: String,
    classes: Vec<String>,
}

struct Out {
    id: String,
    kind: &'static str,
    start: usize,
    end: usize,
    test: bool,
    digest: String,
    bare: Option<String>,
    card_types: Vec<String>,
    refs: BTreeSet<String>,
    defines: Vec<String>,
    parent: Option<usize>,
    scope: &'static str,
    includes: Vec<String>,
    value: Option<Ref>,
    class: Vec<String>,
    rows: Vec<Row>,
    card: bool,
    cards: BTreeSet<String>,
}

struct FileScan {
    file: String,
    parsed: bool,
    raw: String,
    items: Vec<Out>,
}

/// What every file of the batch declares that another file's class attribution may read.
#[derive(Default)]
struct World {
    /// A module-level `const` or `static` by crate, module and name: what its value refers to.
    consts: HashMap<(String, Vec<String>, String), Ref>,
    /// Each module's `use` bindings: the name bound, and the path it stands for as written.
    uses: HashMap<Ctx, HashMap<String, Vec<String>>>,
    /// Each module's glob `use`s, as the path before the `*`.
    globs: HashMap<Ctx, Vec<Vec<String>>>,
    /// Every crate a file of the batch belongs to.
    crates: HashSet<String>,
}

fn scan_file(
    file: &str,
    bytes: Option<&[u8]>,
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
    world: &mut World,
) -> FileScan {
    let raw = bytes.map_or_else(String::new, |b| digest(|h| h.update(b)));
    let parsed = bytes
        .and_then(|b| std::str::from_utf8(b).ok())
        .and_then(|t| syn::parse_file(t).ok());
    let Some(parsed) = parsed else {
        return FileScan {
            file: file.to_string(),
            parsed: false,
            raw,
            items: Vec::new(),
        };
    };
    let ctx = ctx_of(file);
    world.crates.insert(ctx.krate.clone());
    let mut names = CardNames::default();
    names.visit_file(&parsed);
    let mut walker = Walker {
        file,
        read,
        world,
        card: Card { names: names.0 },
        out: Vec::new(),
    };
    walker.items(&parsed.items, &ctx, "", false);
    let mut items = walker.out;
    dedupe(&mut items);
    FileScan {
        file: file.to_string(),
        parsed: true,
        raw,
        items,
    }
}

/// The crate and module a file is, read off its path: the directory above the nearest `src` is the
/// crate, and the components below it are the module (`lib.rs`, `main.rs` and `mod.rs` add none).
/// A file under no `src` is a crate of its own.
fn ctx_of(file: &str) -> Ctx {
    let parts: Vec<&str> = file.split('/').collect();
    let Some(src) = parts.iter().rposition(|p| *p == "src") else {
        return Ctx {
            krate: file.to_string(),
            module: Vec::new(),
        };
    };
    let krate = if src == 0 {
        String::new()
    } else {
        parts[src - 1].replace('-', "_")
    };
    let mut module: Vec<String> = parts[src + 1..]
        .iter()
        .map(|p| p.strip_suffix(".rs").unwrap_or(p).to_string())
        .collect();
    if module
        .last()
        .is_some_and(|m| m == "lib" || m == "main" || m == "mod")
    {
        module.pop();
    }
    Ctx { krate, module }
}

/// Appends `#2`, `#3`, ... to every repeat of an id, in source order.
fn dedupe(items: &mut [Out]) {
    let mut seen = HashMap::<String, usize>::new();
    for item in items.iter_mut() {
        let n = seen.entry(item.id.clone()).or_default();
        *n += 1;
        if *n > 1 {
            item.id = format!("{}#{n}", item.id);
        }
    }
}

struct Walker<'a> {
    file: &'a str,
    read: &'a dyn Fn(&str) -> Option<Vec<u8>>,
    world: &'a mut World,
    /// Reads a link to one of this file's cards as no link, for an item's `bare` digest.
    card: Card,
    out: Vec<Out>,
}

impl Walker<'_> {
    #[allow(clippy::too_many_lines)]
    fn items(&mut self, items: &[Item], ctx: &Ctx, prefix: &str, test: bool) {
        let join = |n: &str| {
            if prefix.is_empty() {
                n.to_string()
            } else {
                format!("{prefix}::{n}")
            }
        };
        for item in items {
            let span = item.span();
            let first = self.out.len();
            match item {
                Item::Fn(f) => {
                    let name = f.sig.ident.to_string();
                    self.push(
                        join(&name),
                        "fn",
                        span,
                        test || is_test(&f.attrs),
                        f.to_token_stream(),
                        vec![name],
                        None,
                    );
                }
                Item::Const(c) => {
                    let name = c.ident.to_string();
                    let at = self.push(
                        join(&name),
                        "const",
                        span,
                        test || is_test(&c.attrs),
                        c.to_token_stream(),
                        vec![name.clone()],
                        None,
                    );
                    self.value(at, ctx, &name, &c.expr, is_card_type(&c.ty));
                    if !is_card_type(&c.ty) {
                        let mut bare = c.clone();
                        self.card.visit_item_const_mut(&mut bare);
                        self.set_bare(at, c.to_token_stream(), bare.to_token_stream());
                    }
                }
                Item::Static(s) => {
                    let name = s.ident.to_string();
                    let at = self.push(
                        join(&name),
                        "static",
                        span,
                        test || is_test(&s.attrs),
                        s.to_token_stream(),
                        vec![name.clone()],
                        None,
                    );
                    self.value(at, ctx, &name, &s.expr, is_card_type(&s.ty));
                    if !is_card_type(&s.ty) {
                        let mut bare = s.clone();
                        self.card.visit_item_static_mut(&mut bare);
                        self.set_bare(at, s.to_token_stream(), bare.to_token_stream());
                    }
                }
                Item::Struct(x) => {
                    let name = x.ident.to_string();
                    let t = test || is_test(&x.attrs);
                    self.push(
                        join(&name),
                        "struct",
                        span,
                        t,
                        x.to_token_stream(),
                        vec![name],
                        None,
                    );
                }
                Item::Enum(x) => {
                    let name = x.ident.to_string();
                    let t = test || is_test(&x.attrs);
                    self.push(
                        join(&name),
                        "enum",
                        span,
                        t,
                        x.to_token_stream(),
                        vec![name],
                        None,
                    );
                }
                Item::Union(x) => {
                    let name = x.ident.to_string();
                    let t = test || is_test(&x.attrs);
                    self.push(
                        join(&name),
                        "union",
                        span,
                        t,
                        x.to_token_stream(),
                        vec![name],
                        None,
                    );
                }
                Item::Type(x) => {
                    let name = x.ident.to_string();
                    let t = test || is_test(&x.attrs);
                    self.push(
                        join(&name),
                        "type",
                        span,
                        t,
                        x.to_token_stream(),
                        vec![name],
                        None,
                    );
                }
                Item::TraitAlias(x) => {
                    let name = x.ident.to_string();
                    let t = test || is_test(&x.attrs);
                    self.push(
                        join(&name),
                        "type",
                        span,
                        t,
                        x.to_token_stream(),
                        vec![name],
                        None,
                    );
                }
                Item::Macro(m) if m.ident.is_some() => {
                    let name = m
                        .ident
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    let t = test || is_test(&m.attrs);
                    self.push(
                        join(&format!("{name}!")),
                        "macro-rules",
                        span,
                        t,
                        m.to_token_stream(),
                        vec![name],
                        None,
                    );
                }
                Item::Macro(m) => self.invocation(m, prefix, test || is_test(&m.attrs)),
                Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        let name = m.ident.to_string();
                        let mut inner_ctx = ctx.clone();
                        inner_ctx.module.push(name.clone());
                        self.items(inner, &inner_ctx, &join(&name), test || is_test(&m.attrs));
                    }
                }
                Item::Impl(i) => {
                    let t = test || is_test(&i.attrs);
                    let self_ty = type_name(&i.self_ty);
                    let base = match &i.trait_ {
                        Some((_, tr, _)) => format!(
                            "<{self_ty} as {}>",
                            tr.segments
                                .last()
                                .map(|s| s.ident.to_string())
                                .unwrap_or_default()
                        ),
                        None => self_ty,
                    };
                    let base = join(&base);
                    let mut header = i.clone();
                    header.items.retain(|it| !matches!(it, ImplItem::Fn(_)));
                    let defines = i
                        .items
                        .iter()
                        .filter_map(|it| match it {
                            ImplItem::Const(c) => Some(c.ident.to_string()),
                            ImplItem::Type(ty) => Some(ty.ident.to_string()),
                            _ => None,
                        })
                        .collect();
                    let rest = self.push(
                        format!("{base}::{{impl}}"),
                        "impl-rest",
                        span,
                        t,
                        header.to_token_stream(),
                        defines,
                        None,
                    );
                    for it in &i.items {
                        if let ImplItem::Fn(f) = it {
                            let name = f.sig.ident.to_string();
                            self.push(
                                format!("{base}::{name}"),
                                "fn",
                                f.span(),
                                t || is_test(&f.attrs),
                                f.to_token_stream(),
                                vec![name],
                                Some(rest),
                            );
                        }
                    }
                }
                Item::Trait(tr) => {
                    let t = test || is_test(&tr.attrs);
                    let name = tr.ident.to_string();
                    let base = join(&name);
                    let mut header = tr.clone();
                    header
                        .items
                        .retain(|it| !matches!(it, TraitItem::Fn(f) if f.default.is_some()));
                    let mut defines = vec![name];
                    for it in &header.items {
                        match it {
                            TraitItem::Fn(f) => defines.push(f.sig.ident.to_string()),
                            TraitItem::Const(c) => defines.push(c.ident.to_string()),
                            TraitItem::Type(ty) => defines.push(ty.ident.to_string()),
                            _ => {}
                        }
                    }
                    let rest = self.push(
                        format!("{base}::{{trait}}"),
                        "trait-rest",
                        span,
                        t,
                        header.to_token_stream(),
                        defines,
                        None,
                    );
                    for it in &tr.items {
                        if let TraitItem::Fn(f) = it
                            && f.default.is_some()
                        {
                            let name = f.sig.ident.to_string();
                            self.push(
                                format!("{base}::{name}"),
                                "fn",
                                f.span(),
                                t,
                                f.to_token_stream(),
                                vec![name],
                                Some(rest),
                            );
                        }
                    }
                }
                Item::Use(u) => {
                    let mut bound = Vec::new();
                    use_bindings(&u.tree, &mut Vec::new(), &mut bound);
                    for (name, path) in &bound {
                        if name == "*" {
                            let globs = self.world.globs.entry(ctx.clone()).or_default();
                            globs.push(path.clone());
                        } else {
                            let uses = self.world.uses.entry(ctx.clone()).or_default();
                            uses.insert(name.clone(), path.clone());
                        }
                    }
                    let mut names: Vec<String> = bound.into_iter().map(|(n, _)| n).collect();
                    names.dedup();
                    let t = test || is_test(&u.attrs);
                    let id = join(&format!("use:{}", names.join(",")));
                    let at = self.push(id, "use", span, t, u.to_token_stream(), names, None);
                    // The same `use` with the card types taken out, and the names only those bind.
                    let mut kept = Vec::new();
                    let bare = match without_cards(u.tree.clone()).map(flatten) {
                        Some(tree) => {
                            use_bindings(&tree, &mut Vec::new(), &mut kept);
                            let mut bare = u.clone();
                            bare.tree = tree;
                            bare.to_token_stream()
                        }
                        None => TokenStream::new(),
                    };
                    let item = &mut self.out[at];
                    item.card_types = item
                        .defines
                        .iter()
                        .filter(|n| !kept.iter().any(|(k, _)| k == *n))
                        .cloned()
                        .collect();
                    self.set_bare(at, u.to_token_stream(), bare);
                }
                Item::ExternCrate(x) => {
                    let name = x
                        .rename
                        .as_ref()
                        .map_or_else(|| x.ident.to_string(), |(_, r)| r.to_string());
                    let t = test || is_test(&x.attrs);
                    let id = join(&format!("use:{name}"));
                    self.push(id, "use", span, t, x.to_token_stream(), vec![name], None);
                }
                Item::ForeignMod(x) => {
                    let names: Vec<String> = x
                        .items
                        .iter()
                        .filter_map(|it| match it {
                            syn::ForeignItem::Fn(f) => Some(f.sig.ident.to_string()),
                            syn::ForeignItem::Static(s) => Some(s.ident.to_string()),
                            syn::ForeignItem::Type(ty) => Some(ty.ident.to_string()),
                            _ => None,
                        })
                        .collect();
                    let id = join(&format!(
                        "extern{{{}}}",
                        names.first().map_or("", String::as_str)
                    ));
                    let t = test || is_test(&x.attrs);
                    self.push(id, "extern", span, t, x.to_token_stream(), names, None);
                }
                other => {
                    self.push(
                        join("{verbatim}"),
                        "verbatim",
                        span,
                        test,
                        other.to_token_stream(),
                        Vec::new(),
                        None,
                    );
                }
            }
            if let (Some(scope), Some(out)) = (scope_of(item), self.out.get_mut(first)) {
                out.scope = scope;
            }
        }
    }

    /// Adds one item over `tokens`, docs taken out, and returns its index.
    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        id: String,
        kind: &'static str,
        span: Span,
        test: bool,
        tokens: TokenStream,
        defines: Vec<String>,
        parent: Option<usize>,
    ) -> usize {
        let stripped = strip_docs(tokens);
        self.push_stripped(id, kind, lines(span), test, &stripped, defines, parent)
    }

    #[allow(clippy::too_many_arguments)]
    fn push_stripped(
        &mut self,
        id: String,
        kind: &'static str,
        (start, end): (usize, usize),
        test: bool,
        stripped: &TokenStream,
        defines: Vec<String>,
        parent: Option<usize>,
    ) -> usize {
        let mut refs = BTreeSet::new();
        idents(stripped, &mut refs);
        for name in &defines {
            refs.remove(name);
        }
        let mut embedded = Vec::new();
        include_sites(stripped, &mut embedded);
        let includes: Vec<String> = embedded
            .iter()
            .map(|lit| resolve_include(self.file, lit))
            .collect();
        let digest = self.digest_of(stripped, &includes);
        self.out.push(Out {
            id,
            kind,
            start,
            end,
            test,
            digest,
            bare: None,
            card_types: Vec::new(),
            refs,
            defines,
            parent,
            scope: "",
            includes,
            value: None,
            class: Vec::new(),
            rows: Vec::new(),
            card: false,
            cards: BTreeSet::new(),
        });
        self.out.len() - 1
    }

    /// The digest of `stripped` with the bytes of each file in `includes` folded in.
    fn digest_of(&self, stripped: &TokenStream, includes: &[String]) -> String {
        let contents: Vec<Option<Vec<u8>>> = includes.iter().map(|p| (self.read)(p)).collect();
        digest(|h| {
            feed(h, stripped.clone(), true);
            for (path, bytes) in includes.iter().zip(&contents) {
                h.update(b"\0include\0");
                h.update(path.as_bytes());
                match bytes {
                    Some(b) => {
                        h.update(b"\0present\0");
                        h.update((b.len() as u64).to_le_bytes());
                        h.update(b);
                    }
                    None => h.update(b"\0missing\0"),
                }
            }
        })
    }

    /// Gives the item at `at` a `bare` digest over `bare`, its tokens with the links to cards taken
    /// out, when that is another digest than its own over `own`.
    fn set_bare(&mut self, at: usize, own: TokenStream, bare: TokenStream) {
        if own.to_string() == bare.to_string() {
            return;
        }
        let digest = self.digest_of(&strip_docs(bare), &self.out[at].includes);
        if digest != self.out[at].digest {
            self.out[at].bare = Some(digest);
        }
    }

    /// Records what a `const` or `static` at `at` refers to: the class a struct literal's `name`
    /// field names, a class name, a path, and an array literal's rows.
    fn value(&mut self, at: usize, ctx: &Ctx, name: &str, expr: &Expr, card: bool) {
        let expr = unref(expr);
        let value = match expr {
            Expr::Struct(s) => s.fields.iter().find_map(|field| match &field.member {
                syn::Member::Named(id) if id == "name" => to_ref(ctx, &field.expr),
                _ => None,
            }),
            other => to_ref(ctx, other),
        };
        if let Expr::Array(array) = expr {
            self.out[at].rows = array
                .elems
                .iter()
                .map(|elem| Row {
                    refs: row_refs(ctx, elem),
                    digest: digest(|h| feed(h, strip_docs(elem.to_token_stream()), true)),
                    classes: Vec::new(),
                })
                .collect();
        }
        if let Some(value) = &value {
            self.world.consts.insert(
                (ctx.krate.clone(), ctx.module.clone(), name.to_string()),
                value.clone(),
            );
        }
        let item = &mut self.out[at];
        item.value = value;
        item.card = card;
    }

    /// A macro invocation at item level, and every `fn` its body spells as an item of its own.
    fn invocation(&mut self, m: &syn::ItemMacro, prefix: &str, test: bool) {
        let join = |n: &str| {
            if prefix.is_empty() {
                n.to_string()
            } else {
                format!("{prefix}::{n}")
            }
        };
        let macro_name = m
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        let body = strip_docs(m.mac.tokens.clone());
        let (inner, rest) = split_fns(&body);
        let inner_names: Vec<String> = inner.iter().map(|f| f.name.clone()).collect();
        let defines: Vec<String> = spelled(&rest)
            .into_iter()
            .filter(|n| !inner_names.contains(n))
            .collect();
        let first = inner_names.first().or(defines.first()).cloned();
        let id = match first {
            Some(f) => join(&format!("{macro_name}!{{{f}}}")),
            None => join(&format!("{macro_name}!")),
        };
        // The invocation's own digest is its path and the body less the inner functions, each of
        // which stands in it as `fn <name>`.
        let mut whole: Vec<TokenTree> = strip_docs(m.mac.path.to_token_stream())
            .into_iter()
            .collect();
        whole.push(TokenTree::Punct(proc_macro2::Punct::new(
            '!',
            proc_macro2::Spacing::Alone,
        )));
        let delimiter = match m.mac.delimiter {
            syn::MacroDelimiter::Paren(_) => Delimiter::Parenthesis,
            syn::MacroDelimiter::Brace(_) => Delimiter::Brace,
            syn::MacroDelimiter::Bracket(_) => Delimiter::Bracket,
        };
        whole.push(TokenTree::Group(Group::new(delimiter, rest)));
        let whole: TokenStream = whole.into_iter().collect();
        let at = self.push_stripped(id, "macro", lines(m.span()), test, &whole, defines, None);
        for f in inner {
            self.push_stripped(
                join(&f.name),
                "fn",
                (f.start, f.end),
                test,
                &f.tokens,
                vec![f.name.clone()],
                Some(at),
            );
        }
    }
}

fn lines(span: Span) -> (usize, usize) {
    (span.start().line, span.end().line)
}

fn is_test(attrs: &[Attribute]) -> bool {
    test_only(attrs)
        || attrs
            .iter()
            .any(|a| a.path().segments.last().is_some_and(|s| s.ident == "test"))
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(p) => p
            .path
            .segments
            .last()
            .map_or_else(|| "?".into(), |s| s.ident.to_string()),
        syn::Type::Reference(r) => type_name(&r.elem),
        other => other.to_token_stream().to_string().replace(' ', ""),
    }
}

/// `tree` with each group of one entry written as that entry, so taking the card types out of
/// `{ClassDoc, CoreClass}` reads as the `CoreClass` it was before one was added. A lone `self` keeps
/// its braces, since `a::{self}` and `a::self` are not the same import.
fn flatten(tree: UseTree) -> UseTree {
    match tree {
        UseTree::Path(mut p) => {
            p.tree = Box::new(flatten(*p.tree));
            UseTree::Path(p)
        }
        UseTree::Group(mut g) => {
            let lone_self = matches!(g.items.first(), Some(UseTree::Name(n)) if n.ident == "self");
            if g.items.len() == 1 && !lone_self {
                return flatten(g.items.pop().expect("one entry").into_value());
            }
            g.items = g.items.into_iter().map(flatten).collect();
            UseTree::Group(g)
        }
        other => other,
    }
}

/// Each name a `use` tree binds, with the path it stands for as written.
fn use_bindings(tree: &UseTree, prefix: &mut Vec<String>, out: &mut Vec<(String, Vec<String>)>) {
    match tree {
        UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            use_bindings(&p.tree, prefix, out);
            prefix.pop();
        }
        UseTree::Name(n) => {
            let name = n.ident.to_string();
            let mut path = prefix.clone();
            if name != "self" {
                path.push(name.clone());
            }
            let bound = if name == "self" {
                prefix.last().cloned().unwrap_or_default()
            } else {
                name
            };
            out.push((bound, path));
        }
        UseTree::Rename(r) => {
            let mut path = prefix.clone();
            if r.ident != "self" {
                path.push(r.ident.to_string());
            }
            out.push((r.rename.to_string(), path));
        }
        UseTree::Glob(_) => out.push(("*".to_string(), prefix.clone())),
        UseTree::Group(g) => {
            for t in &g.items {
                use_bindings(t, prefix, out);
            }
        }
    }
}

// ---- tokens ------------------------------------------------------------------------------------

/// `tokens` with every `#[doc = ...]` and `#![doc = ...]` attribute taken out, at every depth.
fn strip_docs(tokens: TokenStream) -> TokenStream {
    let toks: Vec<TokenTree> = tokens.into_iter().collect();
    let mut out = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        if let Some((after, _)) = doc_attr(&toks, i) {
            i = after;
            continue;
        }
        match &toks[i] {
            TokenTree::Group(g) => {
                let mut group = Group::new(g.delimiter(), strip_docs(g.stream()));
                group.set_span(g.span());
                out.push(TokenTree::Group(group));
            }
            t => out.push(t.clone()),
        }
        i += 1;
    }
    out.into_iter().collect()
}

fn idents(tokens: &TokenStream, into: &mut BTreeSet<String>) {
    for t in tokens.clone() {
        match t {
            TokenTree::Group(g) => idents(&g.stream(), into),
            TokenTree::Ident(id) => {
                let s = id.to_string();
                let s = s.strip_prefix("r#").unwrap_or(&s);
                if !NOT_NAMES.contains(&s) {
                    into.insert(s.to_string());
                }
            }
            _ => {}
        }
    }
}

/// Every `include_str!`, `include_bytes!` and `include!` whose argument is a string literal, as the
/// literal's value, in source order.
fn include_sites(tokens: &TokenStream, out: &mut Vec<String>) {
    let toks: Vec<TokenTree> = tokens.clone().into_iter().collect();
    for (i, t) in toks.iter().enumerate() {
        match t {
            TokenTree::Group(g) => include_sites(&g.stream(), out),
            TokenTree::Ident(id)
                if id == "include_str" || id == "include_bytes" || id == "include" =>
            {
                let bang =
                    matches!(toks.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!');
                let Some(TokenTree::Group(args)) = toks.get(i + 2) else {
                    continue;
                };
                let Some(TokenTree::Literal(lit)) = args.stream().into_iter().next() else {
                    continue;
                };
                let lit = TokenStream::from(TokenTree::Literal(lit));
                if bang && let Ok(value) = syn::parse2::<syn::LitStr>(lit) {
                    out.push(value.value());
                }
            }
            _ => {}
        }
    }
}

/// An embedded path relative to the root: the literal joined onto the including file's directory,
/// with `.` and `..` folded, `/`-separated. An absolute literal stays as written.
fn resolve_include(file: &str, literal: &str) -> String {
    let literal = literal.replace('\\', "/");
    if Path::new(&literal).is_absolute() {
        return literal;
    }
    let base = Path::new(file).parent().unwrap_or_else(|| Path::new(""));
    let mut parts: Vec<String> = Vec::new();
    for c in base.join(&literal).components() {
        match c {
            Component::ParentDir => {
                if parts.pop().is_none() {
                    parts.push("..".into());
                }
            }
            Component::Normal(p) => parts.push(p.to_string_lossy().into_owned()),
            _ => {}
        }
    }
    parts.join("/")
}

/// One `fn` spelled at the top level of a macro body.
struct InnerFn {
    name: String,
    tokens: TokenStream,
    start: usize,
    end: usize,
}

/// The `[attrs] [qualifiers] fn <name> .. { .. }` runs at the top level of a macro body, and the
/// body with each run replaced by `fn <name>`.
fn split_fns(body: &TokenStream) -> (Vec<InnerFn>, TokenStream) {
    let toks: Vec<TokenTree> = body.clone().into_iter().collect();
    let mut inner = Vec::new();
    let mut rest: Vec<TokenTree> = Vec::new();
    // Where the run that may end in a `fn` began: its first token, and how long `rest` was then.
    let mut run: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                run.get_or_insert((i, rest.len()));
                let inner_attr =
                    matches!(toks.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!');
                let end = (i + if inner_attr { 3 } else { 2 }).min(toks.len());
                rest.extend(toks[i..end].iter().cloned());
                i = end;
                continue;
            }
            TokenTree::Ident(id) if id == "fn" => {
                if let Some(TokenTree::Ident(name)) = toks.get(i + 1) {
                    let mut j = i + 2;
                    while j < toks.len()
                        && !matches!(&toks[j], TokenTree::Group(g) if g.delimiter() == Delimiter::Brace)
                    {
                        j += 1;
                    }
                    if j < toks.len() {
                        let (first, mark) = run.unwrap_or((i, rest.len()));
                        rest.truncate(mark);
                        inner.push(InnerFn {
                            name: name.to_string(),
                            tokens: toks[first..=j].iter().cloned().collect(),
                            start: toks[first].span().start().line,
                            end: toks[j].span().end().line,
                        });
                        rest.push(TokenTree::Ident(Ident::new("fn", toks[i].span())));
                        rest.push(TokenTree::Ident(name.clone()));
                        run = None;
                        i = j + 1;
                        continue;
                    }
                }
                run = None;
            }
            TokenTree::Ident(id)
                if id == "pub"
                    || id == "unsafe"
                    || id == "extern"
                    || id == "async"
                    || id == "const" =>
            {
                run.get_or_insert((i, rest.len()));
            }
            // `pub(crate)` and `extern "C"` keep a run going.
            TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis && run.is_some() => {}
            TokenTree::Literal(_) if run.is_some() => {}
            _ => run = None,
        }
        rest.push(toks[i].clone());
        i += 1;
    }
    (inner, rest.into_iter().collect())
}

/// The identifiers a macro body spells where an item's name goes: the first token of the body or
/// of a comma- or semicolon-separated part, or the token after a defining keyword, and never a
/// path segment or a macro name.
fn spelled(rest: &TokenStream) -> Vec<String> {
    let toks: Vec<TokenTree> = rest.clone().into_iter().collect();
    let mut out: Vec<String> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        let TokenTree::Ident(id) = t else { continue };
        let name = id.to_string();
        if NOT_NAMES.contains(&name.as_str()) {
            continue;
        }
        let starts = match i.checked_sub(1).map(|p| &toks[p]) {
            None => true,
            Some(TokenTree::Punct(p)) => {
                (p.as_char() == ',' || p.as_char() == ';')
                    && !(i >= 2
                        && matches!(&toks[i - 2], TokenTree::Punct(q) if q.as_char() == ':'))
            }
            Some(TokenTree::Ident(k)) => DEFINING.contains(&k.to_string().as_str()),
            Some(TokenTree::Group(g)) => g.delimiter() == Delimiter::Bracket,
            Some(TokenTree::Literal(_)) => false,
        };
        let path_or_macro = matches!(toks.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!' || p.as_char() == ':' && p.spacing() == proc_macro2::Spacing::Joint);
        if starts && !path_or_macro && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

// ---- class attribution -------------------------------------------------------------------------

fn unref(expr: &Expr) -> &Expr {
    match expr {
        Expr::Reference(r) => unref(&r.expr),
        Expr::Paren(p) => unref(&p.expr),
        Expr::Group(g) => unref(&g.expr),
        other => other,
    }
}

/// A class reference read off one expression: a string literal, a path, or `<path>.name`.
fn to_ref(ctx: &Ctx, expr: &Expr) -> Option<Ref> {
    match unref(expr) {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => Some(Ref::Lit(s.value())),
            _ => None,
        },
        Expr::Path(p) if p.qself.is_none() => Some(Ref::Path(
            ctx.clone(),
            p.path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect(),
        )),
        Expr::Field(f) if matches!(&f.member, syn::Member::Named(n) if n == "name") => {
            to_ref(ctx, &f.base)
        }
        _ => None,
    }
}

/// The class references one array element holds: its `name` field when it is a struct literal,
/// every string literal and path in it otherwise.
fn row_refs(ctx: &Ctx, elem: &Expr) -> Vec<Ref> {
    if let Expr::Struct(s) = unref(elem) {
        return s
            .fields
            .iter()
            .filter(|f| matches!(&f.member, syn::Member::Named(n) if n == "name"))
            .filter_map(|f| to_ref(ctx, &f.expr))
            .collect();
    }
    struct Refs<'a> {
        ctx: &'a Ctx,
        out: Vec<Ref>,
    }
    impl<'ast> Visit<'ast> for Refs<'_> {
        fn visit_expr(&mut self, e: &'ast Expr) {
            match e {
                Expr::Lit(_) | Expr::Path(_) => {
                    if let Some(r) = to_ref(self.ctx, e) {
                        self.out.push(r);
                    }
                }
                _ => visit::visit_expr(self, e),
            }
        }
    }
    let mut refs = Refs {
        ctx,
        out: Vec::new(),
    };
    refs.visit_expr(elem);
    refs.out
}

/// Whether `s` is a `Core` class name as source writes it: `Core\Name`, `Core\Db\Rows`.
fn is_class_name(s: &str) -> bool {
    let mut parts = s.split('\\');
    parts.next() == Some("Core")
        && s.contains('\\')
        && parts.all(|p| {
            let mut chars = p.chars();
            chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

impl World {
    /// Resolves every class reference in `file` and works out which classes each card serves.
    fn attribute(&self, file: &mut FileScan) {
        for item in &mut file.items {
            if let Some(value) = &item.value {
                item.class = self.resolve(value, 0);
            }
            for row in &mut item.rows {
                let mut classes: Vec<String> =
                    row.refs.iter().flat_map(|r| self.resolve(r, 0)).collect();
                classes.sort();
                classes.dedup();
                row.classes = classes;
            }
            if item.rows.iter().all(|r| r.classes.is_empty()) {
                item.rows.clear();
            }
        }
        // A card serves every class whose class item, or a card serving it, names the card.
        let mut changed = true;
        while changed {
            changed = false;
            for c in 0..file.items.len() {
                if !file.items[c].card {
                    continue;
                }
                let Some(name) = file.items[c].defines.first().cloned() else {
                    continue;
                };
                let mut reach: BTreeSet<String> = BTreeSet::new();
                for item in &file.items {
                    if item.refs.contains(&name) {
                        reach.extend(item.class.iter().cloned());
                        if item.card {
                            reach.extend(item.cards.iter().cloned());
                        }
                    }
                }
                let cards = &mut file.items[c].cards;
                let before = cards.len();
                cards.extend(reach);
                changed |= cards.len() != before;
            }
        }
    }

    /// The class names a reference comes to, following consts and `use` bindings.
    fn resolve(&self, r: &Ref, depth: usize) -> Vec<String> {
        if depth > 16 {
            return Vec::new();
        }
        match r {
            Ref::Lit(s) => {
                if is_class_name(s) {
                    vec![s.clone()]
                } else {
                    Vec::new()
                }
            }
            Ref::Path(ctx, segs) => match self.target(ctx, segs, &mut HashSet::new()) {
                Some(key) => self
                    .consts
                    .get(&key)
                    .map(|value| self.resolve(value, depth + 1))
                    .unwrap_or_default(),
                None => Vec::new(),
            },
        }
    }

    /// The const a path names, as crate, module and name. `seen` holds every module and name
    /// already looked up through a `use`, so a cycle of glob imports ends.
    fn target(
        &self,
        ctx: &Ctx,
        segs: &[String],
        seen: &mut HashSet<(Ctx, String)>,
    ) -> Option<(String, Vec<String>, String)> {
        let (last, head) = segs.split_last()?;
        if head.is_empty() {
            let key = (ctx.krate.clone(), ctx.module.clone(), last.clone());
            if self.consts.contains_key(&key) {
                return Some(key);
            }
            return self.imported(ctx, last, seen);
        }
        let (krate, mut module) = match head[0].as_str() {
            "crate" => (ctx.krate.clone(), head[1..].to_vec()),
            "self" => {
                let mut m = ctx.module.clone();
                m.extend(head[1..].iter().cloned());
                (ctx.krate.clone(), m)
            }
            "super" => {
                let mut m = ctx.module.clone();
                let mut rest = head;
                while rest.first().is_some_and(|s| s == "super") {
                    m.pop();
                    rest = &rest[1..];
                }
                m.extend(rest.iter().cloned());
                (ctx.krate.clone(), m)
            }
            first if self.crates.contains(first) && first != ctx.krate => {
                (first.to_string(), head[1..].to_vec())
            }
            first => {
                if let Some(bound) = self.uses.get(ctx).and_then(|u| u.get(first)) {
                    if !seen.insert((ctx.clone(), first.to_string())) {
                        return None;
                    }
                    let mut whole = bound.clone();
                    whole.extend(segs[1..].iter().cloned());
                    return self.target(ctx, &whole, seen);
                }
                let mut m = ctx.module.clone();
                m.extend(head.iter().cloned());
                (ctx.krate.clone(), m)
            }
        };
        let key = (krate.clone(), std::mem::take(&mut module), last.clone());
        if self.consts.contains_key(&key) {
            return Some(key);
        }
        let there = Ctx {
            krate,
            module: key.1,
        };
        self.imported(&there, last, seen)
    }

    /// The const `name` stands for in module `ctx` through one of its `use`s: a binding of that
    /// name, else the first glob that reaches one.
    fn imported(
        &self,
        ctx: &Ctx,
        name: &String,
        seen: &mut HashSet<(Ctx, String)>,
    ) -> Option<(String, Vec<String>, String)> {
        if !seen.insert((ctx.clone(), name.clone())) {
            return None;
        }
        if let Some(bound) = self.uses.get(ctx).and_then(|u| u.get(name)) {
            return self.target(ctx, bound, seen);
        }
        let globs = self.globs.get(ctx)?;
        globs.iter().find_map(|glob| {
            let mut path = glob.clone();
            path.push(name.clone());
            self.target(ctx, &path, seen)
        })
    }
}

// ---- output ------------------------------------------------------------------------------------

impl FileScan {
    fn json(&self) -> String {
        let items: Vec<String> = self
            .items
            .iter()
            .map(|item| item.json(&self.items))
            .collect();
        format!(
            r#"{{"file":{},"parsed":{},"raw":"{}","items":[{}]}}"#,
            quote_json(&self.file),
            self.parsed,
            self.raw,
            items.join(",")
        )
    }
}

/// Who may name `item` when that is narrower than `pub`: `private` or `crate`, as the module doc says.
/// None for a `pub` item and for a kind of item that carries no `scope`.
fn scope_of(item: &Item) -> Option<&'static str> {
    let vis = match item {
        Item::Fn(x) => &x.vis,
        Item::Const(x) => &x.vis,
        Item::Static(x) => &x.vis,
        Item::Struct(x) => &x.vis,
        Item::Enum(x) => &x.vis,
        Item::Union(x) => &x.vis,
        Item::Type(x) => &x.vis,
        Item::TraitAlias(x) => &x.vis,
        Item::Trait(x) => &x.vis,
        _ => return None,
    };
    match vis {
        syn::Visibility::Public(_) => None,
        syn::Visibility::Restricted(r) if r.path.is_ident("self") => Some("private"),
        syn::Visibility::Restricted(_) => Some("crate"),
        syn::Visibility::Inherited => Some("private"),
    }
}

fn list<'a>(names: impl IntoIterator<Item = &'a String>) -> String {
    let quoted: Vec<String> = names.into_iter().map(|n| quote_json(n)).collect();
    format!("[{}]", quoted.join(","))
}

impl Out {
    fn json(&self, all: &[Out]) -> String {
        let mut s = format!(
            r#"{{"id":{},"kind":"{}","start":{},"end":{},"test":{},"digest":"{}","refs":{},"defines":{}"#,
            quote_json(&self.id),
            self.kind,
            self.start,
            self.end,
            self.test,
            self.digest,
            list(&self.refs),
            list(&self.defines),
        );
        if let Some(bare) = &self.bare {
            s.push_str(&format!(r#","bare":"{bare}""#));
        }
        if !self.card_types.is_empty() {
            s.push_str(&format!(r#","cardTypes":{}"#, list(&self.card_types)));
        }
        if let Some(parent) = self.parent {
            s.push_str(&format!(r#","parent":{}"#, quote_json(&all[parent].id)));
        }
        if !self.scope.is_empty() {
            s.push_str(&format!(r#","scope":"{}""#, self.scope));
        }
        if !self.includes.is_empty() {
            s.push_str(&format!(r#","includes":{}"#, list(&self.includes)));
        }
        if !self.class.is_empty() {
            s.push_str(&format!(r#","class":{}"#, list(&self.class)));
        }
        if !self.rows.is_empty() {
            let rows: Vec<String> = self
                .rows
                .iter()
                .map(|r| {
                    format!(
                        r#"{{"classes":{},"digest":"{}"}}"#,
                        list(&r.classes),
                        r.digest
                    )
                })
                .collect();
            s.push_str(&format!(r#","rows":[{}]"#, rows.join(",")));
        }
        if !self.cards.is_empty() {
            s.push_str(&format!(r#","cards":{}"#, list(&self.cards)));
        }
        s.push('}');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_includes(_: &str) -> Option<Vec<u8>> {
        None
    }

    fn files(sources: &[(&str, &str)]) -> Vec<FileScan> {
        let inputs: Vec<(String, Option<Vec<u8>>)> = sources
            .iter()
            .map(|(path, text)| ((*path).to_string(), Some(text.as_bytes().to_vec())))
            .collect();
        analyse(&inputs, &no_includes)
    }

    fn one(text: &str) -> FileScan {
        files(&[("crates/a/src/lib.rs", text)]).remove(0)
    }

    fn item<'a>(file: &'a FileScan, id: &str) -> &'a Out {
        file.items.iter().find(|i| i.id == id).unwrap_or_else(|| {
            let ids: Vec<&str> = file.items.iter().map(|i| i.id.as_str()).collect();
            panic!("no item `{id}` among {ids:?}")
        })
    }

    #[test]
    fn an_id_is_the_module_path_the_impl_type_and_the_name() {
        let file = one("mod m {\n\
             struct S;\n\
             impl S { fn a() {} }\n\
             impl T for S { fn b() {} }\n\
             trait T { fn b(); fn c() {} }\n\
             fn free() {}\n\
             }\n");
        for (id, kind) in [
            ("m::S", "struct"),
            ("m::S::{impl}", "impl-rest"),
            ("m::S::a", "fn"),
            ("m::<S as T>::{impl}", "impl-rest"),
            ("m::<S as T>::b", "fn"),
            ("m::T::{trait}", "trait-rest"),
            ("m::T::c", "fn"),
            ("m::free", "fn"),
        ] {
            assert_eq!(item(&file, id).kind, kind, "{id}");
        }
        let a = item(&file, "m::S::a");
        assert_eq!(
            a.parent.map(|p| file.items[p].id.as_str()),
            Some("m::S::{impl}")
        );
        assert_eq!((a.start, a.end), (3, 3));
    }

    #[test]
    fn the_digest_ignores_docs_layout_and_line_shifts_and_follows_the_code() {
        let base = one("fn f() -> u8 { 1 }\n");
        let moved = one("\n\n// a comment\n/// A doc comment.\nfn f()\n  -> u8 {\n    1\n}\n");
        let changed = one("fn f() -> u8 { 2 }\n");
        let digest = |file: &FileScan| item(file, "f").digest.clone();
        assert_eq!(digest(&base), digest(&moved));
        assert_ne!(digest(&base), digest(&changed));
        assert_eq!(
            item(&moved, "f").start,
            4,
            "the span starts at the doc comment"
        );
    }

    #[test]
    fn an_embedded_file_is_folded_into_the_digest_by_its_bytes() {
        let text = "const DATA: &str = include_str!(\"../data/table.txt\");\n";
        let inputs = vec![(
            "crates/a/src/lib.rs".to_string(),
            Some(text.as_bytes().to_vec()),
        )];
        let with = |bytes: Option<&'static [u8]>| {
            let read = move |path: &str| {
                assert_eq!(path, "crates/a/data/table.txt");
                bytes.map(<[u8]>::to_vec)
            };
            analyse(&inputs, &read).remove(0)
        };
        let one_table = with(Some(b"one"));
        let other_table = with(Some(b"two"));
        let missing = with(None);
        let data = |file: &FileScan| item(file, "DATA").digest.clone();
        assert_eq!(
            item(&one_table, "DATA").includes,
            ["crates/a/data/table.txt"]
        );
        assert_ne!(data(&one_table), data(&other_table));
        assert_ne!(data(&one_table), data(&missing));
    }

    #[test]
    fn a_macro_invocation_is_named_after_what_it_defines_and_its_fns_are_items() {
        let file = one("helper! {\n\
             /// Docs.\n\
             fn first(ctx) { 1 }\n\
             #[inline]\n\
             fn second(ctx) { 2 }\n\
             }\n\
             unary! {\n\
             /// Docs.\n\
             nvs_core_math_sqrt, \"sqrt\", f64::sqrt\n\
             }\n");
        let block = item(&file, "helper!{first}");
        assert_eq!(block.kind, "macro");
        let second = item(&file, "second");
        assert_eq!(second.kind, "fn");
        assert_eq!(
            second.parent.map(|p| file.items[p].id.as_str()),
            Some("helper!{first}")
        );
        assert_eq!((second.start, second.end), (4, 5));
        let unary = item(&file, "unary!{nvs_core_math_sqrt}");
        assert_eq!(unary.defines, ["nvs_core_math_sqrt"]);
        assert!(unary.refs.contains("unary"), "{:?}", unary.refs);

        let edited = one("helper! {\n\
             fn first(ctx) { 1 }\n\
             #[inline]\n\
             fn second(ctx) { 3 }\n\
             }\n");
        assert_eq!(item(&file, "first").digest, item(&edited, "first").digest);
        assert_ne!(item(&file, "second").digest, item(&edited, "second").digest);
        assert_eq!(
            item(&file, "helper!{first}").digest,
            item(&edited, "helper!{first}").digest,
            "an edit inside one fn leaves the invocation's own tokens alone"
        );
    }

    #[test]
    fn a_repeated_id_takes_an_ordinal_in_source_order() {
        let file = one("#[cfg(unix)]\nfn f() {}\n#[cfg(windows)]\nfn f() {}\n");
        assert_eq!(item(&file, "f").start, 1);
        assert_eq!(item(&file, "f#2").start, 3);
    }

    #[test]
    fn refs_name_what_the_item_uses_and_defines_name_what_it_binds() {
        let file = one("use crate::registry::{CoreClass, CoreMethod as Method};\n\
             use std::io::*;\n\
             macro_rules! twice { ($e:expr) => { $e + $e } }\n\
             pub fn total(x: u32) -> u32 { twice!(LIMIT) + helper(x) }\n");
        let total = item(&file, "total");
        assert!(total.refs.contains("LIMIT") && total.refs.contains("helper"));
        assert!(total.refs.contains("twice"));
        assert!(!total.refs.contains("fn") && !total.refs.contains("u32"));
        assert!(
            !total.refs.contains("total"),
            "an item does not refer to itself"
        );
        assert_eq!(
            item(&file, "use:CoreClass,Method").defines,
            ["CoreClass", "Method"]
        );
        assert_eq!(item(&file, "use:*").defines, ["*"]);
        assert_eq!(item(&file, "twice!").kind, "macro-rules");
    }

    #[test]
    fn a_class_table_row_is_attributed_to_the_class_it_names() {
        let scanned = files(&[
            (
                "crates/nvs-stdlib/src/registry.rs",
                "pub const CLASSES: &[CoreClass] = &[crate::math::CLASS, crate::db::CLASS, OTHER];\n\
                 pub const CAPS: &[(&str, &str)] = &[(crate::uri::NAME, \"parse\"), (r\"Core\\Math\", \"abs\")];\n",
            ),
            (
                "crates/nvs-stdlib/src/math.rs",
                "const CARD: ClassDoc = ClassDoc { short: \"Numbers.\" };\n\
                 const ABS_DOC: MethodDoc = MethodDoc { short: \"abs\", params: &[&N_DOC] };\n\
                 const N_DOC: ParamDoc = ParamDoc { name: \"n\" };\n\
                 const UNUSED_DOC: MethodDoc = MethodDoc { short: \"none\" };\n\
                 pub const CLASS: CoreClass = CoreClass {\n\
                 name: r\"Core\\Math\", doc: Some(&CARD),\n\
                 methods: &[CoreMethod { name: \"abs\", doc: Some(&ABS_DOC) }],\n\
                 };\n",
            ),
            (
                "crates/nvs-stdlib/src/uri.rs",
                "pub const NAME: &str = \"Core\\\\Uri\";\n",
            ),
            (
                "crates/nvs-stdlib/src/db/mod.rs",
                "mod registry;\npub(crate) use self::registry::*;\n",
            ),
            (
                "crates/nvs-stdlib/src/db/registry.rs",
                "use super::NAME_OF_DB as DB;\n\
                 pub(crate) const CLASS: CoreClass = CoreClass { name: DB };\n",
            ),
            ("crates/nvs-stdlib/src/lib.rs", "pub mod db;\n"),
        ]);
        // `NAME_OF_DB` is nowhere in the batch, so the db row stays unattributed.
        let registry = &scanned[0];
        let classes: Vec<Vec<String>> = item(registry, "CLASSES")
            .rows
            .iter()
            .map(|r| r.classes.clone())
            .collect();
        assert_eq!(classes, [vec![r"Core\Math".to_string()], vec![], vec![]]);
        let caps: Vec<Vec<String>> = item(registry, "CAPS")
            .rows
            .iter()
            .map(|r| r.classes.clone())
            .collect();
        assert_eq!(
            caps,
            [
                vec![r"Core\Uri".to_string()],
                vec![r"Core\Math".to_string()]
            ]
        );

        let math = &scanned[1];
        assert_eq!(item(math, "CLASS").class, [r"Core\Math"]);
        for card in ["CARD", "ABS_DOC", "N_DOC"] {
            assert!(item(math, card).cards.contains(r"Core\Math"), "{card}");
        }
        assert!(item(math, "UNUSED_DOC").cards.is_empty());
        assert_eq!(item(&scanned[2], "NAME").class, [r"Core\Uri"]);
    }

    #[test]
    fn a_class_name_reached_through_a_glob_re_export_and_a_use_alias_resolves() {
        let scanned = files(&[
            (
                "crates/nvs-stdlib/src/registry.rs",
                "pub const CLASSES: &[CoreClass] = &[crate::db::CLASS];\n",
            ),
            (
                "crates/nvs-stdlib/src/db/mod.rs",
                "mod registry;\npub(crate) use self::registry::*;\npub const NAME: &str = r\"Core\\Db\";\n",
            ),
            (
                "crates/nvs-stdlib/src/db/registry.rs",
                "use super::NAME as DB;\npub(crate) const CLASS: CoreClass = CoreClass { name: DB };\n",
            ),
            (
                "crates/nvs-stdlib/src/cli.rs",
                "pub const TEXT: CoreClass = CoreClass { name: nvs_runtime::CARRIER };\n",
            ),
            (
                "crates/nvs-runtime/src/lib.rs",
                "mod ctx;\npub use ctx::{CARRIER};\n",
            ),
            (
                "crates/nvs-runtime/src/ctx/mod.rs",
                "mod output;\npub use self::output::*;\n",
            ),
            (
                "crates/nvs-runtime/src/ctx/output.rs",
                "pub const CARRIER: &str = r\"Core\\Cli\\Text\";\n",
            ),
        ]);
        let rows = &item(&scanned[0], "CLASSES").rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].classes, [r"Core\Db"]);
        assert_eq!(item(&scanned[3], "TEXT").class, [r"Core\Cli\Text"]);
    }

    #[test]
    fn a_link_to_a_card_is_outside_the_bare_digest_and_every_other_field_is_inside() {
        let src = |doc: &str, name: &str| {
            format!(
                "use crate::registry::{{ClassDoc, CoreClass}};\n\
                 const CARD: ClassDoc = ClassDoc {{ short: \"A class.\" }};\n\
                 pub const CLASS: CoreClass = CoreClass {{ name: r\"{name}\", doc: {doc} }};\n"
            )
        };
        let none = one(&src("None", r"Core\A"));
        let linked = one(&src("Some(&CARD)", r"Core\A"));
        let renamed = one(&src("Some(&CARD)", r"Core\B"));
        let foreign = one(&src("Some(&other::CARD)", r"Core\A"));
        let bare = |file: &FileScan| {
            let class = item(file, "CLASS");
            class.bare.clone().unwrap_or_else(|| class.digest.clone())
        };
        assert!(item(&none, "CLASS").bare.is_none());
        assert_ne!(item(&none, "CLASS").digest, item(&linked, "CLASS").digest);
        assert_eq!(bare(&none), bare(&linked), "a new link to a card");
        assert_ne!(bare(&linked), bare(&renamed), "the name moved as well");
        assert_ne!(bare(&none), bare(&foreign), "a card of another file");
        assert!(
            item(&linked, "CARD").bare.is_none(),
            "a card has no bare digest"
        );
        assert!(linked.json().contains(r#""bare":""#));
    }

    #[test]
    fn a_use_s_bare_digest_leaves_out_the_card_types_it_imports() {
        let before = one("use crate::registry::{CaseDoc, CoreClass, MethodDoc};\n");
        let after =
            one("use crate::registry::{\n    CaseDoc, ClassDoc, CoreClass, MethodDoc,\n};\n");
        let other =
            one("use crate::registry::{CaseDoc, ClassDoc, CoreClass, CoreTy, MethodDoc};\n");
        let only = |file: &FileScan| file.items[0].bare.clone().expect("a bare digest");
        assert_eq!(only(&before), only(&after));
        assert_ne!(only(&after), only(&other));
        assert_eq!(
            after.items[0].card_types,
            ["CaseDoc", "ClassDoc", "MethodDoc"]
        );
        assert!(
            after
                .json()
                .contains(r#""cardTypes":["CaseDoc","ClassDoc","MethodDoc"]"#)
        );
        let plain = one("use crate::registry::CoreClass;\n");
        assert!(plain.items[0].bare.is_none());
        assert!(plain.items[0].card_types.is_empty());
        let grown = one("use crate::registry::{ClassDoc, CoreClass};\n");
        assert_eq!(
            only(&grown),
            plain.items[0].digest,
            "a group of one is its entry"
        );
        let kept = one("use crate::a::{self};\n");
        assert!(
            kept.items[0].bare.is_none(),
            "a lone `self` keeps its braces"
        );
    }

    #[test]
    fn a_file_that_does_not_parse_has_no_items() {
        let file = one("fn broken( {");
        assert!(!file.parsed);
        assert!(file.items.is_empty());
        assert!(file.json().contains(r#""parsed":false"#));
    }

    #[test]
    fn a_file_path_names_its_crate_and_module() {
        let at = |path: &str| {
            let ctx = ctx_of(path);
            (ctx.krate, ctx.module.join("::"))
        };
        assert_eq!(
            at("crates/nvs-stdlib/src/lib.rs"),
            ("nvs_stdlib".into(), String::new())
        );
        assert_eq!(
            at("crates/nvs-stdlib/src/db/mod.rs"),
            ("nvs_stdlib".into(), "db".into())
        );
        assert_eq!(
            at("crates/nvs-stdlib/src/db/row.rs"),
            ("nvs_stdlib".into(), "db::row".into())
        );
        assert_eq!(
            at("crates/nvs-cli/tests/meta.rs"),
            ("crates/nvs-cli/tests/meta.rs".into(), String::new())
        );
    }

    #[test]
    fn a_json_line_carries_every_field_a_reader_needs() {
        let file = one("pub const X: u8 = helper();\n");
        let line = file.json();
        for field in [
            r#""file":"crates/a/src/lib.rs""#,
            r#""parsed":true"#,
            r#""id":"X""#,
            r#""kind":"const""#,
            r#""start":1"#,
            r#""end":1"#,
            r#""test":false"#,
            r#""refs":["helper"]"#,
            r#""defines":["X"]"#,
        ] {
            assert!(line.contains(field), "{field} in {line}");
        }
    }
}
