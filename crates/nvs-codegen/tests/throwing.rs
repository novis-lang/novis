//! `rule:errors/propagation`'s checked-return propagation: what crosses a frame, what a `catch` stops, the backtrace, and what a throwing frame releases.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

/// The three-frame throw `examples/throw.nvs` runs, verbatim.
const THROWS: &str = "<?nvs
class Deep {
    public static function level3(): void {
        throw new LogicError(\"boom\");
    }
    public static function level2(): void {
        Deep::level3();
    }
    public static function level1(): void {
        Deep::level2();
    }
}
";

/// A helper names the class its failure lands in as a `nvs_runtime::ThrownClass`,
/// and `nvs-runtime` depends on nothing — so this is the one place the roster
/// and `nvs_hir::errors::TREE` are held together. A class named here but absent
/// there would degrade silently to `RuntimeError`
/// (`nvs_runtime::Ctx::set_runtime_error_class` says why it degrades rather
/// than fails).
#[test]
fn every_thrown_class_is_in_the_compiler_s_exception_tree() {
    for class in nvs_runtime::ThrownClass::ALL {
        assert!(
            nvs_hir::errors::is_exception_class(class.name()),
            "{} is not in the exception tree",
            class.name()
        );
    }
    // The default is the one every bare `Fault::thrown` lands in, and
    // `Unit::runtime_error_class` looks it up by exactly this name.
    assert_eq!(
        nvs_runtime::ThrownClass::default().name(),
        "RuntimeError",
        "the default class is what `Ctx::set_runtime_error_class` installs"
    );
}

/// A raise renders the frame it happened in from the site the `throw` was
/// compiled with, and that carrier names the enclosing member and never the
/// label a script frame has — so the runtime restates the one the compiler
/// gives a program's own top-level statements, and this is what keeps the two
/// spellings from drifting into a trace naming the same frame twice.
#[test]
fn the_runtime_and_the_compiler_spell_a_script_frame_alike() {
    assert_eq!(
        nvs_runtime::ENTRY_SCRIPT_FRAME,
        nvs_ir::lower::ENTRY_SCRIPT_LABEL
    );
}

#[test]
fn the_runtime_and_the_compiler_agree_on_every_throwable_slot() {
    use nvs_hir::errors::PROPERTIES;

    assert_eq!(PROPERTIES.len(), nvs_runtime::SLOT_COUNT);
    assert_eq!(PROPERTIES[nvs_runtime::MESSAGE_SLOT], "message");
    assert_eq!(PROPERTIES[nvs_runtime::PREVIOUS_SLOT], "previous");
    assert_eq!(PROPERTIES[nvs_runtime::BACKTRACE_SLOT], "backtrace");
    assert_eq!(PROPERTIES[nvs_runtime::LOCATION_SLOT], "location");

    // And the labels lowering emits actually resolve to those slots, for a
    // *user* subclass as much as for the root — which is the property that
    // lets the runtime reach `backtrace` on a value it knows nothing about.
    let program = lower(
        "<?nvs
class MyError extends IOError {
  public int $code;
           function constructor(int $code) {
    parent::constructor(\"bad\");
             $this->code = $code;
  }
}
",
    );
    for label in ["Throwable", "IOError", "MyError"] {
        let class = program
            .classes
            .iter()
            .find(|c| c.label == label)
            .unwrap_or_else(|| panic!("{label} should be in the class table"));
        assert_eq!(&class.fields[..PROPERTIES.len()], PROPERTIES, "{label}");
    }

    // `rule:core-classes/derive-reports-every-field`'s `issues` and `rule:core-classes/db-error`'s `kind` are properties classes
    // below the root declare, and the runtime writes each by index too — so
    // their slots are held by the same agreement the root's are. Each constant
    // is asserted against its own class rather than against a sibling's:
    // sibling classes declaring a property apiece make the indices coincide,
    // and a class declaring more would break that coincidence without breaking
    // any single line.
    assert_eq!(nvs_hir::errors::ISSUES_SLOT, nvs_runtime::ISSUES_SLOT);
    assert_eq!(nvs_hir::errors::KIND_SLOT, nvs_runtime::KIND_SLOT);
    assert_eq!(nvs_hir::errors::REASON_SLOT, nvs_runtime::REASON_SLOT);
    assert_eq!(nvs_hir::errors::SQL_STATE_SLOT, nvs_runtime::SQL_STATE_SLOT);
    assert_eq!(
        nvs_hir::errors::DRIVER_CODE_SLOT,
        nvs_runtime::DRIVER_CODE_SLOT
    );
    assert_eq!(
        nvs_hir::errors::CONSTRAINT_SLOT,
        nvs_runtime::CONSTRAINT_SLOT
    );
    assert_eq!(nvs_hir::errors::SQL_SLOT, nvs_runtime::SQL_SLOT);
    let parse = program
        .classes
        .iter()
        .find(|c| c.label == "ParseError")
        .expect("ParseError should be in the class table");
    assert_eq!(parse.fields[nvs_runtime::ISSUES_SLOT], "issues");
    let db_error = program
        .classes
        .iter()
        .find(|c| c.label == "Core\\Db\\DbError")
        .expect("Core\\Db\\DbError should be in the class table");
    assert_eq!(db_error.fields[nvs_runtime::KIND_SLOT], "kind");
    assert_eq!(db_error.fields[nvs_runtime::SQL_STATE_SLOT], "sqlState");
    assert_eq!(db_error.fields[nvs_runtime::DRIVER_CODE_SLOT], "driverCode");
    assert_eq!(db_error.fields[nvs_runtime::CONSTRAINT_SLOT], "constraint");
    assert_eq!(db_error.fields[nvs_runtime::SQL_SLOT], "sql");
    let rolled_back = program
        .classes
        .iter()
        .find(|c| c.label == "Core\\Db\\RolledBack")
        .expect("Core\\Db\\RolledBack should be in the class table");
    assert_eq!(rolled_back.fields[nvs_runtime::REASON_SLOT], "reason");
}

#[test]
fn a_throw_crosses_several_frames_and_is_caught() {
    // `rule:errors/propagation`'s whole claim, end to end: no unwinder is involved, each frame
    // returns `THROWN` and its caller branches on it, and the `catch` three
    // frames up sees the message the `throw` built.
    let source = format!(
        "{THROWS}\ntry {{\n    Deep::level1();\n    echo \"not reached\";\n}} \
         catch (Throwable $e) {{\n    echo \"caught: \" . $e->message;\n}}\n"
    );
    assert_eq!(output_of(&source), "caught: boom");
}

#[test]
fn an_uncaught_throw_leaves_the_status_and_the_message_on_the_context() {
    let mut ctx = Ctx::buffered();
    let source = format!("{THROWS}\nDeep::level1();\n");
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), THROWN);
    assert_eq!(ctx.pending().as_deref(), Some("boom"));
}

