//! `rule:core-classes/derive-attribute`'s two derived codecs, lowered: a `#[Json\Derive]` class fills
//! `ir::Class::codec`, a `#[Db\Derive]` one fills `ir::Class::db_codec`, and
//! neither fills the other's.
//!
//! The two lists exist because the two mappings only *usually* agree — a
//! property may carry `#[Json\Field(name: …)]` and `#[Db\Field(name: …)]` at
//! once, with a different wire key under each — so what this asserts is the
//! keys per format rather than that a codec is non-empty. A join that read one
//! table for both formats passes a did-it-lower test and fails the third case
//! below.
//!
//! `ctor_arity` is asserted on the `#[Db\Derive]`-only class for the reason
//! `nvs_runtime::ClassTable::set_db_codec` states: the arity belongs to the
//! `constructor` and not to either mapping, so a class carrying only the row
//! attribute must still arrive with it.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::{Class, Program};

/// Parse, resolve, check, lower — `nvs-cli`'s own `front_end` order, panicking
/// on the first phase that reports, since every fixture here is meant to
/// compile clean.
fn compile(src: &str) -> Program {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();

    let stmts = nvs_syntax::parse_file(map.file(id), &mut diags);
    nvs_syntax::check_declarations(&stmts, map.file(id), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");

    let module = nvs_hir::resolve_file(&stmts, map.file(id), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");

    let files = [nvs_types::ProgramFile {
        src: map.file(id),
        stmts: &stmts,
    }];
    let mut interner = nvs_types::TypeInterner::new();
    let mut exprs = nvs_types::ExprTypeTable::new();
    let enums = nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

    let layouts = nvs_types::build_class_layouts(&files, &module.graph);
    nvs_ir::lower::lower_program("<script>", &files, &exprs, &interner, &enums, &layouts)
}

/// The lowered class whose label ends in `name`, which is how a declaration in
/// a one-file fixture is spelled once the unit prefix is on it.
fn class<'a>(program: &'a Program, name: &str) -> &'a Class {
    program
        .classes
        .iter()
        .find(|class| class.label == name || class.label.ends_with(&format!("\\{name}")))
        .unwrap_or_else(|| {
            panic!(
                "no lowered class `{name}` — the unit has {:?}",
                program
                    .classes
                    .iter()
                    .map(|class| &class.label)
                    .collect::<Vec<_>>()
            )
        })
}

/// Each field's wire key, in the order the codec carries them.
fn keys(codec: &[nvs_types::CodecField]) -> Vec<&str> {
    codec.iter().map(|field| field.key.as_str()).collect()
}

#[test]
fn a_json_derive_class_lowers_a_json_codec_and_no_row_one() {
    let program = compile(
        r#"<?nvs
#[Core\Json\Derive]
class Payload {
    public int $id;
    public string $name;

    public function constructor(int $id, string $name) {
        $this->id = $id;
        $this->name = $name;
    }
}
"#,
    );
    let payload = class(&program, "Payload");
    assert_eq!(keys(&payload.codec), ["id", "name"]);
    assert!(
        payload.db_codec.is_empty(),
        "a class carrying no `#[Db\\Derive]` has no row mapping"
    );
}

#[test]
fn a_db_derive_class_lowers_a_row_codec_and_no_json_one() {
    let program = compile(
        r#"<?nvs
#[Core\Db\Derive]
class Person {
    public int $id;
    public tainted string $name;

    public function constructor(int $id, tainted string $name) {
        $this->id = $id;
        $this->name = $name;
    }
}
"#,
    );
    let person = class(&program, "Person");
    assert_eq!(keys(&person.db_codec), ["id", "name"]);
    assert!(
        person.codec.is_empty(),
        "a class carrying no `#[Json\\Derive]` has no JSON mapping"
    );
    // The slot and parameter joins are the whole point of doing this in
    // `nvs-ir` rather than reading `nvs_types::derive`'s answer: a field is
    // dropped rather than mis-indexed, so a codec that survives the join at
    // full length has both.
    assert_eq!(
        person
            .db_codec
            .iter()
            .map(|field| (field.slot, field.param))
            .collect::<Vec<_>>(),
        [(0, 0), (1, 1)]
    );
    assert_eq!(
        person.ctor_arity, 2,
        "the arity belongs to the constructor, not to either mapping"
    );
}

#[test]
fn a_class_carrying_both_derives_lowers_each_formats_own_keys() {
    let program = compile(
        r#"<?nvs
#[Core\Json\Derive]
#[Core\Db\Derive]
class Account {
    #[Core\Json\Field(name: "userId")]
    #[Core\Db\Field(name: "user_id")]
    public int $user;

    public function constructor(int $user) {
        $this->user = $user;
    }
}
"#,
    );
    let account = class(&program, "Account");
    assert_eq!(keys(&account.codec), ["userId"]);
    assert_eq!(keys(&account.db_codec), ["user_id"]);
}
