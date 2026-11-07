//! `rule:core-classes/markdown-chunks`'s segments: a document read in pieces,
//! cut into runs of whole top-level blocks that the engine reads one at a
//! time, as it would read them inside the whole document.
//!
//! # Where a segment is cut
//!
//! [`Segments`] keeps the text read and not yet given out. It reads the
//! complete lines of it with the engine's first pass alone
//! ([`pulldown_cmark::Parser::blocks`]), and cuts after the
//! second-to-last top-level block, at the next line start. The last block may
//! still grow, and so may a reference definition after the block before it,
//! whose title can go on over the next lines, so both stay in the next
//! segment. A line is read only once it is complete, because its last bytes
//! can change what block it starts. A cut is kept only where the text after
//! it, read alone, has the blocks it has in the whole, which costs a second
//! parse of that text for each of at most two cuts tried.
//!
//! **The text is read again only once it has doubled** since the last read
//! that did not cut. So one block as large as the document costs work linear
//! in its length, and memory of one read past it.
//!
//! **`Core\Markdown\Live` reads it at every line end** instead
//! ([`Segments::live`]), so a block is given out as soon as the next one
//! starts. Each piece that ends a line costs a parse of the text not yet
//! given out, which is the same order of work as rendering that text as the
//! open block, which Live does after every piece anyway.
//!
//! # Why a segment reads as the whole document does
//!
//! A top-level block's parse depends only on its own lines, once the block
//! before it has ended. What a segment cannot see is the rest of the
//! document's definitions: a reference defined in another segment, a
//! footnote label defined there, and the expansion fuel the whole document
//! shares. [`pulldown_cmark::Outside`] holds all three, read in a first pass
//! over the same segments, and each segment's parser reads it first.
//!
//! **What it spends:** the text read and not yet given out, plus an
//! unfinished UTF-8 sequence carried between reads. Because the text doubles
//! before it is read again, and a string's capacity can be twice its length,
//! that is at most four reads past the last top-level block. The first pass
//! keeps every definition ([`pulldown_cmark::Outside::bytes`]).

use std::collections::VecDeque;

use nvs_runtime::Fault;
use pulldown_cmark::{Outside, Parser};

use super::callbacks::Callbacks;
use super::groups::Written;
use super::{ADDED_OUTPUT_SLACK, Renderer, State, options};

/// How many cuts [`Segments`] tries from the end of the text it read, each
/// with a second parse of the text after it.
const TRIED_CUTS: usize = 2;

/// One read of the document: it adds the next bytes to `segments`, or ends
/// them, and returns whether it added any.
pub(super) type Feed<'f> = dyn FnMut(&mut Segments) -> Result<bool, Fault> + 'f;

/// A document's text read in pieces, given out as segments of whole
/// top-level blocks.
#[derive(Debug, Default)]
pub(super) struct Segments {
    /// The prepared text read and not yet given out.
    buffer: String,
    /// The buffer's length at the last read that did not cut, which the
    /// buffer must double before it is read again.
    parsed_at: usize,
    /// Bytes of a UTF-8 sequence a piece ended inside.
    partial: Vec<u8>,
    /// Whether the last piece ended with a carriage return, which is a line
    /// end whether or not a line feed follows.
    cr: bool,
    /// Whether text has been read, so a byte-order mark is no longer at the
    /// start.
    started: bool,
    /// The bytes read so far, to name where invalid UTF-8 is.
    offset: usize,
    /// Whether the last piece has been read.
    ended: bool,
    /// Whether it reads the text at every new line end, not once it has
    /// doubled.
    eager: bool,
}

/// Where the bytes stop being UTF-8, counted from the document's start.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct NotText(pub(super) usize);

impl Segments {
    /// Segments that cut at every line end that completes a block, for text
    /// a program shows as it arrives.
    pub(super) fn live() -> Self {
        Self {
            eager: true,
            ..Self::default()
        }
    }

    /// The text read and not yet given out, its last line perhaps not
    /// complete.
    pub(super) fn unread(&self) -> &str {
        &self.buffer
    }

