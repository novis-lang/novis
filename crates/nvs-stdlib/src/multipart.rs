//! `rule:http-server/an-upload-is-received-only-through-files` and `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`
//! 's multipart body: the parse that turns what arrived on the wire into
//! the parts `Core\Request::files()` yields and the form fields
//! `Core\Request::post()` reads.
//!
//! It pulls the same [`nvs_runtime::RequestBody`] `Core\Request::bodyStream`
//! walks, so nothing here accumulates a body: a file part's bytes are handed out
//! as spans of a buffer that holds one wire chunk plus, at most, one delimiter's
//! worth of held-back tail. § 1's laziness is that property rather than a
//! promise — there is no shape in this module that could hold a whole upload.
//!
//! **It takes the body per call rather than borrowing it once.** The parse
//! outlives any one call into `Core`, while the body lives on the request's
//! [`nvs_runtime::Inbound`] and is re-borrowed out of the context each time; the
//! shape is `crate::request`'s `body_stream_step`, one layer up. A parser
//! holding `&mut dyn RequestBody` would have to be stored beside the very thing
//! it borrows.
//!
//! # Three bounds, and the one that is not here
//!
//! - [`MAX_PARTS`] — `rule:errors/multipart-part-count`'s part count, which bounds *bookkeeping*: a body far inside every byte
//!   cap can still hold a million parts.
//! - [`PART_HEADERS`] — one part's header block, so a part that never ends its
//!   headers cannot grow the buffer.
//! - [`REQUEST_BODY`] — § 2's buffered form fields, which are bytes parsed into
//!   memory and are charged exactly as `body()`'s are.
//!
//! **`upload_total` is not one of them.** `rule:http-server/request-body-and-upload-total-are-two-caps` puts that cap on the
//! wire, in `nvs_server::body`, which is where a body is refused before dispatch
//! and where a chunked one is stopped mid-stream. Charging it a second time here
//! would be a bound this layer cannot enforce for the bytes it never sees — the
//! parts it drains are still the server's to count.
//!
//! **What it spends:** one buffer, holding one wire chunk plus at most one
//! delimiter's worth of tail, plus one part's header block while it is being
//! read, plus § 2's buffered fields under [`REQUEST_BODY`]. All of it is
//! O(in-flight) and none of it grows with the size of an upload.
//!
//! # Refused, never repaired
//!
//! Every ambiguity below is an error rather than a guess, under ADR 0095: a
//! header line with no colon or a folded one, a part with no
//! `Content-Disposition` or no `name`, a parameter declared twice, a quoted
//! value with no closing quote, a boundary that is not RFC 2046's, and a body
//! that ends anywhere but after its closing delimiter.

use std::ops::Range;

use nvs_runtime::RequestBody;

use crate::request::REQUEST_BODY;

/// `rule:errors/multipart-part-count`'s `[limits] max_multipart_parts` default, as a constant until that row
/// exists.
///
/// PHP's own number after CVE-2023-0662, and it counts *every* part: a field
/// part costs the same bookkeeping as a file one, which is what this cap is
/// about.
const MAX_PARTS: usize = 1000;

/// The most one part's header block may hold before it is refused.
///
/// Not a directive of its own: it is the shape of `rule:errors/multipart-part-count`'s per-part
/// accounting applied to the one thing in a part that is read into memory
/// whatever the part turns out to be. Without it a peer that opens a part and
/// then never writes the blank line grows this buffer for as long as it cares
/// to, which no byte cap over the *body* would notice.
const PART_HEADERS: usize = 16 * 1024;

/// The most transport padding accepted between a boundary and its line ending.
///
/// RFC 2046 allows linear whitespace there and says nothing about how much. A
/// number is needed because the alternative is a buffer that grows while the
/// parse is still deciding whether it is looking at a delimiter.
const PADDING: usize = 64;

/// A multipart body being read, one part at a time.
///
/// Single-pass by construction: the cursor only moves forward and nothing that
/// has been handed out is kept, so the type cannot answer the same part twice.
/// That is `rule:http-server/an-upload-is-received-only-through-files`'s "single-pass" stated as a representation rather than
/// as a rule a caller has to keep.
#[derive(Debug)]
pub(crate) struct Multipart {
    /// `CRLF--boundary`, built once: every part ends at one of these, and so
    /// does the preamble.
    delimiter: Vec<u8>,
    /// The wire bytes that have arrived and not yet been passed.
    ///
    /// It opens holding a synthetic `CRLF` so that the *first* delimiter — which
    /// RFC 2046 writes without one, being at the start of the body — is the same
    /// byte sequence as every later one. One shape rather than two, and the
    /// preamble path then differs from the between-parts path only in that it
    /// discards what it walks over.
    buf: Vec<u8>,
    /// The read cursor into [`Self::buf`]. **No index survives a
    /// [`Self::fill`]**, which compacts the buffer by dropping everything before
    /// this point; every loop below recomputes from `at` after one.
    at: usize,
    state: State,
    /// Parts opened so far, against [`MAX_PARTS`].
    parts: usize,
    /// § 2's buffered form fields, in arrival order, for `Core\Request::post()`.
    fields: Vec<(Vec<u8>, Vec<u8>)>,
    /// Bytes of [`Self::fields`] held, against [`REQUEST_BODY`].
    buffered: usize,
    /// Whether the supplier has answered its last chunk.
    ended: bool,
    /// Whether the last error this parse answered came **out of the supplier**
    /// rather than out of the bytes.
    ///
    /// The two are one `Box<str>` here on purpose — this module classifies
    /// nothing — but the member reading it has to classify: a connection that
    /// failed under an upload is an `IOError` and a body that is not the
    /// multipart one it declared is a `ParseError`, and those are different
    /// `catch` names for the caller. Recording the source at the one place it
    /// is known costs a `bool` and saves the reader from guessing at a message.
    from_the_wire: bool,
}

