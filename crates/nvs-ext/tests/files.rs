//! `rule:security/extension-grants-are-an-intersection`'s files half, through a running guest: a
//! guest opens only the roots its entry, its manifest and its caller all grant, writes only under
//! a `write` root, and neither `..` nor a planted symlink takes it out of a root
//! (`rule:security/path-scope-canonicalise-then-prefix`).
//!
//! The guest is a core module written against the canonical ABI, made a component by
//! `wit-component`, so no wasm toolchain is needed. Each case builds its folders under
//! `.agent-tmp/` and removes them when it ends. A symlink case creates the real link and, where the
//! host refuses to make one, prints the reason and returns.

use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_config::Setting;
use nvs_config::capability::Cap;
use nvs_config::extension::Granted;
use nvs_config::resolve::Disk;
use nvs_config::tree::{CapFs, Capabilities};
use nvs_ext::call::{Budget, Host, Level, Request};
use nvs_ext::grants::Caller;
use nvs_ext::load::{Extension, pin as pin_of};
use nvs_ext::manifest::Manifest;
use nvs_ext::source::{FORMAT, Source};
use wasmtime::component::{Component, Val};
use wit_component::{ComponentEncoder, StringEncoding, embed_component_metadata};
use wit_parser::Resolve;

const WIT: &str = "package shop:files;

interface api {
    read: func(path: string) -> s64;
    write: func(path: string) -> s64;
    directories: func() -> u32;
}

world guest {
    import wasi:filesystem/preopens@0.2.12;
    import wasi:filesystem/types@0.2.12;
    export api;
}
";

/// The guest's core module. Each export works under the first preopen. `read` opens `path` and
/// returns how many bytes a read of up to 256 gives. `write` creates or truncates `path` and
/// writes `hello` to it, returning the count written. Both return `-1000` when there is no
/// preopen, and `-1 - code` for a failure, `code` being the `error-code` case's index.
/// `directories` returns how many preopens there are.
const CORE: &str = r#"(module
  (import "wasi:filesystem/preopens@0.2.12" "get-directories" (func $dirs (param i32)))
  (import "wasi:filesystem/types@0.2.12" "[method]descriptor.open-at"
    (func $open (param i32 i32 i32 i32 i32 i32 i32)))
  (import "wasi:filesystem/types@0.2.12" "[method]descriptor.read"
    (func $read (param i32 i64 i64 i32)))
  (import "wasi:filesystem/types@0.2.12" "[method]descriptor.write"
    (func $write (param i32 i32 i32 i64 i32)))
  (import "wasi:filesystem/types@0.2.12" "[resource-drop]descriptor" (func $drop (param i32)))
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 4096))
  (data (i32.const 64) "hello")
  (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
    (local.set $at
      (i32.and
        (i32.add (global.get $heap) (i32.sub (local.get 2) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get 2))))
    (global.set $heap (i32.add (local.get $at) (local.get 3)))
    (local.get $at))
  (func $root (result i32)
    (call $dirs (i32.const 16))
    (if (result i32) (i32.load (i32.const 20))
      (then (i32.load (i32.load (i32.const 16))))
      (else (i32.const -1))))
  (func $open_path (param $dir i32) (param $ptr i32) (param $len i32) (param $how i32)
    (param $flags i32) (result i64)
    (call $open (local.get $dir) (i32.const 1) (local.get $ptr) (local.get $len)
      (local.get $how) (local.get $flags) (i32.const 32))
    (if (result i64) (i32.load8_u (i32.const 32))
      (then (i64.sub (i64.const -1) (i64.extend_i32_u (i32.load8_u (i32.const 36)))))
      (else (i64.extend_i32_u (i32.load (i32.const 36))))))
  (func (export "shop:files/api#read") (param $ptr i32) (param $len i32) (result i64)
    (local $dir i32) (local $fd i64)
    (local.set $dir (call $root))
    (if (i32.eq (local.get $dir) (i32.const -1)) (then (return (i64.const -1000))))
    (local.set $fd (call $open_path (local.get $dir) (local.get $ptr) (local.get $len)
      (i32.const 0) (i32.const 1)))
    (call $drop (local.get $dir))
    (if (i64.lt_s (local.get $fd) (i64.const 0)) (then (return (local.get $fd))))
    (call $read (i32.wrap_i64 (local.get $fd)) (i64.const 256) (i64.const 0) (i32.const 48))
    (call $drop (i32.wrap_i64 (local.get $fd)))
    (if (result i64) (i32.load8_u (i32.const 48))
      (then (i64.sub (i64.const -1) (i64.extend_i32_u (i32.load8_u (i32.const 52)))))
      (else (i64.extend_i32_u (i32.load (i32.const 56))))))
  (func (export "shop:files/api#write") (param $ptr i32) (param $len i32) (result i64)
    (local $dir i32) (local $fd i64)
    (local.set $dir (call $root))
    (if (i32.eq (local.get $dir) (i32.const -1)) (then (return (i64.const -1000))))
    (local.set $fd (call $open_path (local.get $dir) (local.get $ptr) (local.get $len)
      (i32.const 9) (i32.const 2)))
    (call $drop (local.get $dir))
    (if (i64.lt_s (local.get $fd) (i64.const 0)) (then (return (local.get $fd))))
    (call $write (i32.wrap_i64 (local.get $fd)) (i32.const 64) (i32.const 5) (i64.const 0)
      (i32.const 48))
    (call $drop (i32.wrap_i64 (local.get $fd)))
    (if (result i64) (i32.load8_u (i32.const 48))
      (then (i64.sub (i64.const -1) (i64.extend_i32_u (i32.load8_u (i32.const 56)))))
      (else (i64.load (i32.const 56)))))
  (func (export "shop:files/api#directories") (result i32)
    (call $dirs (i32.const 16))
    (i32.load (i32.const 20))))"#;

