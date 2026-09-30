//! What a `[limits]` directive is by the time a request reads it — `rule:errors/on-limit`'s ceilings, resolved once by `Ctx::set_config` and read as bare integers
//! everywhere after it.
//!
//! The cases here ask the *reader*: a directive written as a string with a
//! suffix becomes the number the enforcement path compares against, and — for
//! the one ceiling that describes a tree rather than a request — what a child
//! context inherits of it. They ask a breach only where the question is one a
//! `Ctx` answers on its own, which is `max_script_depth` alone: nothing is over
//! that ceiling until an isolate is asked for, so the refusal needs no task.
//! What each of the others does to a request in flight is `nvs-host`'s
//! `tests/limits.rs`, which needs a task to stop and a safepoint to stop it at.

use std::sync::Arc;

use nvs_config::Snapshot;
use nvs_runtime::{Ctx, Fault, Limit, OutputSink};

/// The snapshot `written` resolves to — both halves of it, because the typed tree and the table a
/// directive is read out of are two readers over one file and a case built on half of it would be
/// pinning a shape the boot path cannot produce.
fn snapshot(written: &str) -> Arc<Snapshot> {
    let table: toml::Table = written.parse().expect("the case writes valid TOML");
    Arc::new(Snapshot {
        config: table
            .clone()
            .try_into()
            .expect("the case writes a block this tree has"),
        table,
        ..Snapshot::default()
    })
}

/// The context reading that configuration, output discarded.
fn ctx_reading(written: &str) -> Ctx {
    let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
    ctx.set_config(snapshot(written));
    ctx
}

/// `rule:errors/on-limit` lists CPU time beside memory: `[limits] cpu_time` is a duration, and what the
/// request holds is nanoseconds — `Ctx::cpu_limit`'s field doc owns why it is cached rather than
/// re-derived, and what measures the time against it.
/// The reading is asserted as the two parts it is split into rather than as `cpu_limit` alone,
/// because the directive is not the ceiling ordinary execution gets: § 1's slice is carved out of it
/// exactly as the memory half is carved out of `[limits] memory`, and the case below is the one that
/// pins the split.
#[test]
fn cpu_time_is_read_as_nanoseconds() {
    let plain = ctx_reading("[limits]\ncpu_time = \"2s\"\n");
    assert_eq!(
        plain.cpu_limit() + plain.fatal_reserve_time(),
        2_000_000_000
    );
    let suffixed = ctx_reading("[limits]\ncpu_time = \"250ms\"\n");
    assert_eq!(
        suffixed.cpu_limit() + suffixed.fatal_reserve_time(),
        250_000_000,
        "a suffixed value is the same measurement, not a different one",
    );
}

/// `rule:errors/on-limit` names `fatal_reserve_time` beside `fatal_reserve_memory`, and `Ctx`'s
/// `reserve_time_within` owns the two numbers this asserts: a 50 ms default, and a quarter of the
/// ceiling wherever a quarter is less.
///
/// Every reading is taken as the pair, because "carved out of" is the whole claim and a reserve
/// that was *added to* the ceiling would answer the same `fatal_reserve_time()` on its own.
#[test]
fn the_fatal_reserve_time_is_carved_out_of_the_cpu_ceiling() {
    let defaulted = ctx_reading("[limits]\ncpu_time = \"2s\"\n");
    assert_eq!(defaulted.fatal_reserve_time(), 50_000_000);
    assert_eq!(
        defaulted.cpu_limit(),
        1_950_000_000,
        "the slice comes out of the request's own ceiling, not out of the machine",
    );

    let asked = ctx_reading("[limits]\ncpu_time = \"2s\"\nfatal_reserve_time = \"300ms\"\n");
    assert_eq!(asked.fatal_reserve_time(), 300_000_000);
    assert_eq!(asked.cpu_limit(), 1_700_000_000);

    // The clamp on both sides of where it starts to bite: a quarter of 200ms is exactly the
    // default, and a quarter of anything shorter is less than it. A reserve that stopped one step
    // early would read plausibly against either half alone.
    assert_eq!(
        ctx_reading("[limits]\ncpu_time = \"200ms\"\n").fatal_reserve_time(),
        50_000_000,
        "the shortest ceiling the default still fits inside a quarter of",
    );
    assert_eq!(
        ctx_reading("[limits]\ncpu_time = \"100ms\"\n").fatal_reserve_time(),
        25_000_000,
        "shorter, so the quarter wins over the default",
    );

    // An asked-for reserve is clamped rather than refused, and a request under no cap has nothing
    // to carve however loudly its configuration asks.
    let greedy = ctx_reading("[limits]\ncpu_time = \"1s\"\nfatal_reserve_time = \"10s\"\n");
    assert_eq!(greedy.fatal_reserve_time(), 250_000_000);
    assert_eq!(greedy.cpu_limit(), 750_000_000);
    assert_eq!(
        ctx_reading("[limits]\nfatal_reserve_time = \"1s\"\n").fatal_reserve_time(),
        0,
        "no ceiling is no slice: there is no room for a handler to be given past",
    );
}

