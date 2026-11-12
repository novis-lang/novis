//! `Core\Json::decodeEach`: the elements of one array, or the values of a JSON
//! Lines stream, read one at a time while a `foreach` loop runs.
//!
//! `rule:core-classes/json-whole-and-stream` is the rule, and the parent
//! module's doc says why the reader is serde_json in a coroutine.
//!
//! # The walk
//!
//! serde_json reads the document from [`Feed`], inside a coroutine with a stack
//! of its own (`nvs_host::suspendable`). The coroutine suspends in two places:
//! when [`Feed`] needs the next chunk, and when an element has been built. Each
//! `advance()` resumes it until one element is built or the document ends. So
//! the walk holds one chunk, the parse's own frames, and one element.
//!
//! **The coroutine builds each element as `Core\Json::decode` builds a
//! document**, with [`Decode`], and calls no compiled code. A typed walk then
//! builds `T` out of that value with [`hydrate`], on the task's own stack in
//! `advance()`, which is where a constructor may run. That is the tree-first
//! path `decodeAs` takes for a class that reads its own document, so every
//! check and every message is the one `decodeAs` makes.
//!
//! **What it spends:** per walk in flight, the source's own chunk
//! (`crate::byte_source`'s cost) and the parse's copy of it, the coroutine's stack (`nvs_host::suspendable`'s cost), one element as a
//! value, and for a typed walk the same element as `T`. A typed element is
//! built twice, once as a value and once as `T`, which is time spent per
//! element in place of a second decoder that runs compiled code across
//! suspensions. Nothing grows with the document.
//!
//! # Outside the pointer
//!
//! [`Toward`] walks the path `$at` names, one step per nesting level. Every
//! value off that path, before the array or after it, is read by [`Skip`]:
//! its syntax and its depth are checked, and nothing is built. The document
//! after the array is read to its end, so trailing text still fails.
//!
//! **A skipped string value is not kept.** serde_json copies a string into its
//! scratch buffer to give a visitor a `&str`, so [`Skip`] does not ask for
//! one. It peeks with `deserialize_option`, and the byte serde_json peeked is
//! the last one [`Feed`] handed out, since `io::Bytes` reads one byte per call.
//! A `"` is read with `deserialize_ignored_any`, whose `ignore_str` keeps
//! nothing. An array or an object is read with [`Skip`] again, so its depth is
//! still checked. `ignore_value` would check none.
//!
//! **What it spends:** a skipped object's keys still pass through
//! serde_json's `MapKey`, which copies each key into scratch. So skipping costs
//! O(longest key), counted against the request, and nothing for its values.
//!
//! # A failure in the middle
//!
//! A syntax error is found by the parse at some byte, which can be after many
//! elements were given to the program. The walk ends there, and the
//! `ParseError` names the byte offset ([`Feed`] counts the bytes it hands
//! out). The elements already given stay given.

use std::cell::Cell;
use std::io;

use nvs_host::{CoroutineResult, Suspendable, Yielder};
use nvs_runtime::{Fault, Tag, ThrownClass, Value};
use serde::de::{DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::byte_source::ByteSource;

use super::{
    DEADLINE_PASSED, Deadline, Decode, ELEMENTS, ELEMENTS_NAME, Lists, Reading, check_codec,
    hydrate, max_depth, repeated,
};

/// The member every message names.
const MEMBER: &str = "Core\\Json::decodeEach";

/// [`ELEMENTS`]' first slot: the key the walk is filed under with
/// `nvs_runtime::Ctx::hold_open_reader`, and `0` once the walk has ended.
const READER_AT: usize = 0;
/// [`ELEMENTS`]' second slot: the element the last `advance()` built, `null`
/// before the first one and after the last.
const ELEMENT_AT: usize = 1;
/// [`ELEMENTS`]' third slot: the value [`ByteSource::from_arg`] gave, which
/// every read of the source is handed. It is in a slot and not in the filed
/// state so that the request's own release frees it.
const SOURCE_AT: usize = 2;

/// What the coroutine suspends with.
enum Up {
    /// [`Feed`] has handed out every byte it has, and needs the next chunk.
    Need,
    /// An element, as the one reference [`Decode`] made, with the lists a
    /// typed walk's [`hydrate`] asks about.
    Element(Value, Option<Lists>),
}

/// What the coroutine is resumed with: the next chunk, or `None` at the end of
/// the source and after an element.
type Down = Option<Vec<u8>>;

/// How the coroutine ends: the whole document read, or the parse's error and
/// the byte offset it was found at.
type Ended = Result<(), (String, u64)>;

/// One walk's state, filed on the request.
struct Walk {
    /// The suspended parse. It is taken out for the length of one `advance()`,
    /// because pulling a chunk needs the context the walk is filed in.
    parse: Option<Suspendable<Down, Up, Ended>>,
    /// Where the chunks come from. It is taken out for the length of one
    /// `advance()`, as the parse is.
    source: Option<ByteSource>,
    /// The class each element is built as, or `None` for `mixed`. It lives in
    /// the compiled unit's constants, which outlive the request.
    class: Option<*const nvs_runtime::ClassDesc>,
    /// An inline shape's wire contract, where the call site wrote one.
    shape: Option<*const nvs_runtime::ShapeCodec>,
    /// Whether each element is a list of `class`.
    list: bool,
}

impl std::fmt::Debug for Walk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Walk")
            .field("source", &self.source)
            .field("typed", &self.class.is_some())
            .finish_non_exhaustive()
    }
}