/// Where the cursor stands between calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Before the first delimiter. RFC 2046's preamble, which is discarded.
    Preamble,
    /// Exactly at a delimiter, with what follows it not yet classified.
    AtDelimiter,
    /// At the blank line that ends a part's headers, whose `CRLF` is also the
    /// first two bytes of the delimiter where the part has no body at all.
    Opening,
    /// Inside a part's body, which is handed out chunk by chunk.
    Body,
    /// The closing delimiter has been read. The epilogue is never pulled.
    Done,
}

/// A file part's header block, parsed.
///
/// Only a *file* part reaches a caller: § 2 makes `filename` the whole
/// distinction, and a part without one is an ordinary form field this module
/// buffers into [`Multipart::fields`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartHead {
    /// The form field name, from `Content-Disposition`.
    pub(crate) name: Vec<u8>,
    /// The client's claimed file name — a claim, never a path.
    pub(crate) filename: Vec<u8>,
    /// The part's declared `Content-Type`, where it declared one.
    pub(crate) content_type: Option<Vec<u8>>,
}

/// What one header block turned out to describe.
#[derive(Debug)]
enum Head {
    /// A file part, § 2's `filename` being present.
    File(PartHead),
    /// An ordinary form field, carrying its name.
    Field(Vec<u8>),
}

/// What the bytes after a delimiter say about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Follow {
    /// `--` follows: this delimiter closes the body.
    Closed,
    /// A part follows, this many bytes after the delimiter's own end.
    Next(usize),
    /// Neither: the bytes only looked like a delimiter and are part of a body.
    Body,
    /// Not enough has arrived to tell which of the three this is.
    Need,
}

/// What a scan for the next delimiter found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scan {
    /// A real delimiter begins here.
    Ends(usize),
    /// Everything before this index is body; what follows needs more bytes.
    More(usize),
}

impl Multipart {
    /// A parse of a body delimited by `boundary`, before anything is pulled.
    pub(crate) fn new(boundary: &[u8]) -> Self {
        let mut delimiter = Vec::with_capacity(boundary.len() + 4);
        delimiter.extend_from_slice(b"\r\n--");
        delimiter.extend_from_slice(boundary);
        Self {
            delimiter,
            buf: b"\r\n".to_vec(),
            at: 0,
            state: State::Preamble,
            parts: 0,
            fields: Vec::new(),
            buffered: 0,
            ended: false,
            from_the_wire: false,
        }
    }

    /// How many parts this parse has opened, field parts included.
    ///
    /// The identity `rule:http-server/a-part-is-consumed-in-one-of-three-ways`'s "valid only while this part is the
    /// iterator's current one" is checked against: a `Core\Request\Part` is
    /// stamped with this at the moment it is answered, so a program holding an
    /// older one is refused rather than handed the current part's bytes. It
    /// counts field parts too, because what it identifies is a position in the
    /// body and not a position among the parts that were answered.
    pub(crate) fn opened(&self) -> usize {
        self.parts
    }

    /// Whether the error this parse last answered came off the connection.
    ///
    /// Meaningful only immediately after an `Err`; see [`Self::from_the_wire`].
    pub(crate) fn failed_on_the_wire(&self) -> bool {
        self.from_the_wire
    }

    /// § 2's buffered form fields, in the order they arrived.
    ///
    /// Arrival order rather than a map, for `crate::request`'s own reason: a
    /// repeated field name is a value a form is allowed to send twice, and the
    /// order of the two is part of what it sent.
    /// Complete after a [`Self::drain`], and a prefix of the form before one:
    /// a field is buffered as the walk passes it, so `Core\Request::post()`
    /// drains before it reads.
    pub(crate) fn fields(&self) -> &[(Vec<u8>, Vec<u8>)] {
        &self.fields
    }

    /// Walks what is left of the body, buffering every field part it passes and
    /// draining every file part.
    ///
    /// `Core\Request::post()`'s whole reading, and the reason `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename` can
    /// promise *every* field rather than the ones that happened to arrive first:
    /// a form is free to write a text input after a file input, and a `post()`
    /// answering what the walk had reached would report that field absent.
    ///
    /// It costs one pass and no memory beyond the fields themselves — the drain
    /// is [`Self::next_part`]'s own, which walks a file part's bytes without
    /// copying them.
    ///
    /// # Errors
    ///
    /// As [`Self::next_part`].
    pub(crate) fn drain(&mut self, body: &mut dyn RequestBody) -> Result<(), Box<str>> {
        while self.next_part(body)?.is_some() {}
        Ok(())
    }

    /// The next **file** part, `Ok(None)` at the closing delimiter.
    ///
    /// Advancing past a part whose body was not consumed **drains it** — ADR
    /// 0105 § 1, where skipping an upload the application does not recognise is
    /// simply not touching it. The drained bytes were charged on the wire on
    /// their way in, so this costs a walk and no memory.
    ///
    /// Field parts are consumed here rather than answered: each is buffered into
    /// [`Self::fields`] and the walk continues, so a caller sees § 2's split
    /// without having to know it.
    ///
    /// # Errors
    ///
    /// A body that cannot be completed, whether because the supplier failed
    /// under it or because what arrived is not a multipart body this parse will
    /// guess at. The message is for the member reading it to turn into its own
    /// throw; nothing here classifies.
    pub(crate) fn next_part(
        &mut self,
        body: &mut dyn RequestBody,
    ) -> Result<Option<PartHead>, Box<str>> {
        loop {
            match self.state {
                State::Done => return Ok(None),
                State::Preamble => self.find_first(body)?,
                // § 1's drain. `chunk` leaves the cursor at the delimiter, so
                // this is the same walk a consuming caller makes, minus the copy.
                State::Opening | State::Body => while self.chunk(body)?.is_some() {},
                State::AtDelimiter => {
                    if !self.open(body)? {
                        return Ok(None);
                    }
                    match self.headers(body)? {
                        Head::File(part) => {
                            self.state = State::Opening;
                            return Ok(Some(part));
                        }
                        Head::Field(name) => self.buffer(body, name)?,
                    }
                }
            }
        }
    }

