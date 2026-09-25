//! `nvs meta --json` — the `Core` registry as JSON, for a consumer outside
//! the build.
//!
//! `rule:tooling/meta-json`
//! is the contract, and this command owns it: every class in
//! [`nvs_stdlib::registry::CLASSES`], every member — static ones first, then
//! instance ones — with the `$name` each of its positional parameters is
//! callable by under `names` (`rule:core-api/shape-rules`
//! R2) and, for a member whose row carries a
//! [`MethodDoc`], its reference card under a `doc` key; every constant of the
//! class beside its members, with its one-sentence card as its `doc`; the
//! class's own card, its `short` alone, under the class's `doc` key; and,
//! top-level beside `classes`, every enum in
//! [`nvs_stdlib::registry::ENUMS`] with its [`EnumDoc`] — top-level because
//! that roster is, an enum having no owner class in the registry. A consumer
//! ignores fields it does not know, so a field may be added here without
//! breaking one; a field may not be renamed or moved.
//!
//! ## What is omitted, and why
//!
//! A row with no documentation has **no `doc` key at all** — a shape the
//! registry's own `every_registry_row_carries_a_reference_card` forbids a
//! shipped row, but that a consumer reading an older toolchain still
//! meets — and a documented row's `short`/`return` strings and
//! `params`/`shape`/`errors`/`cases` arrays appear **only when non-empty** — a
//! class with no constants has no `constants` key either, and a member that
//! throws nothing has no `errors`. That is § 3's field-wise precedence made
//! mechanical: the registry spells "not written yet" as the empty value, and a
//! consumer must be able to tell that apart from "written, and empty" without
//! learning the convention — an absent key is the one spelling that needs no
//! explanation. The website's Core reference, which `bun nv render --website`
//! writes, is the consumer.
//!
//! ## The signature half, and the tables beside the registry
//!
//! A row is **signature** as well as documentation, and `docs/novis.md` —
//! the one-file reference `tools/reference.py` generates — is built from this
//! command alone, so that a member's types never have to be scraped out of
//! the spec a second time. Every member therefore also carries `kind`
//! (`static` or `instance`), `signature` (the spec's own spelling,
//! `length(string $s): uint`), `params` with each positional parameter's
//! type, qualifier and default, `options` for a trailing bag, and `returns`;
//! a class carries `typeParams` and `constructor` when it has them, a
//! constant its `type` and `value`. Beside `classes` and `enums` sit the
//! rosters declared outside the registry's own class table: `exceptions`
//! ([`nvs_hir::errors::TREE`]), `interfaces`
//! ([`nvs_hir::interfaces::RESERVED`]), `attributes`
//! ([`nvs_types::derive::ATTRIBUTES`]), `directives`
//! ([`nvs_config::DIRECTIVES`]) and `capabilities`
//! ([`nvs_stdlib::registry::CAPABILITIES`]). Each is a table already, so this
//! is one `map` per roster and no second home for any of them.
//!
//! `capabilities` is a roster rather than a field on each member row, and
//! `rule:security/capability-declaration-is-one-table` is why: what this
//! runtime can do to a machine stays one screen of one file, so a consumer
//! wanting a member's capability beside its card joins the two rosters on
//! `(class, member)` at render time. Emitting the table here is what makes
//! that join possible outside the binary at all — the table is `nvs-stdlib`'s
//! and a consumer of this document has no other way to read it.
//!
//! ## The program half
//!
//! `rule:tooling/meta-json-takes-a-program` adds one optional argument, and it
//! adds exactly one key: an entry point puts that program's own declarations
//! under `program`, and the registry half above is untouched, so the
//! no-argument form is byte-identical to what it printed before the argument
//! existed. That is the whole seam — one input added to one document, never a
//! fork — and it is what lets `tools/reference.py` and the website's Core
//! reference stay renderers rather than sources
//! (`rule:tooling/one-json-several-renderers`).
//!
//! The program half is the *program*, not the file: [`crate::front_end`]
//! parses, resolves and type-checks the whole `require`/`autoload` graph, and
//! every file it reached contributes its declarations in the order it was
//! loaded. A program that does not check prints nothing and exits non-zero,
//! for the reason `nvs check` does — a document over declarations the compiler
//! refused would document a program that does not exist.
//!
//! Its shape mirrors the registry's rather than inventing one: `classes` and
//! `interfaces` each carry `members` and `constants`, `enums` carry `cases`,
//! and `types` — which the registry has no roster for, a `type` alias being a
//! user declaration only — carries the type it stands for. A body owning
//! aliases carries a `types` of its own, the same card shape under the same
//! key, each named `Owner::Name` as a program reaches it
//! (`rule:types/type-alias`) — never beside `members`, which an alias is not
//! one of: it has no value, no visibility and no runtime existence. A declaration's
//! `doc` is the `///` run above it: its prose under `short`, the key the
//! registry's card already spells prose with, and its two tags as `see` and
//! `example` lists (`rule:tooling/doc-comment-tags-are-see-and-example`). The
//! omission rule is the registry's, key for key: a declaration with no `///`
//! has no `doc`, and no array is ever emitted empty.
//!
//! One key has no registry counterpart: a member carries its `visibility`,
//! because a `Core` member is public or it is not in the registry while a
//! program's is whatever it was written as. It is always present rather than
//! omitted when unwritten — a member has a visibility even where it spelled
//! none — and it is what lets [`crate::doc`] render the surface a package's
//! reader can reach rather than every helper behind it. See [`visibility`].
//!
//! Built and written as JSON rather than as text through the `serde_json`
//! this crate already carries for `rule:routing/api-document-is-generated-from-the-route-table`'s document; the workspace manifest
//! owns why this crate and not another.

