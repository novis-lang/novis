//! `rule:packaging/a-guest-has-no-ambient-authority`. The link half: the WASI interfaces a guest's
//! linker defines are exactly `nvs_ext::wasi::LINKED`, proved by instantiating one probe component
//! per WASI interface the world's WIT holds. The context half: a guest calling those interfaces
//! finds them empty, its output reaches its request's log, and its clock and random bytes are its
//! request's. A guest shaped like a toolchain's libc fits one slot, and its `exit` throws. The
//! world half: `nvs:ext/log` writes to the request's log under the extension's class, and
//! `nvs:ext/settings` reads the extension's block from the request's configuration, which boot
//! checks against the manifest's keys.
//!
//! Each guest is a core module written against the canonical ABI, made a component by
//! `wit-component` from its world, so no wasm toolchain is needed.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::call::{Budget, Class, Failure, Host, Level, Outcome, Request};
use nvs_ext::load::{Extension, pin as pin_of};
use nvs_ext::manifest::Manifest;
use nvs_ext::source::{FORMAT, Source};
use nvs_ext::wasi::LINKED;
use wasmtime::component::{Component, Val};
use wit_component::{ComponentEncoder, StringEncoding, dummy_module, embed_component_metadata};
use wit_parser::{ManglingAndAbi, Resolve};

/// Every WASI interface under `wit/nvs-ext/`, as `wasi:<package>/<interface>@<version>`.
fn wasi_interfaces(resolve: &Resolve) -> Vec<String> {
    let mut out = Vec::new();
    for (_, package) in &resolve.packages {
        if package.name.namespace != "wasi" {
            continue;
        }
        for name in package.interfaces.keys() {
            out.push(format!(
                "wasi:{}/{name}@{}",
                package.name.name,
                package
                    .name
                    .version
                    .as_ref()
                    .expect("a WASI package is versioned")
            ));
        }
    }
    out
}

/// A component importing `interface` alone, and whatever its types are written in.
fn probe(resolve: &mut Resolve, index: usize, interface: &str) -> Vec<u8> {
    let package = resolve
        .push_str(
            format!("probe{index}.wit"),
            &format!("package probe:p{index};\nworld w {{\n    import {interface};\n}}\n"),
        )
        .expect("the probe world parses");
    let world = resolve
        .select_world(&[package], Some("w"))
        .expect("the probe world exists");
    let mut module = dummy_module(resolve, world, ManglingAndAbi::Standard32);
    embed_component_metadata(&mut module, resolve, world, StringEncoding::UTF8)
        .expect("the probe's metadata embeds");
    ComponentEncoder::default()
        .module(&module)
        .expect("the probe module reads")
        .validate(true)
        .encode()
        .expect("the probe componentizes")
}

/// The name of `interface` without its version.
fn unversioned(interface: &str) -> &str {
    interface
        .split_once('@')
        .map_or(interface, |(name, _)| name)
}

#[test]
fn a_guest_linker_defines_only_the_worlds_wasi_interfaces() {
    let mut resolve = Resolve::default();
    resolve
        .push_dir(nvs_repo::path("wit/nvs-ext"))
        .expect("the world's WIT parses");
    let interfaces = wasi_interfaces(&resolve);
    let probes: Vec<(String, Vec<u8>)> = interfaces
        .iter()
        .enumerate()
        .map(|(index, interface)| (interface.clone(), probe(&mut resolve, index, interface)))
        .collect();
    assert!(
        probes.len() > LINKED.len(),
        "the WIT holds the WASI interfaces the host does not link too"
    );
    let mut linked = BTreeSet::new();
    Host::new(1, |linker| {
        for (interface, bytes) in &probes {
            let component = Component::new(linker.engine(), bytes)?;
            if linker.instantiate_pre(&component).is_ok() {
                linked.insert(unversioned(interface).to_owned());
            }
        }
        Ok(())
    })
    .expect("the host starts");
    let expected: BTreeSet<String> = LINKED.iter().map(|&name| name.to_owned()).collect();
    assert_eq!(linked, expected);
}

/// The context guest's world: the WASI it reads, and one export per question it asks of it.
const GUEST_WIT: &str = "package shop:probe;

