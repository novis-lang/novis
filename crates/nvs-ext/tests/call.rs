//! `rule:packaging/a-fresh-instance-per-request`, `rule:packaging/a-guest-call-yields-on-its-core` and
//! `rule:packaging/a-guest-runs-under-the-requests-budget`: one instance per extension per request,
//! a call parked or yielding on its core, and the request's CPU time and memory as its limits.
//!
//! The guest imports two test functions the world does not offer, so its `Extension` is built here
//! rather than by the loader. The core is [`run`]: one thread, one run queue, a waker per task.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use nvs_ext::call::{Failure, Host, Limit, Meter, Request};
use nvs_ext::load::{Extension, pin};
use nvs_ext::manifest::Manifest;
use nvs_ext::source::{FORMAT, Source};
use wasmtime::component::{Component, Val};

const PAGE: u64 = 65536;

/// A component exporting `shop:counter/api`. `echo` calls the `wait` import, which is pending until
/// the test's gate opens; `spin` loops until the `flag` import is true; `bump` adds one to a global
/// and returns it; `grow` grows the memory by its argument in pages.
const GUEST: &str = r#"(component
  (import "wait" (func $wait (param "x" u64) (result u64)))
  (import "flag" (func $flag (result bool)))
  (core func $wait_low (canon lower (func $wait)))
  (core func $flag_low (canon lower (func $flag)))
  (core module $m
    (import "host" "wait" (func $wait (param i64) (result i64)))
    (import "host" "flag" (func $flag (result i32)))
    (memory 1)
    (global $n (mut i64) (i64.const 0))
    (func (export "double") (param i64) (result i64)
      (i64.mul (local.get 0) (i64.const 2)))
    (func (export "bump") (result i64)
      (global.set $n (i64.add (global.get $n) (i64.const 1)))
      (global.get $n))
    (func (export "grow") (param i32) (result i32)
      (memory.grow (local.get 0)))
    (func (export "forever")
      (loop $again (br $again)))
    (func (export "echo") (param i64) (result i64)
      (call $wait (local.get 0)))
    (func (export "spin") (result i64) (local $n i64)
      (block $done
        (loop $again
          (local.set $n (i64.add (local.get $n) (i64.const 1)))
          (br_if $done (call $flag))
          (br $again)))
      (local.get $n)))
  (core instance $host (export "wait" (func $wait_low)) (export "flag" (func $flag_low)))
  (core instance $i (instantiate $m (with "host" (instance $host))))
  (func $double (param "x" s64) (result s64) (canon lift (core func $i "double")))
  (func $bump (result s64) (canon lift (core func $i "bump")))
  (func $grow (param "pages" u32) (result s32) (canon lift (core func $i "grow")))
  (func $forever (canon lift (core func $i "forever")))
  (func $echo (param "x" u64) (result u64) (canon lift (core func $i "echo")))
  (func $spin (result u64) (canon lift (core func $i "spin")))
  (instance $api
    (export "double" (func $double))
    (export "bump" (func $bump))
    (export "grow" (func $grow))
    (export "forever" (func $forever))
    (export "echo" (func $echo))
    (export "spin" (func $spin)))
  (export "shop:counter/api" (instance $api)))"#;

const METHODS: &str = r#"[
  {"name": "double", "params": [{"name": "x", "type": "int"}], "returns": "int"},
  {"name": "bump", "params": [], "returns": "int"},
  {"name": "grow", "params": [{"name": "pages", "type": "int"}], "returns": "int"},
  {"name": "forever", "params": [], "returns": "void"},
  {"name": "echo", "params": [{"name": "x", "type": "int"}], "returns": "int"},
  {"name": "spin", "params": [], "returns": "int"}
]"#;

/// What the `wait` import waits on.
#[derive(Debug, Default)]
struct Gate {
    open: AtomicBool,
    waiter: Mutex<Option<Waker>>,
}

impl Gate {
    fn open(&self) {
        self.open.store(true, Ordering::Release);
        let waiter = self
            .waiter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(waker) = waiter {
            waker.wake();
        }
    }
}

