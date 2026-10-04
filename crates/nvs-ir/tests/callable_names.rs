//! ADR 0006 § *Decision*'s `args:`-by-name, from the one end that can still
//! see a declaration: what a written `Class::method(...)` records on the object
//! it produces, and what an anonymous function deliberately does not.
//!
//! `rule:concurrency/an-upgrade-is-spawn-shaped` makes
//! `Core\Socket::upgrade(Chat::run(...), args: {…})` an isolate entry, and ADR
//! 0006 binds that map to the entry's parameters **by name**. A `callable`
//! carries its arity and its parameter tags and nothing else, so the names are
//! written at the site — `lower::FN_PARAM_NAMES` — and read off the object by
//! `nvs_runtime::callable_param_names`.
//!
//! Both halves are asserted here because neither is observable from a script:
//! `Core\Socket::upgrade` refuses a request no connection offered a slot for,
//! which is every corpus case, so the end-to-end spelling has no home. What the
//! runtime does with what this writes is `nvs-stdlib`'s
//! `an_upgrade_by_static_method_is_the_same_isolate_as_an_upgrade_by_path`.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::{InstKind, Program};

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

/// The reserved field names of every synthesized callable class in `program`,
/// keyed by the class's own label.
///
/// Selected on the reserved first field every environment class carries, which
/// is what tells the synthesized classes from the program's own `Chat`. The `$`
/// in a label says only that no declaration wrote it, and that is true of
/// `nvs_ir::lower::CALLABLE_MARKER`'s field-less descriptor as well.
fn callable_classes(program: &Program) -> Vec<(String, Vec<String>)> {
    program
        .classes
        .iter()
        .filter(|class| {
            class
                .fields
                .first()
                .is_some_and(|first| first == "fn#arity")
        })
        .map(|class| (class.label.clone(), class.fields.clone()))
        .collect()
}

/// Every string constant the program lowers, in no particular order — where a
/// comma-joined name list shows up if one was written at all.
fn const_strings(program: &Program) -> Vec<String> {
    program
        .functions
        .iter()
        .flat_map(|f| f.blocks.iter())
        .flat_map(|b| b.insts.iter())
        .filter_map(|inst| match &inst.kind {
            InstKind::ConstStr(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_method_reference_records_its_targets_parameter_names() {
    // The names are the *target's declared* ones, not the call site's — there
    // is no call site, which is the whole of why they have to be recorded.
    // Third in the field list, which is the index
    // `nvs_runtime::CALLABLE_PARAM_NAMES_SLOT` hints at, and comma-joined in
    // declaration order, which is what `callable_param_names` splits.
    let program = compile(
        "<?nvs
class Chat {
    public static function run(string $room, int $userId): void {}
}
var $entry = Chat::run(...);
",
    );

    let classes = callable_classes(&program);
    assert_eq!(
        classes.len(),
        1,
        "one written `(...)`, one class: {classes:?}"
    );
    assert_eq!(
        classes[0].1,
        vec!["fn#arity", "fn#params", "fn#names"],
        "a method reference's reserved fields, in slot order"
    );
    assert!(
        const_strings(&program).contains(&"room,userId".to_owned()),
        "the target's parameter names were not written at the site: {:?}",
        const_strings(&program)
    );
}

#[test]
fn a_static_target_declaring_no_parameters_records_an_empty_name_list() {
    // The boundary the reader's `filter(|name| !name.is_empty())` exists for:
    // an entry with nothing to bind still records a field, so the answer is an
    // empty list rather than the `None` that means "this callable records no
    // names at all".
    let program = compile(
        "<?nvs
class Chat {
    public static function run(): void {}
}
var $entry = Chat::run(...);
",
    );

    assert_eq!(
        callable_classes(&program)[0].1,
        vec!["fn#arity", "fn#params", "fn#names"],
        "the field is present whatever the target declares"
    );
    assert!(
        const_strings(&program).contains(&String::new()),
        "an empty name list is still written: {:?}",
        const_strings(&program)
    );
}

#[test]
fn an_anon_fn_records_no_parameter_names_at_all() {
    // The absence is load-bearing rather than an omission: a literal's third
    // field is its first *capture*, so `callable_param_names` must be able to
    // tell "no names recorded" from "names recorded, and they are these" — and
    // it does that by asking the descriptor for the field by name. A literal
    // that grew a `fn#names` field would have its capture read as a parameter
    // list by every reader of `rule:security/isolate-shares-nothing`'s binding.
    let program = compile(
        "<?nvs
string $room = \"lobby\";
var $f = fn(int $userId): string => $room;
",
    );

    let classes = callable_classes(&program);
    assert_eq!(classes.len(), 1, "one literal, one class: {classes:?}");
    assert_eq!(
        classes[0].1,
        vec!["fn#arity", "fn#params", "room"],
        "an anonymous function's third field is its first capture"
    );
}