#[test]
fn the_backtrace_names_every_frame_the_throw_left_in_order() {
    // Resolved from Novis's own frame chain — each frame's error path pushes its
    // own label — never from the platform unwinder, which `rule:errors/propagation` makes
    // unavailable through a JIT frame in the first place.
    let mut ctx = Ctx::buffered();
    let source = format!("{THROWS}\nDeep::level1();\n");
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), THROWN);

    let thrown = ctx.take_thrown();
    assert!(
        !thrown.is_none(),
        "an uncaught throw leaves its exception behind"
    );
    let trace = thrown.trace_as_string();
    let frames: Vec<&str> = trace.lines().collect();
    assert_eq!(frames.len(), 4, "{trace}");
    assert!(frames[0].starts_with("#0 Deep::level3() at "), "{trace}");
    assert!(frames[1].starts_with("#1 Deep::level2() at "), "{trace}");
    assert!(frames[2].starts_with("#2 Deep::level1() at "), "{trace}");
    assert!(frames[3].starts_with("#3 <script>() at "), "{trace}");
}

#[test]
fn a_caught_throw_stops_the_backtrace_at_the_frame_that_handled_it() {
    // The trace holds the frames the exception actually unwound *out of*, so a
    // `catch` in the calling frame never appears in it — `nvs_runtime::throwable`
    // owns that rule and why it differs from PHP's construction-time snapshot.
    let source = format!(
        "{THROWS}\ntry {{\n    Deep::level3();\n}} catch (Throwable $e) {{\n    \
         foreach ($e->backtrace as string $frame) {{ echo $frame; }}\n}}\n"
    );
    let out = output_of(&source);
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.starts_with("Deep::level3() at "), "{out}");
}