use std::path::Path;
use std::process::ExitCode;

use nvs_diagnostics::{SourceFile, Span};
use nvs_stdlib::registry::{
    CAPABILITIES, CLASSES, Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreOption, CoreTy,
    ENUMS, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual, class_type_params, constructor_of,
};
use nvs_syntax::ast::{
    ClassDecl, ClassMember, ClassMemberKind, ConstMember, DocComment, DocTagKind, EnumDecl,
    InterfaceDecl, MethodMember, Modifier, Param, PropertyMember, Stmt, StmtKind, TypeAliasDecl,
};
use serde_json::{Map, Value, json};

/// Prints the registry — and, with an entry point, the program's declarations
/// beside it — and returns success.
///
/// The registry half cannot fail: every value is a compile-time constant of
/// `nvs-stdlib`, and serializing a tree of strings does not error. The program
/// half is the front end's exit code when the program does not check, and
/// nothing is printed then.
pub(crate) fn run(entry: Option<&Path>) -> ExitCode {
    let document = match entry {
        Some(entry) => match program_document(entry) {
            Ok(document) => document,
            Err(code) => return code,
        },
        None => document(),
    };
    println!("{document}");
    ExitCode::SUCCESS
}

/// The document [`run`] would print for `entry`, as a value.
///
/// `nvs doc` renders *this* rather than deriving anything of its own, which is
/// `rule:tooling/one-json-several-renderers` held from inside the binary: the
/// renderer shipped here reads the same document a consumer outside it does,
/// so a page can never say something `nvs meta --json` did not.
pub(crate) fn program_document(entry: &Path) -> Result<Value, ExitCode> {
    let checked = crate::front_end(entry)?;
    let mut document = document();
    document
        .as_object_mut()
        .expect("the document is an object")
        .insert("program".into(), program_json(&checked));
    Ok(document)
}

/// The whole document — § 2's outermost object.
///
/// Separate from [`run`] so the tests below can assert on it without
/// capturing standard output, and reachable from [`crate::agent`], which
/// renders this document rather than reading the registry a second time
/// (`rule:tooling/one-json-several-renderers`).
pub(crate) fn document() -> Value {
    json!({
        "classes": CLASSES.iter().map(class_json).collect::<Vec<_>>(),
        "enums": ENUMS.iter().map(enum_json).collect::<Vec<_>>(),
        "exceptions": nvs_hir::errors::TREE.iter().map(exception_json).collect::<Vec<_>>(),
        "interfaces": nvs_hir::interfaces::RESERVED.iter().map(interface_json).collect::<Vec<_>>(),
        "attributes": nvs_types::derive::ATTRIBUTES.iter().map(|name| Value::from(*name)).collect::<Vec<_>>(),
        "directives": nvs_config::DIRECTIVES.iter().map(directive_json).collect::<Vec<_>>(),
        "capabilities": CAPABILITIES.iter().map(capability_json).collect::<Vec<_>>(),
    })
}

/// One class: its name, its type parameters and constructor when it has
/// them, its members — methods before instance members, each roster in the
/// spec's own order — its constants, when it has any, and its own card under
/// `doc` when the row carries one.
fn class_json(class: &CoreClass) -> Value {
    let members: Vec<Value> = class
        .methods
        .iter()
        .map(|member| member_json(member, "static"))
        .chain(
            class
                .instance
                .iter()
                .map(|member| member_json(member, "instance")),
        )
        .collect();
    let mut out = Map::new();
    out.insert("name".into(), Value::from(class.name));
    if let Some(params) = class_type_params(class.name) {
        put_list(&mut out, "typeParams", params, |name| Value::from(*name));
    }
    if let Some(constructor) = constructor_of(class.name) {
        out.insert(
            "constructor".into(),
            member_json(constructor, "constructor"),
        );
    }
    out.insert("members".into(), Value::Array(members));
    put_list(&mut out, "constants", class.constants, constant_json);
    if let Some(doc) = class.doc {
        let mut card = Map::new();
        put_str(&mut card, "short", doc.short);
        out.insert("doc".into(), Value::Object(card));
    }
    Value::Object(out)
}

/// One exception class of the compiler-declared tree: its name, its parent
/// when it has one, and the properties it declares of its own.
fn exception_json(row: &(&str, Option<&str>)) -> Value {
    let (name, parent) = *row;
    let own: &[&str] = nvs_hir::errors::OWN_PROPERTIES
        .iter()
        .find(|(owner, _)| *owner == name)
        .map_or(&[], |(_, properties)| *properties);
    let mut out = Map::new();
    out.insert("name".into(), Value::from(name));
    if let Some(parent) = parent {
        out.insert("parent".into(), Value::from(parent));
    }
    put_list(&mut out, "properties", own, |property| {
        Value::from(*property)
    });
    Value::Object(out)
}

/// One compiler-declared global interface: its name and its type parameters.
fn interface_json(row: &(&str, &[&str])) -> Value {
    let (name, params) = *row;
    let mut out = Map::new();
    out.insert("name".into(), Value::from(name));
    put_list(&mut out, "typeParams", params, |param| Value::from(*param));
    Value::Object(out)
}

