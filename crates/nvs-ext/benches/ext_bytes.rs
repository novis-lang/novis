//! Times a third-party extension's `bytes` crossing and counts the host heap it takes: the ledger
//! fixture's `echoBytes` at 1 KiB, 1 MiB and 16 MiB, called through
//! [`Request::call_values`](nvs_ext::call::Request::call_values) as `nvs run` calls it
//! (`rule:packaging/the-boundary-is-the-cost`). A call's time includes the conversion to the
//! export's WIT values, the copy into guest memory and back, and the conversion back to a Novis
//! value.
//!
//! The heap figures come from a counting global allocator. `allocated_bytes` is every byte the
//! host allocated during one call, and `peak_bytes` is the most the call held at once above what
//! was live before it. Guest linear memory is not on the host heap, so neither counts it.
//!
//! `cargo bench -p nvs-ext --bench ext_bytes -- --record <row>` writes the figures as `<row>`
//! (`before` or `after`) into `benches/results/ext-bytes.json`, and keeps the file's other rows.

use std::alloc::{GlobalAlloc, Layout, System};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use nvs_ext::call::{Host, Meter, Request};
use nvs_ext::convert::Value;
use nvs_ext::load::{Entry, Extension, Loader, pin};

/// The global allocator, counting what it hands out.
struct Counting;

/// Bytes allocated since the process started, growth by `realloc` included.
static ALLOCATED: AtomicU64 = AtomicU64::new(0);
/// Bytes live now.
static LIVE: AtomicU64 = AtomicU64::new(0);
/// The most bytes live at once since [`Counting::reset_peak`].
static PEAK: AtomicU64 = AtomicU64::new(0);

impl Counting {
    fn grew(bytes: usize) {
        let bytes = bytes as u64;
        ALLOCATED.fetch_add(bytes, Ordering::Relaxed);
        let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
        PEAK.fetch_max(live, Ordering::Relaxed);
    }

    fn shrank(bytes: usize) {
        LIVE.fetch_sub(bytes as u64, Ordering::Relaxed);
    }

