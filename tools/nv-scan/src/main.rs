//! What `bun nv`'s build keys read from a Rust file. Each file is parsed with `syn`, and each tier is a
//! digest of the token stream `syn` prints back after that tier's items are taken out of the syntax
//! tree. `tools/nv/keys/scan.ts` says what each tier leaves out and which reader keys on it; this
//! program is how it is read, so no pattern over the text ever decides what a key holds.
//!
//! Input on stdin: each file as its length in bytes on a line of its own, then its bytes. Output on
//! stdout: one JSON object per file, in input order, with the four tier digests and the file's
//! `include_str!`/`include_bytes!` sites. A file `syn` cannot parse is its text in every tier, and
//! its include sites are read off its tokens as if none were in a test module.
//!
//! `nv-scan --items <root>` is the other mode: each file's items, with ids, line spans, digests and
//! references, for observed selection. [`items`] owns its input and output.

mod items;

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::path::Path;

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use sha2::{Digest, Sha256};
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{
    Attribute, Block, Expr, FieldValue, File, ImplItem, Item, ItemImpl, ItemMod, ItemTrait, Member,
    Meta, Stmt, Token, TraitItem, Type, UseTree,
};

/// The registry's reference-card types. `tools/nv/test/keys.test.ts` checks this list against the
/// `pub struct *Doc` declarations in `crates/nvs-stdlib/src/registry.rs`.
const CARD_TYPES: &[&str] = &[
    "CaseDoc",
    "ClassDoc",
    "EnumDoc",
    "ErrorDoc",
    "MethodDoc",
    "ParamDoc",
    "ShapeKeyDoc",
];

/// What `text_direction_codepoint_in_comment` denies. A file with one is its text in every tier,
/// because a comment then changes what `rustc` accepts.
const BIDI: &[char] = &[
    '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}',
    '\u{2069}',
];

fn main() {
    // A deeply nested expression recurses deeply in the parser and the printer.
    let worker = std::thread::Builder::new().stack_size(256 << 20).spawn(run);
    let code = match worker.map(|h| h.join()) {
        Ok(Ok(Ok(()))) => 0,
        Ok(Ok(Err(e))) => {
            eprintln!("nv-scan: {e}");
            1
        }
        _ => {
            eprintln!("nv-scan: the scan panicked");
            1
        }
    };
    std::process::exit(code);
}

fn run() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    if let Some(flag) = args.next() {
        if flag != "--items" {
            return Err(bad("the only argument is `--items <root>`"));
        }
        let root = args
            .next()
            .ok_or_else(|| bad("`--items` needs the root directory"))?;
        return items::run(Path::new(&root));
    }
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;
    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut at = 0;
    while at < input.len() {
        let newline = input[at..]
            .iter()
            .position(|&b| b == b'\n')
            .ok_or_else(|| bad("a length line has no newline"))?;
        let len: usize = std::str::from_utf8(&input[at..at + newline])
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .ok_or_else(|| bad("a length line is not a number"))?;
        let start = at + newline + 1;
        let bytes = input
            .get(start..start + len)
            .ok_or_else(|| bad("a file is shorter than its length line"))?;
        writeln!(out, "{}", analyse(bytes))?;
        at = start + len;
    }
    out.flush()
}

fn bad(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}

/// One file's analysis as a JSON object.
fn analyse(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let parsed = std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| syn::parse_file(t).ok());
    let Some(file) = parsed else {
        let whole = digest(|h| h.update(bytes));
        let includes = text
            .parse::<TokenStream>()
            .map(|ts| sites(ts, false))
            .unwrap_or_default();
        return json([&whole, &whole, &whole, &whole], &includes);
    };
    let bidi = text.contains(BIDI);
    let tier = |tokens: TokenStream, docs: bool| {
        digest(|h| {
            if bidi {
                h.update(bytes);
            }
            feed(h, tokens, docs);
        })
    };

    let full = file.to_token_stream();
    let mut shipped_file = file;
    Shipped.visit_file_mut(&mut shipped_file);
    let shipped = shipped_file.to_token_stream();
    let mut names = CardNames::default();
    names.visit_file(&shipped_file);
    let mut card_file = shipped_file;
    Card { names: names.0 }.visit_file_mut(&mut card_file);
    let card = card_file.to_token_stream();

    // A site in the whole file that the shipped file lacks sat in test code. Both lists are in source
    // order and the shipped one is a subsequence of the whole, so one pass pairs them.
    let kept = sites(shipped.clone(), false);
    let mut next = 0;
    let includes: Vec<(String, bool)> = sites(full.clone(), false)
        .into_iter()
        .map(|(path, _)| {
            let shipped = kept.get(next).is_some_and(|(k, _)| *k == path);
            if shipped {
                next += 1;
            }
            (path, !shipped)
        })
        .collect();

    json(
        [
            &tier(full.clone(), true),
            &tier(full, false),
            &tier(shipped, false),
            &tier(card, false),
        ],
        &includes,
    )
}