struct GateWait(Arc<Gate>);

impl Future for GateWait {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.open.load(Ordering::Acquire) {
            return Poll::Ready(());
        }
        *self.0.waiter.lock().unwrap_or_else(PoisonError::into_inner) = Some(cx.waker().clone());
        if self.0.open.load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// A host whose `wait` import waits on `gate` and whose `flag` import reads `flag`.
struct Fixture {
    host: Host,
    gate: Arc<Gate>,
    flag: Arc<AtomicBool>,
}

fn fixture() -> Fixture {
    let gate = Arc::new(Gate::default());
    let flag = Arc::new(AtomicBool::new(false));
    let host = {
        let gate = Arc::clone(&gate);
        let flag = Arc::clone(&flag);
        Host::new(16, move |linker| {
            let mut root = linker.root();
            root.func_wrap_async("wait", move |_, (x,): (u64,)| {
                let gate = Arc::clone(&gate);
                Box::new(async move {
                    GateWait(gate).await;
                    Ok((x.saturating_mul(2),))
                })
            })?;
            root.func_wrap("flag", move |_, (): ()| Ok((flag.load(Ordering::Acquire),)))
        })
        .expect("the host starts")
    };
    Fixture { host, gate, flag }
}

/// The guest as an extension of `host`, under the entry's `memory` and the manifest's `memory`.
fn extension(host: &Host, entry: Option<u64>, manifest: Option<u64>) -> Extension {
    let bytes = wat::parse_str(GUEST).expect("the test component compiles");
    let memory = manifest.map_or(String::new(), |bytes| format!(r#", "memory": {bytes}"#));
    let manifest = format!(
        r#"{{"manifest": 1, "world": "1.0.0", "class": "Shop\\Counter", "interface": "shop:counter/api", "methods": {METHODS}{memory}}}"#
    );
    Extension {
        path: PathBuf::from("counter.nvsx"),
        sha256: pin(&bytes),
        memory: entry,
        grants: nvs_config::extension::Granted::default(),
        component: Component::new(host.engine(), &bytes).expect("the component compiles"),
        manifest: Manifest::parse(manifest.as_bytes()).expect("the manifest reads"),
        source: Source {
            source: FORMAT,
            files: Vec::new(),
        },
    }
}

fn meter(memory: Option<u64>) -> Arc<Meter> {
    Arc::new(Meter::new(Duration::from_secs(30), memory))
}

type Task<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

#[derive(Default)]
struct Queue {
    ready: Mutex<VecDeque<usize>>,
    signal: Condvar,
}

struct TaskWake {
    id: usize,
    queue: Arc<Queue>,
}

impl Wake for TaskWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.queue
            .ready
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(self.id);
        self.queue.signal.notify_one();
    }
}

/// Runs `tasks` to the end on this thread, in order, each to its next `Pending` before the next.
fn run(tasks: Vec<Task<'_>>) {
    let queue = Arc::new(Queue::default());
    let mut tasks: Vec<Option<Task<'_>>> = tasks.into_iter().map(Some).collect();
    let wakers: Vec<Waker> = (0..tasks.len())
        .map(|id| {
            Waker::from(Arc::new(TaskWake {
                id,
                queue: Arc::clone(&queue),
            }))
        })
        .collect();
    queue
        .ready
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .extend(0..tasks.len());
    while tasks.iter().any(Option::is_some) {
        let id = {
            let mut ready = queue.ready.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let Some(id) = ready.pop_front() {
                    break id;
                }
                let (next, timeout) = queue
                    .signal
                    .wait_timeout(ready, Duration::from_secs(20))
                    .unwrap_or_else(PoisonError::into_inner);
                assert!(
                    !timeout.timed_out(),
                    "every task is parked and none was woken"
                );
                ready = next;
            }
        };
        let Some(task) = tasks[id].as_mut() else {
            continue;
        };
        if task
            .as_mut()
            .poll(&mut Context::from_waker(&wakers[id]))
            .is_ready()
        {
            tasks[id] = None;
        }
    }
}

