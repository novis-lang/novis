//! `Core\Markdown::toHtmlEach`: `rule:core-classes/markdown-chunks`' walk
//! over a source, one block group per `advance()`.
//!
//! # Two passes over one source
//!
//! A reference may be used before it is defined, so the walk reads the
//! document twice. The first `advance()` runs the first pass
//! ([`segments::definitions`]) over the whole source, and every `advance()`
//! after it renders segments of the second pass until a group is ready.
//!
//! **A file that can seek and a `bytes` value are read again from their
//! start** ([`ByteSource::restart`]), so the walk holds one segment, its
//! groups and the definitions. Every other source is read once: a part, a body
//! stream, a walk of chunks and a file that cannot seek. The first pass copies
//! it whole, under the request's memory limit, and the second reads the copy.
//!
//! # Where the callbacks run
//!
//! A callback runs inside the `advance()` that renders its block, so a
//! callback that throws ends the walk there, with its error. The walk's state
//! is taken out of the request's table for one `advance()`, because a callback
//! and a read both need the context it is filed in.
//!
//! **What it spends:** per walk in flight, what [`segments`] holds for one
//! segment and the definitions, the groups one segment wrote, and the
//! source's own buffer. A source that cannot be read again adds its whole
//! length.

use nvs_runtime::{Fault, Tag, Value};
use pulldown_cmark::Outside;

use super::callbacks::{Callables, given};
use super::segments::{self, Segments, not_text};
use super::{GROUPS, GROUPS_NAME, MARKUP};
use crate::byte_source::{ByteSource, READ_CHUNK};

/// The member every message of the walk names.
pub(super) const MEMBER: &str = r"Core\Markdown::toHtmlEach";

/// [`GROUPS`]' first slot: the key the walk is filed under with
/// `nvs_runtime::Ctx::hold_open_reader`, and `0` once the walk has ended.
const READER_AT: usize = 0;
/// [`GROUPS`]' second slot: the value [`ByteSource::from_arg`] gave, which
/// every read is given. The end of the walk clears it.
const SOURCE_AT: usize = 1;
/// [`GROUPS`]' third slot: the `Core\Html\Markup` the last `advance()`
/// wrote, `null` before the first and after the last.
const GROUP_AT: usize = 2;
/// [`GROUPS`]' fourth to seventh slots: the `link`, `image`, `heading` and
/// `code` callbacks, `null` where one was not given. The slots own them, so
/// the request's release frees them.
const CALLBACKS_AT: usize = 3;

/// Where the second pass reads the document from.
enum Input {
    /// The source itself, read again from its start.
    Source(ByteSource),
    /// The whole document, copied in the first pass, and how far the second
    /// has read it.
    Held { bytes: Vec<u8>, at: usize },
}

impl Input {
    /// Adds the next piece to `segments`, or ends them. Returns whether it
    /// added a piece.
    fn feed(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
        holder: Value,
        segments: &mut Segments,
    ) -> Result<bool, Fault> {
        let piece = match self {
            Self::Source(source) => match source.next_chunk(ctx, holder)? {
                Some(chunk) => {
                    segments
                        .push(chunk.bytes())
                        .map_err(|e| not_text(MEMBER, e))?;
                    return Ok(true);
                }
                None => None,
            },
            Self::Held { bytes, at } => {
                let start = *at;
                *at = start.saturating_add(READ_CHUNK).min(bytes.len());
                (start < bytes.len()).then(|| &bytes[start..*at])
            }
        };
        match piece {
            Some(piece) => {
                segments.push(piece).map_err(|e| not_text(MEMBER, e))?;
                Ok(true)
            }
            None => {
                segments.end().map_err(|e| not_text(MEMBER, e))?;
                Ok(false)
            }
        }
    }

    /// Starts the read again at the document's first byte.
    fn restart(&mut self) {
        match self {
            Self::Source(source) => {
                let restarted = source.restart();
                debug_assert!(restarted, "only a source that restarts is read twice");
            }
            Self::Held { at, .. } => *at = 0,
        }
    }

    /// The bytes it holds past the source's own buffer.
    #[cfg(test)]
    fn held(&self) -> usize {
        match self {
            Self::Source(_) => 0,
            Self::Held { bytes, .. } => bytes.capacity(),
        }
    }
}

/// The second pass: the input, the text read and not yet cut, and the
/// renderer's walk.
struct Rendering {
    input: Input,
    segments: Segments,
    walk: segments::Walk,
    /// Whether the last segment has been rendered and the walk finished.
    finished: bool,
}

