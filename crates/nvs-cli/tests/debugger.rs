//! A debug session over a real program: each test compiles a Novis fixture
//! under the debug switch, sets a breakpoint by line, checks the one stop it
//! gives, and resumes the program to its end.
//!
//! The program runs on its own thread, because a stop on a context with no
//! reactor blocks that thread until the session resumes it. The test thread
//! drives the [`Debugger`]. Every wait is on a channel; the bound on each one
//! only turns a hang into a failure that names what did not arrive.

use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_host::debug::{
    BreakpointSpec, DebugFunction, DebugUnit, DebugVariable, Debugger, SiteLine, Stop, StopReason,
    StoppedFrame, VariableType,
};
use nvs_render::{Node, Scalar};

/// The file name every fixture is compiled and broken under.
const FILE: &str = "test.nvs";

/// A guard against a hang, never a wait for something to happen.
const HANG: Duration = Duration::from_secs(60);

/// Runs the whole front end over `source` and lowers it under the debug
/// switch: every class method, every closure and generator body, and the
/// script frame.
fn lower(source: &str) -> nvs_ir::Program {
    nvs_ir::lower::with_debug_slots(|| {
        let mut map = SourceMap::new();
        let id = map.add(FILE, source);
        let src = map.file(id);

        let mut diags = Diagnostics::new();
        let stmts = nvs_syntax::parse_file(src, &mut diags);
        let module = nvs_hir::resolve_file(&stmts, src, &mut diags);
        let mut interner = nvs_types::TypeInterner::new();
        let mut exprs = nvs_types::ExprTypeTable::new();
        let files = [nvs_types::ProgramFile { src, stmts: &stmts }];
        let enums =
            nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
        assert!(
            !diags.has_errors(),
            "the fixture does not type-check: {:?}",
            diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        );

        let layouts = nvs_types::build_class_layouts(&files, &module.graph);
        nvs_ir::lower::lower_file(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &stmts,
            src,
            &exprs,
            &interner,
            &enums,
            &layouts,
        )
    })
}

/// The unit's functions by index, each with its slot table.
fn functions(unit: &nvs_codegen::Unit) -> Vec<DebugFunction> {
    (0u32..)
        .map_while(|index| unit.debug_function(index))
        .map(|name| DebugFunction {
            name: name.to_owned(),
            slots: unit
                .debug_slots(name)
                .unwrap_or_default()
                .iter()
                .map(|slot| DebugVariable {
                    name: slot.name.clone(),
                    secret: slot.secret,
                    ty: VariableType::Other,
                })
                .collect(),
        })
        .collect()
}

/// One fixture compiled on a program thread, with a session over it. The
/// program waits for [`Session::run`] before its script starts.
struct Session {
    debugger: Debugger,
    stops: mpsc::Receiver<Stop>,
    go: mpsc::Sender<()>,
    output: mpsc::Receiver<String>,
    program: JoinHandle<()>,
}

impl Session {
    /// Compiles `source` on a new thread and returns the session over it.
    fn start(source: &'static str) -> Self {
        let (setup, ready) = mpsc::channel();
        let (go, start) = mpsc::channel();
        let (done, output) = mpsc::channel();
        let program = std::thread::spawn(move || {
            let program = lower(source);
            let mut map = SourceMap::new();
            map.add(FILE, source);
            let lines = program
                .stmt_lines(&map)
                .into_iter()
                .map(|(file, line)| SiteLine { file, line })
                .collect();
            let unit = nvs_codegen::compile(&program).expect("the fixture compiles");
            let sites = unit.debug_sites().expect("a debug unit has a site table");
            let owners = (0u32..)
                .zip(&program.functions)
                .flat_map(|(index, f)| std::iter::repeat_n(index, f.stmt_spans.len()))
                .collect();
            let debug = DebugUnit::new(
                sites.clone(),
                lines,
                owners,
                functions(&unit),
                program.stmt_catches(),
            );
            let (debugger, stops) = Debugger::new();
            debugger.load(&debug);
            setup
                .send((debugger.clone(), stops))
                .expect("the test is waiting for the session");
            start.recv().expect("the test started the program");

            let mut ctx = nvs_runtime::Ctx::buffered();
            debugger.attach(&mut ctx, &debug);
            unit.install_in(&mut ctx);
            unit.script()
                .expect("the script frame was compiled")
                .call(&mut ctx)
                .expect("the script ran to completion");
            let out = ctx.take_buffered_output().expect("a buffered context");
            done.send(String::from_utf8(out).expect("the script echoed UTF-8"))
                .expect("the test is waiting for the output");
        });
        let (debugger, stops) = ready
            .recv_timeout(HANG)
            .expect("the program thread did not hand over its session");
        Self {
            debugger,
            stops,
            go,
            output,
            program,
        }
    }

    /// Lets the program's script start.
    fn run(&self) {
        self.go
            .send(())
            .expect("the program thread is waiting to start");
    }

    /// The next stop the program reports.
    fn next_stop(&self) -> Stop {
        self.stops
            .recv_timeout(HANG)
            .expect("the program did not stop at its breakpoint")
    }

    /// Resumes the program, waits for it to end, and returns what it printed.
    fn finish(self) -> String {
        self.debugger.resume();
        let out = self
            .output
            .recv_timeout(HANG)
            .expect("the program did not end after the resume");
        self.program.join().expect("the program thread ended");
        out
    }
}

/// The value of `name` in `frame`, or `None` when the frame does not show it.
fn value<'a>(frame: &'a StoppedFrame, name: &str) -> Option<&'a Node> {
    frame
        .variables
        .iter()
        .find(|variable| variable.name == name)
        .map(|variable| &variable.value)
}

