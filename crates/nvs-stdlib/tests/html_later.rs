//! `rule:core-classes/html-later` on a normal route: `Core\Html::later` called
//! from a request running as an isolate under a real scheduler, with the
//! closures written by hand as compiled code would call them.
//!
//! Each case builds a page the way a template does — text, a placeholder,
//! more text — and reads the body the connection took.

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use nvs_host::{Completion, Isolate, Output, Program, Scheduler};
use nvs_runtime::{Ctx, NvsStr, OutputSink, TaskRoot, Value};
use nvs_stdlib::html::{nvs_core_html_escape, nvs_core_html_later};

thread_local! {
    /// How many slot closures have run on this test's thread.
    static RAN: Cell<usize> = const { Cell::new(0) };
    /// The closure a nesting slot registers a `later` for.
    static INNER: Cell<Value> = const { Cell::new(Value::null()) };
    /// What happened, in order, for the after-response case.
    static ORDER: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn closure_of(invoke: nvs_runtime::NvsFn) -> Value {
    let mut table = nvs_runtime::ClassTable::new();
    let id = table.define("{closure}", &["arity", "params"], &[]);
    table.set_methods(
        id,
        vec![nvs_runtime::MethodRow {
            name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
            code: invoke as *const u8,
            arity: 0,
            param_tags: 0,
            param_names: Vec::new(),
            param_types: Vec::new(),
            public: true,
            protected: false,
            native: false,
        }],
    );
    table.set_closure(id);
    let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
    #[expect(
        unsafe_code,
        reason = "the table is leaked, so the descriptor outlives every \
                  instance made from it — `NvsObj::new`'s whole obligation"
    )]
    let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
    object.set_field(nvs_runtime::CLOSURE_ARITY_SLOT, Value::int(0));
    object.set_field(nvs_runtime::CLOSURE_PARAM_TAGS_SLOT, Value::int(0));
    Value::object(object)
}

/// The body every hand-written closure shares: release the receiver, run
/// `body` on the context, and answer what it returns.
#[expect(
    unsafe_code,
    reason = "`call_closure` passes one live value this callee owes a release, \
              and the address of a live `Value` for the result"
)]
unsafe fn invoke(
    ctx: *mut Ctx,
    args: *const Value,
    out: *mut Value,
    body: impl FnOnce(&mut Ctx) -> Result<Value, i32>,
) -> i32 {
    unsafe {
        (*args).release();
        match body(&mut *ctx) {
            Ok(value) => {
                *out = value;
                nvs_runtime::OK
            }
            Err(status) => {
                *out = Value::null();
                status
            }
        }
    }
}

macro_rules! slot_fn {
    ($name:ident, |$ctx:ident| $body:expr) => {
        #[expect(
            unsafe_code,
            reason = "a closure's entry point, as compiled code has one"
        )]
        unsafe extern "C" fn $name(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
            unsafe { invoke(ctx, args, out, |$ctx: &mut Ctx| $body) }
        }
    };
}

fn ran() {
    RAN.with(|ran| ran.set(ran.get() + 1));
}

/// Parks the slot's task for `duration`, as a `Core` member does: on a helper
/// frame, so a cancellation resumes it and it answers the status.
fn sleep(ctx: &mut Ctx, duration: Duration) -> Result<(), i32> {
    let _frame = nvs_runtime::HelperFrame::enter();
    match nvs_runtime::host::with_current(|host| host.sleep(duration)) {
        Some(nvs_runtime::host::Woken::Cancelled) => match ctx.cancel() {
            nvs_runtime::Fault::Pending(status) => Err(status),
            _ => Err(nvs_runtime::FATAL),
        },
        _ => Ok(()),
    }
}

slot_fn!(echoes_comments, |ctx| {
    ran();
    ctx.write_output(b"<p>comments</p>").expect("a buffer");
    Ok(Value::null())
});

slot_fn!(slow_a, |ctx| {
    ran();
    sleep(ctx, Duration::from_millis(150))?;
    ctx.write_output(b"A").expect("a buffer");
    Ok(Value::null())
});

slot_fn!(slow_b, |ctx| {
    ran();
    sleep(ctx, Duration::from_millis(150))?;
    ctx.write_output(b"B").expect("a buffer");
    Ok(Value::null())
});

slot_fn!(throws, |ctx| {
    ran();
    ctx.set_pending_as(nvs_runtime::ThrownClass::Runtime, "the shop is closed");
    Err(nvs_runtime::THROWN)
});

slot_fn!(sleeps_forever, |ctx| {
    ran();
    sleep(ctx, Duration::from_secs(5))?;
    ctx.write_output(b"too late").expect("a buffer");
    Ok(Value::null())
});

slot_fn!(nests, |ctx| {
    ran();
    ctx.write_output(b"[outer ").expect("a buffer");
    let inner = INNER.with(Cell::get);
    let placeholder = later(ctx, inner, None, None, None);
    ctx.write_output(&text(placeholder)).expect("a buffer");
    ctx.write_output(b"]").expect("a buffer");
    Ok(Value::null())
});

