//! `rule:core-classes/markdown-chunks`'s segments: a document read in pieces,
//! cut into runs of whole top-level blocks that the engine reads one at a
//! time, as it would read them inside the whole document.
//!
//! # Where a segment is cut
//!
//! [`Segments`] keeps the text read and not yet given out. It reads the
//! complete lines of it with the engine's first pass alone
//! ([`pulldown_cmark::Parser::block_ranges`]), and cuts after the
//! second-to-last top-level block, at the next line start. The last block may
//! still grow, and so may a reference definition after the block before it,
//! whose title can go on over the next lines, so both stay in the next
//! segment. A line is read only once it is complete, because its last bytes
//! can change what block it starts.
//!
//! **The text is read again only once it has doubled** since the last read
//! that did not cut. So one block as large as the document costs work linear
//! in its length, and memory of one read past it.
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
}

/// Where the bytes stop being UTF-8, counted from the document's start.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct NotText(pub(super) usize);

impl Segments {
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
    pub(super) fn held(&self) -> usize {
        self.buffer.capacity() + self.partial.capacity()
    }

    /// The next segment, or `None` when more text is needed or, once the
    /// document has ended, when every segment has been given out.
    pub(super) fn next(&mut self) -> Option<String> {
        if self.ended {
            return (!self.buffer.is_empty()).then(|| std::mem::take(&mut self.buffer));
        }
        if self.buffer.len() < self.parsed_at.saturating_mul(2).max(1) {
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
    fn cut(&self) -> Option<usize> {
        let complete = self.buffer.rfind('\n')? + 1;
        let lines = &self.buffer[..complete];
        let blocks = Parser::new_ext(lines, options()).block_ranges();
        let [.., before, last] = &blocks[..] else {
            return None;
        };
        let end = before.end;
        let line_start = if end == 0 || lines.as_bytes()[end - 1] == b'\n' {
            end
        } else {
            lines[end..].find('\n').map_or(complete, |at| end + at + 1)
        };
        let cut = line_start.min(last.start);
        (cut > 0).then_some(cut)
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
}

impl Walk {
    /// A walk over a document of `len` prepared bytes, whose definitions
    /// are `outside`.
    pub(super) fn new(outside: Outside, len: usize, member: &'static str) -> Self {
        Self {
            state: Some(State::new(len.saturating_add(ADDED_OUTPUT_SLACK))),
            outside,
            written: Written::default(),
            ready: VecDeque::new(),
            member,
        }
    }

    /// The next group written and not yet given out.
    pub(super) fn pop(&mut self) -> Option<String> {
        self.ready.pop_front()
    }

    /// The bytes it holds past the renderer's tree: the definitions and the
    /// groups not yet given out.
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
        let state = self.state.take().ok_or_else(ended)?;
        let mut renderer = Renderer {
            text: segment,
            hooks,
            state,
        };
        let parser = Parser::new_with_outside(segment, options(), &self.outside);
        for (event, range) in parser.into_offset_iter() {
            renderer.event(event, range);
            if let Some(fault) = renderer.fault.take() {
                return Err(fault);
            }
            if renderer.group_ends() {
                let (tree, rebuild, trusted) = renderer.take_group();
                let html = self.written.group(&tree, rebuild, trusted, self.member)?;
                self.ready.push_back(html);
            }
        }
        self.state = Some(renderer.state);
        Ok(())
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