#[test]
fn a_throw_caught_in_the_frame_that_raised_it_names_that_frame() {
    // The frame a `catch` beside the `throw` sees is the one the raise rendered
    // for itself, from the site it was compiled with: no landing block ever
    // pushes a label for a throw that does not leave its frame, so this is the
    // whole of the trace — and the frame's own name is the script label, since
    // `<script>` is what this file scope is compiled under.
    let out = output_of(
        "<?nvs
try {
    throw new LogicError(\"boom\");
} catch (Throwable $e) {
    foreach ($e->backtrace as string $frame) { echo $frame; }
}
",
    );
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.starts_with("<script>() at "), "{out}");
}

#[test]
fn an_arithmetic_raise_names_the_frame_and_the_line_it_happened_in() {
    // A checked operator raises inline rather than through a `throw`, and is
    // handed the site of the statement it is in (`nvs_ir::ir::Inst::raise_site`)
    // — so a `catch` beside the arithmetic reads the frame it happened in, on
    // the same terms the `throw` above does.
    let out = output_of(
        "<?nvs
class Ratio {
    public static function of(int $a, int $b): string {
        try {
            return ($a % $b) as string;
        } catch (Throwable $e) {
            foreach ($e->backtrace as string $frame) { echo $frame; }
            echo \"|\";
            return $e->location;
        }
    }
}
echo Ratio::of(1, 0);
",
    );
    assert_eq!(out.lines().count(), 1, "{out}");
    let (frame, location) = out.split_once('|').unwrap_or_else(|| panic!("{out}"));
    assert!(frame.starts_with("Ratio::of() at "), "{out}");
    // The property and the label are two readings of one datum, which is what
    // `rule:errors/a-record-names-where-it-was-produced` asks of a raise: the
    // label is the member's name followed by exactly what `location` holds.
    assert_eq!(frame, format!("Ratio::of() at {location}"), "{out}");
}

#[test]
fn a_fatal_is_never_caught() {
    // ADR 0020: a resource-limit stop is not a `Throwable`, so the `catch`
    // this program wraps around it does not run and the status travels on out.
    let mut ctx = Ctx::buffered();
    ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
    let source = format!(
        "{THROWS}\ntry {{\n    Deep::level1();\n}} catch (Throwable $e) {{\n    \
         echo \"caught\";\n}}\n"
    );
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), FATAL);
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
}

#[test]
fn an_exit_is_caught_by_nothing_and_carries_the_status_it_named() {
    // `exit` is its own ABI status, so the `catch` does not run and neither
    // does the `finally` — PHP's own behaviour, and the reason it is not a
    // `FATAL` is that the code it named survives to the request boundary.
    // docs/adr/README.md § Decisions taken at project start owns both.
    let mut ctx = Ctx::buffered();
    let source = "<?nvs\ntry {\n    exit(3);\n} catch (Throwable $e) {\n    \
                  echo \"caught\";\n} finally {\n    echo \"finally\";\n}\n";
    assert_eq!(run_with(&mut ctx, source).unwrap_err(), EXITED);
    assert_eq!(ctx.exit_code(), 3);
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
}

