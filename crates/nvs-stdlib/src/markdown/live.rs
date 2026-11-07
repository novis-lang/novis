//! `Core\Markdown\Live`: `rule:core-classes/markdown-chunks`' renderer for
//! text that arrives in pieces, such as a language model's answer.
//!
//! # What a piece gives
//!
//! [`Text`] keeps [`Segments::live`], which cuts a segment at every line end
//! that completes a block, and the second pass's [`segments::Walk`] with no
//! first pass before it. Each `push` gives two parts:
//!
//! - **done**: the block groups that ended since the last `push`. They are
//!   final, because nothing that arrives later is rendered into them: the
//!   walk learns a segment's definitions only after it renders it. A
//!   reference defined after a finished block does not change that block.
//! - **open**: the text not yet in a finished group, rendered as if the
//!   document ended there ([`segments::Walk::open`]). It changes nothing the
//!   walk holds, and the next `push` gives it again with what was added.
//!
//! `finish()` ends the text and gives the last groups and the footnotes. A
//! document with no reference used before its definition gives, as its done
//! parts and `finish()` joined, the HTML `toHtml` gives for the whole text.
//!
//! # Where the callbacks run
//!
//! A callback runs when its block is rendered, so a block in the open part
//! runs its callbacks again at every `push` until it is finished. A callback
//! that throws ends the object with its error.
//!
//! **What it spends:** per object in flight, the text not yet in a finished
//! group, the definitions, and the footnotes until `finish()`. Each `push`
//! that ends a line parses that text, and each `push` renders it once more, so
//! one block pushed in many pieces costs work in the square of its length.
//! That is the price of showing the open block after every piece. Every byte
//! is attributed to the request.

use nvs_runtime::{Fault, Tag, Value};

use super::callbacks::{Callables, Callbacks, given};
use super::segments::{self, Segments};
use super::{LIVE, LIVE_NAME, MARKUP, UPDATE};

/// The member every message of the object names.
const MEMBER: &str = r"Core\Markdown\Live";

/// [`LIVE`]'s first slot: the key the object's state is filed under with
/// `nvs_runtime::Ctx::hold_open_reader`, and `0` once it has ended.
const READER_AT: usize = 0;
/// [`LIVE`]'s second to fifth slots: the `link`, `image`, `heading` and
/// `code` callbacks, `null` where one was not given.
const CALLBACKS_AT: usize = 1;

/// The text of one object: what has arrived and not yet been cut, and the
/// walk that renders each segment.
pub(super) struct Text {
    segments: Segments,
    walk: segments::Walk,
}

impl Text {
    /// A document with no text yet.
    pub(super) fn new() -> Self {
        Self {
            segments: Segments::live(),
            walk: segments::Walk::live(MEMBER),
        }
    }

    /// The bytes of text it holds that a `push` renders again.
    fn unread(&self) -> usize {
        self.segments.unread().len()
    }

    /// Adds `piece`, and returns the groups it finished, joined, and the HTML
    /// of the rest.
    ///
    /// # Errors
    ///
    /// The first failing callback's fault, and the rebuild's memory fault.
    pub(super) fn push(
        &mut self,
        piece: &str,
        hooks: &mut dyn Callbacks,
    ) -> Result<(String, String), Fault> {
        self.segments
            .push(piece.as_bytes())
            .map_err(|e| segments::not_text(MEMBER, e))?;
        while let Some(segment) = self.segments.next() {
            self.walk.segment(&segment, hooks)?;
        }
        let done = self.done();
        let open = self.walk.open(self.segments.unread(), hooks)?;
        Ok((done, open))
    }

    /// Ends the text, and returns the groups that were still open and the
    /// footnotes, joined.
    ///
    /// # Errors
    ///
    /// [`Self::push`]'s.
    pub(super) fn finish(&mut self, hooks: &mut dyn Callbacks) -> Result<String, Fault> {
        self.segments
            .end()
            .map_err(|e| segments::not_text(MEMBER, e))?;
        while let Some(segment) = self.segments.next() {
            self.walk.segment(&segment, hooks)?;
        }
        self.walk.finish(hooks)?;
        Ok(self.done())
    }

    /// Every group written and not yet given out, joined.
    fn done(&mut self) -> String {
        std::iter::from_fn(|| self.walk.pop()).collect()
    }
}

/// One object's state, filed on the request. `None` only while a `push` or
/// `finish()` has taken it out.
struct Held {
    text: Option<Box<Text>>,
}

impl std::fmt::Debug for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Held")
            .field("taken", &self.text.is_none())
            .finish()
    }
}

impl nvs_runtime::HeldReader for Held {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The state filed under `key`.
///
/// # Errors
///
/// A [`Fault::fatal`] where this task holds no such state: an object another
/// task returned.
fn held_at(ctx: &mut nvs_runtime::Ctx, key: u64) -> Result<&mut Held, Fault> {
    ctx.open_reader_mut(key)
        .and_then(|reader| reader.as_any_mut().downcast_mut::<Held>())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "A `{LIVE_NAME}` was used in a task other than the one that created it. It \
                 works only in that task."
            ))
        })
}

