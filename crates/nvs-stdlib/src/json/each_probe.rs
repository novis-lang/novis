//! The probe that chose `Core\Json::decodeEach`'s reader: the goal
//! `json-streaming`'s decision 6, options (a) and (c), measured side by side
//! against [`read`] of the whole document.
//!
//! Each option reads the array at `/items` of one document, handed over in
//! chunks of [`CHUNK`] bytes, and builds every element with the decoder the
//! whole-document read uses. The figures and the verdict are in the parent
//! module's doc, § *A document read one element at a time*. Run it on an
//! optimized build:
//! `cargo test --profile proof --lib json::each_probe -- --ignored --nocapture`.
//!
//! Option (b), a pull-reader crate, has no test here. Its value parser and its
//! messages are its own, so it fails the bar's third leg before any figure.

use std::io;

use corosensei::stack::DefaultStack;
use corosensei::{Coroutine, CoroutineResult, Yielder};
use serde::de::{DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

use super::{DEFAULT_MAX_DEPTH_U32, Decode, Value, read};

/// The size of each chunk a source hands over, `Core\Request::bodyStream`'s.
const CHUNK: usize = 64 << 10;

/// The document's text before the array.
const PREFIX: &[u8] = br#"{"meta":{"note":"a string the walk skips","count":3},"items":["#;

/// The document's text after the array.
const SUFFIX: &[u8] = br#"],"end":true}"#;

/// One element of the array: the size ladders' `wide` unit.
fn element(at: usize) -> String {
    format!("{{\"id\":{at},\"name\":\"item-{at}\",\"paid\":true}}")
}

/// The document of at least `size` bytes, in chunks of [`CHUNK`] bytes. Only
/// one chunk exists at a time, so the source itself stays flat.
fn document_chunks(size: usize) -> impl Iterator<Item = Vec<u8>> {
    let mut written = 0usize;
    let mut at = 0usize;
    let mut stage = 0u8;
    let mut carry: Vec<u8> = Vec::new();
    std::iter::from_fn(move || {
        let mut chunk = std::mem::take(&mut carry);
        while chunk.len() < CHUNK {
            match stage {
                0 => {
                    chunk.extend_from_slice(PREFIX);
                    stage = 1;
                }
                1 if written + chunk.len() < size => {
                    if at > 0 {
                        chunk.push(b',');
                    }
                    chunk.extend_from_slice(element(at).as_bytes());
                    at += 1;
                }
                1 => stage = 2,
                2 => {
                    chunk.extend_from_slice(SUFFIX);
                    stage = 3;
                }
                _ => break,
            }
        }
        if chunk.is_empty() {
            return None;
        }
        if chunk.len() > CHUNK {
            carry = chunk.split_off(CHUNK);
        }
        written += chunk.len();
        Some(chunk)
    })
}

/// The whole document of at least `size` bytes, for the baseline.
fn document(size: usize) -> String {
    String::from_utf8(document_chunks(size).flatten().collect()).expect("the document is UTF-8")
}

/// Frees an element the walk gave.
fn release(value: Value) {
    #[expect(unsafe_code, reason = "the walk gave this frame the only reference")]
    unsafe {
        value.release();
    }
}

// ----------------------------------------------------------------------------
// Option (a): serde_json, driven inside a coroutine
// ----------------------------------------------------------------------------

/// What the coroutine suspends with: `None` when it needs the next chunk,
/// and an element when it has built one.
type Up = Option<Value>;

/// What the coroutine is resumed with: the next chunk, or `None` at the end
/// of the source and after an element.
type Down = Option<Vec<u8>>;

/// The bytes serde_json reads, fed one chunk at a time by suspending.
struct Feed<'y> {
    chunk: Vec<u8>,
    at: usize,
    yielder: &'y Yielder<Down, Up>,
}

impl io::Read for Feed<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.at == self.chunk.len() {
            match self.yielder.suspend(None) {
                Some(next) => {
                    self.chunk = next;
                    self.at = 0;
                }
                None => return Ok(0),
            }
        }
        let n = buf.len().min(self.chunk.len() - self.at);
        buf[..n].copy_from_slice(&self.chunk[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

/// The document's top object: the value under `key` is the array to walk,
/// and every other value is checked and skipped.
struct AtKey<'y> {
    key: &'static str,
    yielder: &'y Yielder<Down, Up>,
}

impl<'de> DeserializeSeed<'de> for AtKey<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for AtKey<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an object")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == self.key {
                map.next_value_seed(Elements {
                    yielder: self.yielder,
                })?;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(())
    }
}

/// The array: each element is built by [`Decode`] and handed out by
/// suspending.
struct Elements<'y> {
    yielder: &'y Yielder<Down, Up>,
}

impl<'de> DeserializeSeed<'de> for Elements<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for Elements<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an array")
    }

    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<(), S::Error> {
        let seed = Decode {
            depth: 1,
            max: DEFAULT_MAX_DEPTH_U32,
            lists: None,
        };
        while let Some(value) = seq.next_element_seed(seed)? {
            self.yielder.suspend(Some(value));
        }
        Ok(())
    }
}