/// The three spellings of "no ceiling" answer the same `0`, because a request that may burn any
/// amount of CPU and one whose limit nothing states are the same request downstream. The malformed
/// case is `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s rule seen from here: the file was refused once already, at the boundary
/// that could name the line, so this is not a second place to refuse it.
#[test]
fn an_unstated_uncapped_or_malformed_cpu_time_is_no_ceiling() {
    assert_eq!(
        Ctx::new(OutputSink::Buffer(Vec::new())).cpu_limit(),
        0,
        "a request with no configuration at all",
    );
    assert_eq!(ctx_reading("[limits]\nmemory = \"128M\"\n").cpu_limit(), 0);
    assert_eq!(ctx_reading("[limits]\ncpu_time = false\n").cpu_limit(), 0);
    assert_eq!(
        ctx_reading("[limits]\ncpu_time = \"every other tuesday\"\n").cpu_limit(),
        0,
    );
}

/// `[limits] max_script_depth` is the one ceiling in this block whose *unstated* reading is a
/// number rather than "no cap", and `Ctx::max_script_depth`'s field doc owns why. Asserted here as
/// the whole shape it differs in — unstated, malformed and a written zero all default, and only
/// `false` is off — because a reader that answered `0` for any of those three would still look
/// right on the one line that reads a written value back.
///
/// The zero is the one that has to be asserted from outside: this reader answers a depth and holds
/// `0` as its own sentinel for "no ceiling", so a written zero arriving as a count reaches the same
/// number by the other road, and every net in a deployment that wrote it is off.
#[test]
fn an_unstated_or_malformed_max_script_depth_defaults_while_false_removes_the_ceiling() {
    assert_eq!(
        Ctx::new(OutputSink::Buffer(Vec::new())).max_script_depth(),
        Ctx::DEFAULT_MAX_SCRIPT_DEPTH,
        "a request with no configuration at all is still under the ceiling",
    );
    assert_eq!(
        ctx_reading("[limits]\nmemory = \"128M\"\n").max_script_depth(),
        Ctx::DEFAULT_MAX_SCRIPT_DEPTH,
        "a block that states other ceilings and not this one",
    );
    assert_eq!(
        ctx_reading("[limits]\nmax_script_depth = \"as deep as it goes\"\n").max_script_depth(),
        Ctx::DEFAULT_MAX_SCRIPT_DEPTH,
        "on is the safe direction for a net, so a malformed value is not read as off",
    );
    assert_eq!(
        ctx_reading("[limits]\nmax_script_depth = 0\n").max_script_depth(),
        Ctx::DEFAULT_MAX_SCRIPT_DEPTH,
        "a written zero is read as this reader's own sentinel, so the number an operator is most \
         likely to reach for meaning `no nesting` means `no ceiling` instead",
    );
    assert_eq!(
        ctx_reading("[limits]\nmax_script_depth = \"0\"\n").max_script_depth(),
        Ctx::DEFAULT_MAX_SCRIPT_DEPTH,
        "and the quoted spelling, which is the one every `Core\\Config` value arrives in, takes the \
         same answer by the other parse arm",
    );
    assert_eq!(
        ctx_reading("[limits]\nmax_script_depth = false\n").max_script_depth(),
        0,
        "the one spelling that turns it off, and an operator had to write it",
    );
}

/// A depth written as a bare integer and one written as `rule:config/ini-set-is-core-config-set`'s text spelling are the same
/// reading, and both cross `Core\Config` as text either way. The pair is asserted rather than one
/// of them because `Quantity::parse` reaches them by two different arms.
#[test]
fn max_script_depth_is_read_as_a_count() {
    assert_eq!(
        ctx_reading("[limits]\nmax_script_depth = 8\n").max_script_depth(),
        8
    );
    assert_eq!(
        ctx_reading("[limits]\nmax_script_depth = \"8\"\n").max_script_depth(),
        8,
        "the quoted spelling is the same measurement, not a different one",
    );
}

/// The directive is `System`, so a request may not raise its own recursion ceiling — which is the
/// entire reason `rule:config/three-changeability-classes`'s class was chosen for it, and the one property a reader test can pin
/// without a host to spawn in.
#[test]
fn a_request_cannot_set_its_own_max_script_depth() {
    let mut ctx = ctx_reading("[limits]\nmax_script_depth = 8\n");
    assert!(
        !ctx.config_mut()
            .expect("the case set one")
            .set("max_script_depth", "4096")
    );
    ctx.refresh_limits();
    assert_eq!(
        ctx.max_script_depth(),
        8,
        "the refusal left the file's value in force"
    );
}