fn digest(fill: impl FnOnce(&mut Sha256)) -> String {
    let mut h = Sha256::new();
    fill(&mut h);
    h.finalize()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Every token into `h`, each tagged and length-prefixed so no two streams share a digest. Without
/// `docs`, each run of doc attributes of one style is a single placeholder.
fn feed(h: &mut Sha256, tokens: TokenStream, docs: bool) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    let mut i = 0;
    while i < tokens.len() {
        if !docs && let Some((after, inner)) = doc_attr(&tokens, i) {
            let mut end = after;
            while let Some((next, same)) = doc_attr(&tokens, end) {
                if same != inner {
                    break;
                }
                end = next;
            }
            h.update(if inner { b"D!" as &[u8] } else { b"D" });
            i = end;
            continue;
        }
        match &tokens[i] {
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => (b"(", b")"),
                    Delimiter::Brace => (b"{", b"}"),
                    Delimiter::Bracket => (b"[", b"]"),
                    Delimiter::None => (b"<", b">"),
                };
                h.update(open);
                feed(h, g.stream(), docs);
                h.update(close);
            }
            TokenTree::Ident(id) => text(h, b'i', &id.to_string()),
            TokenTree::Punct(p) => {
                let joint = p.spacing() == Spacing::Joint;
                text(h, if joint { b'j' } else { b'p' }, &p.as_char().to_string());
            }
            TokenTree::Literal(l) => text(h, b'l', &l.to_string()),
        }
        i += 1;
    }
}

fn text(h: &mut Sha256, tag: u8, s: &str) {
    h.update([tag]);
    h.update((s.len() as u64).to_le_bytes());
    h.update(s.as_bytes());
}

/// A `#[doc = ...]` or `#![doc = ...]` attribute at `i`: the index after it, and whether it is inner.
fn doc_attr(tokens: &[TokenTree], i: usize) -> Option<(usize, bool)> {
    let TokenTree::Punct(hash) = tokens.get(i)? else {
        return None;
    };
    if hash.as_char() != '#' {
        return None;
    }
    let inner = matches!(tokens.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!');
    let at = i + 1 + usize::from(inner);
    let TokenTree::Group(g) = tokens.get(at)? else {
        return None;
    };
    if g.delimiter() != Delimiter::Bracket {
        return None;
    }
    let mut body = g.stream().into_iter();
    let is_doc = matches!(body.next(), Some(TokenTree::Ident(id)) if id == "doc")
        && matches!(body.next(), Some(TokenTree::Punct(p)) if p.as_char() == '=');
    is_doc.then_some((at + 1, inner))
}

/// Every `include_str!`/`include_bytes!` whose argument is a string literal, in source order, as the
/// literal's text between its first and last quote.
fn sites(tokens: TokenStream, in_test: bool) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    collect_sites(tokens, in_test, &mut out);
    out
}

fn collect_sites(tokens: TokenStream, in_test: bool, out: &mut Vec<(String, bool)>) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (i, t) in tokens.iter().enumerate() {
        match t {
            TokenTree::Group(g) => collect_sites(g.stream(), in_test, out),
            TokenTree::Ident(id) if id == "include_str" || id == "include_bytes" => {
                let bang =
                    matches!(tokens.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!');
                let Some(TokenTree::Group(args)) = tokens.get(i + 2) else {
                    continue;
                };
                let Some(TokenTree::Literal(lit)) = args.stream().into_iter().next() else {
                    continue;
                };
                let lit = lit.to_string();
                if let (true, Some(a), Some(b)) = (bang, lit.find('"'), lit.rfind('"'))
                    && a < b
                {
                    out.push((lit[a + 1..b].to_string(), in_test));
                }
            }
            _ => {}
        }
    }
}

