//! ``html`…` `` end to end — what a markup literal builds, and what its bytes are.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;
use nvs_ir::ir::{Helper, InstKind};

/// How many of the whole lowered program's instructions `matching` accepts —
/// the cost half of `rule:core-classes/html-literal` is a claim about what is
/// emitted, so these read the IR rather than the output.
fn count_insts(source: &str, matching: impl Fn(&InstKind) -> bool) -> usize {
    lower(source)
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.insts.iter())
        .filter(|inst| matching(&inst.kind))
        .count()
}

/// How many `Core\Html\Markup` carriers `source` builds: one for a literal in
/// value position, never one per hole, and none at all for one at a sink.
fn carriers_built(source: &str) -> usize {
    count_insts(source, |kind| {
        matches!(kind, InstKind::CoreCall { symbol, .. }
            if *symbol == nvs_types::CORE_HTML_MARKUP)
    })
}

/// How many constants `source` folds a whole literal into — the shape that
/// builds no carrier because it needs none.
fn constants_folded(source: &str) -> usize {
    count_insts(source, |kind| matches!(kind, InstKind::ConstMarkup(_)))
}

/// How many writes `source` makes to the output — one per piece is what a
/// literal in a sink position lowers to.
fn writes(source: &str) -> usize {
    count_insts(source, |kind| {
        matches!(
            kind,
            InstKind::HelperCall {
                helper: Helper::EchoMarkup,
                ..
            }
        )
    })
}

/// How many allocation requests the **run** of `source` makes — the compile is
/// outside the window on purpose, and so is whatever the runtime initializes on
/// its first pass through a shape, which the warm-up call pays for.
///
/// The counter is `nvs_runtime::budget`'s, which every build maintains because
/// the memory limit is read off it. Nothing here pins a number: the assertion
/// is one run's count against another's, so an inlining difference moves both
/// sides of it. `tests/arrays.rs` measures the packed array the same way and
/// pins numbers instead, which is why that copy is gated to a debug build.
fn allocations_of_run(source: &str) -> usize {
    let unit = compile(source).expect("the fixture compiles");
    let entry = unit.script().expect("the script frame was compiled");
    let mut ctx = Ctx::buffered();
    entry.call(&mut ctx).expect("the script ran to completion");
    let before = nvs_runtime::budget::allocations();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::allocations() - before
}

/// A literal held in a local: the value escapes the sink, so the carrier is
/// real and the hole is escaped into it.
const ASSIGNED: &str = r#"<?nvs
string $name = "<b>Ann</b>";
Core\Html\Markup $posted = html`<span>posted by {$name}</span>`;
echo Core\Html::toSource($posted, "the test reads back the bytes the literal built");
"#;

/// The same literal as a method's answer — the other half of value position,
/// and the one where the carrier outlives the frame that made it.
const RETURNED: &str = r#"<?nvs
class Page {
    public static function badge(string $name): Core\Html\Markup {
        return html`<b>{$name}</b>`;
    }
}
echo Core\Html::toSource(Page::badge("<x>"), "the test reads back the bytes the literal built");
"#;

/// The same literal at a sink: born and consumed in one place, so it is three
/// writes and no carrier.
const ECHOED: &str = r#"<?nvs
string $name = "<b>Ann</b>";
echo html`<span>posted by {$name}</span>`;
"#;

/// A literal with nothing to fill in, held in a local — the shape whose bytes
/// are known while it is being compiled.
const FOLDED: &str = r#"<?nvs
Core\Html\Markup $posted = html`<span>posted by Ann</span>`;
echo Core\Html::toSource($posted, "the test reads back the bytes the literal built");
"#;

#[test]
fn a_markup_literal_echoed_lowers_to_writes_and_builds_no_markup() {
    assert_eq!(
        output_of(ECHOED),
        "<span>posted by &lt;b&gt;Ann&lt;/b&gt;</span>"
    );
    assert_eq!(carriers_built(ECHOED), 0);
    assert_eq!(writes(ECHOED), 3);
}

#[test]
fn a_markup_literal_assigned_to_a_local_does_build_one() {
    assert_eq!(
        output_of(ASSIGNED),
        "<span>posted by &lt;b&gt;Ann&lt;/b&gt;</span>"
    );
    assert_eq!(carriers_built(ASSIGNED), 1);
}

#[test]
fn a_markup_literal_returned_from_a_function_does_build_one() {
    assert_eq!(output_of(RETURNED), "<b>&lt;x&gt;</b>");
    assert_eq!(carriers_built(RETURNED), 1);
}

#[test]
fn a_hole_holding_a_carrier_is_spliced_raw_and_the_literal_still_builds_one() {
    // `rule:core-classes/html-literal`: a hole already holding a `Markup` is
    // `Markup + Markup` written in interpolation syntax, so its markup reaches
    // the page as markup — and the composition is still one carrier, not one
    // per fragment.
    let source = r#"<?nvs
Core\Html\Markup $inner = html`<em>Ann</em>`;
Core\Html\Markup $posted = html`<span>posted by {$inner}</span>`;
echo Core\Html::toSource($posted, "the test reads back the bytes the literal built");
"#;
    assert_eq!(output_of(source), "<span>posted by <em>Ann</em></span>");
    assert_eq!(carriers_built(source), 1);
    assert_eq!(constants_folded(source), 1);
}

#[test]
fn a_segment_escapes_the_two_delimiters_the_literal_adds_and_nothing_else() {
    // The segment grammar is the double-quoted one plus `` \` `` and `\{`;
    // `<` is trusted text and stays the byte the author wrote, and `&amp;` is
    // text that said `&amp;`.
    let source = r#"<?nvs
Core\Html\Markup $m = html`<b>a\`b</b> \{not a hole} &amp; \t`;
echo Core\Html::toSource($m, "the test reads back the bytes the literal built");
"#;
    assert_eq!(output_of(source), "<b>a`b</b> {not a hole} &amp; \t");
}

/// `rule:core-classes/html-literal`'s *What it costs to run*, first half: a
/// literal with no hole in it is constant-pool data, so it is one
/// `InstKind::ConstMarkup` and no lift at all — where the same literal with a
/// hole is the join and the carrier the tests above count.
#[test]
fn a_hole_free_markup_literal_folds_to_one_constant() {
    assert_eq!(output_of(FOLDED), "<span>posted by Ann</span>");
    assert_eq!(constants_folded(FOLDED), 1);
    assert_eq!(carriers_built(FOLDED), 0);

    // The control, and the reason this test can fail at all: nothing folds a
    // literal whose bytes are not known until it runs.
    assert_eq!(constants_folded(ASSIGNED), 0);
}

/// The second half, which a loop is what actually exposes: the constant is
/// built once, while compiling, so what a run spends does not depend on how
/// many times the literal is evaluated.
#[test]
fn a_hole_free_markup_literal_in_a_loop_allocates_once_for_the_whole_loop() {
    let loops = |rounds: u32| {
        format!(
            r#"<?nvs
Core\Html\Markup $m = html`<i>start</i>`;
int $i = 0;
while ($i < {rounds}) {{
    $m = html`<b>hi</b>`;
    $i = $i + 1;
}}
"#
        )
    };
    assert_eq!(constants_folded(&loops(4)), 2);
    assert_eq!(carriers_built(&loops(4)), 0);
    assert_eq!(
        allocations_of_run(&loops(4)),
        allocations_of_run(&loops(40)),
        "the loop allocated per iteration, so the literal is not one constant"
    );
}
