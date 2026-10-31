//! `rule:packaging/the-first-party-components-are-built-in` as `nvs-ext` sees it: the binary
//! carries the image, intl, PDF and spreadsheet components, and the image component loads with no `[[extension]]` entry and no pin, compiles on its
//! first use and not before, and an entry declaring a `Novis\` class is still refused. Each
//! component's module doc states its embedded size, held within 5% of the bytes here.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::{
    IMAGE, IMAGE_CLASS, IMAGE_SHA256, INTL, INTL_CLASS, PDF, PDF_CLASS, PDF_SHA256, SPREADSHEET,
    SPREADSHEET_CLASS, SPREADSHEET_SHA256,
};
use nvs_ext::call::{Error, Failure, Host, Meter};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Builtin, CacheKey, Entry, Loader, ModuleCache};
use nvs_ext::pack::append_section;
use nvs_ext::section::MANIFEST;
use wasmtime::Engine;

/// A cache in memory, counting what the loader reads and stores.
#[derive(Default)]
struct Memory {
    entries: Mutex<HashMap<CacheKey, Vec<u8>>>,
    gets: Mutex<usize>,
    puts: Mutex<usize>,
}

impl ModuleCache for Memory {
    fn get(&self, key: &CacheKey) -> Option<Vec<u8>> {
        *self.gets.lock().unwrap() += 1;
        self.entries.lock().unwrap().get(key).cloned()
    }

    fn put(&self, key: &CacheKey, bytes: &[u8]) {
        *self.puts.lock().unwrap() += 1;
        self.entries
            .lock()
            .unwrap()
            .insert(key.clone(), bytes.to_vec());
    }
}

impl Memory {
    fn touched(&self) -> (usize, usize) {
        (*self.gets.lock().unwrap(), *self.puts.lock().unwrap())
    }
}

/// The image component among `builtins`.
fn image(builtins: &[Builtin]) -> &Builtin {
    builtins
        .iter()
        .find(|builtin| builtin.manifest().class == "Novis\\Image\\Codec")
        .expect("the image component is built in")
}

/// `future` polled on this thread until it finishes. A guest call yields at every epoch tick, and
/// polling again resumes it.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}

/// A PNG of `width` by `height` RGBA pixels, every one transparent black.
fn png(width: u32, height: u32) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    let row = 1 + 4 * width as usize;
    let raw = vec![0u8; row * height as usize];
    // One stored deflate block, which is enough for a few bytes.
    let mut zlib = vec![0x78, 0x01, 0x01];
    let len = u16::try_from(raw.len()).unwrap();
    zlib.extend_from_slice(&len.to_le_bytes());
    zlib.extend_from_slice(&(!len).to_le_bytes());
    zlib.extend_from_slice(&raw);
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

/// The value of `key` in the shape `value`.
fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    let Value::Array(entries) = value else {
        panic!("a shape was expected, not {value:?}");
    };
    entries
        .iter()
        .find(|(k, _)| *k == Key::String(key.to_owned()))
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("the shape has no `{key}`: {value:?}"))
}

/// The size a component's module doc states, from its sentence "the component is N.N MiB".
fn recorded_mib(path: &Path) -> f64 {
    let doc = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} is not readable: {e}", path.display()));
    let doc = doc
        .lines()
        .take_while(|line| line.starts_with("//!"))
        .map(|line| line.trim_start_matches("//!").trim())
        .collect::<Vec<_>>()
        .join(" ");
    let (_, after) = doc
        .split_once("the component is ")
        .unwrap_or_else(|| panic!("{} states no embedded size", path.display()));
    let (figure, _) = after
        .split_once(" MiB")
        .unwrap_or_else(|| panic!("{}'s size is not in MiB", path.display()));
    figure
        .parse()
        .unwrap_or_else(|e| panic!("{}'s size `{figure}` is not a number: {e}", path.display()))
}