/// One `nvs.toml` directive: its key, its class and when a change applies.
fn directive_json(directive: &nvs_config::Directive) -> Value {
    json!({
        "key": directive.key,
        "class": format!("{:?}", directive.class),
        "apply": format!("{:?}", directive.apply),
    })
}

/// One capability declaration: the class, the member, and the capability that
/// member needs when it needs one.
///
/// A row declaring no capability says the member needs none, and the key is
/// absent rather than `null`, which is this document's omission rule
/// (`rule:tooling/meta-json`) rather than a shape of its own. The capability is
/// spelled as [`nvs_config::Cap::name`] gives it — `fs.read`, the name an
/// operator grants it under — so a reader can paste it into a configuration
/// file without a second mapping.
///
/// Nothing here is joined onto the member row itself:
/// `rule:security/capability-declaration-is-one-table`'s second paragraph
/// rejects a per-member field, and a consumer wanting one joins these two
/// rosters on `(class, member)`.
fn capability_json(row: &(&str, &str, Option<nvs_config::Cap>)) -> Value {
    let (class, member, capability) = *row;
    let mut out = Map::new();
    out.insert("class".into(), Value::from(class));
    out.insert("member".into(), Value::from(member));
    if let Some(capability) = capability {
        out.insert("capability".into(), Value::from(capability.name()));
    }
    Value::Object(out)
}

/// One member: its name, its `kind`, the `$name` each positional parameter is
/// callable by (`rule:core-api/shape-rules`
/// R2), its signature half — `signature`, `params`, `options`, `returns` —
/// and its `doc` only when the row carries one.
///
/// `names` is the row's own `CoreMethod::names`, so a member with no
/// positional parameter has no `names` key — and a trailing options bag has no
/// entry there either, its one name being
/// `nvs_stdlib::registry::OPTIONS_NAME` for every member that has one. It is
/// emitted for **every** row, documented or not, because it is signature and
/// not documentation: a consumer building a call needs it where a reference
/// card is optional. The same holds of every field the module doc's
/// *signature half* names.
fn member_json(member: &CoreMethod, kind: &str) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(member.name));
    out.insert("kind".into(), Value::from(kind));
    put_list(&mut out, "names", member.names, |name| Value::from(*name));
    out.insert("signature".into(), Value::from(signature(member)));
    let params: Vec<Value> = positional_json(member);
    if !params.is_empty() {
        out.insert("params".into(), Value::Array(params));
    }
    if let Some(options) = member.options() {
        put_list(&mut out, "options", options, option_json);
    }
    out.insert("returns".into(), Value::from(ty_string(&member.return_ty)));
    if let Some(doc) = member.doc {
        out.insert("doc".into(), doc_json(doc));
    }
    Value::Object(out)
}

/// Every positional parameter of `member` — the row's `params` without a
/// trailing options bag — as `{name, type, qualifier?, default?, variadic?}`.
///
/// `defaults` aligns to the *end* of the positional list, exactly as a
/// user-declared method's trailing defaults do, so the first `n - d`
/// parameters are required. A variadic tail carries no default and says
/// `variadic` instead. A parameter whose type carries an `rule:security/unclassified-parameter-refuses-tainted`
/// classification names it as `qualifier`; one that carries none — every
/// non-text type, and the unclassified `string`/`bytes` spellings, which
/// refuse a tainted argument — has no key.
fn positional_json(member: &CoreMethod) -> Vec<Value> {
    let positional = member.positional();
    let required = positional.len().saturating_sub(member.defaults.len());
    positional
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let mut out = Map::new();
            out.insert(
                "name".into(),
                Value::from(member.names.get(index).copied().unwrap_or("")),
            );
            match ty {
                CoreTy::Variadic(elem) => {
                    out.insert("type".into(), Value::from(ty_string(elem)));
                    out.insert("variadic".into(), Value::Bool(true));
                }
                _ => {
                    out.insert("type".into(), Value::from(ty_string(ty)));
                    if index >= required
                        && let Some(default) = member.defaults.get(index - required)
                    {
                        out.insert("default".into(), Value::from(const_string(default)));
                    }
                }
            }
            if let Some(qual) = ty.classification() {
                out.insert("qualifier".into(), Value::from(qual_string(qual)));
            }
            Value::Object(out)
        })
        .collect()
}

/// One option of a trailing bag: its name, type and the value an omitted one
/// takes.
fn option_json(option: &CoreOption) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(option.name));
    out.insert("type".into(), Value::from(ty_string(&option.ty)));
    out.insert("default".into(), Value::from(const_string(&option.default)));
    if let Some(qual) = option.ty.classification() {
        out.insert("qualifier".into(), Value::from(qual_string(qual)));
    }
    Value::Object(out)
}