interface api {
    say: func();
    stdin: func() -> s64;
    environment: func() -> u32;
    arguments: func() -> u32;
    cwd: func() -> u32;
    directories: func() -> u32;
    seconds: func() -> s64;
    nanos: func() -> u32;
    draw: func() -> s64;
    fill: func() -> s64;
}

world guest {
    import wasi:cli/environment@0.2.12;
    import wasi:cli/stdin@0.2.12;
    import wasi:cli/stdout@0.2.12;
    import wasi:cli/stderr@0.2.12;
    import wasi:clocks/wall-clock@0.2.12;
    import wasi:random/random@0.2.12;
    import wasi:filesystem/preopens@0.2.12;
    export api;
}
";

/// The context guest's core module. `say` writes `hello\nworld` to stdout and `oops\n` to stderr.
/// `stdin` reads up to 16 bytes and returns how many it read, `-1` at the end of the stream and
/// `-2` for a failed read. The other exports return the length of the list, or `1` for a working
/// directory and `0` for none. `seconds` and `nanos` read the wall clock. `draw` returns
/// `get-random-u64`, and `fill` returns the eight bytes `get-random-bytes` gives, read
/// little-endian. Every import returning more than one value writes it at 16.
const GUEST_CORE: &str = r#"(module
  (import "wasi:cli/stdout@0.2.12" "get-stdout" (func $stdout (result i32)))
  (import "wasi:cli/stderr@0.2.12" "get-stderr" (func $stderr (result i32)))
  (import "wasi:cli/stdin@0.2.12" "get-stdin" (func $stdin (result i32)))
  (import "wasi:io/streams@0.2.12" "[method]output-stream.blocking-write-and-flush"
    (func $write (param i32 i32 i32 i32)))
  (import "wasi:io/streams@0.2.12" "[method]input-stream.read" (func $read (param i32 i64 i32)))
  (import "wasi:io/streams@0.2.12" "[resource-drop]output-stream" (func $drop_out (param i32)))
  (import "wasi:io/streams@0.2.12" "[resource-drop]input-stream" (func $drop_in (param i32)))
  (import "wasi:cli/environment@0.2.12" "get-environment" (func $env (param i32)))
  (import "wasi:cli/environment@0.2.12" "get-arguments" (func $args (param i32)))
  (import "wasi:cli/environment@0.2.12" "initial-cwd" (func $cwd (param i32)))
  (import "wasi:filesystem/preopens@0.2.12" "get-directories" (func $dirs (param i32)))
  (import "wasi:clocks/wall-clock@0.2.12" "now" (func $wall (param i32)))
  (import "wasi:random/random@0.2.12" "get-random-u64" (func $draw (result i64)))
  (import "wasi:random/random@0.2.12" "get-random-bytes" (func $fill (param i64 i32)))
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 4096))
  (data (i32.const 64) "hello\nworld")
  (data (i32.const 96) "oops\n")
  (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
    (local.set $at
      (i32.and
        (i32.add (global.get $heap) (i32.sub (local.get 2) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get 2))))
    (global.set $heap (i32.add (local.get $at) (local.get 3)))
    (local.get $at))
  (func (export "shop:probe/api#say") (local $out i32)
    (local.set $out (call $stdout))
    (call $write (local.get $out) (i32.const 64) (i32.const 11) (i32.const 16))
    (call $drop_out (local.get $out))
    (local.set $out (call $stderr))
    (call $write (local.get $out) (i32.const 96) (i32.const 5) (i32.const 16))
    (call $drop_out (local.get $out)))
  (func (export "shop:probe/api#stdin") (result i64) (local $in i32)
    (local.set $in (call $stdin))
    (call $read (local.get $in) (i64.const 16) (i32.const 16))
    (call $drop_in (local.get $in))
    (if (result i64) (i32.load8_u (i32.const 16))
      (then (i64.sub (i64.extend_i32_u (i32.load8_u (i32.const 20))) (i64.const 2)))
      (else (i64.extend_i32_u (i32.load (i32.const 24))))))
  (func (export "shop:probe/api#environment") (result i32)
    (call $env (i32.const 16))
    (i32.load (i32.const 20)))
  (func (export "shop:probe/api#arguments") (result i32)
    (call $args (i32.const 16))
    (i32.load (i32.const 20)))
  (func (export "shop:probe/api#cwd") (result i32)
    (call $cwd (i32.const 16))
    (i32.load8_u (i32.const 16)))
  (func (export "shop:probe/api#directories") (result i32)
    (call $dirs (i32.const 16))
    (i32.load (i32.const 20)))
  (func (export "shop:probe/api#seconds") (result i64)
    (call $wall (i32.const 16))
    (i64.load (i32.const 16)))
  (func (export "shop:probe/api#nanos") (result i32)
    (call $wall (i32.const 16))
    (i32.load (i32.const 24)))
  (func (export "shop:probe/api#draw") (result i64)
    (call $draw))
  (func (export "shop:probe/api#fill") (result i64)
    (call $fill (i64.const 8) (i32.const 16))
    (i64.load (i32.load (i32.const 16)))))"#;