/// Runs `work` over the state of the object `value`, with its callbacks.
/// `finishing` ends the object afterwards, and so does any error.
///
/// # Errors
///
/// The `RuntimeError` for an object that has ended, the memory limit before
/// `needed` more bytes of text are rendered, and `work`'s.
fn with_text<T>(
    ctx: &mut nvs_runtime::Ctx,
    value: Value,
    member: &str,
    needed: usize,
    finishing: bool,
    work: impl FnOnce(&mut Text, &mut Callables<'_>) -> Result<T, Fault>,
) -> Result<T, Fault> {
    let receiver = crate::instance::receiver(value, &LIVE, member)?;
    let key = crate::instance::slot(receiver, READER_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{LIVE_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                LIVE.slots[READER_AT]
            ))
        })?;
    if key == 0 {
        return Err(Fault::thrown(format!(
            "{LIVE_NAME}::{member}(): this document has ended, because `finish()` was called \
             or a callback threw an error. Create a new one with `Core\\Markdown::live()`."
        )));
    }
    let callbacks: [Value; 4] =
        std::array::from_fn(|at| crate::instance::slot(receiver, CALLBACKS_AT + at));
    let mut text = held_at(ctx, key)?.text.take().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `{LIVE_NAME}` lost its state"
        ))
    })?;
    // The tree and the HTML of the text rendered are each about as long as
    // the text, so the request must afford both first.
    let need = text.unread().saturating_add(needed).saturating_mul(2);
    let result = if nvs_runtime::budget::affords(need) {
        let [link, image, heading, code] = callbacks.map(given);
        let mut hooks = Callables {
            ctx,
            link,
            image,
            heading,
            code,
        };
        work(&mut text, &mut hooks)
    } else {
        Err(ctx.memory_breach().unwrap_or_else(|| {
            Fault::fatal(format!(
                "{LIVE_NAME}::{member}: the text does not fit in the request's memory limit"
            ))
        }))
    };
    if finishing || result.is_err() {
        // Dropping the state frees its text and definitions.
        drop(ctx.take_open_reader(key));
        crate::instance::set_slot(receiver, READER_AT, Value::uint(0));
        for at in 0..4 {
            crate::instance::set_slot(receiver, CALLBACKS_AT + at, Value::null());
        }
    } else {
        held_at(ctx, key)?.text = Some(text);
    }
    result
}

nvs_runtime::nvs_helper! {
    /// `Core\Markdown::live(?callable $link, ?callable $image, ?callable
    /// $heading, ?callable $code): Core\Markdown\Live` —
    /// `rule:core-classes/markdown-chunks`' renderer for text that arrives.
    /// The state is filed on the request, and the object's slots keep the
    /// callbacks.
    fn nvs_core_markdown_live(ctx, args: [4]) {
        let key = ctx.hold_open_reader(Box::new(Held {
            text: Some(Box::new(Text::new())),
        }));
        let callbacks: [Value; 4] = std::array::from_fn(|at| args[at]);
        for callback in callbacks {
            #[expect(
                unsafe_code,
                reason = "the arguments are borrowed, and the object's slots keep a \
                          reference of their own to each callback"
            )]
            unsafe {
                callback.retain();
            }
        }
        let [link, image, heading, code] = callbacks;
        Ok(crate::instance::build(
            &LIVE,
            [Value::uint(key), link, image, heading, code],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$live->push(tainted string $piece): Core\Markdown\Update` — adds the
    /// next piece, and returns the groups it finished and the open rest.
    fn nvs_core_markdown_live_push(ctx, args: [2]) {
        // unreachable from source: the parameter is `CoreTy::Text`.
        let Some(piece) = args[1].as_text() else {
            return Err(Fault::fatal(format!(
                "{LIVE_NAME}::push expected a `string`, got tag {}",
                args[1].tag_byte()
            )));
        };
        let (done, open) = with_text(ctx, args[0], "push", piece.len(), false, |text, hooks| {
            text.push(piece, hooks)
        })?;
        Ok(crate::instance::build(
            &UPDATE,
            [
                crate::instance::build(&MARKUP, [Value::text(&done)]),
                crate::instance::build(&MARKUP, [Value::text(&open)]),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$live->finish(): Core\Html\Markup` — ends the text, and returns the
    /// groups that were open and the footnotes. The object ends with it.
    fn nvs_core_markdown_live_finish(ctx, args: [1]) {
        let rest = with_text(ctx, args[0], "finish", 0, true, |text, hooks| text.finish(hooks))?;
        Ok(crate::instance::build(&MARKUP, [Value::text(&rest)]))
    }
}

/// The `Markup` in [`UPDATE`]'s slot `at`, with a reference for the caller.
///
/// # Errors
///
/// `crate::instance::receiver`'s.
fn update_slot(value: Value, at: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, &UPDATE, member)?;
    let held = crate::instance::slot(receiver, at);
    #[expect(
        unsafe_code,
        reason = "the slot keeps its own reference, so the value returned needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `$update->done(): Core\Html\Markup` — the groups the `push` finished.
    fn nvs_core_markdown_update_done(_ctx, args: [1]) {
        update_slot(args[0], 0, "done")
    }
}

nvs_runtime::nvs_helper! {
    /// `$update->open(): Core\Html\Markup` — the rest, rendered as if the
    /// text ended there.
    fn nvs_core_markdown_update_open(_ctx, args: [1]) {
        update_slot(args[0], 1, "open")
    }
}