/// The spec's own spelling of a row — `name(type $a, type $b = 1, {opt?: T}): ret`,
/// with a variadic tail as `type ...$rest` and a written type-argument list
/// as `name<T>(…)`. Never the class or the `$receiver->`: the consumer knows
/// which class it is reading and `kind` says the rest.
fn signature(member: &CoreMethod) -> String {
    let positional = member.positional();
    let required = positional.len().saturating_sub(member.defaults.len());
    let mut parts: Vec<String> = positional
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let name = member.names.get(index).copied().unwrap_or("");
            match ty {
                CoreTy::Variadic(elem) => format!("{} ...${name}", ty_string(elem)),
                _ if index >= required => match member.defaults.get(index - required) {
                    Some(default) => {
                        format!("{} ${name} = {}", ty_string(ty), const_string(default))
                    }
                    None => format!("{} ${name}", ty_string(ty)),
                },
                _ => format!("{} ${name}", ty_string(ty)),
            }
        })
        .collect();
    if let Some(options) = member.options() {
        parts.push(ty_string(&CoreTy::Options(options)));
    }
    let written = member.written();
    let type_args = if written.is_empty() {
        String::new()
    } else {
        format!("<{}>", written.join(", "))
    };
    format!(
        "{}{type_args}({}): {}",
        member.name,
        parts.join(", "),
        ty_string(&member.return_ty)
    )
}

/// A [`CoreTy`] in the spelling a program writes it.
///
/// One line, because the spelling itself is `nvs_stdlib::registry`'s: it sits
/// beside the type it spells, so the editor's own reading of a row
/// (`nvs_lsp::completion`) and this document cannot describe one member two
/// ways.
fn ty_string(ty: &CoreTy) -> String {
    ty.spelled()
}

/// A [`Const`] as a program would write it: a scalar as its literal, a
/// string JSON-quoted, an enum case by its qualified spelling, and a built
/// instance as the call that builds it.
fn const_string(value: &Const) -> String {
    match value {
        Const::Null => "null".into(),
        // The one default with no written spelling: omitting the key is how a
        // call site asks for it, and writing `null` asks for the opposite
        // (`rule:core-api/omission-is-not-a-written-null`). Spelled as the
        // absence it is rather than as a value nobody can type.
        Const::NeverWritten => "(omitted)".into(),
        Const::Bool(b) => b.to_string(),
        Const::Int(i) => i.to_string(),
        Const::Uint(u) => u.to_string(),
        Const::Float(f) => format!("{f:?}"),
        Const::Str(s) => serde_json::to_string(s).unwrap_or_default(),
        Const::Bytes(bytes) => format!(
            "bytes({})",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ),
        Const::EmptyArray => "[]".into(),
        Const::EnumCase(owner, case) => format!("{owner}::{case}"),
        Const::Built { symbol, args } => {
            // The call that builds it, spelled as a program would write it —
            // the symbol names a member of some registered class, so the
            // class and member are looked up rather than the helper's name
            // printed.
            let call = CLASSES
                .iter()
                .find_map(|class| {
                    class
                        .members()
                        .find(|member| member.symbol == *symbol)
                        .map(|member| format!("{}::{}", class.name, member.name))
                })
                .unwrap_or_else(|| (*symbol).to_owned());
            format!(
                "{call}({})",
                args.iter().map(const_string).collect::<Vec<_>>().join(", ")
            )
        }
        _ => "?".into(),
    }
}

/// A [`Qual`] as the spec's *Qualifier* column spells it.
fn qual_string(qual: Qual) -> &'static str {
    match qual {
        Qual::Contagious => "contagious",
        Qual::Sink => "sink",
        Qual::Neutral => "neutral",
        Qual::Launder => "launder",
        Qual::Reveal => "reveal",
    }
}

/// One reference card, with every empty field left out — see the module doc.
fn doc_json(doc: &MethodDoc) -> Value {
    let mut out = Map::new();
    put_str(&mut out, "short", doc.short);
    put_list(&mut out, "params", doc.params, param_json);
    put_str(&mut out, "return", doc.ret);
    put_list(&mut out, "errors", doc.errors, error_json);
    Value::Object(out)
}

/// One parameter, with its `shape` only for a shape-typed one.
fn param_json(param: &ParamDoc) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(param.name));
    put_str(&mut out, "desc", param.desc);
    put_list(
        &mut out,
        "shape",
        param.shape,
        |key| json!({ "key": key.key, "type": key.ty, "desc": key.desc }),
    );
    Value::Object(out)
}

/// One thrown error.
fn error_json(error: &ErrorDoc) -> Value {
    json!({ "error": error.error, "desc": error.desc })
}

/// One constant: its name, its type and value, and its one-sentence card as
/// `doc` only when written — a constant's whole card is one string, so there
/// is no object to leave empty.
fn constant_json(constant: &CoreConst) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(constant.name));
    out.insert("type".into(), Value::from(ty_string(&constant.ty)));
    out.insert("value".into(), Value::from(const_string(&constant.value)));
    put_str(&mut out, "doc", constant.desc);
    Value::Object(out)
}

/// One enum: its name, and its `doc` only when the row carries one.
fn enum_json(declared: &CoreEnum) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(declared.name));
    if let Some(doc) = declared.doc {
        out.insert("doc".into(), enum_doc_json(doc));
    }
    Value::Object(out)
}

/// One enum's card, with every empty field left out.
fn enum_doc_json(doc: &EnumDoc) -> Value {
    let mut out = Map::new();
    put_str(&mut out, "short", doc.short);
    put_list(
        &mut out,
        "cases",
        doc.cases,
        |case| json!({ "name": case.name, "desc": case.desc }),
    );
    Value::Object(out)
}

// ============================================================================
// The program half (`rule:tooling/meta-json-takes-a-program`)
// ============================================================================