#[test]
fn the_recorded_growth_matches_the_embedded_components() {
    let docs = [
        (nvs_repo::path("extensions/intl/src/lib.rs"), INTL),
        (nvs_repo::path("extensions/image/src/lib.rs"), IMAGE),
        (nvs_repo::path("extensions/pdf/src/lib.rs"), PDF),
        (
            nvs_repo::path("extensions/spreadsheet/src/lib.rs"),
            SPREADSHEET,
        ),
    ];
    for (path, bytes) in docs {
        let recorded = recorded_mib(&path);
        let embedded = bytes.len() as f64 / (1024.0 * 1024.0);
        let drift = (recorded - embedded).abs() / embedded;
        assert!(
            drift <= 0.05,
            "{} says the component is {recorded} MiB, and the binary carries {embedded:.2} MiB: \
             write the new size there",
            path.display(),
        );
    }
}

#[test]
fn the_image_component_is_built_in_and_answers_info_with_no_configuration() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let extension = image(&builtins)
        .extension()
        .expect("the image component compiles and checks");
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let info = block_on(request.call_values(extension, "info", vec![Value::Bytes(png(3, 2))]))
        .expect("`info` answers")
        .expect("`info` returns a value");
    assert_eq!(field(&info, "format"), &Value::Case("Png".to_owned()));
    assert_eq!(field(&info, "width"), &Value::Uint(3));
    assert_eq!(field(&info, "height"), &Value::Uint(2));
    assert_eq!(field(&info, "hasAlpha"), &Value::Bool(true));
    block_on(request.end()).expect("the request ends");
}

/// A built-in world is called typed (`nvs_ext::typed`), so its `bytes` are charged to the request
/// once, at their own size. The dynamic path holds one `Val` per byte, which this budget cannot
/// afford. Each world is called with zeros, which none of them parses.
#[test]
fn a_built_in_call_charges_its_bytes_once_and_not_one_val_per_byte() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    for (class, method) in [(IMAGE_CLASS, "info"), (PDF_CLASS, "read")] {
        let extension = builtins
            .iter()
            .find(|builtin| builtin.manifest().class == class)
            .unwrap_or_else(|| panic!("{class} is built in"))
            .extension()
            .unwrap_or_else(|err| panic!("{class} compiles and checks: {err:?}"));
        let data = vec![0_u8; 4 << 20];
        let unbounded = Arc::new(Meter::new(Duration::from_secs(30), None));
        let request = host.request(unbounded.clone());
        // The PDF `read` takes a `decompression-bound` after its bytes, and neither half set
        // is the shipped default.
        let args = |data: Vec<u8>| {
            let mut args = vec![Value::Bytes(data)];
            if class == PDF_CLASS {
                args.push(Value::Array(vec![
                    (Key::String("maxBytes".to_owned()), Value::Null),
                    (Key::String("maxRatio".to_owned()), Value::Null),
                ]));
            }
            args
        };
        let out = block_on(request.call_values(extension, method, args(data.clone())));
        assert!(
            matches!(out, Err(Failure::Error(Error::Parse(_)))),
            "{class}::{method} does not parse zeros: {out:?}"
        );
        let instance = unbounded.charged();
        block_on(request.end()).expect("the request ends");

        let budget = instance + data.len() as u64 + (1 << 20);
        let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), Some(budget))));
        let out = block_on(request.call_values(extension, method, args(data)));
        assert!(
            matches!(out, Err(Failure::Error(Error::Parse(_)))),
            "{class}::{method} fits in {budget} bytes: {out:?}"
        );
        block_on(request.end()).expect("the request ends");
    }
}