impl Rendering {
    /// The next group, rendering segments until one is ready, or `None` once
    /// the document has ended.
    fn next(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
        holder: Value,
        callbacks: [Value; 4],
    ) -> Result<Option<String>, Fault> {
        loop {
            if let Some(group) = self.walk.pop() {
                return Ok(Some(group));
            }
            if self.finished {
                return Ok(None);
            }
            let segment = {
                let input = &mut self.input;
                let mut feed = |segments: &mut Segments| input.feed(ctx, holder, segments);
                segments::next_segment(&mut self.segments, &mut feed)?
            };
            // A segment's tree and its HTML are each about as long as its
            // text, so the request must afford both before it is rendered.
            let need = segment.as_ref().map_or(0, |s| s.len().saturating_mul(2));
            if !nvs_runtime::budget::affords(need) {
                return Err(over_memory(ctx));
            }
            let [link, image, heading, code] = callbacks.map(given);
            let mut hooks = Callables {
                ctx,
                link,
                image,
                heading,
                code,
            };
            match segment {
                Some(segment) => self.walk.segment(&segment, &mut hooks)?,
                None => {
                    self.walk.finish(&mut hooks)?;
                    self.finished = true;
                }
            }
        }
    }

    /// The bytes the walk holds between two `advance()` calls, past the
    /// source's own buffer and the renderer's open blocks.
    #[cfg(test)]
    fn held(&self) -> usize {
        self.input.held() + self.segments.held() + self.walk.held()
    }
}

/// The first pass over `source`: the definitions, and the input the second
/// pass reads.
fn first_pass(
    ctx: &mut nvs_runtime::Ctx,
    holder: Value,
    mut source: ByteSource,
) -> Result<Rendering, Fault> {
    let mut input = if source.can_restart() {
        Input::Source(source)
    } else {
        let mut bytes = Vec::new();
        while let Some(chunk) = source.next_chunk(ctx, holder)? {
            let piece = chunk.bytes();
            // Growing the copy can double it, so the request must afford the
            // larger of the two before it grows.
            if bytes.capacity() - bytes.len() < piece.len()
                && !nvs_runtime::budget::affords(bytes.len().max(piece.len()))
            {
                drop(chunk);
                return Err(over_memory(ctx));
            }
            bytes.extend_from_slice(piece);
        }
        Input::Held { bytes, at: 0 }
    };
    let (outside, len): (Outside, usize) = {
        let mut feed = |segments: &mut Segments| input.feed(ctx, holder, segments);
        segments::definitions(&mut feed)?
    };
    input.restart();
    Ok(Rendering {
        input,
        segments: Segments::default(),
        walk: segments::Walk::new(outside, len, MEMBER),
        finished: false,
    })
}

/// The fault for a document the request cannot afford.
fn over_memory(ctx: &mut nvs_runtime::Ctx) -> Fault {
    ctx.memory_breach().unwrap_or_else(|| {
        Fault::fatal(format!(
            "{MEMBER}: the document does not fit in the request's memory limit"
        ))
    })
}

/// Where the walk is between two `advance()` calls.
enum Pass {
    /// Nothing has been read yet.
    Unread(ByteSource),
    /// The first pass is done, and the second is rendering.
    Render(Box<Rendering>),
}

/// One walk's state, filed on the request. `None` only while an `advance()`
/// has taken it out.
struct Walk {
    pass: Option<Pass>,
}

impl std::fmt::Debug for Walk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pass = match &self.pass {
            None => "taken",
            Some(Pass::Unread(_)) => "unread",
            Some(Pass::Render(_)) => "rendering",
        };
        f.debug_struct("Walk").field("pass", &pass).finish()
    }
}

impl nvs_runtime::HeldReader for Walk {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The walk filed under `key`.
///
/// # Errors
///
/// A [`Fault::fatal`] where this task holds no such walk: a walk another
/// task returned.
fn walk_at(ctx: &mut nvs_runtime::Ctx, key: u64) -> Result<&mut Walk, Fault> {
    ctx.open_reader_mut(key)
        .and_then(|reader| reader.as_any_mut().downcast_mut::<Walk>())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "A `{GROUPS_NAME}` walk was read in a task other than the one that called \
                 `{MEMBER}`. A walk works only in that task."
            ))
        })
}

/// The next group of the walk filed under `key`.
///
/// # Errors
///
/// [`first_pass`]'s and [`Rendering::next`]'s: a read that fails, a document
/// that is not UTF-8, a callback that throws, and the memory limit.
fn next_group(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    holder: Value,
    callbacks: [Value; 4],
) -> Result<Option<String>, Fault> {
    let pass = walk_at(ctx, key)?.pass.take().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `{GROUPS_NAME}` walk lost its state"
        ))
    })?;
    let mut rendering = match pass {
        Pass::Unread(source) => Box::new(first_pass(ctx, holder, source)?),
        Pass::Render(rendering) => rendering,
    };
    let found = rendering.next(ctx, holder, callbacks);
    walk_at(ctx, key)?.pass = Some(Pass::Render(rendering));
    found
}