    /// The next run of the current part's body, `Ok(None)` at its end.
    ///
    /// The slice is borrowed from this parse's own buffer and is **invalidated
    /// by the next call**, which is [`RequestBody::next_chunk`]'s contract one
    /// layer up and for the same reason: a reader that keeps bytes copies them
    /// into the bound it chose, and a reader writing them to a destination
    /// copies nothing at all.
    ///
    /// # Errors
    ///
    /// As [`Self::next_part`].
    pub(crate) fn next_chunk(
        &mut self,
        body: &mut dyn RequestBody,
    ) -> Result<Option<&[u8]>, Box<str>> {
        let span = self.chunk(body)?;
        Ok(span.map(|span| &self.buf[span]))
    }

    /// [`Self::next_chunk`] as a span, because a `&mut self` method that hands
    /// back a piece of `self` cannot also keep looping over it.
    fn chunk(&mut self, body: &mut dyn RequestBody) -> Result<Option<Range<usize>>, Box<str>> {
        if self.state == State::Opening {
            self.open_body(body)?;
        }
        if self.state != State::Body {
            return Ok(None);
        }
        loop {
            match self.scan(self.at)? {
                Scan::Ends(end) => {
                    let span = self.at..end;
                    self.at = end;
                    self.state = State::AtDelimiter;
                    return Ok(if span.is_empty() { None } else { Some(span) });
                }
                Scan::More(upto) => {
                    if upto > self.at {
                        let span = self.at..upto;
                        self.at = upto;
                        return Ok(Some(span));
                    }
                    if !self.fill(body)? {
                        return Err("the body ended before the part's closing boundary".into());
                    }
                }
            }
        }
    }

    /// Steps past the blank line that ends a part's headers, or ends the part
    /// there when a delimiter begins on that line's own `CRLF`.
    ///
    /// RFC 2046 makes a part's body optional, so `headers CRLF` followed by the
    /// delimiter `CRLF--boundary` is an empty part. Stepping over all four bytes
    /// first would leave `--boundary` looking like content, and the part would
    /// swallow the next one whole. That is a field a proxy in front of this
    /// server saw and `post()` would not, so the two readings must agree.
    fn open_body(&mut self, body: &mut dyn RequestBody) -> Result<(), Box<str>> {
        loop {
            match self.delimiter_at(self.at)? {
                Some(true) => {
                    self.state = State::AtDelimiter;
                    return Ok(());
                }
                Some(false) => {
                    self.at += 2;
                    self.state = State::Body;
                    return Ok(());
                }
                None => {
                    if !self.fill(body)? {
                        return Err("the body ended before the part's closing boundary".into());
                    }
                }
            }
        }
    }

    /// Walks to the first delimiter, discarding RFC 2046's preamble.
    ///
    /// The preamble is unbounded on purpose and costs nothing: [`Self::fill`]
    /// drops everything before the cursor, so what is held is one chunk however
    /// long the peer takes to reach its first boundary. What bounds the *time*
    /// is the wire cap, one layer down.
    fn find_first(&mut self, body: &mut dyn RequestBody) -> Result<(), Box<str>> {
        loop {
            match self.scan(self.at)? {
                Scan::Ends(start) => {
                    self.at = start;
                    self.state = State::AtDelimiter;
                    return Ok(());
                }
                Scan::More(upto) => {
                    self.at = upto;
                    if !self.fill(body)? {
                        return Err("the body carried no opening boundary".into());
                    }
                }
            }
        }
    }

    /// Consumes the delimiter the cursor is sitting on, answering whether a part
    /// follows it.
    fn open(&mut self, body: &mut dyn RequestBody) -> Result<bool, Box<str>> {
        let skip = loop {
            // Recomputed every turn: `fill` compacts, so the index this is
            // derived from is only valid until one runs.
            let after = self.at + self.delimiter.len();
            match self.follows(after)? {
                Follow::Closed => {
                    self.state = State::Done;
                    return Ok(false);
                }
                Follow::Next(skip) => break after + skip,
                Follow::Body => {
                    return Err("a boundary is followed by neither a line ending nor `--`".into());
                }
                Follow::Need => {
                    if !self.fill(body)? {
                        return Err("the body ended at a boundary rather than after one".into());
                    }
                }
            }
        };
        self.at = skip;
        self.parts += 1;
        if self.parts > MAX_PARTS {
            return Err(format!("the body holds more than {MAX_PARTS} parts").into());
        }
        Ok(true)
    }