/// The spreadsheet world is called typed too, with its resources and its `mixed` sources crossing
/// as handles. No export takes `bytes` alone, so an image part carries them: a writer is opened,
/// and its `writerPart` gets an image of zeros, which is no image format.
#[test]
fn a_spreadsheet_call_charges_its_bytes_once_with_a_resource_beside_them() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let extension = builtins
        .iter()
        .find(|builtin| builtin.manifest().class == SPREADSHEET_CLASS)
        .expect("the spreadsheet component is built in")
        .extension()
        .expect("the spreadsheet component compiles and checks");
    let shape = |fields: Vec<(&str, Value)>| {
        Value::Array(
            fields
                .into_iter()
                .map(|(name, value)| (Key::String(name.to_owned()), value))
                .collect(),
        )
    };
    let nulls = |names: &[&str]| shape(names.iter().map(|name| (*name, Value::Null)).collect());
    let options = shape(vec![
        (
            "properties",
            nulls(&[
                "title", "subject", "author", "company", "keywords", "created",
            ]),
        ),
        ("target", Value::String("memory".to_owned())),
        ("ods", Value::Bool(false)),
    ]);
    let data = vec![0_u8; 4 << 20];
    let part = |data: Vec<u8>| {
        let Value::Array(mut fields) = nulls(&["width", "height", "offsetX", "offsetY"]) else {
            unreachable!("a shape is an array");
        };
        fields.push((Key::String("at".to_owned()), Value::String("A1".to_owned())));
        fields.push((Key::String("image".to_owned()), Value::Bytes(data)));
        Value::Array(fields)
    };
    let write = |request: &nvs_ext::call::Request, data: Vec<u8>| {
        let writer = block_on(request.call_values(extension, "writerOpen", vec![options.clone()]))
            .expect("a writer opens")
            .expect("`writerOpen` returns a writer");
        assert!(
            matches!(writer, Value::Resource { ref name, .. } if name == "Writer"),
            "the request keeps the writer: {writer:?}"
        );
        block_on(request.call_values(extension, "writerPart", vec![writer, part(data)]))
    };

    let unbounded = Arc::new(Meter::new(Duration::from_secs(30), None));
    let request = host.request(unbounded.clone());
    let out = write(&request, data.clone());
    assert!(
        matches!(out, Err(Failure::Error(Error::Invalid(_)))),
        "zeros are no image: {out:?}"
    );
    let instance = unbounded.charged();
    block_on(request.end()).expect("the request ends");

    let budget = instance + data.len() as u64 + (1 << 20);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), Some(budget))));
    let out = write(&request, data);
    assert!(
        matches!(out, Err(Failure::Error(Error::Invalid(_)))),
        "the image part fits in {budget} bytes: {out:?}"
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn a_built_in_component_needs_no_extension_entry_and_no_pin() {
    let builtins = Loader::new(&Engine::default())
        .builtins()
        .expect("the built-in components load");
    let image = image(&builtins);
    let digest: String = IMAGE_SHA256.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(image.sha256(), digest);
    assert_eq!(image.sha256(), nvs_ext::load::pin(IMAGE));
    let extension = image.extension().expect("the image component loads");
    assert_eq!(extension.sha256, digest);
    assert_eq!(extension.memory, None);
    assert_eq!(extension.grants, nvs_config::extension::Granted::default());
}

/// The binary carries exactly four components, and the PDF and spreadsheet ones are pinned by the
/// binary as the image one is: no `[[extension]]` entry, no memory setting and no grant.
#[test]
fn the_spreadsheet_component_is_one_of_four_built_in() {
    let builtins = Loader::new(&Engine::default())
        .builtins()
        .expect("the built-in components load");
    let mut classes: Vec<_> = builtins
        .iter()
        .map(|builtin| builtin.manifest().class.clone())
        .collect();
    classes.sort();
    assert_eq!(
        classes,
        [IMAGE_CLASS, INTL_CLASS, PDF_CLASS, SPREADSHEET_CLASS]
    );
    for (class, bytes, sha256) in [
        (PDF_CLASS, PDF, PDF_SHA256),
        (SPREADSHEET_CLASS, SPREADSHEET, SPREADSHEET_SHA256),
    ] {
        let builtin = builtins
            .iter()
            .find(|builtin| builtin.manifest().class == class)
            .unwrap_or_else(|| panic!("{class} is built in"));
        let digest: String = sha256.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(builtin.sha256(), digest);
        assert_eq!(builtin.sha256(), nvs_ext::load::pin(bytes));
        let extension = builtin
            .extension()
            .unwrap_or_else(|err| panic!("{class} loads: {err:?}"));
        assert_eq!(extension.sha256, digest);
        assert_eq!(extension.memory, None);
        assert_eq!(extension.grants, nvs_config::extension::Granted::default());
    }
}

/// Each built-in component is named for its crate's directory under `extensions/`, whose
/// `manifest.json` declares its class, which is how observed selection tells which component a
/// changed path belongs to. Reading the table records only that it was read, and a use of one
/// component records that component's name.
#[test]
fn a_built_in_component_is_named_for_its_directory_and_records_its_use() {
    let (builtins, read) = nvs_footprint::capture(|| {
        Loader::new(&Engine::default())
            .builtins()
            .expect("the built-in components load")
    });
    assert_eq!(read, ["builtins\tread"]);
    for builtin in &builtins {
        let path = nvs_repo::path(&format!("extensions/{}/manifest.json", builtin.name()));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{} reads: {err}", path.display()));
        let manifest: serde_json::Value = serde_json::from_str(&text).expect("a manifest is JSON");
        assert_eq!(manifest["class"], builtin.manifest().class.as_str());
        assert_eq!(builtin.manifest().builtin.as_deref(), Some(builtin.name()));
    }
    let pdf = builtins
        .iter()
        .find(|builtin| builtin.name() == "pdf")
        .expect("pdf is built in");
    let ((), used) = nvs_footprint::capture(|| {
        pdf.extension().expect("the pdf component loads");
    });
    assert_eq!(used, ["component\tpdf"]);
}

#[test]
fn a_built_in_component_is_compiled_on_first_use_and_not_before() {
    let cache = Arc::new(Memory::default());
    let builtins = Loader::new(&Engine::default())
        .with_cache(cache.clone())
        .builtins()
        .expect("the built-in components load");
    let image = image(&builtins);
    assert!(!image.compiled());
    assert_eq!(cache.touched(), (0, 0), "loading compiled something");
    image.extension().expect("the image component compiles");
    assert!(image.compiled());
    assert_eq!(cache.touched(), (1, 1));
    image
        .extension()
        .expect("the image component is still loaded");
    assert_eq!(cache.touched(), (1, 1), "a second use compiled it again");
    let key = CacheKey {
        pin: image.sha256().to_owned(),
        environment: Loader::new(&Engine::default()).environment().to_owned(),
    };
    assert!(cache.entries.lock().unwrap().contains_key(&key));
}

#[test]
fn an_extension_declaring_a_novis_class_is_refused_as_reserved_with_the_built_in_set_loaded() {
    let loader = Loader::new(&Engine::default());
    let builtins = loader.builtins().expect("the built-in components load");
    image(&builtins)
        .extension()
        .expect("the image component loads");
    let component = wat::parse_str(
        r#"(component
  (core module $m (func (export "f")))
  (core instance $i (instantiate $m))
  (func $f (canon lift (core func $i "f")))
  (instance $api (export "f" (func $f)))
  (export "shop:image/codec" (instance $api)))"#,
    )
    .expect("the test component compiles");
    let manifest = r#"{"manifest": 1, "world": "1.0.0", "class": "Novis\\Image\\Codec", "interface": "shop:image/codec", "methods": [{"name": "f", "params": [], "returns": "void"}]}"#;
    let bytes = append_section(component, MANIFEST, manifest.as_bytes());
    let entry = Entry {
        path: PathBuf::from("codec.nvsx"),
        sha256: nvs_ext::load::pin(&bytes),
        memory: None,
        grants: nvs_config::extension::Granted::default(),
    };
    let refused = loader
        .load_bytes(&entry, &bytes)
        .expect_err("an entry declaring a `Novis\\` class loaded");
    assert_eq!(refused.entry, "codec.nvsx");
    assert!(
        refused
            .reason
            .contains("is under `Novis\\`, which only Novis declares in"),
        "{refused}"
    );
}