const GUEST_METHODS: &str = r#"[
  {"name": "say", "params": [], "returns": "void"},
  {"name": "stdin", "params": [], "returns": "int"},
  {"name": "environment", "params": [], "returns": "int"},
  {"name": "arguments", "params": [], "returns": "int"},
  {"name": "cwd", "params": [], "returns": "int"},
  {"name": "directories", "params": [], "returns": "int"},
  {"name": "seconds", "params": [], "returns": "int"},
  {"name": "nanos", "params": [], "returns": "int"},
  {"name": "draw", "params": [], "returns": "int"},
  {"name": "fill", "params": [], "returns": "int"}
]"#;

/// The core module `core`, componentized against the world `guest` in `wit`.
fn guest_bytes(wit: &str, core: &str) -> Vec<u8> {
    let mut resolve = Resolve::default();
    resolve
        .push_dir(nvs_repo::path("wit/nvs-ext"))
        .expect("the world's WIT parses");
    let package = resolve
        .push_str("guest.wit", wit)
        .expect("the guest's world parses");
    let world = resolve
        .select_world(&[package], Some("guest"))
        .expect("the guest's world exists");
    let mut module = wat::parse_str(core).expect("the guest's core module compiles");
    embed_component_metadata(&mut module, &resolve, world, StringEncoding::UTF8)
        .expect("the guest's metadata embeds");
    ComponentEncoder::default()
        .module(&module)
        .expect("the guest's core module reads")
        .validate(true)
        .encode()
        .expect("the guest componentizes")
}

/// The context guest as an extension of `host`, declaring `Shop\Probe`.
fn guest(host: &Host) -> Extension {
    extension(
        host,
        &guest_bytes(GUEST_WIT, GUEST_CORE),
        "Shop\\\\Probe",
        "shop:probe/api",
        GUEST_METHODS,
    )
}

/// The component `bytes` as an extension of `host`, declaring `class` over `interface`'s
/// `methods`. `class` is written as it reads inside a JSON string.
fn extension(host: &Host, bytes: &[u8], class: &str, interface: &str, methods: &str) -> Extension {
    declaring(
        host,
        bytes,
        &format!(
            r#"{{"manifest": 1, "world": "1.0.0", "class": "{class}", "interface": "{interface}", "methods": {methods}}}"#
        ),
    )
}

/// The component `bytes` as an extension of `host`, declared by the manifest `manifest`.
fn declaring(host: &Host, bytes: &[u8], manifest: &str) -> Extension {
    Extension {
        path: PathBuf::from("probe.nvsx"),
        sha256: pin_of(bytes),
        memory: None,
        grants: nvs_config::extension::Granted::default(),
        component: Component::new(host.engine(), bytes).expect("the guest compiles"),
        manifest: Manifest::parse(manifest.as_bytes()).expect("the manifest reads"),
        source: Source {
            source: FORMAT,
            files: Vec::new(),
        },
    }
}

/// One record a guest wrote to its request's log: the level, the channel and the message.
type Record = (Level, String, String);

/// A request generator whose sequence its seed fixes: SplitMix64.
#[derive(Debug, Default)]
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A request budget with no limits, a fixed wall clock, a seeded generator and the settings
/// blocks of its configuration, which keeps every record written to its log.
#[derive(Debug, Default)]
struct Recorder {
    records: Mutex<Vec<Record>>,
    clock: Duration,
    generator: Mutex<Seeded>,
    settings: BTreeMap<String, serde_json::Map<String, serde_json::Value>>,
}