    fn reset_peak() {
        PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

// SAFETY: every call forwards to `System` with the same arguments, and only counts around it.
#[allow(unsafe_code, reason = "a global allocator is an unsafe trait")]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is `System::alloc`'s.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            Self::grew(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is `System::alloc_zeroed`'s.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            Self::grew(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract is `System::dealloc`'s.
        unsafe { System.dealloc(ptr, layout) };
        Self::shrank(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller's contract is `System::realloc`'s.
        let out = unsafe { System.realloc(ptr, layout, new_size) };
        if !out.is_null() {
            Self::shrank(layout.size());
            Self::grew(new_size);
        }
        out
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// The sizes each call echoes, with the name each row has.
const SIZES: [(&str, usize); 3] = [
    ("1 KiB", 1 << 10),
    ("1 MiB", 1 << 20),
    ("16 MiB", 16 << 20),
];
/// Untimed calls at each size before the timed ones.
const WARMUP: usize = 2;
/// Timed calls at each size. The median is the figure.
const RUNS: usize = 9;

struct Unpark(std::thread::Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Runs `call` to the end on this thread.
fn block_on<T>(call: impl Future<Output = T>) -> T {
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut call = std::pin::pin!(call);
    loop {
        if let Poll::Ready(out) = call.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::park_timeout(Duration::from_millis(100));
    }
}

/// The committed ledger fixture, loaded as `nvs run` loads it.
fn ledger(host: &Host) -> Extension {
    let path = nvs_repo::path("tests/conformance/ext/fixtures/ledger.nvsx");
    let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let entry = Entry {
        path: PathBuf::from("ledger.nvsx"),
        sha256: pin(&bytes),
        memory: None,
        grants: nvs_config::extension::Granted::default(),
    };
    Loader::new(host.engine())
        .load_bytes(&entry, &bytes)
        .expect("the ledger fixture loads")
}

/// What one timed call took.
struct Run {
    ns: u64,
    allocated: u64,
    peak: u64,
}

/// One `echoBytes` call with `data`, timed and counted. `data` is cloned before the clock starts.
fn echo(request: &Request, ledger: &Extension, data: &[u8]) -> Run {
    let args = vec![Value::Bytes(data.to_vec())];
    let live = LIVE.load(Ordering::Relaxed);
    Counting::reset_peak();
    let allocated = ALLOCATED.load(Ordering::Relaxed);
    let start = Instant::now();
    let out = block_on(request.call_values(ledger, "echoBytes", args));
    let ns = u64::try_from(start.elapsed().as_nanos()).expect("a call fits in u64 nanoseconds");
    let run = Run {
        ns,
        allocated: ALLOCATED.load(Ordering::Relaxed) - allocated,
        peak: PEAK.load(Ordering::Relaxed).saturating_sub(live),
    };
    match out {
        Ok(Some(Value::Bytes(back))) => assert!(back == data, "`echoBytes` changed its bytes"),
        other => panic!("`echoBytes` returned {other:?}"),
    }
    run
}

/// The median of `figures`.
fn median(mut figures: Vec<u64>) -> u64 {
    figures.sort_unstable();
    figures[figures.len() / 2]
}

#[allow(clippy::print_stdout, reason = "a bench reports on stdout")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let record = args
        .iter()
        .position(|arg| arg == "--record")
        .map(|at| args.get(at + 1).expect("`--record` names its row").clone());
    let host = Host::new(1, |_| Ok(())).expect("the host starts");
    let ledger = ledger(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(3600), None)));
    let mut sizes = serde_json::Map::new();
    for (name, size) in SIZES {
        let data: Vec<u8> = (0..size)
            .map(|at| u8::try_from(at % 251).expect("a byte"))
            .collect();
        for _ in 0..WARMUP {
            echo(&request, &ledger, &data);
        }
        let runs: Vec<Run> = (0..RUNS).map(|_| echo(&request, &ledger, &data)).collect();
        let ns = median(runs.iter().map(|run| run.ns).collect());
        let allocated = median(runs.iter().map(|run| run.allocated).collect());
        let peak = median(runs.iter().map(|run| run.peak).collect());
        #[allow(clippy::cast_precision_loss, reason = "figures for a person to read")]
        let per_byte = |figure: u64| figure as f64 / size as f64;
        #[allow(clippy::cast_precision_loss, reason = "milliseconds for a person to read")]
        let ms = ns as f64 / 1e6;
        println!(
            "{name:<7} {ms:>10.3} ms  allocated {allocated:>12} B ({:.1} per byte)  peak {peak:>12} B ({:.1} per byte)",
            per_byte(allocated),
            per_byte(peak),
        );
        sizes.insert(
            name.to_owned(),
            serde_json::json!({
                "bytes": size,
                "median_ns": ns,
                "allocated_bytes": allocated,
                "peak_bytes": peak,
            }),
        );
    }
    block_on(request.end()).expect("the request ends");
    if let Some(row) = record {
        let path = nvs_repo::path("benches/results/ext-bytes.json");
        let mut figures = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .unwrap_or_else(|| serde_json::json!({ "rows": {} }));
        figures["how"] = serde_json::json!(format!(
            "`cargo bench -p nvs-ext --bench ext_bytes -- --record <row>`: the median of {RUNS} calls of the ledger fixture's `echoBytes` at each size after {WARMUP} untimed ones, in a release build, through `Request::call_values`. `allocated_bytes` is every host heap byte one call allocated, and `peak_bytes` the most it held at once; guest linear memory is not on the host heap. `before` is the dynamic path, which crosses a byte list as one wasmtime `Val` per byte; `after` is the host-held `bytes` resource."
        ));
        figures["rows"][row.as_str()] = serde_json::json!({
            "host": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
            "sizes": sizes,
        });
        let mut text = serde_json::to_string_pretty(&figures).expect("the figures serialize");
        text.push('\n');
        std::fs::write(&path, text).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        println!("wrote {}", path.display());
    }
}