#[test]
fn a_bare_exit_is_status_zero_and_a_string_operand_is_written_first() {
    // PHP's two spellings of one construct: `exit(n)` names a status, and
    // `exit("…")` writes a message and leaves the status at zero.
    let mut ctx = Ctx::buffered();
    assert_eq!(run_with(&mut ctx, "<?nvs\nexit;\n").unwrap_err(), EXITED);
    assert_eq!(ctx.exit_code(), 0);

    let mut ctx = Ctx::buffered();
    assert_eq!(
        run_with(&mut ctx, "<?nvs\nexit(\"bye\");\n").unwrap_err(),
        EXITED
    );
    assert_eq!(ctx.exit_code(), 0);
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b"bye"[..]));
}

#[test]
fn a_frame_that_throws_releases_the_strings_it_still_held() {
    // The error path's refcount cleanup, observed rather than assumed: the
    // caught exception's message is the only allocation still alive once the
    // dust settles, so a landing block that skipped its releases would leave
    // the local's buffer behind. `nvs-ir`'s landing blocks are what put the
    // releases there; this checks the backend actually emits them.
    let source = format!(
        "{THROWS}\ntry {{\n    string $held = \"kept alive\";\n    Deep::level1();\n    \
         echo $held;\n}} catch (Throwable $e) {{\n    echo \"caught: \" . $e->message;\n}}\n"
    );
    assert_eq!(output_of(&source), "caught: boom");
}

/// The running balance of bytes outstanding on the calling thread, so the
/// guard below can say a frame handed its locals back rather than only that its
/// refcounts looked balanced — and the monotonic total beside it, which is what
/// keeps a balanced-at-zero reading from also being a vacuous one.
///
/// `nvs_runtime::budget`'s, because the memory limit is read off the same
/// counters and every build therefore maintains them. This binary installs no
/// allocator of its own, and could not: a `#[global_allocator]` is chosen once
/// per binary and `nvs-runtime` registers one in every `not(test)` build.
///
/// **Only used in a debug build.** What the gate keeps out is an optimized
/// build's inlining: the guard below is pinned against what an unoptimized
/// build allocates, and nothing has measured it under `--release`.
#[cfg(debug_assertions)]
use nvs_runtime::budget::{allocated_bytes, live_bytes};