    /// Adds `piece`, the next bytes of the document.
    ///
    /// # Errors
    ///
    /// [`NotText`] when the bytes so far are not UTF-8.
    pub(super) fn push(&mut self, piece: &[u8]) -> Result<(), NotText> {
        let joined;
        let bytes = if self.partial.is_empty() {
            piece
        } else {
            let mut held = std::mem::take(&mut self.partial);
            held.extend_from_slice(piece);
            joined = held;
            &joined[..]
        };
        let start = self.offset - (bytes.len() - piece.len());
        self.offset += piece.len();
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) if error.error_len().is_none() => {
                let valid = error.valid_up_to();
                self.partial = bytes[valid..].to_vec();
                std::str::from_utf8(&bytes[..valid]).expect("the valid prefix")
            }
            Err(error) => return Err(NotText(start + error.valid_up_to())),
        };
        self.append(text);
        Ok(())
    }

    /// Ends the document.
    ///
    /// # Errors
    ///
    /// [`NotText`] when it ends inside a UTF-8 sequence.
    pub(super) fn end(&mut self) -> Result<(), NotText> {
        self.ended = true;
        if self.partial.is_empty() {
            Ok(())
        } else {
            Err(NotText(self.offset - self.partial.len()))
        }
    }

    /// `text` prepared as `super::prepared` prepares a whole document, and
    /// added to the buffer.
    fn append(&mut self, mut text: &str) {
        if !self.started && !text.is_empty() {
            self.started = true;
            text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
        }
        self.buffer.reserve(text.len());
        for ch in text.chars() {
            let cr = std::mem::replace(&mut self.cr, false);
            match ch {
                '\n' if cr => {}
                '\0' => self.buffer.push('\u{FFFD}'),
                '\r' => {
                    self.cr = true;
                    self.buffer.push('\n');
                }
                _ => self.buffer.push(ch),
            }
        }
    }

    /// How many bytes it holds.
    #[cfg(test)]
    pub(super) fn held(&self) -> usize {
        self.buffer.capacity() + self.partial.capacity()
    }

    /// How many bytes of text it has read and not yet cut.
    pub(super) fn len(&self) -> usize {
        self.buffer.len() + self.partial.len()
    }

    /// The next segment, or `None` when more text is needed or, once the
    /// document has ended, when every segment has been given out.
    pub(super) fn next(&mut self) -> Option<String> {
        if self.ended {
            return (!self.buffer.is_empty()).then(|| std::mem::take(&mut self.buffer));
        }
        let due = if self.eager {
            // A line completed since the last read.
            let read = self.parsed_at.min(self.buffer.len());
            self.buffer.as_bytes()[read..].contains(&b'\n')
        } else {
            self.buffer.len() >= self.parsed_at.saturating_mul(2).max(1)
        };
        if !due {
            return None;
        }
        let Some(cut) = self.cut() else {
            self.parsed_at = self.buffer.len();
            return None;
        };
        let rest = self.buffer.split_off(cut);
        self.parsed_at = rest.len();
        Some(std::mem::replace(&mut self.buffer, rest))
    }

    /// Where the buffer's complete lines can be cut: after the second-to-last
    /// top-level block they hold, at the next line start.
    ///
    /// A cut is kept only where the text after it, read alone, has the same
    /// blocks as it has in the whole. A block's range can run over a reference
    /// definition that starts the paragraph after it: in `*\n[a]: /u\n*`, the
    /// list's range ends at the second `*`, which continues that paragraph,
    /// and read alone it is a list. Such a cut moves to the pair before. Only
    /// [`TRIED_CUTS`] pairs are tried, so the work stays linear; when none
    /// holds, the text is read again once it has grown.
    fn cut(&self) -> Option<usize> {
        let complete = self.buffer.rfind('\n')? + 1;
        let lines = &self.buffer[..complete];
        let at_line_start = |at: usize| at == 0 || lines.as_bytes()[at - 1] == b'\n';
        let blocks = Parser::new_ext(lines, options()).blocks();
        let pairs = blocks.len().saturating_sub(1);
        (pairs.saturating_sub(TRIED_CUTS)..pairs)
            .rev()
            .find_map(|at| {
                let (before, after) = (&blocks[at].0, &blocks[at + 1..]);
                let end = before.end;
                let line_start = if at_line_start(end) {
                    end
                } else {
                    lines[end..].find('\n').map_or(complete, |at| end + at + 1)
                };
                let cut = line_start.min(after[0].0.start);
                if cut == 0 || !at_line_start(cut) {
                    return None;
                }
                let alone = Parser::new_ext(&lines[cut..], options()).blocks();
                let same = alone.len() == after.len()
                    && alone.iter().zip(after).all(|((read, kind), (whole, its))| {
                        kind == its
                            && read.start + cut == whole.start
                            && read.end + cut == whole.end
                    });
                same.then_some(cut)
            })
    }
}