impl Recorder {
    fn fixed(clock: Duration, seed: u64) -> Self {
        Self {
            clock,
            generator: Mutex::new(Seeded(seed)),
            ..Self::default()
        }
    }

    fn records(&self) -> Vec<Record> {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Budget for Recorder {
    fn cpu_spent(&self) -> bool {
        false
    }

    fn charge(&self, _bytes: i64) -> bool {
        true
    }

    fn log(&self, level: Level, channel: &str, message: &str) {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((level, channel.to_owned(), message.to_owned()));
    }

    fn wall_clock(&self) -> Duration {
        self.clock
    }

    fn monotonic_clock(&self) -> u64 {
        0
    }

    fn random(&self, out: &mut [u8]) {
        let mut generator = self
            .generator
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for chunk in out.chunks_mut(8) {
            let bytes = generator.next().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }

    fn setting(&self, block: &str, key: &str) -> Option<serde_json::Value> {
        self.settings
            .get(block)
            .and_then(|values| values.get(key))
            .cloned()
    }
}

/// Polls `future` on this thread until it is ready. Every guest here runs to its end without
/// waiting on anything, so a pending poll only means an epoch tick yielded.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
    }
}

/// A host with one slot linking nothing but the world, a request on it keeping its log in a
/// [`Recorder`], and the context guest.
fn context() -> (Request, Extension, Arc<Recorder>) {
    context_of(Recorder::default())
}

/// [`context`], with `recorder` as the request's budget.
fn context_of(recorder: Recorder) -> (Request, Extension, Arc<Recorder>) {
    let host = Host::new(1, |_| Ok(())).expect("the host starts");
    let extension = guest(&host);
    let recorder = Arc::new(recorder);
    let request = host.request(Arc::clone(&recorder) as Arc<dyn Budget>);
    (request, extension, recorder)
}

fn ask(request: &Request, extension: &Extension, method: &str) -> Result<Vec<Val>, Failure> {
    block_on(request.call(extension, method, &[]))
}

#[test]
fn a_guest_sees_no_environment_no_arguments_and_no_directories() {
    let (request, extension, _) = context();
    for method in ["environment", "arguments", "cwd", "directories"] {
        let out = ask(&request, &extension, method).expect("the call returns");
        assert_eq!(out, [Val::U32(0)], "`{method}` found something");
    }
}

#[test]
fn a_guest_reading_stdin_reads_nothing() {
    let (request, extension, _) = context();
    let out = ask(&request, &extension, "stdin").expect("the call returns");
    assert_eq!(out, [Val::S64(-1)], "stdin is at the end of its stream");
}

#[test]
fn a_guest_writing_to_stdout_and_stderr_reaches_the_request_log_at_debug() {
    let (request, extension, recorder) = context();
    ask(&request, &extension, "say").expect("the call returns");
    let record = |message: &str| (Level::Debug, "Shop\\Probe".to_owned(), message.to_owned());
    assert_eq!(
        recorder.records(),
        [record("hello"), record("world"), record("oops")]
    );
}

#[test]
fn a_guest_reading_the_wall_clock_under_a_fixed_clock_reads_the_fixed_time() {
    let fixed = Duration::new(1_700_000_000, 123_456_789);
    let (request, extension, _) = context_of(Recorder::fixed(fixed, 0));
    let seconds = ask(&request, &extension, "seconds").expect("the call returns");
    let nanos = ask(&request, &extension, "nanos").expect("the call returns");
    assert_eq!(seconds, [Val::S64(1_700_000_000)]);
    assert_eq!(nanos, [Val::U32(123_456_789)]);
}

#[test]
fn a_guest_drawing_random_bytes_under_a_seed_draws_the_seeded_bytes() {
    let seed = 42;
    let (request, extension, _) = context_of(Recorder::fixed(Duration::ZERO, seed));
    let draw = ask(&request, &extension, "draw").expect("the call returns");
    let fill = ask(&request, &extension, "fill").expect("the call returns");
    let mut expected = Seeded(seed);
    let as_int = |value: u64| Val::S64(i64::from_le_bytes(value.to_le_bytes()));
    assert_eq!(
        [draw, fill],
        [[as_int(expected.next())], [as_int(expected.next())]]
    );
}

/// A guest importing every WASI interface the world links but `wasi:io/error`, as a toolchain's
/// libc does at startup.
const LIBC_WIT: &str = "package shop:libc;

interface api {
    run: func() -> u32;
    quit: func();
}

world guest {
    import wasi:cli/environment@0.2.12;
    import wasi:cli/exit@0.2.12;
    import wasi:cli/stdin@0.2.12;
    import wasi:cli/stdout@0.2.12;
    import wasi:cli/stderr@0.2.12;
    import wasi:clocks/monotonic-clock@0.2.12;
    import wasi:clocks/wall-clock@0.2.12;
    import wasi:random/random@0.2.12;
    import wasi:random/insecure@0.2.12;
    import wasi:random/insecure-seed@0.2.12;
    import wasi:filesystem/types@0.2.12;
    import wasi:filesystem/preopens@0.2.12;
    export api;
}
";

/// The libc-shaped guest's core module, with a function table of its own as a C or Rust guest
/// has. `run` does what a libc's startup does: it opens its streams, reads its environment, its
/// arguments, its working directory and its preopens, reads both clocks and draws from all three
/// generators, then writes `ready` through its table. It returns how many environment variables,
/// arguments, working directories and preopens it found. `quit` calls `exit` with an error.
const LIBC_CORE: &str = r#"(module
  (type $void (func))
  (import "wasi:cli/environment@0.2.12" "get-environment" (func $env (param i32)))
  (import "wasi:cli/environment@0.2.12" "get-arguments" (func $args (param i32)))
  (import "wasi:cli/environment@0.2.12" "initial-cwd" (func $cwd (param i32)))
  (import "wasi:cli/exit@0.2.12" "exit" (func $exit (param i32)))
  (import "wasi:cli/stdin@0.2.12" "get-stdin" (func $stdin (result i32)))
  (import "wasi:cli/stdout@0.2.12" "get-stdout" (func $stdout (result i32)))
  (import "wasi:cli/stderr@0.2.12" "get-stderr" (func $stderr (result i32)))
  (import "wasi:io/streams@0.2.12" "[method]output-stream.blocking-write-and-flush"
    (func $write (param i32 i32 i32 i32)))
  (import "wasi:io/streams@0.2.12" "[resource-drop]output-stream" (func $drop_out (param i32)))
  (import "wasi:io/streams@0.2.12" "[resource-drop]input-stream" (func $drop_in (param i32)))
  (import "wasi:clocks/monotonic-clock@0.2.12" "now" (func $monotonic (result i64)))
  (import "wasi:clocks/wall-clock@0.2.12" "now" (func $wall (param i32)))
  (import "wasi:random/random@0.2.12" "get-random-bytes" (func $random (param i64 i32)))
  (import "wasi:random/insecure@0.2.12" "get-insecure-random-u64" (func $insecure (result i64)))
  (import "wasi:random/insecure-seed@0.2.12" "insecure-seed" (func $seed (param i32)))
  (import "wasi:filesystem/types@0.2.12" "filesystem-error-code" (func $fs_error (param i32 i32)))
  (import "wasi:filesystem/preopens@0.2.12" "get-directories" (func $dirs (param i32)))
  (memory (export "memory") 1)
  (table 1 funcref)
  (elem (i32.const 0) $greet)
  (global $heap (mut i32) (i32.const 4096))
  (data (i32.const 64) "ready\n")
  (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
    (local.set $at
      (i32.and
        (i32.add (global.get $heap) (i32.sub (local.get 2) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get 2))))
    (global.set $heap (i32.add (local.get $at) (local.get 3)))
    (local.get $at))
  (func $greet (local $out i32)
    (local.set $out (call $stdout))
    (call $write (local.get $out) (i32.const 64) (i32.const 6) (i32.const 16))
    (call $drop_out (local.get $out)))
  (func (export "shop:libc/api#run") (result i32) (local $found i32)
    (call $drop_in (call $stdin))
    (call $drop_out (call $stderr))
    (call $env (i32.const 16))
    (local.set $found (i32.load (i32.const 20)))
    (call $args (i32.const 16))
    (local.set $found (i32.add (local.get $found) (i32.load (i32.const 20))))
    (call $cwd (i32.const 16))
    (local.set $found (i32.add (local.get $found) (i32.load8_u (i32.const 16))))
    (call $dirs (i32.const 16))
    (local.set $found (i32.add (local.get $found) (i32.load (i32.const 20))))
    (drop (call $monotonic))
    (call $wall (i32.const 16))
    (call $random (i64.const 16) (i32.const 16))
    (drop (call $insecure))
    (call $seed (i32.const 16))
    (call_indirect (type $void) (i32.const 0))
    (local.get $found))
  (func (export "shop:libc/api#quit")
    (call $exit (i32.const 1))))"#;