/// The tree a stop shows for the int `n`.
fn int(n: i64) -> Node {
    nvs_runtime::record::node(nvs_runtime::Value::int(n))
}

/// The text of a string node.
fn text(node: Option<&Node>) -> Option<&str> {
    match node {
        Some(Node::Scalar(Scalar::Str { text, .. })) => Some(text.as_str()),
        _ => None,
    }
}

fn at(line: u32) -> Option<SiteLine> {
    Some(SiteLine {
        file: FILE.to_owned(),
        line,
    })
}

const METHOD: &str = "<?nvs
class Shop {
    public static function total(int $price, int $count): int {
        int $sum = $price * $count;
        int $withTax = $sum + 2;
        return $withTax;
    }
}
int $order = Shop::total(4, 3);
echo $order;
";

#[test]
fn a_breakpoint_in_a_method_stops_at_its_line_with_the_parameters_and_locals() {
    let session = Session::start(METHOD);
    assert!(
        session
            .debugger
            .set_breakpoints(FILE, &BreakpointSpec::lines(&[5]))[0]
            .verified
    );
    session.run();

    let stop = session.next_stop();
    assert_eq!(stop.reason, StopReason::Breakpoint);
    let [inner, outer] = stop.frames.as_slice() else {
        panic!("the method and the script frame, got {:#?}", stop.frames);
    };
    assert_eq!(inner.name, "Shop::total");
    assert_eq!(inner.at, at(5));
    assert_eq!(value(inner, "price"), Some(&int(4)), "{inner:#?}");
    assert_eq!(value(inner, "count"), Some(&int(3)), "{inner:#?}");
    assert_eq!(value(inner, "sum"), Some(&int(12)), "{inner:#?}");
    // `$withTax` is assigned by the statement the stop is on, so it has no
    // value yet.
    assert_eq!(value(inner, "withTax"), None, "{inner:#?}");

    assert_eq!(outer.name, "<script>");
    assert_eq!(outer.at, at(9));
    assert_eq!(value(outer, "order"), None, "{outer:#?}");

    assert_eq!(session.finish(), "14");
}

const CLOSURE: &str = "<?nvs
int $fee = 5;
var $add = fn (int $amount): int => {
    int $total = $amount + $fee;
    return $total;
};
echo $add(10);
";

#[test]
fn a_breakpoint_in_a_closure_stops_at_its_line_with_its_parameter_and_a_captured_variable() {
    let session = Session::start(CLOSURE);
    assert!(
        session
            .debugger
            .set_breakpoints(FILE, &BreakpointSpec::lines(&[5]))[0]
            .verified
    );
    session.run();

    let stop = session.next_stop();
    assert_eq!(stop.reason, StopReason::Breakpoint);
    let inner = stop.frames.first().expect("a stop has a frame");
    assert_eq!(inner.at, at(5));
    assert_eq!(value(inner, "amount"), Some(&int(10)), "{inner:#?}");
    assert_eq!(value(inner, "fee"), Some(&int(5)), "{inner:#?}");
    assert_eq!(value(inner, "total"), Some(&int(15)), "{inner:#?}");
    let script = stop.frames.last().expect("a stop has a frame");
    assert_eq!(script.name, "<script>", "{:#?}", stop.frames);
    assert_eq!(script.at, at(7));

    assert_eq!(session.finish(), "15");
}

const GENERATOR: &str = "<?nvs
class Order {
    public static function lines(int $count): Iterator<int> {
        int $n = 1;
        while ($n <= $count) {
            yield $n * 10;
            $n = $n + 1;
        }
    }
}
foreach (Order::lines(3) as int $line) {
    echo $line, \"\\n\";
}
";

#[test]
fn a_breakpoint_in_a_generator_stops_on_a_later_iteration_with_the_loop_variable() {
    let session = Session::start(GENERATOR);
    assert!(
        session
            .debugger
            .set_breakpoints(FILE, &BreakpointSpec::lines(&[6]))[0]
            .verified
    );
    session.run();

    let first = session.next_stop();
    let inner = first.frames.first().expect("a stop has a frame");
    assert_eq!(inner.at, at(6));
    assert_eq!(value(inner, "n"), Some(&int(1)), "{inner:#?}");
    session.debugger.resume();

    // The second `yield`: the loop has run once, so `$n` is 2.
    let second = session.next_stop();
    assert_eq!(second.reason, StopReason::Breakpoint);
    let inner = second.frames.first().expect("a stop has a frame");
    assert_eq!(inner.at, at(6));
    assert_eq!(value(inner, "n"), Some(&int(2)), "{inner:#?}");
    assert_eq!(value(inner, "count"), Some(&int(3)), "{inner:#?}");

    // This clears every breakpoint, so the program runs to its end.
    session
        .debugger
        .set_breakpoints(FILE, &BreakpointSpec::lines(&[]));
    assert_eq!(session.finish(), "10\n20\n30\n");
}

const SECRET: &str = "<?nvs
secret string $token = \"sk-1234\";
string $shop = \"Shop\";
echo $shop;
";

#[test]
fn a_secret_local_shows_redacted_and_a_plain_local_beside_it_shows_its_value() {
    let session = Session::start(SECRET);
    assert!(
        session
            .debugger
            .set_breakpoints(FILE, &BreakpointSpec::lines(&[4]))[0]
            .verified
    );
    session.run();

    let stop = session.next_stop();
    let [script] = stop.frames.as_slice() else {
        panic!("only the script frame, got {:#?}", stop.frames);
    };
    assert_eq!(script.at, at(4));
    assert_eq!(value(script, "token"), Some(&Node::Redacted), "{script:#?}");
    assert_eq!(text(value(script, "shop")), Some("Shop"), "{script:#?}");

    assert_eq!(session.finish(), "Shop");
}