/// What `read` and `write` return with no preopen.
const NO_ROOT: i64 = -1000;
/// What they return for `not-permitted`, the 32nd case of `error-code`.
const NOT_PERMITTED: i64 = -32;
/// What they return for `read-only`, the 34th case.
const READ_ONLY: i64 = -34;

/// The guest, componentized against its world.
fn guest_bytes() -> Vec<u8> {
    let mut resolve = Resolve::default();
    resolve
        .push_dir(nvs_repo::path("wit/nvs-ext"))
        .expect("the world's WIT parses");
    let package = resolve
        .push_str("guest.wit", WIT)
        .expect("the guest's world parses");
    let world = resolve
        .select_world(&[package], Some("guest"))
        .expect("the guest's world exists");
    let mut module = wat::parse_str(CORE).expect("the guest's core module compiles");
    embed_component_metadata(&mut module, &resolve, world, StringEncoding::UTF8)
        .expect("the guest's metadata embeds");
    ComponentEncoder::default()
        .module(&module)
        .expect("the guest's core module reads")
        .validate(true)
        .encode()
        .expect("the guest componentizes")
}

/// The guest as an extension of `host`, its entry granting `entry` and its manifest requesting
/// `requests`, written as the manifest's JSON object.
fn extension(host: &Host, entry: Granted, requests: &str) -> Extension {
    let bytes = guest_bytes();
    let manifest = format!(
        r#"{{"manifest": 1, "world": "1.0.0", "class": "Shop\\Files", "interface": "shop:files/api",
        "requests": {requests},
        "methods": [
          {{"name": "read", "params": [{{"name": "path", "type": "string"}}], "returns": "int"}},
          {{"name": "write", "params": [{{"name": "path", "type": "string"}}], "returns": "int"}},
          {{"name": "directories", "params": [], "returns": "int"}}
        ]}}"#
    );
    Extension {
        path: PathBuf::from("files.nvsx"),
        sha256: pin_of(&bytes),
        memory: None,
        grants: entry,
        component: Component::new(host.engine(), &bytes).expect("the guest compiles"),
        manifest: Manifest::parse(manifest.as_bytes()).expect("the manifest reads"),
        source: Source {
            source: FORMAT,
            files: Vec::new(),
        },
    }
}

/// A request budget with no limits whose caller holds `capabilities`, narrowed by `narrowed`.
#[derive(Debug, Default)]
struct Held {
    capabilities: Option<Capabilities>,
    narrowed: Option<Vec<Cap>>,
}

impl Budget for Held {
    fn cpu_spent(&self) -> bool {
        false
    }

    fn charge(&self, _bytes: i64) -> bool {
        true
    }

    fn log(&self, _level: Level, _channel: &str, _message: &str) {}

    fn wall_clock(&self) -> Duration {
        Duration::ZERO
    }

    fn monotonic_clock(&self) -> u64 {
        0
    }

    fn random(&self, out: &mut [u8]) {
        out.fill(0);
    }

    fn setting(&self, _block: &str, _key: &str) -> Option<serde_json::Value> {
        None
    }

    fn caller(&self) -> Caller<'_> {
        Caller {
            capabilities: self.capabilities.as_ref(),
            narrowed: self.narrowed.as_deref(),
        }
    }
}