const LIBC_METHODS: &str = r#"[
  {"name": "run", "params": [], "returns": "int"},
  {"name": "quit", "params": [], "returns": "void"}
]"#;

/// A host with one slot linking nothing but the world, a request on it keeping its log in a
/// [`Recorder`], and the libc-shaped guest.
fn libc() -> (Request, Extension, Arc<Recorder>) {
    let host = Host::new(1, |_| Ok(())).expect("the host starts");
    let extension = extension(
        &host,
        &guest_bytes(LIBC_WIT, LIBC_CORE),
        "Shop\\\\Libc",
        "shop:libc/api",
        LIBC_METHODS,
    );
    let recorder = Arc::new(Recorder::default());
    let request = host.request(Arc::clone(&recorder) as Arc<dyn Budget>);
    (request, extension, recorder)
}

#[test]
fn a_libc_shaped_guest_loads_and_runs_with_no_authority() {
    let (request, extension, recorder) = libc();
    let out = ask(&request, &extension, "run").expect("the call returns");
    assert_eq!(out, [Val::U32(0)], "the guest found something to hold");
    assert_eq!(
        recorder.records(),
        [(Level::Debug, "Shop\\Libc".to_owned(), "ready".to_owned())]
    );
}

/// The world guest's world: `nvs:ext/log` and `nvs:ext/settings`, and one export per question.
const WORLD_WIT: &str = "package shop:geo;

