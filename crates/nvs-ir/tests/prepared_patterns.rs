//! `rule:expressions/intrinsic-list-is-closed`'s second effect, lowered: what
//! the checker prepared out of a written literal reaches the member as
//! `nvs_stdlib::registry::PREPARED_MEMBERS`' argument 0.
//!
//! The channel is the thing under test, not the tier —
//! `nvs-types`' own `intrinsics` suite owns which engine a pattern routes to,
//! and asserting it again here would be a second opinion about a fact this
//! crate only carries. What can only be asserted here is that it *arrives*: at
//! the right call, in the right slot, and with the zero word written for a call
//! site that prepared nothing, which is what stops a helper from reading past
//! its own arguments.
//!
//! A member off that roster is the other half of the same claim. The slot is
//! not a free constant every `Core` call grew — a roster of two hundred rows
//! paying one instruction each for a fact five of them read would be exactly
//! the open extension point that rule refuses.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::{InstKind, Prepared, Program};

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

/// Every prepared constant the program emitted, in emission order.
fn prepared(program: &Program) -> Vec<Option<Prepared>> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match inst.kind {
            InstKind::PreparedConst { fact } => Some(fact),
            _ => None,
        })
        .collect()
}

/// Every call to `symbol`, as its argument list — `nvs_ir::ir::InstKind::CoreCall`
/// is the one shape a Tier 0 `Core` member is reached through.
fn core_calls<'a>(program: &'a Program, symbol: &str) -> Vec<&'a [nvs_ir::ids::ValueId]> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match &inst.kind {
            InstKind::CoreCall {
                symbol: called,
                args,
            } if *called == symbol => Some(args.as_slice()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_prepared_tier_reaches_the_call_it_was_settled_at() {
    // The three shapes a roster member's argument takes, in written order: a
    // pattern the linear engine expresses, one only the backtracking engine
    // does (a lookbehind), and one the program computed. The third is the
    // reason the slot exists for every call rather than for the ones that
    // prepared something — nothing is refused for being dynamic, so nothing may
    // be *omitted* for it either.
    let program = compile(
        "<?nvs
var $text = '[a-z]+';
var $linear = Core\\Regex::compile('[a-z]+\\d{2,3}');
var $fancy = Core\\Regex::compile('(?<=USD )\\d+');
var $built = Core\\Regex::compile($text);
",
    );

    assert_eq!(
        prepared(&program),
        vec![
            Some(Prepared::RegexLinear),
            Some(Prepared::RegexBacktracking),
            None,
        ],
        "the tiers the checker settled did not reach the calls they were settled at"
    );
}

#[test]
fn the_prepared_word_is_argument_zero_of_every_call_on_the_roster() {
    // The ABI half: the word is ahead of the member's own arguments, and the
    // value in that slot is the constant this call emitted rather than some
    // earlier one's. A channel that filed the fact under the literal's span
    // would still pass the test above with two calls in one program and fail
    // this one.
    let program = compile(
        "<?nvs
var $fancy = Core\\Regex::compile('(?<=USD )\\d+');
",
    );

    let words: Vec<_> = program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter(|inst| matches!(inst.kind, InstKind::PreparedConst { .. }))
        .filter_map(|inst| inst.result)
        .collect();
    let calls = core_calls(&program, "nvs_core_regex_compile");

    assert_eq!(calls.len(), 1, "the fixture lowered no `compile` call");
    assert_eq!(
        calls[0].len(),
        6,
        "`compile` takes its pattern, the four flags of its options bag, and the prepared word"
    );
    assert_eq!(
        words.first().copied(),
        calls[0].first().copied(),
        "the prepared word is not the call's argument 0"
    );
}

#[test]
fn a_member_off_the_roster_is_handed_no_prepared_word() {
    // `matches` reads the same pattern language and is not on the roster: it
    // takes the pattern as an ordinary argument and compiles it at the call.
    // What it must not have grown is a slot, which is what keeps the roster's
    // cost proportional to the members that read one.
    let program = compile(
        "<?nvs
var $hit = Core\\Regex::matches('USD 12', '[a-z]+');
",
    );

    assert!(
        prepared(&program).is_empty(),
        "a member off `PREPARED_MEMBERS` emitted a prepared constant"
    );
    let calls = core_calls(&program, "nvs_core_regex_matches");
    assert_eq!(calls.len(), 1, "the fixture lowered no `matches` call");
    assert_eq!(
        calls[0].len(),
        2,
        "`matches` grew an argument it has no roster row for"
    );
}