    /// Reads one part's header block and says what it describes.
    fn headers(&mut self, body: &mut dyn RequestBody) -> Result<Head, Box<str>> {
        // A block of no headers at all is the blank line by itself, and it has
        // to be recognised *before* the search for that blank line: `\r\n\r\n`
        // would otherwise be looked for inside the part's own body, and found
        // there by anything that happens to carry one. Either way the cursor is
        // left on the blank line, which `open_body` steps over.
        let end = loop {
            let rest = &self.buf[self.at..];
            if rest.starts_with(b"\r\n") {
                return head_of(&[]);
            }
            if let Some(off) = find(rest, b"\r\n\r\n") {
                break self.at + off;
            }
            if rest.len() > PART_HEADERS {
                return Err(format!("a part's headers hold more than {PART_HEADERS} bytes").into());
            }
            if !self.fill(body)? {
                return Err("the body ended inside a part's headers".into());
            }
        };
        let head = head_of(&self.buf[self.at..end])?;
        self.at = end + 2;
        Ok(head)
    }

    /// Reads a field part's body into [`Self::fields`] — § 2's "the server
    /// buffers it and `post()` works exactly as it does for a urlencoded form".
    ///
    /// **The bound is checked before the copy**, as `Core\Request::body`'s is:
    /// a chunk that would carry the total past [`REQUEST_BODY`] is refused while
    /// it is still a span of the wire buffer, so what this holds never exceeds
    /// the number it was given.
    fn buffer(&mut self, body: &mut dyn RequestBody, name: Vec<u8>) -> Result<(), Box<str>> {
        self.state = State::Opening;
        let mut value = Vec::new();
        while let Some(span) = self.chunk(body)? {
            if self.buffered + span.len() > REQUEST_BODY {
                return Err(
                    format!("this body's form fields hold more than {REQUEST_BODY} bytes").into(),
                );
            }
            self.buffered += span.len();
            value.extend_from_slice(&self.buf[span]);
        }
        self.fields.push((name, value));
        Ok(())
    }

    /// The first real delimiter at or after `from`, reading the bytes between as
    /// body.
    ///
    /// A delimiter is only a delimiter where RFC 2046 says so — `--` or a line
    /// ending has to follow it — so a body carrying `CRLF--boundary` inside a
    /// longer token keeps those bytes rather than ending at them. Getting this
    /// wrong truncates an upload whose content happens to quote its own
    /// boundary, which is the one corruption a client cannot detect from the
    /// outside.
    ///
    /// The search is a byte scan for the delimiter's own lead rather than a
    /// substring search: the delimiter is short, the body is not, and a
    /// window-by-window comparison would be the body's length times the
    /// boundary's.
    fn scan(&self, from: usize) -> Result<Scan, Box<str>> {
        let mut i = from;
        while let Some(off) = self.buf[i..].iter().position(|&byte| byte == b'\r') {
            let start = i + off;
            match self.delimiter_at(start)? {
                None => return Ok(Scan::More(start)),
                Some(true) => return Ok(Scan::Ends(start)),
                Some(false) => i = start + 1,
            }
        }
        Ok(Scan::More(self.buf.len()))
    }

    /// Whether a real delimiter begins at `start`, or `None` where not enough
    /// has arrived to tell.
    fn delimiter_at(&self, start: usize) -> Result<Option<bool>, Box<str>> {
        let rest = &self.buf[start..];
        if rest.len() < self.delimiter.len() {
            return Ok(if self.delimiter.starts_with(rest) {
                None
            } else {
                Some(false)
            });
        }
        if !rest.starts_with(&self.delimiter) {
            return Ok(Some(false));
        }
        Ok(match self.follows(start + self.delimiter.len())? {
            Follow::Need => None,
            Follow::Closed | Follow::Next(_) => Some(true),
            Follow::Body => Some(false),
        })
    }

    /// Classifies the bytes at `after`, which is one past a delimiter's end.
    fn follows(&self, after: usize) -> Result<Follow, Box<str>> {
        match self.buf.get(after) {
            None => return Ok(Follow::Need),
            Some(b'-') => {
                return Ok(match self.buf.get(after + 1) {
                    None => Follow::Need,
                    Some(b'-') => Follow::Closed,
                    Some(_) => Follow::Body,
                });
            }
            Some(_) => {}
        }
        let mut i = after;
        while let Some(b' ' | b'\t') = self.buf.get(i) {
            i += 1;
            if i - after > PADDING {
                return Err(
                    format!("a boundary carries more than {PADDING} bytes of padding").into(),
                );
            }
        }
        Ok(match (self.buf.get(i), self.buf.get(i + 1)) {
            (None, _) | (Some(b'\r'), None) => Follow::Need,
            (Some(b'\r'), Some(b'\n')) => Follow::Next(i + 2 - after),
            _ => Follow::Body,
        })
    }

    /// Pulls one chunk off the supplier, answering `false` at the end of the
    /// body.
    ///
    /// **This compacts**, dropping everything before the cursor, which is what
    /// keeps the buffer O(one chunk) over a body of any size — and what makes
    /// every index taken before a call to it stale afterwards.
    fn fill(&mut self, body: &mut dyn RequestBody) -> Result<bool, Box<str>> {
        if self.ended {
            return Ok(false);
        }
        let pulled = body.next_chunk().inspect_err(|_| {
            // The one place a failure is known to be the supplier's rather than
            // the body's, which is what [`Self::from_the_wire`] exists to carry
            // out to a member that has to pick a `catch` name.
            self.from_the_wire = true;
        })?;
        let Some(chunk) = pulled else {
            self.ended = true;
            return Ok(false);
        };
        if self.at > 0 {
            self.buf.drain(..self.at);
            self.at = 0;
        }
        self.buf.extend_from_slice(chunk);
        Ok(true)
    }
}