interface api {
    note: func();
    precision: func() -> s64;
    zoom: func() -> s64;
    colour: func() -> s64;
}

world guest {
    import nvs:ext/log@1.0.0;
    import nvs:ext/settings@1.0.0;
    export api;
}
";

/// The world guest's core module. `note` writes `ready` at `info` and `lost` at `critical`.
/// `precision`, `zoom` and `colour` read the setting of that name: its value when it is an `int`,
/// `-1` for `none` and `-2` for another case. `get` writes its `option<setting>` at 32: the
/// option's case at 32, the setting's case at 40, and its payload at 48.
const WORLD_CORE: &str = r#"(module
  (import "nvs:ext/log@1.0.0" "write" (func $write (param i32 i32 i32)))
  (import "nvs:ext/settings@1.0.0" "get" (func $get (param i32 i32 i32)))
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 4096))
  (data (i32.const 64) "ready")
  (data (i32.const 80) "lost")
  (data (i32.const 96) "precision")
  (data (i32.const 112) "zoom")
  (data (i32.const 128) "colour")
  (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
    (local.set $at
      (i32.and
        (i32.add (global.get $heap) (i32.sub (local.get 2) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get 2))))
    (global.set $heap (i32.add (local.get $at) (local.get 3)))
    (local.get $at))
  (func $setting (param $key i32) (param $len i32) (result i64)
    (call $get (local.get $key) (local.get $len) (i32.const 32))
    (if (result i64) (i32.eqz (i32.load8_u (i32.const 32)))
      (then (i64.const -1))
      (else
        (if (result i64) (i32.eq (i32.load8_u (i32.const 40)) (i32.const 1))
          (then (i64.load (i32.const 48)))
          (else (i64.const -2))))))
  (func (export "shop:geo/api#note")
    (call $write (i32.const 1) (i32.const 64) (i32.const 5))
    (call $write (i32.const 4) (i32.const 80) (i32.const 4)))
  (func (export "shop:geo/api#precision") (result i64)
    (call $setting (i32.const 96) (i32.const 9)))
  (func (export "shop:geo/api#zoom") (result i64)
    (call $setting (i32.const 112) (i32.const 4)))
  (func (export "shop:geo/api#colour") (result i64)
    (call $setting (i32.const 128) (i32.const 6))))"#;