/// The four rosters a program contributes, each in the order its declarations
/// were loaded and then written.
///
/// One accumulator rather than four out-parameters, because the walk that
/// fills them is one walk: a declaration's roster is decided by its statement
/// kind, and the namespace it sits in is the same fact for all four.
#[derive(Default)]
struct Declared {
    classes: Vec<Value>,
    interfaces: Vec<Value>,
    enums: Vec<Value>,
    types: Vec<Value>,
}

/// Every declaration of the program `checked` is, in the registry's own shape.
///
/// The order is `nvs_hir::resolve_program`'s: the entry file first, then each
/// file its `require`/`autoload` graph reached, and within a file the order
/// they were written. A roster with nothing in it is left out, the same
/// omission rule the registry half follows.
fn program_json(checked: &crate::Checked) -> Value {
    let mut declared = Declared::default();
    for file in &checked.files {
        let src = checked.map.file(file.id);
        collect(&file.stmts, src, "", &mut declared);
    }
    let mut out = Map::new();
    put_values(&mut out, "classes", declared.classes);
    put_values(&mut out, "interfaces", declared.interfaces);
    put_values(&mut out, "enums", declared.enums);
    put_values(&mut out, "types", declared.types);
    Value::Object(out)
}

/// Walks one statement list, routing each declaration to its roster.
///
/// `enclosing` is the namespace the list already sits in. A bracketed
/// `namespace N { ... }` recurses with `N`; the statement form has no body and
/// applies to the rest of *this* list, which is what the running local holds.
/// Nothing else descends: a declaration inside a function body or a branch is
/// not a declaration this language has.
fn collect(stmts: &[Stmt], src: &SourceFile, enclosing: &str, out: &mut Declared) {
    let mut namespace = enclosing.to_owned();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(decl) => {
                let name = decl
                    .name
                    .map(|name| text(src, name.span).to_owned())
                    .unwrap_or_default();
                match &decl.body {
                    Some(block) => collect(&block.stmts, src, &name, out),
                    None => namespace = name,
                }
            }
            StmtKind::ClassDecl(decl) => out.classes.push(user_class_json(decl, src, &namespace)),
            StmtKind::InterfaceDecl(decl) => {
                out.interfaces
                    .push(user_interface_json(decl, src, &namespace));
            }
            StmtKind::EnumDecl(decl) => out.enums.push(user_enum_json(decl, src, &namespace)),
            StmtKind::TypeAliasDecl(decl) => out.types.push(user_alias_json(
                decl,
                decl.doc.as_ref(),
                src,
                qualify(&namespace, text(src, decl.name.span)),
            )),
            _ => {}
        }
    }
}

/// One user class: its name, its members, its constants and its own card —
/// [`class_json`]'s shape, minus the `typeParams` and `constructor` keys a
/// registry class carries because the compiler owns them.
fn user_class_json(decl: &ClassDecl, src: &SourceFile, namespace: &str) -> Value {
    let mut out = Map::new();
    let name = qualify(namespace, text(src, decl.name.span));
    out.insert("name".into(), Value::from(name.as_str()));
    body_json(&decl.members, src, &name, &mut out);
    put_doc(&mut out, decl.doc.as_ref(), src);
    Value::Object(out)
}

/// One user interface, in a class's shape: an interface declares members and
/// constants exactly as a class does, and a consumer rendering either renders
/// the same keys.
fn user_interface_json(decl: &InterfaceDecl, src: &SourceFile, namespace: &str) -> Value {
    let mut out = Map::new();
    let name = qualify(namespace, text(src, decl.name.span));
    out.insert("name".into(), Value::from(name.as_str()));
    body_json(&decl.members, src, &name, &mut out);
    put_doc(&mut out, decl.doc.as_ref(), src);
    Value::Object(out)
}

/// The `members`, `constants` and `types` of a class or interface body, under
/// `owner`'s own qualified name.
///
/// `members` is always present, empty or not, because the registry's is; a
/// class with no constants has no `constants` key, because the registry's does
/// not either, and a body owning no alias has no `types` key for the same
/// reason. A member the parser recovered over contributes nothing — it
/// has no name to be documented under.
fn body_json(members: &[ClassMember], src: &SourceFile, owner: &str, out: &mut Map<String, Value>) {
    let mut declared = Vec::new();
    let mut constants = Vec::new();
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(method) => {
                declared.push(user_method_json(method, member.doc.as_ref(), src));
            }
            ClassMemberKind::Property(property) => {
                declared.push(user_property_json(property, member.doc.as_ref(), src));
            }
            ClassMemberKind::Const(constant) => {
                constants.push(user_constant_json(constant, member.doc.as_ref(), src));
            }
            _ => {}
        }
    }
    out.insert("members".into(), Value::Array(declared));
    put_values(out, "constants", constants);
    put_values(out, "types", alias_cards(members, src, owner));
}