/// Whether `content_type` declares a `multipart/form-data` body at all — the
/// question that comes *before* [`boundary_of`]'s.
///
/// The two are apart because the answers are different facts for the member
/// asking them. A request that is not multipart carries no file parts, and
/// `Core\Request::files()` walks it empty rather than refusing it: "this
/// request sent no files" is exactly true of a `GET`, and a throw there would
/// make every handler write the content-type check the runtime has already
/// done. A request that *says* it is multipart and then does not say how is
/// `rule:errors/ambiguous-input-refused`'s ambiguity and is refused. One `Err` covering both would have
/// forced `files()` to choose between refusing every `GET` and swallowing a
/// body whose boundary it could not find.
pub(crate) fn is_multipart(content_type: &[u8]) -> bool {
    media_type(content_type).eq_ignore_ascii_case(b"multipart/form-data")
}

/// The `boundary` a `Content-Type` declares, checked against RFC 2046's own
/// grammar for one.
///
/// # Errors
///
/// A header that is not `multipart/form-data`, declares no boundary, declares
/// two, or declares one no conforming sender would have written. A boundary is
/// the one token this parse trusts to appear inside a body, so a repaired one
/// would be a truncation rule chosen by the peer.
pub(crate) fn boundary_of(content_type: &[u8]) -> Result<Vec<u8>, Box<str>> {
    if !is_multipart(content_type) {
        return Err("the request body is not `multipart/form-data`".into());
    }
    let boundary = parameter(content_type, b"boundary")?
        .ok_or_else(|| Box::<str>::from("the `Content-Type` declared no `boundary`"))?;
    // RFC 2046: 1 to 70 characters from a closed set, and never a trailing
    // space, which no sender can round-trip through a header anyway.
    if boundary.is_empty() || boundary.len() > 70 {
        return Err("the `boundary` is not between 1 and 70 characters".into());
    }
    if boundary.ends_with(b" ") {
        return Err("the `boundary` ends with a space".into());
    }
    if !boundary.iter().all(|&byte| is_bchar(byte)) {
        return Err("the `boundary` carries a character RFC 2046 does not allow".into());
    }
    Ok(boundary)
}

/// One header block's lines, read as § 2's file-or-field question.
fn head_of(lines: &[u8]) -> Result<Head, Box<str>> {
    let mut disposition: Option<&[u8]> = None;
    let mut content_type: Option<&[u8]> = None;
    let mut rest = lines;
    while !rest.is_empty() {
        let (line, tail) = match find(rest, b"\r\n") {
            Some(at) => (&rest[..at], &rest[at + 2..]),
            None => (rest, &rest[rest.len()..]),
        };
        rest = tail;
        let colon = line
            .iter()
            .position(|&byte| byte == b':')
            .ok_or_else(|| Box::<str>::from("a part's header line carries no `:`"))?;
        let name = &line[..colon];
        // An empty name, a name with a space in it, or a line beginning with
        // whitespace — RFC 5322's obsolete folding — are all the same refusal:
        // two readers would disagree about where the header ends.
        if name.is_empty() || name.iter().any(u8::is_ascii_whitespace) {
            return Err("a part's header line does not begin with a field name".into());
        }
        if line.contains(&b'\n') {
            return Err("a part's header line carries a bare line feed".into());
        }
        let value = trim(&line[colon + 1..]);
        let seen = if name.eq_ignore_ascii_case(b"content-disposition") {
            &mut disposition
        } else if name.eq_ignore_ascii_case(b"content-type") {
            &mut content_type
        } else {
            continue;
        };
        if seen.is_some() {
            return Err("a part declares one of its headers twice".into());
        }
        *seen = Some(value);
    }
    let disposition =
        disposition.ok_or_else(|| Box::<str>::from("a part declared no `Content-Disposition`"))?;
    if !media_type(disposition).eq_ignore_ascii_case(b"form-data") {
        return Err("a part's `Content-Disposition` is not `form-data`".into());
    }
    let name = parameter(disposition, b"name")?
        .ok_or_else(|| Box::<str>::from("a part's `Content-Disposition` declared no `name`"))?;
    // § 2: a part is a file part iff `filename` is there, which is RFC 7578's
    // own distinction. RFC 5987's `filename*` is deliberately not a second one —
    // inventing a rule the senders do not share is what § 2 refuses.
    Ok(match parameter(disposition, b"filename")? {
        Some(filename) => Head::File(PartHead {
            name,
            filename,
            content_type: content_type.map(<[u8]>::to_vec),
        }),
        None => Head::Field(name),
    })
}

/// The media type a header opens with: everything before its first parameter.
fn media_type(header: &[u8]) -> &[u8] {
    match header.iter().position(|&byte| byte == b';') {
        Some(at) => trim(&header[..at]),
        None => trim(header),
    }
}

/// The value of the `; want=…` parameter of `header`, unquoted.
///
/// # Errors
///
/// A parameter with no value, an unterminated quoted value, or `want` declared
/// twice — the last because two values for one name is precisely the ambiguity
/// a reader would otherwise resolve by picking, and the two readers that pick
/// differently is the whole of a smuggling bug.
fn parameter(header: &[u8], want: &[u8]) -> Result<Option<Vec<u8>>, Box<str>> {
    let Some(at) = header.iter().position(|&byte| byte == b';') else {
        return Ok(None);
    };
    let mut found: Option<Vec<u8>> = None;
    let mut rest = &header[at + 1..];
    loop {
        let (name, value, tail) = split_parameter(rest)?;
        if !name.is_empty() && name.eq_ignore_ascii_case(want) {
            if found.is_some() {
                return Err(format!(
                    "`{}` is declared twice in one header",
                    String::from_utf8_lossy(want)
                )
                .into());
            }
            found = Some(value);
        }
        match tail {
            None => break,
            Some(tail) => rest = tail,
        }
    }
    Ok(found)
}