impl nvs_runtime::HeldReader for Walk {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ----------------------------------------------------------------------------
// Inside the coroutine
// ----------------------------------------------------------------------------

/// The bytes serde_json reads, one chunk at a time. It suspends with
/// [`Up::Need`] when a chunk is used up, and counts every byte it hands out.
struct Feed<'y> {
    /// The chunk being read.
    chunk: Vec<u8>,
    /// How much of it is read.
    at: usize,
    /// How many bytes of the source were handed out.
    handed: &'y Cell<u64>,
    /// The last byte handed out: the one serde_json has peeked, where it has.
    last: &'y Cell<u8>,
    /// The coroutine's side of the switch.
    yielder: &'y Yielder<Down, Up>,
}

impl io::Read for Feed<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.at == self.chunk.len() {
            match self.yielder.suspend(Up::Need) {
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
        if let Some(&byte) = buf[..n].last() {
            self.last.set(byte);
        }
        let n_bytes = u64::try_from(n).unwrap_or(u64::MAX);
        self.handed.set(self.handed.get().saturating_add(n_bytes));
        Ok(n)
    }
}

/// What every step of the walk shares.
#[derive(Clone, Copy)]
struct Walker<'w, 'y> {
    /// The deepest nesting allowed, counted from the document.
    max: u32,
    /// Whether elements are built for [`hydrate`], which asks which arrays
    /// were JSON lists.
    typed: bool,
    /// The coroutine's side of the switch.
    yielder: &'y Yielder<Down, Up>,
    /// Set once the pointer's array is reached.
    found: &'w Cell<bool>,
    /// [`Feed`]'s last byte handed out, which [`Skip`] reads.
    last: &'w Cell<u8>,
    /// The request's deadline, counted once per value the walk builds or
    /// skips. [`Deadline`] owns how often it is read and what that costs.
    deadline: &'w Deadline,
}

impl<'w> Walker<'w, '_> {
    /// The [`Decode`] for a value at `depth`.
    const fn decode(self, depth: u32) -> Decode<'w> {
        Decode {
            depth,
            max: self.max,
            lists: None,
            deadline: Some(self.deadline),
        }
    }

    /// The [`Skip`] for a value off the path at `depth`.
    const fn skip(self, depth: u32) -> Skip<'w> {
        Skip {
            depth,
            max: self.max,
            last: self.last,
            deadline: self.deadline,
        }
    }

    /// Builds the value `deserializer` is at as an element, at `depth`, and
    /// suspends with it. `false` where there was none.
    fn element<'de, S: SeqAccess<'de>>(self, seq: &mut S, depth: u32) -> Result<bool, S::Error> {
        let lists = self.typed.then(Lists::default);
        let seed = Decode {
            lists: lists.as_ref(),
            ..self.decode(depth)
        };
        let Some(value) = seq.next_element_seed(seed)? else {
            return Ok(false);
        };
        self.yielder.suspend(Up::Element(value, lists));
        Ok(true)
    }
}

/// The scalar visits of a visitor that accepts every scalar and builds nothing.
macro_rules! accepted_scalars {
    () => {
        fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
            Ok(())
        }
        fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
            Ok(())
        }
        fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
            Ok(())
        }
        fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
            Ok(())
        }
        fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
            Ok(())
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
            Ok(())
        }
    };
}

/// A value off the pointer's path: its syntax and its depth are checked, and
/// nothing is built. A string value is not kept either (§ *Outside the
/// pointer*).
#[derive(Clone, Copy)]
struct Skip<'w> {
    /// This value's nesting level.
    depth: u32,
    /// The deepest nesting allowed.
    max: u32,
    /// [`Feed`]'s last byte handed out.
    last: &'w Cell<u8>,
    /// The walk's deadline poll, which counts each skipped value too.
    deadline: &'w Deadline,
}

impl<'de> DeserializeSeed<'de> for Skip<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        if self
            .deadline
            .borrow_mut()
            .as_mut()
            .is_some_and(nvs_runtime::DeadlineBatch::stopped)
        {
            return Err(<D::Error as serde::de::Error>::custom(DEADLINE_PASSED));
        }
        // `deserialize_option` peeks at the value's first byte and consumes
        // nothing but a `null`, so `visit_some` chooses how to read the rest.
        deserializer.deserialize_option(self)
    }
}

impl<'de> Visitor<'de> for Skip<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a JSON value")
    }

    accepted_scalars!();

    fn visit_none<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        if self.last.get() == b'"' {
            deserializer.deserialize_ignored_any(IgnoredAny).map(|_| ())
        } else {
            deserializer.deserialize_any(self)
        }
    }

    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<(), S::Error> {
        let inner = self.opened()?;
        while seq.next_element_seed(inner)?.is_some() {}
        Ok(())
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
        let inner = self.opened()?;
        while map.next_key::<IgnoredAny>()?.is_some() {
            map.next_value_seed(inner)?;
        }
        Ok(())
    }
}

