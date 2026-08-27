//! ADR 0002's checked-return propagation: what crosses a frame, what a `catch` stops, the backtrace, and what a throwing frame releases.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

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

    // ADR 0071 § 5's `issues` is the one property any class below the root
    // declares, and the runtime writes it by index too — so its slot is held
    // by the same agreement the four above are.
    assert_eq!(nvs_hir::errors::ISSUES_SLOT, nvs_runtime::ISSUES_SLOT);
    let parse = program
        .classes
        .iter()
        .find(|c| c.label == "ParseError")
        .expect("ParseError should be in the class table");
    assert_eq!(parse.fields[nvs_runtime::ISSUES_SLOT], "issues");
}

#[test]
fn a_throw_crosses_several_frames_and_is_caught() {
    // ADR 0002's whole claim, end to end: no unwinder is involved, each frame
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
    // own label — never from the platform unwinder, which ADR 0002 makes
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