/// One parameter read off a header: its name, its value with the quoting
/// undone, and whatever follows the `;` that ended it.
type Parameter<'a> = (&'a [u8], Vec<u8>, Option<&'a [u8]>);

/// One `name=value` parameter off the front of `rest`, and what follows its
/// `;`.
fn split_parameter(rest: &[u8]) -> Result<Parameter<'_>, Box<str>> {
    let rest = trim_start(rest);
    if rest.is_empty() {
        return Ok((&[], Vec::new(), None));
    }
    let stop = rest
        .iter()
        .position(|&byte| byte == b'=' || byte == b';')
        .filter(|&at| rest[at] == b'=')
        .ok_or_else(|| Box::<str>::from("a header parameter carries no value"))?;
    let name = trim(&rest[..stop]);
    let after = trim_start(&rest[stop + 1..]);
    if after.first() != Some(&b'"') {
        let (value, tail) = match after.iter().position(|&byte| byte == b';') {
            Some(at) => (&after[..at], Some(&after[at + 1..])),
            None => (after, None),
        };
        return Ok((name, trim(value).to_vec(), tail));
    }
    // RFC 2045's quoted string, where a backslash quotes the byte after it —
    // which is how a filename carrying a quote or a semicolon arrives.
    let mut value = Vec::new();
    let mut i = 1;
    loop {
        match after.get(i) {
            None => return Err("a quoted header parameter has no closing quote".into()),
            Some(b'"') => {
                i += 1;
                break;
            }
            Some(b'\\') => {
                let quoted = after
                    .get(i + 1)
                    .ok_or_else(|| Box::<str>::from("a quoted header parameter ends in a `\\`"))?;
                value.push(*quoted);
                i += 2;
            }
            Some(byte) => {
                value.push(*byte);
                i += 1;
            }
        }
    }
    let tail = trim_start(&after[i..]);
    match tail.first() {
        None => Ok((name, value, None)),
        Some(b';') => Ok((name, value, Some(&tail[1..]))),
        Some(_) => Err("a quoted header parameter is followed by something other than `;`".into()),
    }
}

/// Whether `byte` is RFC 2046's `bchars`.
fn is_bchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&byte)
}

/// `bytes` without its leading spaces and tabs.
fn trim_start(bytes: &[u8]) -> &[u8] {
    let at = bytes
        .iter()
        .position(|byte| !matches!(byte, b' ' | b'\t'))
        .unwrap_or(bytes.len());
    &bytes[at..]
}

/// `bytes` without its leading or trailing spaces and tabs.
fn trim(bytes: &[u8]) -> &[u8] {
    let bytes = trim_start(bytes);
    let end = bytes
        .iter()
        .rposition(|byte| !matches!(byte, b' ' | b'\t'))
        .map_or(0, |at| at + 1);
    &bytes[..end]
}

/// The first index at which `needle` occurs in `hay`.
///
/// The needles here are two and four bytes long and the haystacks are one header
/// block, so the obvious walk is the right one; the body scan, which is neither,
/// is [`Multipart::scan`] instead.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&at| hay[at..].starts_with(needle))
}

#[cfg(test)]
mod tests {
    use super::{MAX_PARTS, Multipart, PartHead, REQUEST_BODY, boundary_of};
    use nvs_runtime::RequestBody;

    /// A [`RequestBody`] handing one body out in fixed-size pieces, because a
    /// chunk boundary is the wire's and means nothing: every parse below has to
    /// answer the same thing at every piece size.
    struct Wire {
        body: Vec<u8>,
        at: usize,
        piece: usize,
    }

    impl Wire {
        fn new(body: &[u8], piece: usize) -> Self {
            Self {
                body: body.to_vec(),
                at: 0,
                piece,
            }
        }
    }

    impl RequestBody for Wire {
        fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>> {
            if self.at >= self.body.len() {
                return Ok(None);
            }
            let end = (self.at + self.piece).min(self.body.len());
            let span = self.at..end;
            self.at = end;
            Ok(Some(&self.body[span]))
        }
    }

    /// A body over `--bnd`, one entry per part: a name, the file name it claims
    /// where it is a file part, and its content.
    fn body(parts: &[(&str, Option<&str>, &str)]) -> Vec<u8> {
        let mut wire = Vec::new();
        for (name, filename, content) in parts {
            wire.extend_from_slice(b"--bnd\r\nContent-Disposition: form-data; name=\"");
            wire.extend_from_slice(name.as_bytes());
            wire.push(b'"');
            if let Some(filename) = filename {
                wire.extend_from_slice(b"; filename=\"");
                wire.extend_from_slice(filename.as_bytes());
                wire.push(b'"');
            }
            wire.extend_from_slice(b"\r\n\r\n");
            wire.extend_from_slice(content.as_bytes());
            wire.extend_from_slice(b"\r\n");
        }
        wire.extend_from_slice(b"--bnd--\r\n");
        wire
    }

    /// Every file part of a body, read whole, beside the parse that read them —
    /// which is where the field parts went.
    type Walked = (Vec<(PartHead, Vec<u8>)>, Multipart);

    /// Every file part of `wire`, read whole, at the given piece size.
    fn walk(wire: &[u8], piece: usize) -> Result<Walked, Box<str>> {
        let mut supplier = Wire::new(wire, piece);
        let mut parse = Multipart::new(b"bnd");
        let mut read = Vec::new();
        while let Some(head) = parse.next_part(&mut supplier)? {
            let mut content = Vec::new();
            while let Some(chunk) = parse.next_chunk(&mut supplier)? {
                content.extend_from_slice(chunk);
            }
            read.push((head, content));
        }
        Ok((read, parse))
    }