/// The next segment `feed` gives, reading as many pieces as it takes, or
/// `None` once the document has ended.
///
/// # Errors
///
/// `feed`'s.
pub(super) fn next_segment(
    segments: &mut Segments,
    feed: &mut Feed<'_>,
) -> Result<Option<String>, Fault> {
    loop {
        if let Some(segment) = segments.next() {
            return Ok(Some(segment));
        }
        if segments.ended {
            return Ok(None);
        }
        if !feed(segments)? {
            // The feed ended the document, so the rest is the last segment.
            return Ok(segments.next());
        }
    }
}

/// The first pass: every definition of the document `feed` gives, read from
/// each segment's first pass alone, with the fuel the whole document allows.
///
/// # Errors
///
/// `feed`'s.
pub(super) fn definitions(feed: &mut Feed<'_>) -> Result<(Outside, usize), Fault> {
    let mut segments = Segments::default();
    let mut outside = Outside::default();
    let mut len = 0usize;
    while let Some(segment) = next_segment(&mut segments, feed)? {
        len = len.saturating_add(segment.len());
        outside.add(&Parser::new_ext(&segment, options()));
    }
    outside.allow(len);
    Ok((outside, len))
}

/// The second pass: the renderer's state between segments, and the groups
/// it has written and not yet given out.
pub(super) struct Walk {
    /// `None` only once a callback has failed, which ends the walk.
    state: Option<State>,
    outside: Outside,
    written: Written,
    ready: VecDeque<String>,
    member: &'static str,
    /// For a walk over text that arrives ([`Self::live`]), the text of the
    /// group rendered and not yet ended, which [`Self::open`] renders again.
    /// `None` for a walk over a whole source.
    pending: Option<String>,
}

impl Walk {
    /// A walk over text that arrives in pieces, whose length is not known.
    /// Each segment adds its own length to the budget and the fuel
    /// ([`Self::segment`]), and its definitions once it is rendered.
    pub(super) fn live(member: &'static str) -> Self {
        let mut outside = Outside::default();
        outside.allow(0);
        Self {
            state: Some(State::new(ADDED_OUTPUT_SLACK)),
            outside,
            written: Written::default(),
            ready: VecDeque::new(),
            member,
            pending: Some(String::new()),
        }
    }

    /// A walk over a document of `len` prepared bytes, whose definitions
    /// are `outside`.
    pub(super) fn new(outside: Outside, len: usize, member: &'static str) -> Self {
        Self {
            state: Some(State::new(len.saturating_add(ADDED_OUTPUT_SLACK))),
            outside,
            written: Written::default(),
            ready: VecDeque::new(),
            member,
            pending: None,
        }
    }

    /// The next group written and not yet given out.
    pub(super) fn pop(&mut self) -> Option<String> {
        self.ready.pop_front()
    }

    /// The bytes it holds past the renderer's tree: the definitions and the
    /// groups not yet given out.
    #[cfg(test)]
    pub(super) fn held(&self) -> usize {
        self.outside.bytes() + self.ready.iter().map(String::capacity).sum::<usize>()
    }

    /// Renders `segment`, keeping each group that ends in it.
    ///
    /// # Errors
    ///
    /// The first failing callback's fault, and `Written::group`'s.
    pub(super) fn segment(
        &mut self,
        segment: &str,
        hooks: &mut dyn Callbacks,
    ) -> Result<(), Fault> {
        let mut state = self.state.take().ok_or_else(ended)?;
        if self.pending.is_some() {
            state.budget = state.budget.saturating_add(segment.len());
            self.outside
                .set_fuel(self.outside.fuel().saturating_add(segment.len()));
        }
        let mut renderer = Renderer {
            text: segment,
            hooks,
            state,
        };
        // Where the last group that ended in this segment ends.
        let mut ended_at = None;
        let parser = Parser::new_with_outside(segment, options(), &self.outside);
        for (event, range) in parser.into_offset_iter() {
            let end = range.end;
            renderer.event(event, range);
            if let Some(fault) = renderer.fault.take() {
                return Err(fault);
            }
            if renderer.group_ends() {
                let (tree, rebuild, trusted) = renderer.take_group();
                let html = self.written.group(&tree, rebuild, trusted, self.member)?;
                self.ready.push_back(html);
                ended_at = Some(end);
            }
        }
        let open = renderer.stack.len() > 1 || !renderer.stack[0].children.is_empty();
        self.state = Some(renderer.state);
        if let Some(pending) = &mut self.pending {
            if ended_at.is_some() || !open {
                pending.clear();
            }
            if open {
                let from = ended_at.unwrap_or(0).min(segment.len());
                pending.push_str(segment.get(from..).unwrap_or(segment));
            }
            // A later segment reads this one's definitions. One already
            // defined keeps its first definition, as in the whole document.
            self.outside.add(&Parser::new_ext(segment, options()));
        }
        Ok(())
    }