/// The other half of the neighbour above, over the exit `rule:errors/escalation-ladder` keeps out of
/// every `catch`: a `FATAL` leaves the frame too, so its locals are released on
/// the way out or they are lost for good.
///
/// The observation has to be the allocator's balance rather than a program's
/// output, because nothing in the program runs after a fatal — that is what a
/// fatal is. It cannot be the valgrind sweep either: `examples/fatal.nvs` is on
/// that sweep's skip list for exiting non-zero by design.
///
/// `Core\Arr::countBy` over a `float`-keyed subject is the trigger, because it
/// is a `Fault::fatal` reachable from source *after* two locals are live —
/// `nvs_stdlib`'s `key_bytes` raises it, and `docs/agent/playbook.md` records
/// that no handler sees it. Both locals are genuine allocations: the string is
/// a concatenation rather than a literal (a literal is an address in the
/// unit's data section, `tests/strings.rs`), and the array is a heap buffer of
/// its own.
#[cfg(debug_assertions)]
#[test]
fn a_fatal_releases_the_frames_locals() {
    /// The held string's length, big enough that no other allocation the run
    /// makes could be mistaken for it.
    const FILLER: usize = 512;

    // Half a kilobyte of it, so the balance below cannot be satisfied by a run
    // that never allocated the local at all: the assertion on `allocated_bytes`
    // is what makes the zero delta mean something, and no other allocation this
    // run makes is anywhere near that size.
    let filler = "x".repeat(FILLER);
    let source = format!(
        "<?nvs\nstring $held = \"{filler}\" . \"!\";\n\
         array<float> $floats = [1.5, 2.5];\n\
         echo Core\\Json::encode(Core\\Arr::countBy($floats));\n"
    );
    let unit = compile(&source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");

    // The run is measured, not the compile: what is under test is what the
    // compiled code hands back, and the front end's own allocations would
    // swamp two buffers. The thread's empty-array singleton is taken first —
    // it is one header per thread rather than a frame's local, and the
    // fixture's `[1.5, 2.5]` would otherwise be the call that allocates it.
    nvs_runtime::prime_empty_array();
    let before = live_bytes();
    let spent = allocated_bytes();
    assert_eq!(entry.call(&mut ctx).err(), Some(FATAL));
    assert!(
        allocated_bytes() - spent >= FILLER,
        "the local's buffer did not come through this allocator, so the \
         balance below would prove nothing"
    );

    // The fatal's message is a `String` the context owns until it is taken,
    // and it is the one allocation the run is *meant* to leave behind.
    let pending = ctx.take_pending();
    assert_eq!(
        pending.as_deref(),
        Some("Core\\Arr::countBy expected an `int|string` key, got tag 4")
    );
    drop(pending);
    drop(ctx.take_buffered_output());

    assert_eq!(
        live_bytes(),
        before,
        "a fatal left the frame's locals allocated"
    );
}

/// Three frames, each holding a `finally` of its own, with the innermost one
/// ending the script through `Core\Script::finish()`.
///
/// Each `finally` echoes its own depth, so the output alone says which of them
/// ran and in what order — the whole of what makes the fourth ending an ending
/// rather than `exit` under another name.
const FINISHES: &str = "<?nvs
class Deep {
    public static function level3(): void {
        try {
            Core\\Script::finish();
            echo \"past the finish\";
        } finally {
            echo \"3\";
        }
    }
    public static function level2(): void {
        try {
            Deep::level3();
        } finally {
            echo \"2\";
        }
    }
    public static function level1(): void {
        try {
            Deep::level2();
        } finally {
            echo \"1\";
        }
    }
}
";

#[test]
fn a_finish_runs_every_finally_between_the_call_and_the_root() {
    // `rule:errors/propagation`'s throw path is the path a `finally` body lives
    // on, which is what `nvs_ir::lower`'s `lower_finish` buys by raising a
    // marker instead of returning a status: every enclosing region's
    // finally-and-re-raise block runs on the way out, the script's own included.
    let mut ctx = Ctx::buffered();
    let source =
        format!("{FINISHES}\ntry {{\n    Deep::level1();\n}} finally {{\n    echo \"root\";\n}}\n");
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), THROWN);
    assert_eq!(
        ctx.take_buffered_output().as_deref(),
        Some(&b"321root"[..]),
        "a finally between the call and the root did not run"
    );
}

#[test]
fn a_finish_runs_them_innermost_first() {
    // Three regions in one frame, so the order under test is the nesting order
    // rather than the frame order the neighbour above covers.
    let mut ctx = Ctx::buffered();
    let source = "<?nvs\ntry {\n    try {\n        try {\n            \
                  Core\\Script::finish();\n        } finally {\n            echo \"inner\";\n        \
                  }\n    } finally {\n        echo \"middle\";\n    }\n} finally {\n    \
                  echo \"outer\";\n}\n";
    assert_eq!(run_with(&mut ctx, source).unwrap_err(), THROWN);
    assert_eq!(
        ctx.take_buffered_output().as_deref(),
        Some(&b"innermiddleouter"[..]),
        "the regions ran in some order other than innermost first"
    );
}