impl Skip<'_> {
    /// The level inside an array or an object opened at this one, or
    /// [`Decode`]'s refusal where this level may not open one.
    fn opened<E: serde::de::Error>(self) -> Result<Self, E> {
        let decode = Decode {
            depth: self.depth,
            max: self.max,
            lists: None,
            deadline: None,
        };
        if self.depth >= self.max {
            return Err(decode.too_deep());
        }
        Ok(Self {
            depth: self.depth + 1,
            ..self
        })
    }
}

/// The value at the start of `path`, at `depth`: the array to walk where the
/// path is empty, and otherwise the object or array the next step is inside.
struct Toward<'p, 'w, 'y> {
    /// The steps still to take, each a decoded reference token.
    path: &'p [String],
    /// This value's nesting level.
    depth: u32,
    /// What every step shares.
    walker: Walker<'w, 'y>,
}

impl<'de> DeserializeSeed<'de> for Toward<'_, '_, '_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl Toward<'_, '_, '_> {
    /// The level inside an array or an object opened here, or [`Decode`]'s
    /// refusal where this level may not open one.
    fn opened<E: serde::de::Error>(&self) -> Result<u32, E> {
        if self.depth >= self.walker.max {
            return Err(self.walker.decode(self.depth).too_deep());
        }
        Ok(self.depth + 1)
    }
}

impl<'de> Visitor<'de> for Toward<'_, '_, '_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a JSON value")
    }

    // A scalar is where the path ends without an array: the walk reports
    // that once the document is read.
    accepted_scalars!();

    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<(), S::Error> {
        let inner = self.opened()?;
        let Some((step, rest)) = self.path.split_first() else {
            self.walker.found.set(true);
            while self.walker.element(&mut seq, inner)? {}
            return Ok(());
        };
        let wanted = index_of(step);
        let mut at = 0usize;
        loop {
            let more = if Some(at) == wanted {
                seq.next_element_seed(Toward {
                    path: rest,
                    depth: inner,
                    walker: self.walker,
                })?
            } else {
                seq.next_element_seed(self.walker.skip(inner))?
            };
            if more.is_none() {
                return Ok(());
            }
            at += 1;
        }
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
        let inner = self.opened()?;
        let Some((step, rest)) = self.path.split_first() else {
            return Err(serde::de::Error::custom(
                "the value the pointer names is an object, not an array",
            ));
        };
        let mut taken = false;
        while let Some(key) = map.next_key::<String>()? {
            if key == *step {
                // A key on the path written twice would make the walk read two
                // arrays as one, so it is refused as `decode` refuses it.
                if taken {
                    return Err(repeated(&key));
                }
                taken = true;
                map.next_value_seed(Toward {
                    path: rest,
                    depth: inner,
                    walker: self.walker,
                })?;
            } else {
                map.next_value_seed(self.walker.skip(inner))?;
            }
        }
        Ok(())
    }
}

/// A reference token as an array index: digits with no leading zero, as
/// RFC 6901 § 4 writes one. `None` for any other token, which no element has.
fn index_of(token: &str) -> Option<usize> {
    let digits = !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit());
    if !digits || (token.len() > 1 && token.starts_with('0')) {
        return None;
    }
    token.parse().ok()
}

/// The reference tokens of a JSON Pointer (RFC 6901), decoded: `~1` is `/`
/// and `~0` is `~`. `''` is the document itself and has none.
///
/// # Errors
///
/// A `LogicError` for a pointer that does not start with `/`, or that has a
/// `~` not followed by `0` or `1`.
fn tokens_of(pointer: &str) -> Result<Vec<String>, Fault> {
    if pointer.is_empty() {
        return Ok(Vec::new());
    }
    let bad = |why: &str| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!("{MEMBER}(): the pointer {pointer:?} {why}"),
        )
    };
    let Some(rest) = pointer.strip_prefix('/') else {
        return Err(bad("does not start with `/`"));
    };
    rest.split('/')
        .map(|token| {
            let mut out = String::with_capacity(token.len());
            let mut chars = token.chars();
            while let Some(c) = chars.next() {
                if c != '~' {
                    out.push(c);
                    continue;
                }
                match chars.next() {
                    Some('0') => out.push('~'),
                    Some('1') => out.push('/'),
                    _ => return Err(bad("has a `~` that is not `~0` or `~1`")),
                }
            }
            Ok(out)
        })
        .collect()
}