/// Option (a): walks `/items` of the chunks, calling `each` per element.
fn coroutine_walk(
    mut chunks: impl Iterator<Item = Vec<u8>>,
    mut each: impl FnMut(Value),
) -> Result<(), String> {
    let stack = DefaultStack::new(nvs_host::SPARE_STACK_SIZE).expect("a probe stack");
    let mut walk = Coroutine::with_stack(stack, |yielder: &Yielder<Down, Up>, first: Down| {
        let feed = Feed {
            chunk: first.unwrap_or_default(),
            at: 0,
            yielder,
        };
        let mut deserializer = serde_json::Deserializer::from_reader(feed);
        deserializer.disable_recursion_limit();
        AtKey {
            key: "items",
            yielder,
        }
        .deserialize(&mut deserializer)
        .and_then(|()| deserializer.end())
        .map_err(|why| why.to_string())
    });
    let mut input = chunks.next();
    loop {
        match walk.resume(input.take()) {
            CoroutineResult::Yield(Some(value)) => each(value),
            CoroutineResult::Yield(None) => input = chunks.next(),
            CoroutineResult::Return(done) => return done,
        }
    }
}

// ----------------------------------------------------------------------------
// Option (c): a framer that finds each element's bytes
// ----------------------------------------------------------------------------

/// Option (c): finds each element's bytes in the array after `"items":` and
/// hands them to [`read`]. It checks nothing outside the array, which a real
/// framer would have to.
fn framer_walk(
    chunks: impl Iterator<Item = Vec<u8>>,
    mut each: impl FnMut(Value),
) -> Result<(), String> {
    const OPEN: &[u8] = br#""items":["#;
    let mut pending: Vec<u8> = Vec::new();
    let (mut depth, mut in_str, mut escaped, mut started, mut done) = (0u32, false, false, false, false);
    let mut emit = |bytes: &mut Vec<u8>| -> Result<(), String> {
        let text = std::str::from_utf8(bytes).map_err(|why| why.to_string())?;
        if !text.trim().is_empty() {
            each(read(text, DEFAULT_MAX_DEPTH_U32).map_err(|why| why.to_string())?);
        }
        bytes.clear();
        Ok(())
    };
    for chunk in chunks {
        let mut i = 0;
        if !started {
            let found = chunk
                .windows(OPEN.len())
                .position(|w| w == OPEN)
                .ok_or("the probe's prefix is in the first chunk")?;
            i = found + OPEN.len();
            started = true;
        }
        let mut start = i;
        while i < chunk.len() && !done {
            let byte = chunk[i];
            if in_str {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    in_str = false;
                }
            } else {
                match byte {
                    b'"' => in_str = true,
                    b'{' | b'[' => depth += 1,
                    b',' | b']' if depth == 0 => {
                        pending.extend_from_slice(&chunk[start..i]);
                        emit(&mut pending)?;
                        start = i + 1;
                        done = byte == b']';
                    }
                    b'}' | b']' => depth -= 1,
                    _ => {}
                }
            }
            i += 1;
        }
        if !done {
            pending.extend_from_slice(&chunk[start..]);
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// The figures
// ----------------------------------------------------------------------------

/// The fastest of `reps` runs of `work`, in nanoseconds per byte.
fn fastest(bytes: usize, reps: u32, mut work: impl FnMut()) -> f64 {
    let mut best = std::time::Duration::MAX;
    for _ in 0..reps {
        let started = std::time::Instant::now();
        work();
        best = best.min(started.elapsed());
    }
    #[expect(clippy::cast_precision_loss, reason = "a printed figure")]
    let per_byte = best.as_nanos() as f64 / bytes as f64;
    per_byte
}

/// Prints decision 6's three figures for options (a) and (c): time per byte
/// at 1 MiB beside the whole-document read, peak memory from 1 MiB to
/// 256 MiB, and the message each gives for one malformed element.
#[test]
#[ignore = "a measurement that prints its figures, run by hand"]
#[expect(clippy::print_stderr, reason = "the figures are this test's output")]
fn decode_each_reader_probe() {
    let size = 1 << 20;
    let whole = document(size);
    let bytes = whole.len();
    let baseline = fastest(bytes, 30, || {
        release(read(&whole, DEFAULT_MAX_DEPTH_U32).expect("the document decodes"));
    });
    let mut count = 0usize;
    let coroutine = fastest(bytes, 30, || {
        coroutine_walk(document_chunks(size), |v| {
            count += 1;
            release(v);
        })
        .expect("option (a) walks the document");
    });
    let framer = fastest(bytes, 30, || {
        framer_walk(document_chunks(size), release).expect("option (c) walks the document");
    });
    eprintln!("time at {bytes} bytes, ns per byte: whole read {baseline:.2}, (a) coroutine {coroutine:.2}, (c) framer {framer:.2}");
    eprintln!("elements per walk: {}", count / 30);

    eprintln!("size        (a) peak bytes  (c) peak bytes");
    for size in [1 << 20, 16 << 20, 256 << 20] {
        let ctx = nvs_runtime::Ctx::buffered();
        coroutine_walk(document_chunks(size), release).expect("option (a) walks the document");
        let peak_a = ctx.memory_peak();
        drop(ctx);
        let ctx = nvs_runtime::Ctx::buffered();
        framer_walk(document_chunks(size), release).expect("option (c) walks the document");
        let peak_c = ctx.memory_peak();
        drop(ctx);
        eprintln!("{size:>10}  {peak_a:>14}  {peak_c:>14}");
    }

    let broken = r#"{"items":[{"id":1},{"id":2},{"id":}]}"#;
    eprintln!("whole read: {}", read(broken, DEFAULT_MAX_DEPTH_U32).map(release).expect_err("broken"));
    let one = || std::iter::once(broken.as_bytes().to_vec());
    eprintln!("(a): {}", coroutine_walk(one(), release).expect_err("broken"));
    eprintln!("(c): {}", framer_walk(one(), release).expect_err("broken"));
}