/// A caller holding `fs.read` under `read` and `fs.write` under `write`, each root canonical as a
/// snapshot's is.
fn holding(read: &[&Path], write: &[&Path]) -> Held {
    let list = |roots: &[&Path]| {
        Some(Setting::List(
            roots
                .iter()
                .map(|root| {
                    nvs_config::capability::resolved(root, &Disk)
                        .expect("the root resolves")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect(),
        ))
    };
    Held {
        capabilities: Some(Capabilities {
            fs: Some(CapFs {
                read: list(read),
                write: list(write),
            }),
            ..Capabilities::default()
        }),
        narrowed: None,
    }
}

/// A folder of its own under `.agent-tmp/`, holding `geo/cities.txt`, `other/secret.txt` and an
/// empty `out/`, removed when the case ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(case: &str) -> Self {
        let root =
            nvs_repo::path(".agent-tmp").join(format!("ext-files-{case}-{}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        for dir in ["geo/sub", "other", "out"] {
            fs::create_dir_all(root.join(dir)).expect("the scratch folder is made");
        }
        fs::write(root.join("geo/cities.txt"), "Vienna\n").expect("the file is written");
        fs::write(root.join("other/secret.txt"), "nope").expect("the file is written");
        Self(root)
    }

    fn at(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

/// Polls `future` on this thread until it is ready.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
    }
}

/// A host with a slot for each of a case's requests, and the guest on it as `entry` and
/// `requests` declare it.
fn host_with(entry: Granted, requests: &str) -> (Host, Extension) {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = extension(&host, entry, requests);
    (host, extension)
}

fn request(host: &Host, held: Held) -> Request {
    host.request(Arc::new(held) as Arc<dyn Budget>)
}

/// What `method` returns for `path`.
fn ask(request: &Request, extension: &Extension, method: &str, path: &str) -> i64 {
    let out = block_on(request.call(extension, method, &[Val::String(path.to_owned())]))
        .expect("the call returns");
    match out.as_slice() {
        [Val::S64(value)] => *value,
        other => panic!("`{method}` returned {other:?}"),
    }
}

fn directories(request: &Request, extension: &Extension) -> u32 {
    let out = block_on(request.call(extension, "directories", &[])).expect("the call returns");
    match out.as_slice() {
        [Val::U32(count)] => *count,
        other => panic!("`directories` returned {other:?}"),
    }
}

fn reads(scratch: &Scratch, read: &[&str]) -> Granted {
    Granted {
        read: read.iter().map(|root| scratch.at(root)).collect(),
        ..Granted::default()
    }
}

const READS: &str = r#"{"read": ["geo/"]}"#;
const WRITES: &str = r#"{"write": ["out/"]}"#;

// covers: tools:config/extension-grants-what-a-component-may-reach
#[test]
fn a_guest_reads_a_file_under_a_root_all_three_grant() {
    let scratch = Scratch::new("reads");
    let (host, extension) = host_with(reads(&scratch, &["geo"]), READS);
    let request = request(&host, holding(&[&scratch.at("geo")], &[]));
    assert_eq!(directories(&request, &extension), 1);
    assert_eq!(ask(&request, &extension, "read", "cities.txt"), 7);
    assert_eq!(ask(&request, &extension, "read", "sub/../cities.txt"), 7);
}

#[test]
fn a_guest_read_outside_the_entrys_roots_fails() {
    let scratch = Scratch::new("entry");
    let (host, extension) = host_with(reads(&scratch, &["geo"]), READS);
    // The caller holds the whole folder, and the entry only `geo`.
    let request = request(&host, holding(&[&scratch.0], &[]));
    assert_eq!(directories(&request, &extension), 1);
    assert_eq!(
        ask(&request, &extension, "read", "../other/secret.txt"),
        NOT_PERMITTED
    );
}

#[test]
fn a_guest_read_the_manifest_did_not_request_fails() {
    let scratch = Scratch::new("manifest");
    let (host, extension) = host_with(reads(&scratch, &["geo"]), r#"{"connect": []}"#);
    let request = request(&host, holding(&[&scratch.0], &[]));
    assert_eq!(directories(&request, &extension), 0);
    assert_eq!(ask(&request, &extension, "read", "cities.txt"), NO_ROOT);
}

#[test]
fn a_guest_read_outside_the_callers_roots_fails() {
    let scratch = Scratch::new("caller");
    // The entry grants the whole folder, and the caller holds only `geo`, which is what the guest
    // opens.
    let (host, extension) = host_with(reads(&scratch, &["."]), READS);
    let request = request(&host, holding(&[&scratch.at("geo")], &[]));
    assert_eq!(directories(&request, &extension), 1);
    assert_eq!(ask(&request, &extension, "read", "cities.txt"), 7);
    assert_eq!(
        ask(&request, &extension, "read", "../other/secret.txt"),
        NOT_PERMITTED
    );
    let none = self::request(&host, Held::default());
    assert_eq!(directories(&none, &extension), 0);
}

#[test]
fn a_guest_write_to_a_read_only_root_fails() {
    let scratch = Scratch::new("read-only");
    let (host, extension) = host_with(reads(&scratch, &["out"]), r#"{"read": ["out/"]}"#);
    let out = scratch.at("out");
    let request = request(&host, holding(&[&out], &[&out]));
    assert_eq!(ask(&request, &extension, "write", "new.txt"), READ_ONLY);
    assert!(!scratch.at("out/new.txt").exists());
}

#[test]
fn a_guest_writes_a_file_under_a_write_root() {
    let scratch = Scratch::new("writes");
    let entry = Granted {
        write: vec![scratch.at("out")],
        ..Granted::default()
    };
    let (host, extension) = host_with(entry, WRITES);
    let out = scratch.at("out");
    let request = request(&host, holding(&[&out], &[&out]));
    assert_eq!(ask(&request, &extension, "write", "new.txt"), 5);
    assert_eq!(
        fs::read_to_string(scratch.at("out/new.txt")).expect("the file was written"),
        "hello"
    );
    // A caller holding `fs.write` without `fs.read` gets no root to open.
    let blind = self::request(&host, holding(&[], &[&out]));
    assert_eq!(ask(&blind, &extension, "write", "other.txt"), NO_ROOT);
}

#[test]
fn a_guest_walking_up_with_dot_dot_out_of_a_root_fails() {
    let scratch = Scratch::new("dot-dot");
    let (host, extension) = host_with(reads(&scratch, &["geo"]), READS);
    let request = request(&host, holding(&[&scratch.0], &[]));
    for path in [
        "..",
        "../other/secret.txt",
        "sub/../../other/secret.txt",
        "../geo/cities.txt",
        "/etc/passwd",
    ] {
        assert_eq!(
            ask(&request, &extension, "read", path),
            NOT_PERMITTED,
            "`{path}` left the root"
        );
    }
    let entry = Granted {
        write: vec![scratch.at("out")],
        ..Granted::default()
    };
    let (host, extension) = host_with(entry, WRITES);
    let request = self::request(&host, holding(&[&scratch.0], &[&scratch.0]));
    assert_eq!(
        ask(&request, &extension, "write", "../escaped.txt"),
        NOT_PERMITTED
    );
    assert!(!scratch.at("escaped.txt").exists());
}

#[test]
#[allow(
    clippy::print_stdout,
    reason = "a host that refuses to make a link says why the case did not run"
)]
fn a_symlink_planted_under_a_root_does_not_carry_the_grant_to_its_target() {
    let scratch = Scratch::new("symlink");
    if let Err(reason) = plant(&scratch) {
        println!("skipped: this host does not create a symlink ({reason})");
        return;
    }
    let (host, extension) = host_with(reads(&scratch, &["geo"]), READS);
    let request = request(&host, holding(&[&scratch.0], &[]));
    assert_eq!(ask(&request, &extension, "read", "cities.txt"), 7);
    for path in ["away/secret.txt", "secret.txt", "nowhere.txt"] {
        assert_eq!(
            ask(&request, &extension, "read", path),
            NOT_PERMITTED,
            "`{path}` followed a link out of the root"
        );
    }
}

/// Plants three links under `geo`: `away` to the folder `other`, `secret.txt` to the file in it,
/// and `nowhere.txt` to a file in `other` that does not exist yet.
fn plant(scratch: &Scratch) -> std::io::Result<()> {
    let links = [
        ("other", "geo/away", true),
        ("other/secret.txt", "geo/secret.txt", false),
        ("other/later.txt", "geo/nowhere.txt", false),
    ];
    for (target, link, dir) in links {
        link_to(&scratch.at(target), &scratch.at(link), dir)?;
    }
    Ok(())
}

#[cfg(unix)]
fn link_to(target: &Path, link: &Path, _dir: bool) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn link_to(target: &Path, link: &Path, dir: bool) -> std::io::Result<()> {
    if dir {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

#[test]
fn the_preopens_come_from_the_creating_contexts_snapshot() {
    let scratch = Scratch::new("snapshot");
    let (host, extension) = host_with(reads(&scratch, &["geo"]), READS);
    let geo = scratch.at("geo");
    // Three requests on one host, each with its own caller: one holding `geo`, one whose snapshot
    // grants nothing, and one an isolate's `grants: []` narrowed to nothing.
    let held = request(&host, holding(&[&geo], &[]));
    let bare = request(&host, Held::default());
    let narrowed = request(
        &host,
        Held {
            narrowed: Some(Vec::new()),
            ..holding(&[&geo], &[])
        },
    );
    assert_eq!(ask(&held, &extension, "read", "cities.txt"), 7);
    assert_eq!(ask(&bare, &extension, "read", "cities.txt"), NO_ROOT);
    assert_eq!(ask(&narrowed, &extension, "read", "cities.txt"), NO_ROOT);
}