/// The coroutine's body: the whole parse of one source.
fn read_source(
    yielder: &Yielder<Down, Up>,
    path: &[String],
    lines: bool,
    max: u32,
    typed: bool,
    deadline: nvs_runtime::DeadlineBatch,
) -> Ended {
    let handed = Cell::new(0u64);
    let found = Cell::new(false);
    let last = Cell::new(0u8);
    let deadline = Deadline::new(Some(deadline));
    let feed = Feed {
        chunk: Vec::new(),
        at: 0,
        handed: &handed,
        last: &last,
        yielder,
    };
    let mut deserializer = serde_json::Deserializer::from_reader(feed);
    // Depth is `Decode`'s and `Skip`'s to bound, as `parse` in the parent
    // module says, and the coroutine's stack holds the deepest allowed.
    deserializer.disable_recursion_limit();
    let walker = Walker {
        max,
        typed,
        yielder,
        found: &found,
        last: &last,
        deadline: &deadline,
    };
    let read = if lines {
        each_line(&mut deserializer, walker)
    } else {
        Toward {
            path,
            depth: 1,
            walker,
        }
        .deserialize(&mut deserializer)
        .and_then(|()| deserializer.end())
    };
    let at = handed.get().saturating_sub(1);
    read.map_err(|why| (why.to_string(), at))?;
    if !lines && !found.get() {
        return Err((
            "the document has no array where the pointer points".to_owned(),
            at,
        ));
    }
    Ok(())
}

/// JSON Lines: each value of the stream is an element, at depth 1.
fn each_line<R: io::Read>(
    deserializer: &mut serde_json::Deserializer<serde_json::de::IoRead<R>>,
    walker: Walker<'_, '_>,
) -> Result<(), serde_json::Error> {
    loop {
        // `end` reads whitespace up to the next value and fails with
        // "trailing characters" only when one follows, so a pass is the end of
        // the stream and that failure is the next value.
        match deserializer.end() {
            Ok(()) => return Ok(()),
            Err(why) if why.is_io() => return Err(why),
            Err(_) => {}
        }
        let lists = walker.typed.then(Lists::default);
        let value = Decode {
            lists: lists.as_ref(),
            ..walker.decode(1)
        }
        .deserialize(&mut *deserializer)?;
        walker.yielder.suspend(Up::Element(value, lists));
    }
}

// ----------------------------------------------------------------------------
// The helpers
// ----------------------------------------------------------------------------

/// The walk filed under `key`, as this module filed it.
///
/// # Errors
///
/// A [`Fault::fatal`] where this task holds no such walk: a walk another task
/// returned, since handle keys are unique in the process.
fn walk_at(ctx: &mut nvs_runtime::Ctx, key: u64) -> Result<&mut Walk, Fault> {
    ctx.open_reader_mut(key)
        .and_then(|reader| reader.as_any_mut().downcast_mut::<Walk>())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "A `{ELEMENTS_NAME}` walk was read in a task other than the one that called \
                 `{MEMBER}`. A walk works only in that task."
            ))
        })
}

/// The `ParseError` for a parse that failed at byte `at` of the source.
fn unparsed(why: &str, at: u64) -> Fault {
    let message = format!("{MEMBER}(): {why}, at byte {at}");
    let issues = crate::issue::list([("", message.as_str())]);
    Fault::thrown_with_issues(ThrownClass::Parse, message, issues)
}

/// Resumes the walk filed under `key` until it builds an element, which comes
/// back as `T`, or ends, which comes back `None`.
///
/// # Errors
///
/// [`ByteSource::next_chunk`]'s, [`unparsed`]'s, and [`hydrate`]'s for an
/// element that is not a `T`.
fn next_element(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    holder: Value,
) -> Result<Option<Value>, Fault> {
    let walk = walk_at(ctx, key)?;
    let mut parse = walk.parse.take().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `{ELEMENTS_NAME}` walk lost its parse"
        ))
    })?;
    let mut source = walk.source.take().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `{ELEMENTS_NAME}` walk lost its source"
        ))
    })?;
    let (class, shape, list) = (walk.class, walk.shape, walk.list);
    let mut input: Down = None;
    let raw = loop {
        match parse.resume(input.take()) {
            // The parse copies each chunk into its feed, since it reads the
            // chunk across `advance()` calls and the source lends it only
            // until the next read.
            // `rule:http-server/time-is-bounded-inside-a-helper`: one poll
            // per chunk, which is one relaxed load per chunk read. The parse
            // also polls once every 256 values inside the chunk.
            CoroutineResult::Yield(Up::Need) => {
                nvs_runtime::poll_deadline(ctx, MEMBER)?;
                input = source
                    .next_chunk(ctx, holder)?
                    .map(|chunk| chunk.bytes().to_vec());
            }
            CoroutineResult::Yield(Up::Element(value, lists)) => break Some((value, lists)),
            CoroutineResult::Return(Ok(())) => break None,
            // A parse the deadline stopped is the deadline's FATAL. The flag
            // stays set once it is set, so this poll finds it.
            CoroutineResult::Return(Err((why, at))) => {
                nvs_runtime::poll_deadline(ctx, MEMBER)?;
                return Err(unparsed(&why, at));
            }
        }
    };
    let walk = walk_at(ctx, key)?;
    walk.source = Some(source);
    let Some((value, lists)) = raw else {
        return Ok(None);
    };
    walk.parse = Some(parse);
    let Some(class) = class else {
        return Ok(Some(value));
    };
    let lists = lists.unwrap_or_default();
    #[expect(
        unsafe_code,
        reason = "the descriptor and the contract came out of the compiled unit's \
                  constants, which outlive the request, and the value is the one \
                  reference the parse made, which `hydrate` takes"
    )]
    unsafe {
        hydrate(
            ctx,
            class,
            shape,
            value,
            list,
            Reading::Wire(&lists),
            MEMBER,
        )
        .map(Some)
    }
}