    /// `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`: `filename` is the whole distinction, so the same part
    /// shape answers as a file or as a form field depending on one parameter —
    /// and the fields go where `post()` will read them rather than to the
    /// caller.
    #[test]
    fn a_part_is_a_file_part_iff_content_disposition_carries_a_filename() {
        let wire = body(&[
            ("title", None, "a report"),
            ("upload", Some("report.pdf"), "%PDF-1.7"),
            ("tags", None, "one"),
        ]);
        let (read, parse) = walk(&wire, 7).expect("the body parses");
        assert_eq!(read.len(), 1, "only the part claiming a filename is a file");
        assert_eq!(read[0].0.name, b"upload");
        assert_eq!(read[0].0.filename, b"report.pdf");
        assert_eq!(read[0].1, b"%PDF-1.7");
        let fields: Vec<_> = parse
            .fields()
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        assert_eq!(
            fields,
            vec![
                (b"title".to_vec(), b"a report".to_vec()),
                (b"tags".to_vec(), b"one".to_vec()),
            ],
            "the two field parts are buffered in arrival order"
        );
    }

    /// A chunk boundary is the wire's and means nothing: the same body pulled
    /// one byte at a time and in one piece has to answer identically.
    ///
    /// Asserted as an invariance over the whole sweep rather than at one size,
    /// because a parse that assumes a delimiter arrives whole is right at every
    /// size but the ones that split it.
    #[test]
    fn a_part_is_read_across_whatever_chunk_boundaries_the_wire_chose() {
        let wire = body(&[
            ("title", None, "a report"),
            ("upload", Some("report.pdf"), "0123456789abcdef"),
        ]);
        let (want, _) = walk(&wire, wire.len()).expect("the body parses in one piece");
        for piece in [1, 2, 3, 5, 8, 13, 64, 4096] {
            let (read, parse) = walk(&wire, piece).unwrap_or_else(|why| {
                panic!("the body parses at {piece} bytes a chunk: {why}");
            });
            assert_eq!(read, want, "at {piece} bytes a chunk");
            assert_eq!(parse.fields()[0].1, b"a report", "at {piece} bytes a chunk");
        }
    }

    /// An empty part may end at the blank line after its headers, because RFC
    /// 2046 makes a body optional and that line's `CRLF` then begins the
    /// delimiter. A browser writes one more `CRLF` for an empty file, and both
    /// forms answer the same parts: the empty file, and the field after it.
    ///
    /// The first form once read the next part as the empty file's content, so
    /// the field after it reached no `post()`. A proxy reading the body the
    /// RFC's way would have seen that field and this server would not.
    #[test]
    fn an_empty_part_ends_at_the_delimiter_on_its_own_blank_line() {
        let head = b"--bnd\r\nContent-Disposition: form-data; name=\"empty\"; filename=\"\"\r\n";
        let tail =
            b"--bnd\r\nContent-Disposition: form-data; name=\"note\"\r\n\r\nafter\r\n--bnd--\r\n";
        for blank in [&b"\r\n"[..], b"\r\n\r\n"] {
            let mut wire = head.to_vec();
            wire.extend_from_slice(blank);
            wire.extend_from_slice(tail);
            for piece in [1, 2, 3, 5, 8, 64, 4096] {
                let (read, parse) = walk(&wire, piece).unwrap_or_else(|why| {
                    panic!("the body parses at {piece} bytes a chunk: {why}");
                });
                assert_eq!(read.len(), 1, "at {piece} bytes a chunk");
                assert_eq!(read[0].0.name, b"empty");
                assert_eq!(read[0].1, b"", "the empty file stays empty at {piece}");
                assert_eq!(
                    parse.fields(),
                    [(b"note".to_vec(), b"after".to_vec())],
                    "the field after it is buffered at {piece}"
                );
            }
        }
    }

    /// A part whose content quotes its own boundary keeps those bytes: RFC 2046
    /// ends a part at a delimiter only where `--` or a line ending follows it.
    ///
    /// The failure this pins is silent — the part would be handed over
    /// truncated, and nothing downstream could tell.
    #[test]
    fn a_delimiter_the_bytes_only_look_like_stays_inside_the_part() {
        let content = "before\r\n--bndish\r\nafter\r\n--bn\r\nend";
        let wire = body(&[("upload", Some("f.bin"), content)]);
        for piece in [1, 4, 9, 4096] {
            let (read, _) = walk(&wire, piece).unwrap_or_else(|why| {
                panic!("the body parses at {piece} bytes a chunk: {why}");
            });
            assert_eq!(read.len(), 1);
            assert_eq!(
                read[0].1,
                content.as_bytes(),
                "the part kept its whole content at {piece} bytes a chunk"
            );
        }
    }

    /// `rule:http-server/an-upload-is-received-only-through-files`: advancing past a part nobody consumed drains it, so
    /// ignoring an upload is not touching it rather than a call.
    #[test]
    fn advancing_past_an_unconsumed_part_drains_it() {
        let wire = body(&[
            ("a", Some("a.bin"), "first"),
            ("b", Some("b.bin"), "second"),
            ("c", Some("c.bin"), "third"),
        ]);
        let mut supplier = Wire::new(&wire, 3);
        let mut parse = Multipart::new(b"bnd");
        let mut names = Vec::new();
        while let Some(head) = parse.next_part(&mut supplier).expect("the body parses") {
            names.push(head.filename);
        }
        assert_eq!(
            names,
            vec![b"a.bin".to_vec(), b"b.bin".into(), b"c.bin".into()]
        );
    }