fn json(tiers: [&str; 4], includes: &[(String, bool)]) -> String {
    let sites: Vec<String> = includes
        .iter()
        .map(|(path, in_test)| format!(r#"{{"path":{},"inTest":{in_test}}}"#, quote_json(path)))
        .collect();
    format!(
        r#"{{"docs":"{}","code":"{}","shipped":"{}","card":"{}","includes":[{}]}}"#,
        tiers[0],
        tiers[1],
        tiers[2],
        tiers[3],
        sites.join(",")
    )
}

fn quote_json(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---- shipped: what a build without `cfg(test)` compiles --------------------------------------------

/// Does `#[cfg(...)]` on these attributes shut the item off when `test` is not set?
fn test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && a.parse_args::<Meta>().ok().and_then(|m| cfg_value(&m)) == Some(false)
    })
}

/// A `cfg` predicate's value with `test` unset, or `None` when it depends on anything else.
fn cfg_value(meta: &Meta) -> Option<bool> {
    match meta {
        Meta::Path(p) if p.is_ident("test") => Some(false),
        Meta::List(l) => {
            let args = l
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()?;
            let values: Vec<Option<bool>> = args.iter().map(cfg_value).collect();
            if l.path.is_ident("all") {
                if values.contains(&Some(false)) {
                    Some(false)
                } else if values.iter().all(|v| *v == Some(true)) {
                    Some(true)
                } else {
                    None
                }
            } else if l.path.is_ident("any") {
                if values.contains(&Some(true)) {
                    Some(true)
                } else if values.iter().all(|v| *v == Some(false)) {
                    Some(false)
                } else {
                    None
                }
            } else if l.path.is_ident("not") && values.len() == 1 {
                values[0].map(|v| !v)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::ExternCrate(i) => &i.attrs,
        Item::Fn(i) => &i.attrs,
        Item::ForeignMod(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Macro(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::TraitAlias(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        Item::Union(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

fn impl_item_attrs(item: &ImplItem) -> &[Attribute] {
    match item {
        ImplItem::Const(i) => &i.attrs,
        ImplItem::Fn(i) => &i.attrs,
        ImplItem::Type(i) => &i.attrs,
        ImplItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

fn trait_item_attrs(item: &TraitItem) -> &[Attribute] {
    match item {
        TraitItem::Const(i) => &i.attrs,
        TraitItem::Fn(i) => &i.attrs,
        TraitItem::Type(i) => &i.attrs,
        TraitItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

fn stmt_attrs(stmt: &Stmt) -> &[Attribute] {
    match stmt {
        Stmt::Local(l) => &l.attrs,
        Stmt::Item(i) => item_attrs(i),
        Stmt::Macro(m) => &m.attrs,
        Stmt::Expr(..) => &[],
    }
}

/// Takes out every item, impl or trait member and statement that `cfg(test)` shuts off.
struct Shipped;

impl VisitMut for Shipped {
    fn visit_file_mut(&mut self, f: &mut File) {
        f.items.retain(|i| !test_only(item_attrs(i)));
        visit_mut::visit_file_mut(self, f);
    }

    fn visit_item_mod_mut(&mut self, m: &mut ItemMod) {
        if let Some((_, items)) = &mut m.content {
            items.retain(|i| !test_only(item_attrs(i)));
        }
        visit_mut::visit_item_mod_mut(self, m);
    }

    fn visit_item_impl_mut(&mut self, i: &mut ItemImpl) {
        i.items.retain(|it| !test_only(impl_item_attrs(it)));
        visit_mut::visit_item_impl_mut(self, i);
    }

    fn visit_item_trait_mut(&mut self, t: &mut ItemTrait) {
        t.items.retain(|it| !test_only(trait_item_attrs(it)));
        visit_mut::visit_item_trait_mut(self, t);
    }

    fn visit_block_mut(&mut self, b: &mut Block) {
        b.stmts.retain(|s| !test_only(stmt_attrs(s)));
        visit_mut::visit_block_mut(self, b);
    }
}

// ---- card: what a program runs, without the reference cards ----------------------------------------

fn is_card_type(ty: &Type) -> bool {
    match ty {
        Type::Path(p) => {
            p.qself.is_none()
                && p.path.segments.last().is_some_and(|s| {
                    s.arguments.is_none() && CARD_TYPES.contains(&s.ident.to_string().as_str())
                })
        }
        Type::Reference(r) => is_card_type(&r.elem),
        Type::Slice(s) => is_card_type(&s.elem),
        Type::Array(a) => is_card_type(&a.elem),
        Type::Paren(p) => is_card_type(&p.elem),
        Type::Group(g) => is_card_type(&g.elem),
        _ => false,
    }
}

fn is_card_item(item: &Item) -> bool {
    match item {
        Item::Const(c) => is_card_type(&c.ty),
        Item::Static(s) => is_card_type(&s.ty),
        _ => false,
    }
}

/// The names of the file's card constants and statics.
#[derive(Default)]
struct CardNames(HashSet<String>);

impl<'a> Visit<'a> for CardNames {
    fn visit_item(&mut self, item: &'a Item) {
        match item {
            Item::Const(c) if is_card_type(&c.ty) => {
                self.0.insert(c.ident.to_string());
            }
            Item::Static(s) if is_card_type(&s.ty) => {
                self.0.insert(s.ident.to_string());
            }
            _ => {}
        }
        visit::visit_item(self, item);
    }

    fn visit_impl_item(&mut self, item: &'a ImplItem) {
        if let ImplItem::Const(c) = item
            && is_card_type(&c.ty)
        {
            self.0.insert(c.ident.to_string());
        }
        visit::visit_impl_item(self, item);
    }
}

/// Takes out every card constant, reads `doc: Some(&CARD)` as `doc: None` for a card of this file,
/// and takes the card types out of `use` lists.
struct Card {
    names: HashSet<String>,
}

impl Card {
    fn items(&self, items: &mut Vec<Item>) {
        items.retain_mut(|item| match item {
            _ if is_card_item(item) => false,
            Item::Use(u) => match without_cards(u.tree.clone()) {
                Some(tree) => {
                    u.tree = tree;
                    true
                }
                None => false,
            },
            _ => true,
        });
    }

    /// `Some(&NAME)`, where `NAME` is one of this file's card constants.
    fn links_a_card(&self, expr: &Expr) -> bool {
        let Expr::Call(call) = expr else { return false };
        let Expr::Path(some) = &*call.func else {
            return false;
        };
        if !some.path.is_ident("Some") || call.args.len() != 1 {
            return false;
        }
        let Expr::Reference(r) = &call.args[0] else {
            return false;
        };
        let Expr::Path(name) = &*r.expr else {
            return false;
        };
        r.mutability.is_none()
            && name.qself.is_none()
            && name
                .path
                .get_ident()
                .is_some_and(|id| self.names.contains(&id.to_string()))
    }
}

/// `tree` without the card types it imports, or `None` when nothing is left.
fn without_cards(tree: UseTree) -> Option<UseTree> {
    let card = |id: &proc_macro2::Ident| CARD_TYPES.contains(&id.to_string().as_str());
    match tree {
        UseTree::Name(n) if card(&n.ident) => None,
        UseTree::Rename(r) if card(&r.ident) => None,
        UseTree::Path(mut p) => {
            p.tree = Box::new(without_cards(*p.tree)?);
            Some(UseTree::Path(p))
        }
        UseTree::Group(mut g) => {
            let items: Punctuated<UseTree, Token![,]> =
                g.items.into_iter().filter_map(without_cards).collect();
            if items.is_empty() {
                return None;
            }
            g.items = items;
            Some(UseTree::Group(g))
        }
        other => Some(other),
    }
}

impl VisitMut for Card {
    fn visit_file_mut(&mut self, f: &mut File) {
        self.items(&mut f.items);
        visit_mut::visit_file_mut(self, f);
    }

    fn visit_item_mod_mut(&mut self, m: &mut ItemMod) {
        if let Some((_, items)) = &mut m.content {
            self.items(items);
        }
        visit_mut::visit_item_mod_mut(self, m);
    }

    fn visit_item_impl_mut(&mut self, i: &mut ItemImpl) {
        i.items
            .retain(|it| !matches!(it, ImplItem::Const(c) if is_card_type(&c.ty)));
        visit_mut::visit_item_impl_mut(self, i);
    }

    fn visit_block_mut(&mut self, b: &mut Block) {
        b.stmts
            .retain(|s| !matches!(s, Stmt::Item(i) if is_card_item(i)));
        visit_mut::visit_block_mut(self, b);
    }

    fn visit_field_value_mut(&mut self, fv: &mut FieldValue) {
        if matches!(&fv.member, Member::Named(id) if id == "doc") && self.links_a_card(&fv.expr) {
            fv.expr = syn::parse_quote!(None);
        }
        visit_mut::visit_field_value_mut(self, fv);
    }
}