/// One user method: its name, its `kind`, its signature as written, and its
/// card.
///
/// `kind` is the registry's own vocabulary, `constructor` included: a class's
/// constructor is spelled `constructor` in Novis, so the name decides it and no
/// second rule is needed.
fn user_method_json(method: &MethodMember, doc: Option<&DocComment>, src: &SourceFile) -> Value {
    let kind = match text(src, method.name) {
        "constructor" => "constructor",
        _ if method.modifiers.contains(&Modifier::Static) => "static",
        _ => "instance",
    };
    let params: Vec<String> = method
        .params
        .iter()
        .map(|param| param_signature(param, src))
        .collect();
    let mut signature = format!("{}({})", text(src, method.name), params.join(", "));
    if let Some(ret) = &method.return_type {
        signature.push_str(": ");
        signature.push_str(text(src, ret.span));
    }
    let mut out = Map::new();
    out.insert("name".into(), Value::from(text(src, method.name)));
    out.insert("kind".into(), Value::from(kind));
    out.insert(
        "visibility".into(),
        Value::from(visibility(&method.modifiers)),
    );
    out.insert("signature".into(), Value::from(signature));
    put_doc(&mut out, doc, src);
    Value::Object(out)
}

/// One parameter as it was written — `inout` and `...` included, and the
/// default as its own source text.
///
/// Read off the source rather than re-printed from the tree for the reason
/// every span here is: the source is the text, so a spelling the grammar
/// accepts can never come back out as a spelling it does not.
fn param_signature(param: &Param, src: &SourceFile) -> String {
    let mut out = String::new();
    if param.inout {
        out.push_str("inout ");
    }
    if let Some(ty) = &param.ty {
        out.push_str(text(src, ty.span));
        out.push(' ');
    }
    if param.variadic {
        out.push_str("...");
    }
    out.push_str(text(src, param.name));
    if let Some(default) = &param.default {
        out.push_str(" = ");
        out.push_str(text(src, default.span));
    }
    out
}

/// One member's visibility, which is `public` where none was written — the
/// visibility a member has, not the one it spelled.
///
/// The registry half has no counterpart: a `Core` member is public or it is not
/// in the registry. Carried on the program half because a renderer over this
/// document has to know what a package's *reader* can reach — `nvs doc` shows
/// the public surface, and `nvs check --strict-docs` reports the same set from
/// the other side (`rule:tooling/strict-docs`). `private(set)` is a visibility
/// for writes rather than for the declaration and is not it.
fn visibility(modifiers: &[Modifier]) -> &'static str {
    if modifiers.contains(&Modifier::Private) {
        "private"
    } else if modifiers.contains(&Modifier::Protected) {
        "protected"
    } else {
        "public"
    }
}

/// One user property: its name with the `$` a program writes it with, its
/// declared type as its signature, and its card.
fn user_property_json(
    property: &PropertyMember,
    doc: Option<&DocComment>,
    src: &SourceFile,
) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(text(src, property.name)));
    out.insert("kind".into(), Value::from("property"));
    out.insert(
        "visibility".into(),
        Value::from(visibility(&property.modifiers)),
    );
    out.insert(
        "signature".into(),
        Value::from(format!(
            "{} {}",
            text(src, property.ty.span),
            text(src, property.name)
        )),
    );
    put_doc(&mut out, doc, src);
    Value::Object(out)
}

/// One user constant: [`constant_json`]'s keys, with `type` absent where the
/// declaration left the type out — which PHP 8.3 allows and this document
/// reports rather than guesses.
fn user_constant_json(constant: &ConstMember, doc: Option<&DocComment>, src: &SourceFile) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(text(src, constant.name)));
    out.insert(
        "visibility".into(),
        Value::from(visibility(&constant.modifiers)),
    );
    if let Some(ty) = &constant.ty {
        out.insert("type".into(), Value::from(text(src, ty.span)));
    }
    out.insert("value".into(), Value::from(text(src, constant.value.span)));
    put_doc(&mut out, doc, src);
    Value::Object(out)
}

/// One user enum: its name, its backing type when written, its cases with
/// their own cards, and its own card.
///
/// A case's card sits on the case rather than in the enum's, which is where
/// the registry's [`EnumDoc`] keeps it — a `Core` enum's cases are documented
/// in one Rust literal, and a program's are documented each above its own
/// line.
fn user_enum_json(decl: &EnumDecl, src: &SourceFile, namespace: &str) -> Value {
    let mut out = Map::new();
    let name = qualify(namespace, text(src, decl.name.span));
    out.insert("name".into(), Value::from(name.as_str()));
    if let Some(backing) = &decl.backing {
        out.insert("backing".into(), Value::from(text(src, backing.span)));
    }
    let cases: Vec<Value> = decl
        .cases
        .iter()
        .map(|case| {
            let mut out = Map::new();
            out.insert("name".into(), Value::from(text(src, case.name.span)));
            if let Some(value) = &case.value {
                out.insert("value".into(), Value::from(text(src, value.span)));
            }
            put_doc(&mut out, case.doc.as_ref(), src);
            Value::Object(out)
        })
        .collect();
    out.insert("cases".into(), Value::Array(cases));
    put_values(&mut out, "types", alias_cards(&decl.members, src, &name));
    put_doc(&mut out, decl.doc.as_ref(), src);
    Value::Object(out)
}

/// One `type` alias: its name, the type expression it stands for, and its card.
///
/// `name` is the spelling a program reaches the alias by, and it is the whole
/// difference between the two declaration sites `rule:types/type-alias` names:
/// `N\Greeting` for one written at file scope, `N\Greeter::Card` for one a body
/// owns. `doc` is the `///` run above it, which hangs on the [`ClassMember`] at
/// the second site and on the declaration itself at the first.
fn user_alias_json(
    decl: &TypeAliasDecl,
    doc: Option<&DocComment>,
    src: &SourceFile,
    name: String,
) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(name));
    out.insert("type".into(), Value::from(text(src, decl.ty.span)));
    put_doc(&mut out, doc, src);
    Value::Object(out)
}