/// The ceiling a request tree is charged against is its root's, as it stands now: a `cpu_time` the
/// root narrows after it started under no cap reaches the handle a sampler holds, and a child that
/// narrows its own leaves the tree's where the root put it — `rule:security/isolate-budget-is-the-trees`.
#[test]
fn a_narrowed_cpu_time_reaches_the_trees_handle_from_the_root_and_never_from_a_child() {
    let mut root = ctx_reading("[limits]\nmemory = \"128M\"\n");
    let charged = root.safepoint_view();
    assert_eq!(
        charged.cpu_limit(),
        0,
        "a tree under no cap carried a ceiling"
    );

    // What `Core\Config::set` does: the overlay moves, then the cached ceilings are re-read.
    assert!(
        root.config_mut()
            .expect("the case set one")
            .set("limits.cpu_time", "200ms")
    );
    root.refresh_limits();
    assert!(root.cpu_limit() > 0, "the narrowing was not read");
    assert_eq!(charged.cpu_limit(), root.cpu_limit());

    let mut child = root.isolate(OutputSink::Buffer(Vec::new()));
    assert!(
        child
            .config_mut()
            .expect("the child inherits one")
            .set("limits.cpu_time", "100ms")
    );
    child.refresh_limits();
    assert!(child.cpu_limit() < root.cpu_limit());
    assert_eq!(
        charged.cpu_limit(),
        root.cpu_limit(),
        "a child moved the ceiling its whole tree is charged against"
    );
}

/// A `spawn script` chain is counted on the contexts it crosses: the request is depth `0` and each
/// isolate is one deeper than whatever built it, carrying the ceiling down unchanged.
///
/// The ceiling is asserted at every level beside the depth, because `Ctx::isolate` clones the
/// configuration too — a child that re-read the file rather than inheriting the number would answer
/// the same `8` here while silently widening a ceiling its parent had narrowed, which is the one
/// failure this propagation exists to prevent.
#[test]
fn a_child_context_is_one_deeper_than_its_parent_and_inherits_the_ceiling() {
    let request = ctx_reading("[limits]\nmax_script_depth = 8\n");
    assert_eq!(
        request.script_depth(),
        0,
        "the request that started the tree"
    );

    let child = request.isolate(OutputSink::Buffer(Vec::new()));
    assert_eq!(child.script_depth(), 1);
    assert_eq!(child.max_script_depth(), 8);

    let grandchild = child.isolate(OutputSink::Buffer(Vec::new()));
    assert_eq!(grandchild.script_depth(), 2);
    assert_eq!(grandchild.max_script_depth(), 8);

    // A parent that narrowed the ceiling for itself narrowed it for everything beneath it. The
    // direction `Ctx::isolate`'s comment owns, and the reason the number is copied rather than
    // re-read out of the configuration crossing beside it.
    let mut narrowed = ctx_reading("[limits]\nmax_script_depth = 8\n");
    narrowed.set_max_script_depth(2);
    let under = narrowed
        .isolate(OutputSink::Buffer(Vec::new()))
        .isolate(OutputSink::Buffer(Vec::new()));
    assert_eq!(under.max_script_depth(), 2, "not the 8 the file still says");
    assert_eq!(under.script_depth(), 2);
}

/// The refusal that ceiling owes, asked of the child that does not exist yet — `Ctx::script_depth_breach`'s
/// own doc owns why the question is asked at the parent rather than polled at a safepoint.
///
/// **Both sides of the bound, named together**: the deepest context that may still spawn and the
/// first that may not. A check written one off answers plausibly against either half alone, and the
/// half it would get wrong is the one an operator only meets in production. The message is asserted
/// for both numbers rather than for its wording, because `max_script_depth = 2` and "a chain three
/// deep" are the two facts a reader needs and the sentence around them is not a promise.
#[test]
fn a_spawn_past_max_script_depth_is_a_breach_naming_the_depth_and_the_ceiling() {
    let request = ctx_reading("[limits]\nmax_script_depth = 2\n");
    assert!(
        request.script_depth_breach().is_none(),
        "the request itself is depth 0, and the child it would build is 1"
    );

    let child = request.isolate(OutputSink::Buffer(Vec::new()));
    assert!(
        child.script_depth_breach().is_none(),
        "at depth 1 the child would be 2 — the ceiling itself, which is allowed"
    );

    let grandchild = child.isolate(OutputSink::Buffer(Vec::new()));
    let Some(Fault::Fatal(message)) = grandchild.script_depth_breach() else {
        panic!("a spawn from depth 2 under a ceiling of 2 is the first one refused");
    };
    assert!(
        message.contains("depth 3"),
        "the depth the refused child would have reached: {message}"
    );
    assert!(
        message.contains("ceiling of 2"),
        "the number the operator wrote: {message}"
    );

    // What makes the report branchable rather than only readable: the handler `rule:errors/on-limit` hands a
    // report to sees the directive's own spelling, not a sentence it would have to match against.
    assert_eq!(Limit::ScriptDepth.name(), "max_script_depth");

    // `false` is no ceiling at all, and a tree under one is never over it however deep the chain
    // already runs — the asymmetry the case above this one pins on the reader, asserted here on the
    // enforcement side so the two cannot drift apart.
    let mut uncapped = ctx_reading("[limits]\nmax_script_depth = false\n");
    for _ in 0..4 {
        uncapped = uncapped.isolate(OutputSink::Buffer(Vec::new()));
    }
    assert_eq!(uncapped.script_depth(), 4);
    assert!(uncapped.script_depth_breach().is_none());
}