/// The world guest's manifest: `Shop\Geo`, reading the `[ext.geo]` block's `precision` and
/// `zoom`, two `int`s whose defaults are `6` and `3`.
const WORLD_MANIFEST: &str = r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Geo",
  "interface": "shop:geo/api",
  "methods": [
    {"name": "note", "params": [], "returns": "void"},
    {"name": "precision", "params": [], "returns": "int"},
    {"name": "zoom", "params": [], "returns": "int"},
    {"name": "colour", "params": [], "returns": "int"}
  ],
  "settings": {"name": "geo", "keys": [
    {"name": "precision", "type": "int", "default": 6},
    {"name": "zoom", "type": "int", "default": 3}
  ]}}"#;

/// The settings block `json`, as a configuration snapshot hands it to a budget.
fn block(json: &str) -> serde_json::Map<String, serde_json::Value> {
    serde_json::from_str(json).expect("the block reads")
}

/// A host with one slot, a request on it whose budget is `recorder`, and the world guest.
fn world(recorder: Recorder) -> (Request, Extension, Arc<Recorder>) {
    let host = Host::new(1, |_| Ok(())).expect("the host starts");
    let extension = declaring(&host, &guest_bytes(WORLD_WIT, WORLD_CORE), WORLD_MANIFEST);
    let recorder = Arc::new(recorder);
    let request = host.request(Arc::clone(&recorder) as Arc<dyn Budget>);
    (request, extension, recorder)
}

#[test]
fn a_guest_writing_to_nvs_ext_log_reaches_the_request_log_under_its_name() {
    let (request, extension, recorder) = world(Recorder::default());
    ask(&request, &extension, "note").expect("the call returns");
    assert_eq!(
        recorder.records(),
        [
            (Level::Info, "Shop\\Geo".to_owned(), "ready".to_owned()),
            (Level::Critical, "Shop\\Geo".to_owned(), "lost".to_owned()),
        ]
    );
}

#[test]
fn a_guest_reads_its_settings_block_from_the_request_snapshot() {
    let mut recorder = Recorder::default();
    recorder
        .settings
        .insert("geo".to_owned(), block(r#"{"precision": 9}"#));
    let (request, extension, _) = world(recorder);
    let read = |method: &str| ask(&request, &extension, method).expect("the call returns");
    assert_eq!(read("precision"), [Val::S64(9)], "the request's value");
    assert_eq!(read("zoom"), [Val::S64(3)], "a key the block does not set");
    assert_eq!(
        read("colour"),
        [Val::S64(-1)],
        "a key the manifest does not declare"
    );

    let (request, extension, _) = world(Recorder::default());
    let out = ask(&request, &extension, "precision").expect("the call returns");
    assert_eq!(
        out,
        [Val::S64(6)],
        "a request with no block reads the default"
    );
}

#[test]
fn an_unknown_key_in_an_extension_settings_block_is_refused_at_boot() {
    let manifest = Manifest::parse(WORLD_MANIFEST.as_bytes()).expect("the manifest reads");
    let settings = manifest.settings.expect("the manifest declares a block");
    assert_eq!(
        settings.check(&block(r#"{"precision": 9, "zoom": 1}"#)),
        Ok(())
    );

    let refused = settings
        .check(&block(r#"{"precision": 9, "colour": "red"}"#))
        .expect_err("an unknown key is refused");
    assert!(refused.contains("`[ext.geo]`"), "{refused}");
    assert!(refused.contains("`colour`"), "{refused}");

    let refused = settings
        .check(&block(r#"{"zoom": "far"}"#))
        .expect_err("a value of another type is refused");
    assert!(refused.contains("`zoom`"), "{refused}");
    assert!(refused.contains("`int`"), "{refused}");
}

#[test]
fn a_guest_calling_exit_throws_extension_error_and_the_next_call_succeeds() {
    let (request, extension, _) = libc();
    let failure = ask(&request, &extension, "quit").expect_err("`exit` does not return");
    assert!(matches!(failure, Failure::Trap(_)), "{failure:?}");
    let Outcome::Throw(class, message) = failure.outcome() else {
        panic!("`exit` is not a limit: {failure:?}");
    };
    assert_eq!(class, Class::Extension);
    assert!(message.contains("`exit`"), "{message}");
    let out = ask(&request, &extension, "run").expect("the next call returns");
    assert_eq!(out, [Val::U32(0)]);
}