/// The `type` aliases a body owns, each named `Owner::Name` under `owner`'s own
/// already-qualified name.
///
/// They go in the roster the file-scope form already fills rather than beside
/// `members`: an alias declares no value, takes no visibility and is gone by
/// runtime, so a consumer rendering both declaration sites renders one card
/// shape twice instead of telling two apart.
fn alias_cards(members: &[ClassMember], src: &SourceFile, owner: &str) -> Vec<Value> {
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::TypeAlias(alias) => Some(user_alias_json(
                alias,
                member.doc.as_ref(),
                src,
                format!("{owner}::{}", text(src, alias.name.span)),
            )),
            _ => None,
        })
        .collect()
}

/// Inserts `doc` only where a `///` run was written above the declaration.
///
/// A run that says nothing still emits an empty object, exactly as an unwritten
/// registry card does: the absent key means "not documented", and an empty one
/// means "documented, and the author wrote nothing" — a distinction
/// `nvs check --strict-docs` is about to depend on.
fn put_doc(out: &mut Map<String, Value>, doc: Option<&DocComment>, src: &SourceFile) {
    let Some(doc) = doc else {
        return;
    };
    let mut card = Map::new();
    let prose = prose(doc, src);
    if !prose.is_empty() {
        card.insert("short".into(), Value::from(prose));
    }
    for (key, kind) in [("see", DocTagKind::See), ("example", DocTagKind::Example)] {
        let named: Vec<Value> = doc
            .tags
            .iter()
            .filter(|tag| tag.kind == kind)
            .map(|tag| Value::from(text(src, tag.argument).trim()))
            .collect();
        put_values(&mut card, key, named);
    }
    out.insert("doc".into(), Value::Object(card));
}

/// A `///` run's prose: every line that is not a tag, with its marker off.
///
/// One space after the marker is the separator and comes off with it; anything
/// further in is the author's own indentation, which a fenced code block in the
/// Markdown depends on. A tag line is one whose text begins with `@`, and that
/// is exact rather than approximate — the parser refused every spelling but the
/// two, so nothing else can start that way.
fn prose(doc: &DocComment, src: &SourceFile) -> String {
    let lines: Vec<&str> = doc
        .lines
        .iter()
        .map(|line| {
            let line = text(src, *line).trim_start();
            let line = line.strip_prefix("///").unwrap_or(line);
            line.strip_prefix(' ').unwrap_or(line)
        })
        .filter(|line| !line.trim_start().starts_with('@'))
        .collect();
    lines.join("\n").trim().to_owned()
}

/// `namespace` and `name` joined the way a program writes a qualified name,
/// and `name` alone at file scope.
fn qualify(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{namespace}\\{name}")
    }
}

/// A span's source text, or the empty string where the file cannot answer —
/// which a span this walk holds never is, every one of them having come from
/// this very file's parse.
fn text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

/// Inserts `key` as an array only when at least one value was built.
///
/// [`put_list`]'s rule over values that are already [`Value`]s, so the program
/// half omits an empty roster for the same reason the registry half does.
fn put_values(out: &mut Map<String, Value>, key: &str, values: Vec<Value>) {
    if !values.is_empty() {
        out.insert(key.into(), Value::Array(values));
    }
}

/// Inserts `key` only when `value` is written.
fn put_str(out: &mut Map<String, Value>, key: &str, value: &'static str) {
    if !value.is_empty() {
        out.insert(key.into(), Value::from(value));
    }
}

/// Inserts `key` as an array only when `items` holds at least one.
fn put_list<T>(out: &mut Map<String, Value>, key: &str, items: &[T], each: impl Fn(&T) -> Value) {
    if !items.is_empty() {
        out.insert(key.into(), Value::Array(items.iter().map(each).collect()));
    }
}

#[cfg(test)]
mod tests {
    use nvs_stdlib::registry::{CaseDoc, ShapeKeyDoc};

    use super::*;

    /// A card with every field written, including a shape-typed parameter —
    /// which no implemented member declares yet, so the emitter's half of
    /// [`ShapeKeyDoc`] is held here rather than through the binary.
    const FULL: MethodDoc = MethodDoc {
        short: "Does a thing.",
        params: &[ParamDoc {
            name: "parts",
            desc: "The parts.",
            shape: &[ShapeKeyDoc {
                key: "hour",
                ty: "int",
                desc: "The hour.",
            }],
        }],
        ret: "The thing.",
        errors: &[ErrorDoc {
            error: "LogicError",
            desc: "When it cannot.",
        }],
    };

    /// A card with nothing written yet.
    const EMPTY: MethodDoc = MethodDoc {
        short: "",
        params: &[],
        ret: "",
        errors: &[],
    };

    /// An enum's card with both fields written.
    const FULL_ENUM: EnumDoc = EnumDoc {
        short: "Chooses a thing.",
        cases: &[CaseDoc {
            name: "One",
            desc: "The first thing.",
        }],
    };

    /// An enum's card with nothing written yet.
    const EMPTY_ENUM: EnumDoc = EnumDoc {
        short: "",
        cases: &[],
    };