/// One `advance()`: the next element parked in [`ELEMENT_AT`], and whether
/// there was one. The end of the walk and a failure both give the walk up.
///
/// # Errors
///
/// [`next_element`]'s.
fn step(ctx: &mut nvs_runtime::Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = crate::instance::receiver(value, &ELEMENTS, member)?;
    let key = crate::instance::slot(receiver, READER_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{ELEMENTS_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                ELEMENTS.slots[READER_AT]
            ))
        })?;
    if key == 0 {
        crate::instance::set_slot(receiver, ELEMENT_AT, Value::null());
        return Ok(Value::bool(false));
    }
    let holder = crate::instance::slot(receiver, SOURCE_AT);
    match next_element(ctx, key, holder) {
        Ok(Some(element)) => {
            crate::instance::set_slot(receiver, ELEMENT_AT, element);
            Ok(Value::bool(true))
        }
        ended => {
            // Dropping the walk frees its parse, and a parse still suspended
            // unwinds its stack here.
            drop(ctx.take_open_reader(key));
            crate::instance::set_slot(receiver, READER_AT, Value::uint(0));
            crate::instance::set_slot(receiver, ELEMENT_AT, Value::null());
            crate::instance::set_slot(receiver, SOURCE_AT, Value::null());
            ended.map(|_| Value::bool(false))
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Json::decodeEach<T>(Core\Bytes::Source|Core\Bytes::Chunks $source,
    /// string $at = '', { lines?: bool, maxDepth?: uint }): Iterable<T>`. The
    /// source is `bytes`, a file, a part, a body stream or chunks, read forward
    /// one chunk at a time.
    ///
    /// Slots 0 to 2 are `registry::WRITTEN_CLASS_MEMBERS`' block, as
    /// `decodeAs`'s are. A zero word in slot 0 is `mixed`: each element is
    /// built as `Core\Json::decode` builds a document.
    ///
    /// **Nothing is read here.** The arguments are checked, the walk is filed
    /// on the request, and the first read is the first `advance()`.
    fn nvs_core_json_decode_each(ctx, args: [7]) {
        let class = args[0].as_class_desc();
        let list = args[1].as_bool().unwrap_or(false);
        let shape = args[2].as_shape_codec();
        if let Some(class) = class {
            #[expect(
                unsafe_code,
                reason = "the descriptor and the contract came out of the compiled \
                          unit's constants, which outlive this call"
            )]
            unsafe {
                check_codec(class, shape, MEMBER)?;
            }
        }
        let pointer = args[4].as_text().ok_or_else(|| {
            Fault::fatal(format!("{MEMBER} expected a string for `at`, got tag {}", args[4].tag_byte()))
        })?;
        let path = tokens_of(pointer)?;
        let lines = args[5].as_bool().ok_or_else(|| {
            Fault::fatal(format!("{MEMBER} expected a bool for `lines`, got tag {}", args[5].tag_byte()))
        })?;
        if lines && !path.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{MEMBER}(): JSON Lines has no document to point into, so `at` must be ''"),
            ));
        }
        let max = max_depth(&args[6], MEMBER)?;

        let (source, holder) = ByteSource::from_arg(ctx, args[3], MEMBER)?;

        let typed = class.is_some();
        let deadline = nvs_runtime::DeadlineBatch::new(ctx);
        let parse = nvs_host::suspendable(move |yielder: &Yielder<Down, Up>, _: Down| {
            read_source(yielder, &path, lines, max, typed, deadline)
        });
        let parse = match parse {
            Ok(parse) => parse,
            Err(why) => {
                #[expect(unsafe_code, reason = "the reference taken above, which no slot owns yet")]
                unsafe {
                    holder.release();
                }
                return Err(Fault::thrown(format!(
                    "{MEMBER}(): the walk could not reserve its stack: {why}"
                )));
            }
        };
        let key = ctx.hold_open_reader(Box::new(Walk {
            parse: Some(parse),
            source: Some(source),
            class,
            shape,
            list,
        }));
        Ok(crate::instance::build(
            &ELEMENTS,
            [Value::uint(key), Value::null(), holder],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<T>::iterate()` on a `Core\Json\Elements`: the walk itself,
    /// for `Core\Csv\Rows`' reason.
    fn nvs_core_json_elements_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &ELEMENTS, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<T>::advance()` on a `Core\Json\Elements`: builds the next
    /// element, and gives the walk up at the end of the document.
    fn nvs_core_json_elements_advance(ctx, args: [1]) {
        let stepped = step(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<T>::current()` on a `Core\Json\Elements`: the element the
    /// last `advance()` built, and the only one the walk holds.
    fn nvs_core_json_elements_current(_ctx, args: [1]) {
        let read = crate::instance::receiver(args[0], &ELEMENTS, nvs_runtime::sequence::CURRENT)
            .map(|receiver| {
                let held = crate::instance::slot(receiver, ELEMENT_AT);
                #[expect(
                    unsafe_code,
                    reason = "the slot keeps its reference until the next `advance`, so \
                              the value handed back needs one of its own"
                )]
                unsafe {
                    held.retain();
                }
                held
            });
        crate::cursor::consume(args[0]);
        read
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, OutputSink, ThrownClass, Value, call};

    /// Frees the one reference this frame owns.
    fn released(value: Value) {
        #[expect(unsafe_code, reason = "the case owns the one reference it releases")]
        unsafe {
            value.release();
        }
    }

    /// `text` as chunks of `size` bytes, the way `bodyStream()` gives a body.
    fn chunks(text: &str, size: usize) -> Value {
        let mut array = NvsArray::new();
        for piece in text.as_bytes().chunks(size) {
            array.append(Value::bytes(NvsStr::new(piece)));
        }
        Value::array(array)
    }

    /// The walk `decodeEach<mixed>` gives over `source`, at `at`.
    fn walk(ctx: &mut Ctx, source: Value, at: &str, lines: bool) -> Result<Value, i32> {
        let args = [
            Value::null(),
            Value::bool(false),
            Value::null(),
            source,
            Value::text(at),
            Value::bool(lines),
            Value::uint(u64::from(super::super::DEFAULT_MAX_DEPTH_U32)),
        ];
        let answer = call(super::nvs_core_json_decode_each, ctx, &args);
        for arg in args {
            released(arg);
        }
        answer
    }

    /// One `advance()` and its `current()`: the element, `None` at the end.
    fn next(ctx: &mut Ctx, elements: Value) -> Result<Option<Value>, Fault> {
        if super::step(ctx, elements)?.as_bool() != Some(true) {
            return Ok(None);
        }
        let receiver = elements.obj_ptr().expect("the walk is an object");
        let held = crate::instance::slot(receiver, super::ELEMENT_AT);
        #[expect(unsafe_code, reason = "the case keeps a reference of its own")]
        unsafe {
            held.retain();
        }
        Ok(Some(held))
    }

    /// Every element of a walk over `text` at `at`, as JSON integers.
    fn ints(text: &str, at: &str, lines: bool) -> Vec<i64> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let elements = walk(&mut ctx, chunks(text, 3), at, lines).expect("the walk starts");
        let mut out = Vec::new();
        while let Some(element) = next(&mut ctx, elements).expect("the document is valid") {
            out.push(element.as_int().expect("every element is an int"));
            released(element);
        }
        released(elements);
        out
    }

    /// `rule:http-server/time-is-bounded-inside-a-helper`: the walk polls the
    /// deadline before it reads each chunk, and once every 256 values inside
    /// one chunk. After the deadline has passed, `advance()` is the deadline's
    /// FATAL within one poll gap.
    // covers: Core\Json::decodeEach
    #[test]
    #[cfg_attr(
        miri,
        ignore = "walks a thousand elements on a coroutine, which Miri runs for minutes"
    )]
    fn a_walk_stops_at_an_expired_deadline() {
        let gap = nvs_runtime::DEADLINE_POLL_BATCH;
        let text = format!("[{}0]", "0,".repeat(gap * 4));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let elements = walk(&mut ctx, chunks(&text, 64), "", false).expect("the walk starts");
        ctx.expire_deadline();
        let first = next(&mut ctx, elements);
        assert!(
            matches!(first, Err(Fault::Fatal(_))),
            "the first chunk is not read"
        );
        released(elements);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let elements =
            walk(&mut ctx, chunks(&text, text.len()), "", false).expect("the walk starts");
        let first = next(&mut ctx, elements)
            .expect("the document is valid")
            .expect("the list has elements");
        released(first);
        ctx.expire_deadline();
        let mut read = 0_usize;
        let ended = loop {
            match next(&mut ctx, elements) {
                Ok(Some(element)) => {
                    released(element);
                    read += 1;
                }
                ended => break ended,
            }
        };
        assert!(
            matches!(ended, Err(Fault::Fatal(_))),
            "the walk inside one chunk stops"
        );
        assert!(read < gap, "{read} elements were read after the deadline");
        released(elements);
    }

    /// The message and class of a thrown fault.
    fn thrown(fault: Fault) -> (ThrownClass, String) {
        match fault {
            Fault::Thrown(class, message) | Fault::ThrownWithSlots(class, message, _) => {
                (class, message.into_owned())
            }
            other => panic!("expected a thrown fault, got {other:?}"),
        }
    }

    /// A document of `count` small objects at `/items`, after a `meta` value of
    /// `skipped` objects the walk does not build.
    fn document(count: usize, skipped: usize) -> String {
        let element = |at: usize| format!("{{\"id\":{at},\"name\":\"item-{at}\"}}");
        let list = |n: usize| (0..n).map(element).collect::<Vec<_>>().join(",");
        format!(
            "{{\"meta\":[{}],\"items\":[{}],\"end\":true}}",
            list(skipped),
            list(count)
        )
    }

    /// The request's peak memory while a walk over `text` at `/items` reads
    /// every element from a file and frees each one.
    ///
    /// The file is padded with trailing spaces to four read chunks. A file's
    /// read buffer is its size, at most a chunk, so without the padding a
    /// small document would pay for smaller buffers than a large one, and the
    /// comparison would measure the buffers and not what the walk holds.
    fn peak_of_walk(text: &str) -> usize {
        let dir = nvs_repo::scratch("json-each");
        let path = dir.join("document.json");
        let floor = 4 * crate::byte_source::READ_CHUNK;
        let padded = format!("{text}{}", " ".repeat(floor.saturating_sub(text.len())));
        std::fs::write(&path, padded).expect("the case writes its document");
        let mut ctx = Ctx::buffered();
        let file = std::fs::File::open(&path).expect("the case wrote this file");
        let key = ctx.hold_open_file(file);
        let handle = crate::io::file_handle(
            key,
            Value::text(&path.display().to_string()),
            nvs_runtime::capability::Access::Read,
        );
        let elements = walk(&mut ctx, handle, "/items", false).expect("the walk starts");
        let mut count = 0usize;
        while let Some(element) = next(&mut ctx, elements).expect("the document is valid") {
            released(element);
            count += 1;
        }
        assert!(count > 0, "the walk read the array");
        released(elements);
        ctx.memory_peak()
    }

    /// The walk holds one element: an element the program keeps has no other
    /// owner once the walk moves on, and memory stays flat from a small
    /// document to one sixty times its size.
    // covers: Core\Json::decodeEach
    #[test]
    #[cfg_attr(
        miri,
        ignore = "the walk's coroutine stack maps a guard page, which Miri does not emulate"
    )]
    fn decode_each_holds_one_element_at_a_time() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let text = r#"[{"a":[1]},{"a":[2]},{"a":[3]}]"#;
        let elements = walk(&mut ctx, chunks(text, 4), "", false).expect("the walk starts");
        let first = next(&mut ctx, elements).unwrap().expect("a first element");
        let ptr = first.array_ptr().expect("an object is an array");
        let second = next(&mut ctx, elements).unwrap().expect("a second element");
        #[expect(unsafe_code, reason = "the case keeps a reference to `first`")]
        let owners = unsafe { NvsArray::refcount_of(ptr) };
        assert_eq!(owners, 1, "the walk let go of the element it moved past");
        released(first);
        released(second);
        assert!(next(&mut ctx, elements).unwrap().is_some());
        assert!(next(&mut ctx, elements).unwrap().is_none());
        released(elements);

        let small = peak_of_walk(&document(1_000, 0));
        let large = peak_of_walk(&document(60_000, 0));
        assert!(small > 0, "the request's memory is counted");
        assert!(
            large.saturating_sub(small) < 64 << 10,
            "the peak grew from {small} to {large} bytes with the document"
        );
    }

    /// The pointer names an array by object keys and array indexes, with
    /// `~1` and `~0` decoded, and `''` is the document itself.
    // covers: Core\Json::decodeEach
    #[test]
    #[cfg_attr(
        miri,
        ignore = "the walk's coroutine stack maps a guard page, which Miri does not emulate"
    )]
    fn decode_each_reads_the_array_its_pointer_names() {
        let text = r#"{"a":{"items":[0]},"list":[[1],[2,3,4]],"a/b":{"~":[5,6]},"items":[]}"#;
        assert_eq!(ints(text, "/list/1", false), [2, 3, 4]);
        assert_eq!(ints(text, "/a/items", false), [0]);
        assert_eq!(ints(text, "/a~1b/~0", false), [5, 6]);
        assert_eq!(ints(text, "/items", false), Vec::<i64>::new());
        assert_eq!(ints("[7,8]", "", false), [7, 8]);

        let mut ctx = Ctx::new(OutputSink::Sink);
        for (pointer, why) in [
            ("/missing", "no array"),
            ("/a", "an object"),
            ("/list/01", "no array"),
        ] {
            let elements =
                walk(&mut ctx, chunks(text, 5), pointer, false).expect("the walk starts");
            let fault = next(&mut ctx, elements).expect_err("the pointer names no array");
            let (class, message) = thrown(fault);
            assert_eq!(class, ThrownClass::Parse, "{message}");
            assert!(message.contains(why), "{pointer}: {message}");
            released(elements);
        }
        assert!(
            walk(&mut ctx, chunks(text, 5), "items", false).is_err(),
            "a pointer starts with `/`"
        );
        assert!(
            walk(&mut ctx, chunks(text, 5), "/a~2", false).is_err(),
            "`~2` is not an escape"
        );
    }

    /// A value before the array is read and checked but never built: a walk
    /// after many skipped objects peaks where a walk with none does. A value
    /// off the path is still held to the depth bound and the syntax.
    // covers: Core\Json::decodeEach
    #[test]
    #[cfg_attr(
        miri,
        ignore = "the walk's coroutine stack maps a guard page, which Miri does not emulate"
    )]
    fn decode_each_skips_a_value_outside_the_pointer_without_building_it() {
        let plain = peak_of_walk(&document(100, 0));
        let skipping = peak_of_walk(&document(100, 50_000));
        assert!(
            skipping.saturating_sub(plain) < 64 << 10,
            "skipping grew the peak from {plain} to {skipping} bytes"
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let deep = format!(
            "{{\"meta\":{}1{},\"items\":[1]}}",
            "[".repeat(600),
            "]".repeat(600)
        );
        let elements = walk(&mut ctx, chunks(&deep, 64), "/items", false).expect("the walk starts");
        let (_, message) = thrown(next(&mut ctx, elements).expect_err("too deep"));
        assert!(message.contains("maxDepth"), "{message}");
        released(elements);

        let broken = r#"{"meta":{"a" 1},"items":[1]}"#;
        let elements = walk(&mut ctx, chunks(broken, 4), "/items", false).expect("the walk starts");
        assert!(
            next(&mut ctx, elements).is_err(),
            "a skipped value is checked for syntax"
        );
        released(elements);
    }

    /// A skipped string value is not kept: a walk past an 8 MiB string peaks
    /// where a walk without it does. A string inside a skipped array is not
    /// kept either, and a broken one still fails.
    // covers: Core\Json::decodeEach
    #[test]
    #[cfg_attr(
        miri,
        ignore = "the walk's coroutine stack maps a guard page, which Miri does not emulate"
    )]
    fn decode_each_does_not_keep_a_skipped_string() {
        let items = (0..100)
            .map(|at| at.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let huge = "x".repeat(8 << 20);
        let plain = peak_of_walk(&format!("{{\"items\":[{items}],\"end\":true}}"));
        for text in [
            format!("{{\"meta\":\"{huge}\",\"items\":[{items}],\"end\":true}}"),
            format!("{{\"meta\":[1, \"{huge}\"],\"items\":[{items}],\"end\":\"{huge}\"}}"),
        ] {
            let skipping = peak_of_walk(&text);
            assert!(
                skipping.saturating_sub(plain) < 64 << 10,
                "a skipped string grew the peak from {plain} to {skipping} bytes"
            );
        }

        let mut ctx = Ctx::new(OutputSink::Sink);
        let broken = r#"{"meta":"a\qb","items":[1]}"#;
        let elements = walk(&mut ctx, chunks(broken, 4), "/items", false).expect("the walk starts");
        assert!(
            next(&mut ctx, elements).is_err(),
            "a skipped string is checked for syntax"
        );
        released(elements);
        assert_eq!(
            ints(r#"{"a":null,"b":"s","items":[1,2]}"#, "/items", false),
            [1, 2]
        );
    }

    /// A syntax error after some elements throws where it is found: the
    /// elements before it were given, the `ParseError` names the byte, and the
    /// walk has ended. Text after the document fails the same way.
    // covers: Core\Json::decodeEach
    #[test]
    #[cfg_attr(
        miri,
        ignore = "the walk's coroutine stack maps a guard page, which Miri does not emulate"
    )]
    fn decode_each_throws_where_a_failure_is_found_after_yielded_elements() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        for (text, given) in [("[1,2,3,}", 3), (r#"{"items":[1,2]} x"#, 2)] {
            let at = if text.starts_with('[') { "" } else { "/items" };
            let elements = walk(&mut ctx, chunks(text, 2), at, false).expect("the walk starts");
            for _ in 0..given {
                released(
                    next(&mut ctx, elements)
                        .expect("an element before the error")
                        .expect("one"),
                );
            }
            let (class, message) = thrown(next(&mut ctx, elements).expect_err("the error"));
            assert_eq!(class, ThrownClass::Parse);
            assert!(
                message.starts_with("Core\\Json::decodeEach(): "),
                "{message}"
            );
            assert!(message.contains(", at byte "), "{message}");
            assert!(
                next(&mut ctx, elements).unwrap().is_none(),
                "the walk ended"
            );
            released(elements);
        }
    }

    /// JSON Lines gives one element per value, blank lines between them are
    /// allowed, and a pointer is refused.
    // covers: Core\Json::decodeEach
    #[test]
    #[cfg_attr(
        miri,
        ignore = "the walk's coroutine stack maps a guard page, which Miri does not emulate"
    )]
    fn decode_each_reads_json_lines() {
        assert_eq!(ints("1\n2\n\n3\n", "", true), [1, 2, 3]);
        assert_eq!(ints("4", "", true), [4]);
        assert_eq!(ints("", "", true), Vec::<i64>::new());

        let mut ctx = Ctx::new(OutputSink::Sink);
        let elements =
            walk(&mut ctx, chunks("{\"a\":1}\n{\"a\":2}\n", 3), "", true).expect("starts");
        let mut count = 0;
        while let Some(element) = next(&mut ctx, elements).expect("valid lines") {
            assert!(element.array_ptr().is_some(), "each line is an object");
            released(element);
            count += 1;
        }
        assert_eq!(count, 2);
        released(elements);

        let elements = walk(&mut ctx, chunks("1\n{\n", 3), "", true).expect("starts");
        released(next(&mut ctx, elements).unwrap().expect("the first line"));
        assert!(next(&mut ctx, elements).is_err(), "a broken line throws");
        released(elements);

        assert!(
            walk(&mut ctx, chunks("1\n", 3), "/items", true).is_err(),
            "`at` must be ''"
        );
    }
}