slot_fn!(notes_later, |ctx| {
    sleep(ctx, Duration::from_millis(20))?;
    ORDER.with(|order| order.borrow_mut().push("later"));
    ctx.write_output(b"x").expect("a buffer");
    Ok(Value::null())
});

slot_fn!(notes_deferred, |_ctx| {
    ORDER.with(|order| order.borrow_mut().push("deferred"));
    Ok(Value::null())
});

/// A `Core\Html\Markup` holding `text`, escaped.
fn markup(ctx: &mut Ctx, text: &str) -> Value {
    nvs_runtime::call(
        nvs_core_html_escape,
        ctx,
        &[Value::str(NvsStr::new(text.as_bytes()))],
    )
    .expect("escaping a string cannot fail")
}

/// The bytes a `Markup` carries.
fn text(markup: Value) -> Vec<u8> {
    let ptr = markup.obj_ptr().expect("a carrier is an object");
    #[expect(
        unsafe_code,
        reason = "the carrier is live and the read borrows its one slot"
    )]
    let slot =
        unsafe { nvs_runtime::object::nvs_object_field_get(ptr, nvs_runtime::CARRIER_TEXT_SLOT) };
    slot.as_str_bytes().expect("a string").to_vec()
}

/// `Core\Html::later($fn, {placeholder, error, deadline})`.
fn later(
    ctx: &mut Ctx,
    closure: Value,
    placeholder: Option<&str>,
    error: Option<&str>,
    deadline: Option<Value>,
) -> Value {
    let placeholder = placeholder.map_or_else(Value::null, |text| markup(ctx, text));
    let error = error.map_or_else(Value::null, |text| markup(ctx, text));
    nvs_runtime::call(
        nvs_core_html_later,
        ctx,
        &[closure, placeholder, error, deadline.unwrap_or_default()],
    )
    .expect("`later` registered its slot")
}

