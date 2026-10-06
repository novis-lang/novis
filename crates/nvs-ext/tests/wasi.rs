//! `rule:packaging/a-guest-has-no-ambient-authority`. The link half: the WASI interfaces a guest's
//! linker defines are exactly `nvs_ext::wasi::LINKED`, proved by instantiating one probe component
//! per WASI interface the world's WIT holds. The context half: a guest calling those interfaces
//! finds them empty, and its output reaches its request's log.
//!
//! The guest of the context half is a core module written against the canonical ABI, made a
//! component by `wit-component` from [`GUEST_WIT`], so no wasm toolchain is needed.

use std::collections::BTreeSet;
use std::future::Future;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

use nvs_ext::call::{Budget, Failure, Host, Level, Request};
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
}

world guest {
    import wasi:cli/environment@0.2.12;
    import wasi:cli/stdin@0.2.12;
    import wasi:cli/stdout@0.2.12;
    import wasi:cli/stderr@0.2.12;
    import wasi:filesystem/preopens@0.2.12;
    export api;
}
";

/// The context guest's core module. `say` writes `hello\nworld` to stdout and `oops\n` to stderr.
/// `stdin` reads up to 16 bytes and returns how many it read, `-1` at the end of the stream and
/// `-2` for a failed read. The other exports return the length of the list, or `1` for a working
/// directory and `0` for none. Every import returning more than one value writes it at 16.
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
    (i32.load (i32.const 20))))"#;

const GUEST_METHODS: &str = r#"[
  {"name": "say", "params": [], "returns": "void"},
  {"name": "stdin", "params": [], "returns": "int"},
  {"name": "environment", "params": [], "returns": "int"},
  {"name": "arguments", "params": [], "returns": "int"},
  {"name": "cwd", "params": [], "returns": "int"},
  {"name": "directories", "params": [], "returns": "int"}
]"#;

/// The context guest, componentized.
fn guest_bytes() -> Vec<u8> {
    let mut resolve = Resolve::default();
    resolve
        .push_dir(nvs_repo::path("wit/nvs-ext"))
        .expect("the world's WIT parses");
    let package = resolve
        .push_str("guest.wit", GUEST_WIT)
        .expect("the guest's world parses");
    let world = resolve
        .select_world(&[package], Some("guest"))
        .expect("the guest's world exists");
    let mut module = wat::parse_str(GUEST_CORE).expect("the guest's core module compiles");
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
    let bytes = guest_bytes();
    let manifest = format!(
        r#"{{"manifest": 1, "world": "1.0.0", "class": "Shop\\Probe", "interface": "shop:probe/api", "methods": {GUEST_METHODS}}}"#
    );
    Extension {
        path: PathBuf::from("probe.nvsx"),
        sha256: pin_of(&bytes),
        memory: None,
        component: Component::new(host.engine(), &bytes).expect("the guest compiles"),
        manifest: Manifest::parse(manifest.as_bytes()).expect("the manifest reads"),
        source: Source {
            source: FORMAT,
            files: Vec::new(),
        },
    }
}

/// One record a guest wrote to its request's log: the level, the channel and the message.
type Record = (Level, String, String);

/// A request budget with no limits, which keeps every record written to its log.
#[derive(Debug, Default)]
struct Recorder {
    records: Mutex<Vec<Record>>,
}

impl Recorder {
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

/// A host linking nothing but the world, a request on it keeping its log in a [`Recorder`], and
/// the context guest. `wit-component` gives the guest a shim and a fixup core instance beside
/// its own, so the host has a slot for each.
fn context() -> (Request, Extension, Arc<Recorder>) {
    let host = Host::new(3, |_| Ok(())).expect("the host starts");
    let extension = guest(&host);
    let recorder = Arc::new(Recorder::default());
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