    /// The HTML of `rest`, the text after the last segment, as it would be
    /// if the document ended there. The group rendered and not yet ended is
    /// rendered again in front of it. Nothing the walk holds changes, so the
    /// next segment renders as if this had not run.
    ///
    /// # Errors
    ///
    /// The first failing callback's fault, and `Written::group`'s.
    pub(super) fn open(&self, rest: &str, hooks: &mut dyn Callbacks) -> Result<String, Fault> {
        let walked = self.state.as_ref().ok_or_else(ended)?;
        let pending = self.pending.as_deref().unwrap_or("");
        if pending.is_empty() && rest.trim().is_empty() {
            return Ok(String::new());
        }
        let text = format!("{pending}{rest}");
        let mut state = State::new(walked.budget.saturating_add(text.len()));
        if text.contains("[^") {
            // A footnote referenced before keeps its number.
            state.numbers.clone_from(&walked.numbers);
            state.order.clone_from(&walked.order);
        }
        let fuel = self.outside.fuel();
        self.outside.set_fuel(fuel.saturating_add(text.len()));
        let mut renderer = Renderer {
            text: &text,
            hooks,
            state,
        };
        let mut failed = None;
        for (event, range) in
            Parser::new_with_outside(&text, options(), &self.outside).into_offset_iter()
        {
            renderer.event(event, range);
            if let Some(fault) = renderer.fault.take() {
                failed = Some(fault);
                break;
            }
        }
        self.outside.set_fuel(fuel);
        if let Some(fault) = failed {
            return Err(fault);
        }
        let (tree, rebuild, trusted) = renderer.unfinished();
        self.written
            .clone()
            .group(&tree, rebuild, trusted, self.member)
    }

    /// Ends the document: the last group and the footnotes.
    ///
    /// # Errors
    ///
    /// `Written::group`'s.
    pub(super) fn finish(&mut self, hooks: &mut dyn Callbacks) -> Result<(), Fault> {
        let state = self.state.take().ok_or_else(ended)?;
        let renderer = Renderer {
            text: "",
            hooks,
            state,
        };
        let (tree, rebuild, trusted) = renderer.finish();
        if !tree.children.is_empty() {
            let html = self.written.group(&tree, rebuild, trusted, self.member)?;
            self.ready.push_back(html);
        }
        Ok(())
    }
}

/// The fault of a walk read after it ended. Unreachable from source: a walk
/// whose callback failed is not read again.
fn ended() -> Fault {
    Fault::fatal("a Markdown walk was read after it ended".to_owned())
}

/// A feed over `bytes`, `piece` bytes at a time, from its start.
pub(super) fn feed_bytes<'b>(
    bytes: &'b [u8],
    piece: usize,
    member: &'b str,
) -> impl FnMut(&mut Segments) -> Result<bool, Fault> + 'b {
    let mut at = 0;
    move |segments| {
        if at >= bytes.len() {
            segments.end().map_err(|e| not_text(member, e))?;
            return Ok(false);
        }
        let end = at.saturating_add(piece.max(1)).min(bytes.len());
        segments
            .push(&bytes[at..end])
            .map_err(|e| not_text(member, e))?;
        at = end;
        Ok(true)
    }
}

/// The catchable `ParseError` for a document that is not UTF-8, naming the
/// offset of the first wrong byte.
pub(super) fn not_text(member: &str, NotText(offset): NotText) -> Fault {
    Fault::thrown_as(
        nvs_runtime::ThrownClass::Parse,
        format!("{member}: byte {offset} of the document is not valid UTF-8"),
    )
}

/// `markdown` read `piece` bytes at a time and written one block group at a
/// time, in both passes.
///
/// # Errors
///
/// [`Walk::segment`]'s.
pub(super) fn grouped_in(
    markdown: &[u8],
    piece: usize,
    member: &'static str,
    hooks: &mut dyn Callbacks,
) -> Result<Vec<String>, Fault> {
    let (outside, len) = definitions(&mut feed_bytes(markdown, piece, member))?;
    let mut walk = Walk::new(outside, len, member);
    let mut segments = Segments::default();
    let mut feed = feed_bytes(markdown, piece, member);
    let mut out = Vec::new();
    while let Some(segment) = next_segment(&mut segments, &mut feed)? {
        walk.segment(&segment, hooks)?;
        out.extend(std::iter::from_fn(|| walk.pop()));
    }
    walk.finish(hooks)?;
    out.extend(std::iter::from_fn(|| walk.pop()));
    Ok(out)
}