/// `call` run alone on a core.
fn block_on<T>(call: impl Future<Output = T>) -> T {
    let out = RefCell::new(None);
    run(vec![Box::pin(async {
        *out.borrow_mut() = Some(call.await);
    })]);
    out.into_inner().expect("the task finished")
}

fn int(out: &Result<Vec<Val>, Failure>) -> i64 {
    match out.as_deref() {
        Ok([Val::S64(n)]) => *n,
        Ok([Val::U64(n)]) => i64::try_from(*n).expect("the result fits"),
        Ok([Val::S32(n)]) => i64::from(*n),
        other => panic!("one integer was expected, not {other:?}"),
    }
}

fn call(
    request: &Request,
    extension: &Extension,
    method: &str,
    args: &[Val],
) -> Result<Vec<Val>, Failure> {
    block_on(request.call(extension, method, args))
}

#[test]
fn a_guest_call_returns_its_value_to_the_calling_task() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let request = fx.host.request(meter(None));
    assert_eq!(int(&call(&request, &ext, "double", &[Val::S64(21)])), 42);
}

#[test]
fn a_request_creates_an_instance_at_its_first_call_and_drops_it_at_its_end() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let request = fx.host.request(meter(None));
    assert_eq!(fx.host.live_instances(), 0);
    call(&request, &ext, "double", &[Val::S64(1)]).expect("the call returns");
    assert_eq!((request.instances(), fx.host.live_instances()), (1, 1));
    call(&request, &ext, "double", &[Val::S64(2)]).expect("the call returns");
    assert_eq!((request.instances(), fx.host.live_instances()), (1, 1));
    drop(request);
    assert_eq!(fx.host.live_instances(), 0);
}

#[test]
fn a_request_that_calls_no_extension_creates_no_instance() {
    let fx = fixture();
    let budget = meter(None);
    let request = fx.host.request(budget.clone());
    assert_eq!((request.instances(), fx.host.live_instances()), (0, 0));
    drop(request);
    assert_eq!((fx.host.live_instances(), budget.charged()), (0, 0));
}

#[test]
fn a_guest_global_written_in_one_request_is_not_seen_by_the_next() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let first = fx.host.request(meter(None));
    assert_eq!(int(&call(&first, &ext, "bump", &[])), 1);
    assert_eq!(int(&call(&first, &ext, "bump", &[])), 2);
    drop(first);
    let next = fx.host.request(meter(None));
    assert_eq!(int(&call(&next, &ext, "bump", &[])), 1);
}

#[test]
fn two_tasks_of_one_request_calling_one_extension_both_finish() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let request = fx.host.request(meter(None));
    let events = RefCell::new(Vec::new());
    run(vec![
        Box::pin(async {
            let out = request.call(&ext, "echo", &[Val::U64(1)]).await;
            events.borrow_mut().push(format!("first {}", int(&out)));
        }),
        Box::pin(async {
            let out = request.call(&ext, "echo", &[Val::U64(2)]).await;
            events.borrow_mut().push(format!("second {}", int(&out)));
        }),
        Box::pin(async {
            assert_eq!(
                request.instances(),
                1,
                "the second task shares the first one's instance"
            );
            events.borrow_mut().push("open".to_owned());
            fx.gate.open();
        }),
    ]);
    assert_eq!(events.into_inner(), ["open", "first 2", "second 4"]);
    assert_eq!(fx.host.live_instances(), 1);
}

#[test]
fn a_guest_parked_on_a_pending_host_import_lets_another_task_on_its_core_run() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let request = fx.host.request(meter(None));
    let events = RefCell::new(Vec::new());
    run(vec![
        Box::pin(async {
            let out = request.call(&ext, "echo", &[Val::U64(21)]).await;
            events.borrow_mut().push(format!("guest {}", int(&out)));
        }),
        Box::pin(async {
            events.borrow_mut().push("neighbour".to_owned());
            fx.gate.open();
        }),
    ]);
    assert_eq!(events.into_inner(), ["neighbour", "guest 42"]);
}