#[test]
fn a_finish_three_frames_down_still_reaches_the_root() {
    // The marker arrives at the root intact: each frame returned `THROWN` and
    // re-raised the very object it was handed, so what is pending at the root
    // is still the class `nvs_runtime::is_finish` classifies an ending by —
    // `rule:observability/three-endings-fire-the-exit-queue`'s report is read
    // off exactly this.
    let mut ctx = Ctx::buffered();
    let source = format!("{FINISHES}\nDeep::level1();\necho \"past the call\";\n");
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), THROWN);
    assert_eq!(
        ctx.pending_class().as_deref(),
        Some(nvs_runtime::FINISH_MARKER_NAME)
    );
    assert_eq!(
        ctx.take_buffered_output().as_deref(),
        Some(&b"321"[..]),
        "the script carried on past the ending it named"
    );
}

#[test]
fn a_catch_throwable_does_not_admit_a_finish() {
    // The widest arm a program can write, over the ending that is not a
    // failure: the marker is a second, parentless root of the exception tree
    // (`nvs_hir::errors::FINISH_MARKER`), so the arm dispatch's
    // self-or-ancestor walk is false for it and the region's `finally` is all
    // that runs. A `catch` that admitted it would turn an ending into a value
    // a handler could swallow.
    let mut ctx = Ctx::buffered();
    let source = format!(
        "{FINISHES}\ntry {{\n    Deep::level1();\n}} catch (Throwable $e) {{\n    \
         echo \"caught\";\n}} finally {{\n    echo \"root\";\n}}\n"
    );
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), THROWN);
    assert_eq!(
        ctx.take_buffered_output().as_deref(),
        Some(&b"321root"[..]),
        "a catch arm took the ending, or the finally beside it did not run"
    );
}

#[test]
fn a_finish_is_still_a_finish_after_it_has_passed_a_catch_region() {
    // Passing an arm is not a promotion: nothing in the dispatch rewraps the
    // object it declined, so the class at the root is still the one
    // `nvs_runtime::is_finish` classifies an ending by — the property a host
    // reads its report off after the marker has crossed three frames and two
    // regions that each had a chance to take it.
    let mut ctx = Ctx::buffered();
    let source = format!(
        "{FINISHES}\ntry {{\n    try {{\n        Deep::level1();\n    }} \
         catch (LogicError $inner) {{\n        echo \"inner caught\";\n    }}\n}} \
         catch (Throwable $outer) {{\n    echo \"outer caught\";\n}}\n"
    );
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), THROWN);
    assert_eq!(
        ctx.pending_class().as_deref(),
        Some(nvs_runtime::FINISH_MARKER_NAME),
        "the object a catch region declined did not arrive at the root intact"
    );
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b"321"[..]));
}

/// The `a_fatal_releases_the_frames_locals` guard above, over the ending that
/// leaves a frame through its landing block rather than through a fatal: a
/// finish takes the same error path, so the same releases have to be on it or
/// a request that ended deliberately leaks everything it still held.
///
/// The allocation the run is meant to leave behind is the marker itself, which
/// is pending at the root until something takes it — so it is taken here, and
/// the balance is read after.
#[cfg(debug_assertions)]
#[test]
fn a_finish_inside_a_try_releases_every_local_of_that_frame() {
    /// The held string's length, big enough that no other allocation the run
    /// makes could be mistaken for it.
    const FILLER: usize = 512;

    let filler = "x".repeat(FILLER);
    let source = format!(
        "<?nvs\nstring $held = \"{filler}\" . \"!\";\n\
         array<float> $floats = [1.5, 2.5];\n\
         try {{\n    Core\\Script::finish();\n}} finally {{\n    echo \"f\";\n}}\n"
    );
    let unit = compile(&source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");

    nvs_runtime::prime_empty_array();
    let before = live_bytes();
    let spent = allocated_bytes();
    assert_eq!(entry.call(&mut ctx).err(), Some(THROWN));
    assert!(
        allocated_bytes() - spent >= FILLER,
        "the local's buffer did not come through this allocator, so the \
         balance below would prove nothing"
    );

    let marker = ctx.take_thrown();
    drop(marker);
    drop(ctx.take_buffered_output());

    assert_eq!(
        live_bytes(),
        before,
        "a finish left the frame's locals allocated"
    );
}