/// One `advance()`: the next group parked in [`GROUP_AT`], and whether there
/// was one. The end of the walk and a failure both give the walk up.
///
/// # Errors
///
/// [`next_group`]'s.
fn step(ctx: &mut nvs_runtime::Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = crate::instance::receiver(value, &GROUPS, member)?;
    let key = crate::instance::slot(receiver, READER_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{GROUPS_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                GROUPS.slots[READER_AT]
            ))
        })?;
    if key == 0 {
        crate::instance::set_slot(receiver, GROUP_AT, Value::null());
        return Ok(Value::bool(false));
    }
    let holder = crate::instance::slot(receiver, SOURCE_AT);
    let callbacks = std::array::from_fn(|at| crate::instance::slot(receiver, CALLBACKS_AT + at));
    match next_group(ctx, key, holder, callbacks) {
        Ok(Some(group)) => {
            let markup = crate::instance::build(&MARKUP, [Value::text(&group)]);
            crate::instance::set_slot(receiver, GROUP_AT, markup);
            Ok(Value::bool(true))
        }
        ended => {
            // Dropping the walk frees its text and closes its descriptor.
            drop(ctx.take_open_reader(key));
            crate::instance::set_slot(receiver, READER_AT, Value::uint(0));
            crate::instance::set_slot(receiver, GROUP_AT, Value::null());
            crate::instance::set_slot(receiver, SOURCE_AT, Value::null());
            for at in 0..4 {
                crate::instance::set_slot(receiver, CALLBACKS_AT + at, Value::null());
            }
            ended.map(|_| Value::bool(false))
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Markdown::toHtmlEach(Core\Bytes::Source|Core\Bytes::Chunks
    /// $markdown, ?callable $link, ?callable $image, ?callable $heading,
    /// ?callable $code): Core\Markdown\Groups` —
    /// `rule:core-classes/markdown-chunks`' walk.
    ///
    /// **Nothing is read here.** The source is taken, the walk is filed on
    /// the request, and the first read is the first `advance()`, as
    /// `Core\Json::decodeEach`'s is.
    ///
    /// # Errors
    ///
    /// [`ByteSource::from_arg`]'s, such as the `RuntimeError` for a file that
    /// is already closed.
    fn nvs_core_markdown_to_html_each(ctx, args: [5]) {
        let (source, holder) = ByteSource::from_arg(ctx, args[0], MEMBER)?;
        let key = ctx.hold_open_reader(Box::new(Walk {
            pass: Some(Pass::Unread(source)),
        }));
        let callbacks: [Value; 4] = std::array::from_fn(|at| args[1 + at]);
        for callback in callbacks {
            #[expect(
                unsafe_code,
                reason = "the arguments are borrowed, and the walk's slots keep a \
                          reference of their own to each callback"
            )]
            unsafe {
                callback.retain();
            }
        }
        let [link, image, heading, code] = callbacks;
        Ok(crate::instance::build(
            &GROUPS,
            [Value::uint(key), holder, Value::null(), link, image, heading, code],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<Core\Html\Markup>::iterate()` on a `Core\Markdown\Groups`:
    /// the walk itself, for `Core\Csv\Rows`' reason.
    fn nvs_core_markdown_groups_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &GROUPS, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Core\Html\Markup>::advance()` on a `Core\Markdown\Groups`:
    /// writes the next block group, and gives the walk up at the end.
    fn nvs_core_markdown_groups_advance(ctx, args: [1]) {
        let stepped = step(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Core\Html\Markup>::current()` on a `Core\Markdown\Groups`:
    /// the group the last `advance()` wrote, and the only one the walk holds.
    fn nvs_core_markdown_groups_current(_ctx, args: [1]) {
        let read = crate::instance::receiver(args[0], &GROUPS, nvs_runtime::sequence::CURRENT)
            .map(|receiver| {
                let held = crate::instance::slot(receiver, GROUP_AT);
                #[expect(
                    unsafe_code,
                    reason = "the slot keeps its reference until the next `advance`, so \
                              the value returned needs one of its own"
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

/// The walk's state between two `advance()` calls: the bytes it holds, and
/// whether it reads its source again.
#[cfg(test)]
pub(super) fn held_by(ctx: &mut nvs_runtime::Ctx, groups: Value) -> Option<(usize, bool)> {
    let receiver = crate::instance::receiver(groups, &GROUPS, "a test").ok()?;
    let key = crate::instance::slot(receiver, READER_AT).as_uint()?;
    match walk_at(ctx, key).ok()?.pass.as_ref()? {
        Pass::Unread(_) => Some((0, true)),
        Pass::Render(rendering) => Some((
            rendering.held(),
            matches!(rendering.input, Input::Source(_)),
        )),
    }
}

/// One `advance()` of `groups`, and the group it wrote.
#[cfg(test)]
pub(super) fn advanced(ctx: &mut nvs_runtime::Ctx, groups: Value) -> Result<Option<String>, Fault> {
    if !step(ctx, groups)?.as_bool().unwrap_or(false) {
        return Ok(None);
    }
    let receiver = crate::instance::receiver(groups, &GROUPS, "a test")?;
    let group = crate::instance::slot(receiver, GROUP_AT);
    let markup = crate::instance::receiver(group, &MARKUP, "a test")?;
    Ok(crate::instance::slot(markup, 0)
        .as_text()
        .map(str::to_owned))
}