#[test]
fn a_long_guest_call_yields_to_another_task_on_its_core_at_an_epoch_tick() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let request = fx.host.request(meter(None));
    let events = RefCell::new(Vec::new());
    run(vec![
        Box::pin(async {
            // Only the neighbour sets the flag, so the call returns only if a tick yielded it.
            let out = request.call(&ext, "spin", &[]).await;
            int(&out);
            events.borrow_mut().push("guest");
        }),
        Box::pin(async {
            events.borrow_mut().push("neighbour");
            fx.flag.store(true, Ordering::Release);
        }),
    ]);
    assert_eq!(events.into_inner(), ["neighbour", "guest"]);
}

#[test]
fn an_infinite_guest_loop_stops_at_the_request_cpu_limit() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let request = fx
        .host
        .request(Arc::new(Meter::new(Duration::from_millis(50), None)));
    let started = Instant::now();
    let out = call(&request, &ext, "forever", &[]);
    assert_eq!(out, Err(Failure::Limit(Limit::Cpu)));
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(
        request.instances(),
        0,
        "the failed call dropped its instance"
    );
}

#[test]
fn a_guest_memory_growth_is_charged_to_its_request() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let budget = meter(None);
    let request = fx.host.request(budget.clone());
    call(&request, &ext, "double", &[Val::S64(1)]).expect("the call returns");
    assert_eq!(budget.charged(), PAGE, "the initial page is charged");
    assert_eq!(int(&call(&request, &ext, "grow", &[Val::U32(2)])), 1);
    assert_eq!(budget.charged(), 3 * PAGE);
    drop(request);
    assert_eq!(
        budget.charged(),
        0,
        "the request's end gives the memory back"
    );
}

#[test]
fn a_guest_growing_past_the_request_memory_limit_is_stopped() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let budget = meter(Some(4 * PAGE));
    let request = fx.host.request(budget.clone());
    assert_eq!(int(&call(&request, &ext, "grow", &[Val::U32(2)])), 1);
    let out = call(&request, &ext, "grow", &[Val::U32(5)]);
    assert_eq!(out, Err(Failure::Limit(Limit::Memory)));
    assert_eq!(
        budget.charged(),
        0,
        "the stopped instance gave its memory back"
    );
}

/// How many pages the guest reaches, one page at a time, under the three limits, in pages.
fn reach(request: u64, entry: u64, manifest: u64) -> u64 {
    let fx = fixture();
    let ext = extension(&fx.host, Some(entry * PAGE), Some(manifest * PAGE));
    let request = fx.host.request(meter(Some(request * PAGE)));
    let mut pages = 1;
    loop {
        match call(&request, &ext, "grow", &[Val::U32(1)]) {
            Ok(_) => pages += 1,
            Err(Failure::Limit(Limit::Memory)) => return pages,
            Err(other) => panic!("a memory limit was expected, not {other}"),
        }
    }
}

#[test]
fn the_guest_memory_limit_is_the_least_of_the_request_remainder_the_entry_and_the_manifest() {
    assert_eq!(reach(3, 5, 7), 3, "the request's remainder is the least");
    assert_eq!(reach(7, 3, 5), 3, "the entry's ceiling is the least");
    assert_eq!(reach(5, 7, 3), 3, "the manifest's ceiling is the least");
}

#[test]
fn a_compiled_component_is_shared_by_every_core() {
    let fx = fixture();
    let ext = extension(&fx.host, None, None);
    let cores: Vec<_> = (0..2_i64)
        .map(|core| {
            let host = fx.host.clone();
            let ext = ext.clone();
            std::thread::spawn(move || {
                let request = host.request(meter(None));
                int(&call(&request, &ext, "double", &[Val::S64(core)]))
            })
        })
        .collect();
    let out: Vec<i64> = cores
        .into_iter()
        .map(|core| core.join().expect("the core finishes"))
        .collect();
    assert_eq!(out, [0, 2]);
}