/// Runs `page` as a served request answering HTML, and answers what the
/// connection took.
fn serve(page: impl FnOnce(&mut Ctx) + 'static) -> Completion {
    let taken: std::rc::Rc<RefCell<Option<Completion>>> = std::rc::Rc::default();
    let into = std::rc::Rc::clone(&taken);
    // A reactor, so a slot that sleeps parks its task instead of the thread.
    let _reactor =
        nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
    let mut sched = Scheduler::new();
    sched.spawn(
        Ctx::new(OutputSink::Body(Vec::new())),
        TaskRoot::Request,
        move |connection: &mut Ctx| {
            let program: Program = Box::new(move |request: &mut Ctx, _args| {
                request.set_core_classes(nvs_stdlib::core_class_desc);
                page(request);
                Value::null()
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(connection)
                .expect("the isolate was refused before it started");
            *into.borrow_mut() = Some(running.join(connection));
        },
    );
    // Driven until nothing is parked, so a sleeping slot's timer fires.
    while nvs_host::run_until_idle(&mut sched)
        .expect("the loop failed")
        .parked
        != 0
    {}
    taken.take().expect("the request answered")
}

fn body(completion: &Completion) -> String {
    String::from_utf8(completion.output.clone()).expect("the page is text")
}

#[test]
fn later_output_replaces_its_placeholder_in_the_assembled_body() {
    let done = serve(|ctx| {
        assert_eq!(
            ctx.carrier(),
            nvs_runtime::CARRIER_HTML_MARKUP,
            "not an HTML response"
        );
        ctx.write_output(b"<h1>Blog</h1>").expect("a buffer");
        let slot = later(
            ctx,
            closure_of(echoes_comments),
            Some("<p>Loading</p>"),
            None,
            None,
        );
        let bytes = text(slot);
        assert!(
            bytes.starts_with(b"<?start name=\"nvs-") && bytes.ends_with(b"<?end>"),
            "the placeholder is not a slot: {}",
            String::from_utf8_lossy(&bytes)
        );
        ctx.write_output(&bytes).expect("a buffer");
        ctx.write_output(b"<footer>").expect("a buffer");
    });
    assert!(done.ok, "the page failed");
    assert_eq!(body(&done), "<h1>Blog</h1><p>comments</p><footer>");
}

#[test]
fn later_closures_run_concurrently_and_the_page_waits_for_the_slowest() {
    let began = Instant::now();
    let done = serve(|ctx| {
        let a = later(ctx, closure_of(slow_a), None, None, None);
        let b = later(ctx, closure_of(slow_b), None, None, None);
        ctx.write_output(&text(a)).expect("a buffer");
        ctx.write_output(b"-").expect("a buffer");
        ctx.write_output(&text(b)).expect("a buffer");
    });
    let took = began.elapsed();
    assert!(done.ok, "the page failed");
    assert_eq!(
        body(&done),
        "A-B",
        "the page went out before both slots were filled"
    );
    assert!(
        took < Duration::from_millis(280),
        "two 150 ms slots took {took:?}, so they did not run at the same time"
    );
}

#[test]
fn a_nested_later_is_filled_in_the_same_pass() {
    INNER.with(|inner| inner.set(closure_of(echoes_comments)));
    let done = serve(|ctx| {
        let outer = later(ctx, closure_of(nests), None, None, None);
        ctx.write_output(&text(outer)).expect("a buffer");
    });
    assert!(done.ok, "the page failed");
    assert_eq!(body(&done), "[outer <p>comments</p>]");
}

#[test]
fn a_later_that_throws_fills_its_slot_with_its_error_fragment() {
    let done = serve(|ctx| {
        let broken = later(ctx, closure_of(throws), None, Some("unavailable"), None);
        let fine = later(ctx, closure_of(echoes_comments), None, None, None);
        ctx.write_output(&text(broken)).expect("a buffer");
        ctx.write_output(b"|").expect("a buffer");
        ctx.write_output(&text(fine)).expect("a buffer");
    });
    assert!(done.ok, "one slot's throw failed the whole page");
    assert_eq!(body(&done), "unavailable|<p>comments</p>");
}

#[test]
fn a_later_past_its_deadline_fills_its_slot_with_its_error_fragment() {
    let began = Instant::now();
    let done = serve(|ctx| {
        let deadline = nvs_runtime::call(
            nvs_stdlib::time::nvs_core_time_duration_milliseconds,
            ctx,
            &[Value::int(50)],
        )
        .expect("a duration");
        let slow = later(
            ctx,
            closure_of(sleeps_forever),
            None,
            Some("timed out"),
            Some(deadline),
        );
        ctx.write_output(&text(slow)).expect("a buffer");
    });
    assert!(done.ok, "a slot's deadline failed the whole page");
    assert_eq!(body(&done), "timed out");
    assert!(
        began.elapsed() < Duration::from_secs(2),
        "the deadline did not stop the slot"
    );
}

#[test]
fn a_later_whose_placeholder_is_never_written_is_cancelled_and_warned() {
    RAN.with(|ran| ran.set(0));
    let done = serve(|ctx| {
        let _unused = later(ctx, closure_of(echoes_comments), None, None, None);
        ctx.write_output(b"<p>no slot here</p>").expect("a buffer");
    });
    assert!(done.ok, "an unwritten placeholder failed the page");
    assert_eq!(body(&done), "<p>no slot here</p>");
    assert_eq!(
        RAN.with(Cell::get),
        0,
        "the closure ran although nothing showed its output"
    );
}

#[test]
fn a_placeholder_written_twice_throws() {
    let done = serve(|ctx| {
        let slot = later(ctx, closure_of(echoes_comments), None, None, None);
        let bytes = text(slot);
        ctx.write_output(&bytes).expect("a buffer");
        ctx.write_output(&bytes).expect("a buffer");
    });
    assert!(
        !done.ok,
        "a placeholder written twice did not fail the request"
    );
}

#[test]
fn later_outside_an_html_response_runs_in_place() {
    let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
    ctx.set_core_classes(nvs_stdlib::core_class_desc);
    let answer = later(
        &mut ctx,
        closure_of(echoes_comments),
        Some("<p>Loading</p>"),
        None,
        None,
    );
    assert_eq!(text(answer), b"<p>comments</p>");
    assert_eq!(ctx.take_buffered_output().unwrap_or_default(), b"");
}

#[test]
fn a_visitor_string_that_copies_the_placeholder_fills_nothing() {
    let done = serve(|ctx| {
        let slot = later(ctx, closure_of(echoes_comments), None, None, None);
        let bytes = text(slot);
        // A visitor who learned the placeholder submits it as a comment.
        let visitor = markup(ctx, &String::from_utf8_lossy(&bytes));
        ctx.write_output(&text(visitor)).expect("a buffer");
        ctx.write_output(b"|").expect("a buffer");
        ctx.write_output(&bytes).expect("a buffer");
    });
    assert!(done.ok, "the page failed");
    let page = body(&done);
    assert!(
        page.starts_with("&lt;?start"),
        "the visitor's copy was not escaped: {page}"
    );
    assert!(
        page.ends_with("|<p>comments</p>"),
        "the real slot was not filled: {page}"
    );
    assert_eq!(
        page.matches("<p>comments</p>").count(),
        1,
        "a slot was filled twice: {page}"
    );
}

#[test]
fn after_response_work_runs_after_the_last_later() {
    ORDER.with(|order| order.borrow_mut().clear());
    let done = serve(|ctx| {
        assert_eq!(ctx.defer(closure_of(notes_deferred), 0), Ok(()));
        let slot = later(ctx, closure_of(notes_later), None, None, None);
        ctx.write_output(&text(slot)).expect("a buffer");
    });
    assert!(done.ok, "the page failed");
    assert_eq!(
        ORDER.with(|order| order.borrow().clone()),
        ["later", "deferred"]
    );
}