    /// § 2's shape, key for key, from a card that fills every field.
    #[test]
    fn a_full_card_emits_every_field_of_the_contract() {
        assert_eq!(
            doc_json(&FULL),
            json!({
                "short": "Does a thing.",
                "params": [ { "name": "parts", "desc": "The parts.",
                              "shape": [ { "key": "hour", "type": "int", "desc": "The hour." } ] } ],
                "return": "The thing.",
                "errors": [ { "error": "LogicError", "desc": "When it cannot." } ]
            })
        );
    }

    /// An unwritten field is an absent key, never an empty one — the module
    /// doc's *What is omitted* rule.
    #[test]
    fn an_empty_card_emits_an_empty_object() {
        assert_eq!(doc_json(&EMPTY), json!({}));
    }

    /// § 2's enum shape, key for key.
    #[test]
    fn a_full_enum_card_emits_every_field_of_the_contract() {
        assert_eq!(
            enum_doc_json(&FULL_ENUM),
            json!({
                "short": "Chooses a thing.",
                "cases": [ { "name": "One", "desc": "The first thing." } ]
            })
        );
    }

    /// The same omission rule, on an enum's card.
    #[test]
    fn an_empty_enum_card_emits_an_empty_object() {
        assert_eq!(enum_doc_json(&EMPTY_ENUM), json!({}));
    }

    /// A constant with nothing written is its name alone; one with a card
    /// carries it as `doc`.
    #[test]
    fn a_constant_carries_its_doc_only_when_written() {
        let unwritten = CoreConst {
            name: "X",
            ty: nvs_stdlib::registry::CoreTy::Int,
            value: nvs_stdlib::registry::Const::Int(1),
            desc: "",
        };
        assert_eq!(
            constant_json(&unwritten),
            json!({ "name": "X", "type": "int", "value": "1" })
        );
        let written = CoreConst {
            desc: "One.",
            ..unwritten
        };
        assert_eq!(
            constant_json(&written),
            json!({ "name": "X", "type": "int", "value": "1", "doc": "One." })
        );
    }

    /// The signature half spells a row the way the spec's column does:
    /// defaults aligned to the end, a bag last, a written type argument on
    /// the name — read off rows the registry ships.
    #[test]
    fn a_signature_is_the_specs_own_spelling() {
        let find = |class: &str, member: &str| -> &'static CoreMethod {
            nvs_stdlib::registry::class(class)
                .expect("a registered class")
                .members()
                .find(|row| row.name == member)
                .expect("a registered member")
        };
        assert_eq!(
            signature(find(r"Core\Str", "length")),
            "length(string $s): uint"
        );
        assert_eq!(
            signature(find(r"Core\Str", "format")),
            "format(string $template, mixed ...$arguments): string"
        );
        assert!(signature(find(r"Core\Arr", "sort")).starts_with("sort(array<T> $a, {"));
        assert!(signature(find(r"Core\Json", "decodeAs")).starts_with("decodeAs<T>("));
    }

    /// Every member of every class carries its signature keys, and every
    /// roster beside the registry is a non-empty table.
    #[test]
    fn every_member_carries_its_signature_half_and_every_roster_is_present() {
        let document = document();
        for class in document["classes"].as_array().expect("an array") {
            for member in class["members"].as_array().expect("an array") {
                assert!(member["kind"].is_string(), "{member}");
                assert!(member["signature"].is_string(), "{member}");
                assert!(member["returns"].is_string(), "{member}");
            }
        }
        for roster in ["exceptions", "interfaces", "attributes", "directives"] {
            assert!(
                !document[roster].as_array().expect("an array").is_empty(),
                "{roster} is empty"
            );
        }
        assert_eq!(document["exceptions"][0]["name"], "Throwable");
        assert!(document["exceptions"][0]["properties"].is_array());
    }

    /// Every class in the registry appears, and every one of its members —
    /// methods and then instance members, in the rosters' own order — and
    /// every one of its constants, in roster order, under a key that is absent
    /// for a class with none.
    #[test]
    fn the_document_lists_every_class_member_and_constant_in_registry_order() {
        let document = document();
        let classes = document["classes"].as_array().expect("an array");
        assert_eq!(classes.len(), CLASSES.len());
        for (class, emitted) in CLASSES.iter().zip(classes) {
            assert_eq!(emitted["name"], class.name);
            let names: Vec<&str> = emitted["members"]
                .as_array()
                .expect("an array")
                .iter()
                .map(|m| m["name"].as_str().expect("a name"))
                .collect();
            let expected: Vec<&str> = class
                .methods
                .iter()
                .chain(class.instance)
                .map(|m| m.name)
                .collect();
            assert_eq!(names, expected, "{}", class.name);

            let constants: Vec<&str> = emitted["constants"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|c| c["name"].as_str().expect("a name"))
                        .collect()
                })
                .unwrap_or_default();
            let expected: Vec<&str> = class.constants.iter().map(|c| c.name).collect();
            assert_eq!(constants, expected, "{}", class.name);
        }
    }

    /// Every enum in the registry appears, top-level, in the roster's order.
    #[test]
    fn the_document_lists_every_enum_in_roster_order() {
        let document = document();
        let names: Vec<&str> = document["enums"]
            .as_array()
            .expect("an array")
            .iter()
            .map(|e| e["name"].as_str().expect("a name"))
            .collect();
        let expected: Vec<&str> = ENUMS.iter().map(|e| e.name).collect();
        assert_eq!(names, expected);
    }
}