    /// `rule:errors/multipart-part-count`'s part count, asserted on both sides: the cap is a cap only
    /// if the last accepted body and the first refused one are one part apart.
    #[test]
    fn the_part_count_cap_is_asserted_at_the_last_accepted_and_the_first_refused() {
        for (parts, accepted) in [(MAX_PARTS, true), (MAX_PARTS + 1, false)] {
            let rows: Vec<_> = (0..parts).map(|_| ("f", None, "x")).collect();
            let wire = body(&rows);
            let walked = walk(&wire, 8192);
            assert_eq!(
                walked.is_ok(),
                accepted,
                "a body of {parts} parts against a cap of {MAX_PARTS}"
            );
        }
    }

    /// § 2's buffered fields are bytes parsed into memory, so they are charged
    /// against the same number `Core\Request::body` is — and the check is made
    /// before the copy, which is what makes it a bound rather than a report.
    #[test]
    fn a_field_part_is_buffered_against_the_in_memory_bound() {
        let held = "x".repeat(REQUEST_BODY);
        let wire = body(&[("note", None, &held)]);
        let (read, parse) = walk(&wire, 64 * 1024).expect("a field at the bound is buffered");
        assert!(read.is_empty());
        assert_eq!(parse.fields()[0].1.len(), REQUEST_BODY);

        let over = "x".repeat(REQUEST_BODY + 1);
        let wire = body(&[("note", None, &over)]);
        assert!(
            walk(&wire, 64 * 1024).is_err(),
            "one byte past the bound is refused"
        );
    }

    /// A body that stops anywhere but after its closing delimiter is refused
    /// rather than reported as a short read, which is the whole of `rule:errors/ambiguous-input-refused`
    /// applied to a transfer that did not finish.
    ///
    /// Both sides, because the cut that is *not* a truncation is one byte from
    /// the ones that are: RFC 2046's epilogue follows the closing delimiter and
    /// is never read, so a body ending at `--bnd--` arrived whole while one
    /// ending at `--bnd-` did not.
    #[test]
    fn a_body_that_does_not_end_at_its_closing_boundary_is_refused() {
        let whole = body(&[("upload", Some("f.bin"), "0123456789")]);
        assert!(
            walk(&whole[..whole.len() - 2], 5).is_ok(),
            "a closing delimiter with no epilogue after it ends the body"
        );
        for cut in [
            whole.len() - 3,
            whole.len() - 4,
            whole.len() - 12,
            whole.len() / 2,
            8,
        ] {
            assert!(
                walk(&whole[..cut], 5).is_err(),
                "a body cut at {cut} of {} bytes is refused",
                whole.len()
            );
        }
        assert!(walk(b"nothing like a multipart body", 5).is_err());
    }

    /// A part that does not say what it is, in each of the ways it can fail to,
    /// is refused rather than guessed at.
    #[test]
    fn a_part_that_declares_nothing_usable_is_refused() {
        for block in [
            "\r\n",
            "Content-Disposition: form-data\r\n\r\n",
            "Content-Disposition: attachment; name=\"a\"\r\n\r\n",
            "Content-Disposition form-data; name=\"a\"\r\n\r\n",
            " Content-Disposition: form-data; name=\"a\"\r\n\r\n",
            "Content-Disposition: form-data; name=\"a\r\n\r\n",
            "Content-Disposition: form-data; name=\"a\"; name=\"b\"\r\n\r\n",
            "Content-Disposition: form-data; name\r\n\r\n",
        ] {
            let mut wire = Vec::from("--bnd\r\n");
            wire.extend_from_slice(block.as_bytes());
            wire.extend_from_slice(b"body\r\n--bnd--\r\n");
            assert!(
                walk(&wire, 4096).is_err(),
                "a part opening with {block:?} is refused"
            );
        }
    }

    /// RFC 2046's own grammar for a boundary, and the four ways a header can
    /// fail to carry one.
    #[test]
    fn a_boundary_is_read_off_content_type_and_a_malformed_one_is_refused() {
        assert_eq!(
            boundary_of(b"multipart/form-data; boundary=abc").expect("a plain boundary"),
            b"abc"
        );
        assert_eq!(
            boundary_of(b"MULTIPART/FORM-DATA ; BOUNDARY=\"a b\"").expect("a quoted boundary"),
            b"a b"
        );
        assert_eq!(
            boundary_of(b"multipart/form-data; charset=utf-8; boundary=abc")
                .expect("a boundary after another parameter"),
            b"abc"
        );
        let long = format!("multipart/form-data; boundary={}", "x".repeat(71));
        for header in [
            "application/json",
            "multipart/form-data",
            "multipart/form-data; boundary=",
            "multipart/form-data; boundary=a; boundary=b",
            "multipart/form-data; boundary=\"abc",
            "multipart/form-data; boundary=a\"b",
            &long,
        ] {
            assert!(
                boundary_of(header.as_bytes()).is_err(),
                "`{header}` declares no usable boundary"
            );
        }
    }

    /// A quoted `filename` carrying the two bytes that would otherwise end it,
    /// which is the case the quoted-string escape exists for.
    #[test]
    fn a_quoted_filename_may_carry_a_semicolon_and_a_quote() {
        let wire = concat!(
            "--bnd\r\n",
            "Content-Disposition: form-data; name=\"f\"; filename=\"a;b\\\"c.txt\"\r\n",
            "Content-Type: text/plain\r\n",
            "\r\n",
            "held\r\n",
            "--bnd--\r\n"
        );
        let (read, _) = walk(wire.as_bytes(), 6).expect("the body parses");
        assert_eq!(read[0].0.filename, b"a;b\"c.txt");
        assert_eq!(read[0].0.content_type.as_deref(), Some(&b"text/plain"[..]));
    }
}
