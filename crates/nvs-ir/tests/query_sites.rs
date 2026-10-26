//! `rule:observability/a-query-span-names-its-call-site`, lowered: each
//! `Core\Db` call that sends a statement passes one constant, the call's site,
//! as `nvs_stdlib::registry::QUERY_SITE_MEMBERS`' last argument.
//!
//! The site is the call and not its SQL, so two calls with one statement text
//! are two sites. A callable reference would pass the zero id, but
//! `$db->query(...)` does not lower yet: its options bag panics the thunk, so
//! that path has no case here.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::{InstKind, Program};

/// Parse, resolve, check, lower — `prepared_patterns.rs`' harness, with the
/// source map handed back for the site table.
fn compile(src: &str) -> (Program, SourceMap) {
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
    let program =
        nvs_ir::lower::lower_program("<script>", &files, &exprs, &interner, &enums, &layouts);
    (program, map)
}

/// Every query site constant the program emitted, `None` for the zero id.
fn site_constants(program: &Program) -> Vec<bool> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match inst.kind {
            InstKind::QuerySiteConst { site } => Some(site.is_some()),
            _ => None,
        })
        .collect()
}

/// The last argument of every call to `symbol` is a query site constant.
fn last_arguments_are_sites(program: &Program, symbol: &str) -> usize {
    let mut calls = 0;
    for function in &program.functions {
        let insts: Vec<_> = function
            .blocks
            .iter()
            .flat_map(|block| &block.insts)
            .collect();
        for inst in &insts {
            let InstKind::CoreCall {
                symbol: called,
                args,
            } = &inst.kind
            else {
                continue;
            };
            if *called != symbol {
                continue;
            }
            calls += 1;
            let last = *args.last().expect("a query member takes arguments");
            let defined = insts
                .iter()
                .find(|other| other.result == Some(last))
                .expect("the last argument is defined in the same function");
            assert!(
                matches!(defined.kind, InstKind::QuerySiteConst { .. }),
                "`{symbol}`'s last argument is the call site id, not {:?}",
                defined.kind
            );
        }
    }
    calls
}

#[test]
fn two_calls_with_the_same_sql_are_two_sites() {
    let src = "<?nvs\n\
               var $db = Core\\Db::connect('main');\n\
               var $a = $db->query('select 1', []);\n\
               var $b = $db->query('select 1', []);\n";
    let (program, map) = compile(src);

    assert_eq!(site_constants(&program), [true, true]);
    assert_eq!(
        last_arguments_are_sites(&program, "nvs_core_db_connection_query"),
        2
    );
    let sites = program.query_site_table(&map);
    let lines: Vec<u32> = sites.iter().map(|site| site.line).collect();
    assert_eq!(
        lines,
        [3, 4],
        "one row per call, in the order codegen numbers them"
    );
    assert!(sites.iter().all(|site| site.file == "t.nvs"));
    assert_eq!(
        sites[0].text_hash,
        blake3::hash(src.as_bytes()).to_hex().to_string(),
        "the hash is BLAKE3 over the file's text"
    );
    assert_eq!(sites[0].text_hash, sites[1].text_hash);
}

#[test]
fn every_statement_member_passes_a_site_and_others_do_not() {
    let (program, _) = compile(
        "<?nvs\n\
         var $db = Core\\Db::connect('main');\n\
         var $w = $db->execute('delete from t', []);\n\
         var $n = $db->executeMany('insert into t values (?)', [[1], [2]]);\n\
         foreach ($db->stream('select 1', []) as var $row) { }\n\
         var $d = $db->driver();\n",
    );
    assert_eq!(site_constants(&program), [true, true, true]);
    assert_eq!(
        last_arguments_are_sites(&program, "nvs_core_db_connection_execute"),
        1
    );
    assert_eq!(
        last_arguments_are_sites(&program, "nvs_core_db_connection_execute_many"),
        1
    );
    assert_eq!(
        last_arguments_are_sites(&program, "nvs_core_db_connection_stream"),
        1
    );
}

#[test]
fn the_table_has_one_row_per_site_across_every_function() {
    let (program, map) = compile(
        "<?nvs\n\
         class Notes {\n\
             public static function count(Core\\Db\\Queryable $db): uint {\n\
                 return $db->query('select 1', [])->count();\n\
             }\n\
         }\n\
         var $db = Core\\Db::connect('main');\n\
         var $n = Notes::count($db);\n\
         var $w = $db->execute('delete from t', []);\n",
    );
    let mut lines: Vec<u32> = program
        .query_site_table(&map)
        .iter()
        .map(|site| site.line)
        .collect();
    lines.sort_unstable();
    assert_eq!(lines, [4, 9], "a call through the interface is a site too");
    assert_eq!(program.query_sites().len(), 2);
}
