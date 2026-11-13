//! `Core\Json`'s own reader: a pull tokenizer over RFC 8259 text.
//!
//! [`Reader::next`] gives one [`Event`] per call: a scalar, an object key, or
//! the start or end of a container. The reader checks the whole grammar itself,
//! against one bit per open level held inline, and never recurses, so the
//! depth of a document is a loop bound and no caller needs a spare stack.
//! `rule:packaging/somebody-elses-specification-is-a-crate` argues why Novis
//! owns this parser, and `rule:http-server/engine-depth-is-bounded-per-decoder`
//! bounds its depth.
//!
//! **Every message is `serde_json`'s.** The text, the line and the column of a
//! syntax error are the ones `serde_json::from_slice` gives for the same bytes,
//! because `.out` files pin them. The column counts bytes, as the crate's does.
//! The test `the_reader_and_the_oracle_agree_on_every_saved_json_seed` holds
//! that over every saved fuzz input, with two named exceptions: the crate's own
//! depth limit of 128, and an integer literal past `u64::MAX` or below
//! `i64::MIN`. The crate widens that literal to a `float`, or refuses it as out
//! of range once it passes `f64::MAX`. The reader gives it as [`Number::Big`],
//! with its digit count, so the caller can refuse it whatever its length.
//!
//! **One text or many chunks.** A [`Source`] gives the text one chunk at a
//! time. A token may cross two chunks, and then the reader copies the part it
//! has read into its scratch buffer before it moves on. A string that sits
//! inside one chunk and has no escape is given as a slice of that chunk, with
//! no copy.
//!
//! **What it spends:** the bits for [`LEVELS`] open levels, 128 bytes held in
//! the reader, and one scratch buffer per read. The buffer holds the longest
//! string with escapes, or the longest token cut by a chunk end, and its growth
//! is asked from the request's budget first. Both are freed with the reader.

use std::fmt;

use super::DEPTH_CEILING;

/// The levels the reader keeps a bit for: every `maxDepth` a call may ask for.
pub(crate) const LEVELS: usize = 1024;

const _: () = assert!(LEVELS as u64 == DEPTH_CEILING);

/// The words of open-level bits, one bit per level.
const WORDS: usize = LEVELS / 64;

/// How many leading digits [`Digits`] keeps for a message.
const HEAD: usize = 32;

/// Where a [`Reader`] reads its text from, one chunk at a time.
pub(crate) trait Source {
    /// The current chunk, whole.
    fn chunk(&self) -> &[u8];

    /// Moves to the next chunk, and returns `false` at the end of the text.
    /// After `false`, [`Source::chunk`] still gives the last chunk.
    fn advance(&mut self) -> bool;
}

/// A whole text in one chunk.
#[derive(Debug)]
pub(crate) struct Text<'a>(pub(crate) &'a [u8]);

impl Source for Text<'_> {
    fn chunk(&self) -> &[u8] {
        self.0
    }

    fn advance(&mut self) -> bool {
        false
    }
}

/// A text cut into chunks of `size` bytes, for the checks that read a document
/// from chunks.
#[derive(Debug)]
pub(crate) struct Cut<'a> {
    chunk: &'a [u8],
    rest: &'a [u8],
    size: usize,
}

impl<'a> Cut<'a> {
    /// `bytes` in chunks of `size` bytes, and at least one.
    pub(crate) fn new(bytes: &'a [u8], size: usize) -> Self {
        let size = size.max(1);
        let (chunk, rest) = bytes.split_at(size.min(bytes.len()));
        Self { chunk, rest, size }
    }
}

impl Source for Cut<'_> {
    fn chunk(&self) -> &[u8] {
        self.chunk
    }

    fn advance(&mut self) -> bool {
        if self.rest.is_empty() {
            return false;
        }
        let (chunk, rest) = self.rest.split_at(self.size.min(self.rest.len()));
        self.chunk = chunk;
        self.rest = rest;
        true
    }
}

/// One token of the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Event<'a> {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number.
    Number(Number),
    /// A string value.
    Str(&'a str),
    /// An object key. The `:` after it is read with the value.
    Key(&'a str),
    /// `[`.
    StartArray,
    /// `]`.
    EndArray,
    /// `{`.
    StartObject,
    /// `}`.
    EndObject,
}

/// A number, as the document wrote it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Number {
    /// An integer literal that fits `int`.
    Int(i64),
    /// A positive integer literal past `i64::MAX` that fits `u64`.
    Uint(u64),
    /// A literal with a fraction or an exponent, and `-0`.
    Float(f64),
    /// An integer literal past `u64::MAX`, or below `i64::MIN`.
    Big {
        /// Whether the literal starts with `-`.
        negative: bool,
        /// Its digits.
        digits: Digits,
    },
}

/// The leading digits of a long integer literal, and how many it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Digits {
    head: [u8; HEAD],
    count: usize,
}

impl Digits {
    /// The first [`HEAD`] of `digits`, and their count.
    fn of(digits: &[u8]) -> Self {
        let mut head = [0; HEAD];
        let kept = digits.len().min(HEAD);
        head[..kept].copy_from_slice(&digits[..kept]);
        Self {
            head,
            count: digits.len(),
        }
    }

    /// How many digits the literal has.
    pub(crate) const fn count(&self) -> usize {
        self.count
    }
}

/// The first 32 digits, then `...` when the literal has more.
impl fmt::Display for Digits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &digit in &self.head[..self.count.min(HEAD)] {
            write!(f, "{}", char::from(digit))?;
        }
        if self.count > HEAD {
            f.write_str("...")?;
        }
        Ok(())
    }
}

/// Why a document did not read, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Error {
    code: Code,
    line: usize,
    column: usize,
}

/// `serde_json`'s form: the message, then `at line L column C`.
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {} column {}",
            self.code, self.line, self.column
        )
    }
}

/// What went wrong. Each message is the one `serde_json` gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Code {
    /// The text ended inside an array.
    EofWhileParsingList,
    /// The text ended inside an object.
    EofWhileParsingObject,
    /// The text ended inside a string.
    EofWhileParsingString,
    /// The text ended where a value belongs.
    EofWhileParsingValue,
    /// A key is not followed by `:`.
    ExpectedColon,
    /// An array element is not followed by `,` or `]`.
    ExpectedListCommaOrEnd,
    /// An object member is not followed by `,` or `}`.
    ExpectedObjectCommaOrEnd,
    /// A `null`, `true` or `false` is misspelled.
    ExpectedSomeIdent,
    /// No value starts with this byte.
    ExpectedSomeValue,
    /// A `\` is followed by something JSON does not define.
    InvalidEscape,
    /// A number breaks the RFC 8259 grammar.
    InvalidNumber,
    /// A number with a fraction or an exponent is past `f64::MAX`.
    NumberOutOfRange,
    /// A string from bytes is not UTF-8.
    InvalidUnicodeCodePoint,
    /// A string holds a byte below `0x20`.
    ControlCharacterWhileParsingString,
    /// An object member does not start with a string.
    KeyMustBeAString,
    /// A `\u` escape is a surrogate without its pair.
    LoneLeadingSurrogateInHexEscape,
    /// A `,` comes right before `]` or `}`.
    TrailingComma,
    /// Something follows the document.
    TrailingCharacters,
    /// A leading surrogate is not followed by `\u`.
    UnexpectedEndOfHexEscape,
    /// A container opens past the reader's `maxDepth`.
    TooDeep(u32),
    /// The scratch buffer would cross the request's memory limit.
    OverMemory,
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EofWhileParsingList => "EOF while parsing a list",
            Self::EofWhileParsingObject => "EOF while parsing an object",
            Self::EofWhileParsingString => "EOF while parsing a string",
            Self::EofWhileParsingValue => "EOF while parsing a value",
            Self::ExpectedColon => "expected `:`",
            Self::ExpectedListCommaOrEnd => "expected `,` or `]`",
            Self::ExpectedObjectCommaOrEnd => "expected `,` or `}`",
            Self::ExpectedSomeIdent => "expected ident",
            Self::ExpectedSomeValue => "expected value",
            Self::InvalidEscape => "invalid escape",
            Self::InvalidNumber => "invalid number",
            Self::NumberOutOfRange => "number out of range",
            Self::InvalidUnicodeCodePoint => "invalid unicode code point",
            Self::ControlCharacterWhileParsingString => {
                "control character (\\u0000-\\u001F) found while parsing a string"
            }
            Self::KeyMustBeAString => "key must be a string",
            Self::LoneLeadingSurrogateInHexEscape => "lone leading surrogate in hex escape",
            Self::TrailingComma => "trailing comma",
            Self::TrailingCharacters => "trailing characters",
            Self::UnexpectedEndOfHexEscape => "unexpected end of hex escape",
            Self::TooDeep(max) => {
                return write!(f, "nesting is deeper than the `maxDepth` of {max}");
            }
            Self::OverMemory => "the document does not fit in the request's memory limit",
        })
    }
}

/// What the next byte that is not whitespace may be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    /// Any value: the document's start, or after a `:`.
    Value,
    /// A first element or `]`.
    FirstElement,
    /// `,` or `]`.
    NextElement,
    /// A first key or `}`.
    FirstMember,
    /// `:`.
    Colon,
    /// `,` or `}`.
    NextMember,
    /// Nothing: the document has ended.
    End,
}

/// Where a string's bytes are, until [`Reader::next`] gives them.
#[derive(Clone, Copy, Debug)]
enum Span {
    /// `chunk[from..to]`.
    Chunk(usize, usize),
    /// The whole scratch buffer.
    Scratch,
}

/// An event before its string is resolved.
#[derive(Clone, Copy, Debug)]
enum Token {
    Null,
    Bool(bool),
    Number(Number),
    Str(Span),
    Key(Span),
    StartArray,
    EndArray,
    StartObject,
    EndObject,
}

/// The pull tokenizer. Once [`Reader::next`] gives an error, the reader is not
/// read again.
#[derive(Debug)]
pub(crate) struct Reader<S> {
    source: S,
    /// The next byte's index in the current chunk.
    at: usize,
    /// The offset of the current chunk's first byte in the whole text.
    base: usize,
    /// Whether the source has no chunk left.
    ended: bool,
    /// The line of the next byte, from 1, and the offset its line starts at.
    line: usize,
    line_start: usize,
    /// One bit per open level: set for an object, clear for an array.
    open: [u64; WORDS],
    depth: usize,
    /// The most levels open at once, which is the call's `maxDepth` less one:
    /// a scalar document is depth 1.
    max_open: usize,
    max_depth: u32,
    expect: Expect,
    /// A string with escapes, or a token cut by a chunk end.
    scratch: Vec<u8>,
    /// Whether the token being read is in [`Reader::scratch`].
    copied: bool,
    /// Where the token being read starts in the current chunk, while a chunk
    /// change must keep it.
    token: Option<usize>,
}

impl<S: Source> Reader<S> {
    /// A reader of `source` that refuses a container opened past `max_depth`,
    /// counted as `Core\Json::decode` counts it: a scalar document is depth 1,
    /// so `[1]` is depth 2. `max_depth` is at most [`LEVELS`].
    pub(crate) fn new(source: S, max_depth: u32) -> Self {
        let max_open = usize::try_from(max_depth)
            .unwrap_or(LEVELS)
            .clamp(1, LEVELS)
            - 1;
        Self {
            source,
            at: 0,
            base: 0,
            ended: false,
            line: 1,
            line_start: 0,
            open: [0; WORDS],
            depth: 0,
            max_open,
            max_depth,
            expect: Expect::Value,
            scratch: Vec::new(),
            copied: false,
            token: None,
        }
    }

    /// The next event, or `None` once the document and the whitespace after it
    /// have been read to the end of the text.
    ///
    /// # Errors
    ///
    /// The first place the text breaks RFC 8259, a container past `maxDepth`,
    /// or a scratch buffer the request cannot afford.
    pub(crate) fn next(&mut self) -> Result<Option<Event<'_>>, Error> {
        let Some(token) = self.step()? else {
            return Ok(None);
        };
        Ok(Some(match token {
            Token::Null => Event::Null,
            Token::Bool(value) => Event::Bool(value),
            Token::Number(number) => Event::Number(number),
            Token::Str(span) => Event::Str(self.text(span)?),
            Token::Key(span) => Event::Key(self.text(span)?),
            Token::StartArray => Event::StartArray,
            Token::EndArray => Event::EndArray,
            Token::StartObject => Event::StartObject,
            Token::EndObject => Event::EndObject,
        }))
    }

    /// A string's bytes as text, refused where they are not UTF-8. The
    /// position is `serde_json`'s: the column after the closing quote, less the
    /// bytes from the first invalid one to the string's end.
    fn text(&self, span: Span) -> Result<&str, Error> {
        let bytes = match span {
            Span::Chunk(from, to) => self.source.chunk().get(from..to).unwrap_or_default(),
            Span::Scratch => &self.scratch,
        };
        std::str::from_utf8(bytes).map_err(|why| Error {
            code: Code::InvalidUnicodeCodePoint,
            line: self.line,
            column: (self.offset() - self.line_start)
                .saturating_sub(bytes.len() - why.valid_up_to()),
        })
    }

    /// The offset of the next byte in the whole text.
    const fn offset(&self) -> usize {
        self.base + self.at
    }

    /// `code` at the next byte's offset: where `serde_json` reports an error
    /// in a byte it has already read.
    const fn fail(&self, code: Code) -> Error {
        Error {
            code,
            line: self.line,
            column: self.offset() - self.line_start,
        }
    }

    /// `code` one past the byte `peeked`, which is unread: where `serde_json`
    /// reports an error in a byte it has only looked at. At the text's end it
    /// is the end.
    const fn fail_peeked(&self, code: Code, peeked: Option<u8>) -> Error {
        match peeked {
            Some(b'\n') => Error {
                code,
                line: self.line + 1,
                column: 0,
            },
            Some(_) => Error {
                code,
                line: self.line,
                column: self.offset() + 1 - self.line_start,
            },
            None => self.fail(code),
        }
    }

    /// The next byte, unread, or `None` at the end of the text.
    #[inline]
    fn peek(&mut self) -> Result<Option<u8>, Error> {
        match self.source.chunk().get(self.at) {
            Some(&byte) => Ok(Some(byte)),
            None => self.next_chunk(),
        }
    }

    /// Moves to the next chunk that holds a byte and gives that byte. A token
    /// that has started is copied into the scratch buffer first, since the
    /// chunk it started in is gone after the move.
    #[cold]
    fn next_chunk(&mut self) -> Result<Option<u8>, Error> {
        loop {
            if self.ended {
                return Ok(None);
            }
            let kept = self.scratch.len();
            if let Some(start) = self.token {
                let tail = self.source.chunk().get(start..).unwrap_or_default();
                if !extend(&mut self.scratch, tail) {
                    return Err(self.fail(Code::OverMemory));
                }
            }
            let len = self.source.chunk().len();
            if !self.source.advance() {
                // The last chunk stays readable, so the copy is not needed.
                self.scratch.truncate(kept);
                self.ended = true;
                return Ok(None);
            }
            if self.token.is_some() {
                self.token = Some(0);
                self.copied = true;
            }
            self.base += len;
            self.at = 0;
            if let Some(&byte) = self.source.chunk().first() {
                return Ok(Some(byte));
            }
        }
    }

    /// Reads one byte, counting a line it ends, or `None` at the end of the
    /// text.
    fn next_byte(&mut self) -> Result<Option<u8>, Error> {
        let byte = self.peek()?;
        if let Some(byte) = byte {
            self.read_one(byte);
        }
        Ok(byte)
    }

    /// Reads the byte [`Reader::peek`] gave.
    const fn read_one(&mut self, byte: u8) {
        self.at += 1;
        if byte == b'\n' {
            self.line += 1;
            self.line_start = self.offset();
        }
    }

    /// Reads whitespace and gives the byte after it, unread.
    fn whitespace(&mut self) -> Result<Option<u8>, Error> {
        loop {
            while let Some(&byte) = self.source.chunk().get(self.at) {
                match byte {
                    b' ' | b'\t' | b'\r' | b'\n' => self.read_one(byte),
                    _ => return Ok(Some(byte)),
                }
            }
            if self.next_chunk()?.is_none() {
                return Ok(None);
            }
        }
    }

    /// The next token, with the grammar around it checked.
    fn step(&mut self) -> Result<Option<Token>, Error> {
        let token = match self.expect {
            Expect::Value => match self.whitespace()? {
                Some(byte) => self.value(byte)?,
                None => return Err(self.fail_peeked(Code::EofWhileParsingValue, None)),
            },
            Expect::FirstElement => match self.whitespace()? {
                Some(b']') => self.close(Token::EndArray),
                Some(byte) => self.value(byte)?,
                None => return Err(self.fail_peeked(Code::EofWhileParsingList, None)),
            },
            Expect::NextElement => match self.whitespace()? {
                Some(b']') => self.close(Token::EndArray),
                Some(b',') => {
                    self.at += 1;
                    match self.whitespace()? {
                        peeked @ Some(b']') => {
                            return Err(self.fail_peeked(Code::TrailingComma, peeked));
                        }
                        Some(byte) => self.value(byte)?,
                        None => return Err(self.fail_peeked(Code::EofWhileParsingValue, None)),
                    }
                }
                peeked @ Some(_) => {
                    return Err(self.fail_peeked(Code::ExpectedListCommaOrEnd, peeked));
                }
                None => return Err(self.fail_peeked(Code::EofWhileParsingList, None)),
            },
            Expect::FirstMember => match self.whitespace()? {
                Some(b'}') => self.close(Token::EndObject),
                Some(b'"') => self.key()?,
                peeked @ Some(_) => return Err(self.fail_peeked(Code::KeyMustBeAString, peeked)),
                None => return Err(self.fail_peeked(Code::EofWhileParsingObject, None)),
            },
            Expect::Colon => match self.whitespace()? {
                Some(b':') => {
                    self.at += 1;
                    match self.whitespace()? {
                        Some(byte) => self.value(byte)?,
                        None => return Err(self.fail_peeked(Code::EofWhileParsingValue, None)),
                    }
                }
                peeked @ Some(_) => return Err(self.fail_peeked(Code::ExpectedColon, peeked)),
                None => return Err(self.fail_peeked(Code::EofWhileParsingObject, None)),
            },
            Expect::NextMember => match self.whitespace()? {
                Some(b'}') => self.close(Token::EndObject),
                Some(b',') => {
                    self.at += 1;
                    match self.whitespace()? {
                        Some(b'"') => self.key()?,
                        peeked @ Some(b'}') => {
                            return Err(self.fail_peeked(Code::TrailingComma, peeked));
                        }
                        peeked @ Some(_) => {
                            return Err(self.fail_peeked(Code::KeyMustBeAString, peeked));
                        }
                        None => return Err(self.fail_peeked(Code::EofWhileParsingValue, None)),
                    }
                }
                peeked @ Some(_) => {
                    return Err(self.fail_peeked(Code::ExpectedObjectCommaOrEnd, peeked));
                }
                None => return Err(self.fail_peeked(Code::EofWhileParsingObject, None)),
            },
            Expect::End => match self.whitespace()? {
                peeked @ Some(_) => {
                    return Err(self.fail_peeked(Code::TrailingCharacters, peeked));
                }
                None => return Ok(None),
            },
        };
        Ok(Some(token))
    }

    /// The value that starts with `byte`, which is unread.
    fn value(&mut self, byte: u8) -> Result<Token, Error> {
        let token = match byte {
            b'n' => {
                self.at += 1;
                self.ident(b"ull")?;
                Token::Null
            }
            b't' => {
                self.at += 1;
                self.ident(b"rue")?;
                Token::Bool(true)
            }
            b'f' => {
                self.at += 1;
                self.ident(b"alse")?;
                Token::Bool(false)
            }
            b'-' | b'0'..=b'9' => Token::Number(self.number(byte)?),
            b'"' => {
                self.at += 1;
                Token::Str(self.string()?)
            }
            b'[' | b'{' => return self.open(byte),
            _ => return Err(self.fail_peeked(Code::ExpectedSomeValue, Some(byte))),
        };
        self.after_value();
        Ok(token)
    }

    /// What may follow a value: the next member of the container it is in, or
    /// the document's end.
    const fn after_value(&mut self) {
        self.expect = match self.depth.checked_sub(1) {
            None => Expect::End,
            Some(top) => {
                if self.open[top / 64] & (1 << (top % 64)) == 0 {
                    Expect::NextElement
                } else {
                    Expect::NextMember
                }
            }
        };
    }

    /// Opens the container `byte` starts, which is unread.
    fn open(&mut self, byte: u8) -> Result<Token, Error> {
        if self.depth >= self.max_open {
            return Err(self.fail_peeked(Code::TooDeep(self.max_depth), Some(byte)));
        }
        let (word, bit) = (self.depth / 64, 1 << (self.depth % 64));
        self.at += 1;
        self.depth += 1;
        Ok(if byte == b'{' {
            self.open[word] |= bit;
            self.expect = Expect::FirstMember;
            Token::StartObject
        } else {
            self.open[word] &= !bit;
            self.expect = Expect::FirstElement;
            Token::StartArray
        })
    }

    /// Closes the innermost container at its `]` or `}`, which is unread.
    const fn close(&mut self, token: Token) -> Token {
        self.at += 1;
        self.depth -= 1;
        self.after_value();
        token
    }

    /// The key whose `"` is unread.
    fn key(&mut self) -> Result<Token, Error> {
        self.at += 1;
        let span = self.string()?;
        self.expect = Expect::Colon;
        Ok(Token::Key(span))
    }

    /// The rest of `null`, `true` or `false`.
    fn ident(&mut self, rest: &[u8]) -> Result<(), Error> {
        for &want in rest {
            match self.next_byte()? {
                None => return Err(self.fail(Code::EofWhileParsingValue)),
                Some(byte) if byte != want => return Err(self.fail(Code::ExpectedSomeIdent)),
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// The string whose opening `"` is read, up to and with its closing `"`.
    fn string(&mut self) -> Result<Span, Error> {
        self.scratch.clear();
        self.copied = false;
        self.token = Some(self.at);
        loop {
            let chunk = self.source.chunk();
            self.at += stop_at(chunk.get(self.at..).unwrap_or_default());
            match chunk.get(self.at).copied() {
                Some(b'"') => {
                    let from = self.token.take().unwrap_or(0);
                    let to = self.at;
                    self.at += 1;
                    if !self.copied {
                        return Ok(Span::Chunk(from, to));
                    }
                    if !extend(&mut self.scratch, chunk.get(from..to).unwrap_or_default()) {
                        return Err(self.fail(Code::OverMemory));
                    }
                    return Ok(Span::Scratch);
                }
                Some(b'\\') => {
                    let from = self.token.take().unwrap_or(0);
                    if !extend(
                        &mut self.scratch,
                        chunk.get(from..self.at).unwrap_or_default(),
                    ) {
                        return Err(self.fail(Code::OverMemory));
                    }
                    self.copied = true;
                    self.at += 1;
                    self.escape()?;
                    self.token = Some(self.at);
                }
                Some(byte) => {
                    self.read_one(byte);
                    return Err(self.fail(Code::ControlCharacterWhileParsingString));
                }
                None => {
                    if self.next_chunk()?.is_none() {
                        return Err(self.fail(Code::EofWhileParsingString));
                    }
                }
            }
        }
    }

    /// The escape after a `\`, written into the scratch buffer.
    fn escape(&mut self) -> Result<(), Error> {
        let Some(byte) = self.next_byte()? else {
            return Err(self.fail(Code::EofWhileParsingString));
        };
        let written = match byte {
            b'"' | b'\\' | b'/' => byte,
            b'b' => 0x08,
            b'f' => 0x0c,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'u' => return self.unicode(),
            _ => return Err(self.fail(Code::InvalidEscape)),
        };
        self.push(&[written])
    }

    /// The `\u` escape whose `u` is read, with the second half of a surrogate
    /// pair. A surrogate without its pair is refused.
    fn unicode(&mut self) -> Result<(), Error> {
        let first = self.hex()?;
        let point = match first {
            0xDC00..=0xDFFF => return Err(self.fail(Code::LoneLeadingSurrogateInHexEscape)),
            0xD800..=0xDBFF => {
                for want in *b"\\u" {
                    match self.peek()? {
                        None => return Err(self.fail(Code::EofWhileParsingString)),
                        Some(byte) => {
                            self.read_one(byte);
                            if byte != want {
                                return Err(self.fail(Code::UnexpectedEndOfHexEscape));
                            }
                        }
                    }
                }
                let second = self.hex()?;
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return Err(self.fail(Code::LoneLeadingSurrogateInHexEscape));
                }
                0x1_0000 + ((u32::from(first - 0xD800) << 10) | u32::from(second - 0xDC00))
            }
            _ => u32::from(first),
        };
        let Some(point) = char::from_u32(point) else {
            return Err(self.fail(Code::InvalidUnicodeCodePoint));
        };
        self.push(point.encode_utf8(&mut [0; 4]).as_bytes())
    }

    /// Four hex digits. Fewer than four bytes before the end is the end of a
    /// string, and any byte that is not a hex digit is an invalid escape once
    /// all four are read.
    fn hex(&mut self) -> Result<u16, Error> {
        let mut value = 0;
        let mut valid = true;
        for _ in 0..4 {
            let Some(byte) = self.next_byte()? else {
                return Err(self.fail(Code::EofWhileParsingString));
            };
            match char::from(byte).to_digit(16) {
                Some(digit) => value = (value << 4) | u16::try_from(digit).unwrap_or(0),
                None => valid = false,
            }
        }
        if valid {
            Ok(value)
        } else {
            Err(self.fail(Code::InvalidEscape))
        }
    }

    /// Writes `bytes` into the scratch buffer, asking the budget first.
    fn push(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if extend(&mut self.scratch, bytes) {
            Ok(())
        } else {
            Err(self.fail(Code::OverMemory))
        }
    }

    /// The number that starts with `first`, which is unread: RFC 8259's
    /// grammar, read as `serde_json` reads it, with its errors where it reports
    /// them.
    fn number(&mut self, first: u8) -> Result<Number, Error> {
        self.scratch.clear();
        self.copied = false;
        self.token = Some(self.at);
        let negative = first == b'-';
        if negative {
            self.at += 1;
        }
        // The integer part, kept as a `u64` until it no longer fits.
        let mut value: u64 = 0;
        let mut fits = true;
        // Whether any digit before the exponent is not `0`.
        let mut nonzero = false;
        match self.next_byte()? {
            None => return Err(self.fail(Code::EofWhileParsingValue)),
            Some(b'0') => {
                let peeked = self.peek()?;
                if matches!(peeked, Some(b'0'..=b'9')) {
                    return Err(self.fail_peeked(Code::InvalidNumber, peeked));
                }
            }
            Some(digit @ b'1'..=b'9') => {
                value = u64::from(digit - b'0');
                nonzero = true;
                while let Some(digit @ b'0'..=b'9') = self.peek()? {
                    self.at += 1;
                    if fits {
                        match value
                            .checked_mul(10)
                            .and_then(|v| v.checked_add(u64::from(digit - b'0')))
                        {
                            Some(next) => value = next,
                            None => fits = false,
                        }
                    }
                }
            }
            Some(_) => return Err(self.fail(Code::InvalidNumber)),
        }
        let mut float = false;
        if self.peek()? == Some(b'.') {
            self.at += 1;
            float = true;
            let mut any = false;
            while let Some(digit @ b'0'..=b'9') = self.peek()? {
                self.at += 1;
                any = true;
                nonzero |= digit != b'0';
            }
            if !any {
                let peeked = self.peek()?;
                let code = if peeked.is_some() {
                    Code::InvalidNumber
                } else {
                    Code::EofWhileParsingValue
                };
                return Err(self.fail_peeked(code, peeked));
            }
        }
        if matches!(self.peek()?, Some(b'e' | b'E')) {
            self.at += 1;
            float = true;
            let mut positive = true;
            match self.peek()? {
                Some(b'+') => self.at += 1,
                Some(b'-') => {
                    self.at += 1;
                    positive = false;
                }
                _ => {}
            }
            let mut exponent: i32 = match self.next_byte()? {
                None => return Err(self.fail(Code::EofWhileParsingValue)),
                Some(digit @ b'0'..=b'9') => i32::from(digit - b'0'),
                Some(_) => return Err(self.fail(Code::InvalidNumber)),
            };
            while let Some(digit @ b'0'..=b'9') = self.peek()? {
                self.at += 1;
                match exponent
                    .checked_mul(10)
                    .and_then(|e| e.checked_add(i32::from(digit - b'0')))
                {
                    Some(next) => exponent = next,
                    // `serde_json`'s overflow: a value that is not zero with
                    // a huge positive exponent is out of range at this digit,
                    // and anything else is zero.
                    None => {
                        if nonzero && positive {
                            return Err(self.fail(Code::NumberOutOfRange));
                        }
                        while let Some(b'0'..=b'9') = self.peek()? {
                            self.at += 1;
                        }
                        self.token = None;
                        return Ok(Number::Float(if negative { -0.0 } else { 0.0 }));
                    }
                }
            }
        }
        if !float && fits && (!negative || value <= 1 << 63) {
            self.token = None;
            return Ok(if !negative {
                i64::try_from(value).map_or(Number::Uint(value), Number::Int)
            } else if value == 0 {
                Number::Float(-0.0)
            } else {
                Number::Int(0_i64.wrapping_sub_unsigned(value))
            });
        }
        if !float {
            let digits = Digits::of(&self.token_bytes(usize::from(negative))?);
            return Ok(Number::Big { negative, digits });
        }
        let parsed = std::str::from_utf8(&self.token_bytes(0)?)
            .ok()
            .and_then(|text| text.parse::<f64>().ok());
        match parsed {
            Some(number) if number.is_finite() => Ok(Number::Float(number)),
            Some(_) => Err(self.fail(Code::NumberOutOfRange)),
            None => Err(self.fail(Code::InvalidNumber)),
        }
    }

    /// The token just read, from its first byte past `skip`, and the token is
    /// over. Only a number reads it, so a short copy is all it costs.
    fn token_bytes(&mut self, skip: usize) -> Result<Vec<u8>, Error> {
        let start = self.token.take().unwrap_or(0);
        let chunk = self.source.chunk().get(start..self.at).unwrap_or_default();
        let mut bytes = Vec::new();
        let head = if self.copied { &self.scratch[..] } else { &[] };
        if !extend(&mut bytes, head) || !extend(&mut bytes, chunk) {
            return Err(self.fail(Code::OverMemory));
        }
        bytes.drain(..skip.min(bytes.len()));
        Ok(bytes)
    }
}

/// Appends `bytes` to `scratch`, asking the request's budget for any growth
/// first. `false` where the budget or the allocator refuses.
fn extend(scratch: &mut Vec<u8>, bytes: &[u8]) -> bool {
    let spare = scratch.capacity() - scratch.len();
    if bytes.len() > spare {
        let want = scratch
            .len()
            .saturating_add(bytes.len())
            .max(scratch.capacity().saturating_mul(2))
            .max(16);
        if !nvs_runtime::budget::affords(want - scratch.capacity())
            || scratch.try_reserve_exact(want - scratch.len()).is_err()
        {
            return false;
        }
    }
    scratch.extend_from_slice(bytes);
    true
}

/// The index of the first byte in `bytes` that ends a run of plain string
/// text: `"`, `\` or a control byte. Eight bytes are tested at a time, with the
/// zero-byte test from Alan Mycroft's `strlen`, which is exact for the lowest
/// match in a word.
fn stop_at(bytes: &[u8]) -> usize {
    const ONES: u64 = u64::MAX / 255;
    const HIGH: u64 = ONES << 7;
    let (words, rest) = bytes.as_chunks::<8>();
    let mut index = 0;
    for &word in words {
        let w = u64::from_le_bytes(word);
        let control = w.wrapping_sub(ONES * 0x20) & !w;
        let quote = w ^ (ONES * u64::from(b'"'));
        let quote = quote.wrapping_sub(ONES) & !quote;
        let backslash = w ^ (ONES * u64::from(b'\\'));
        let backslash = backslash.wrapping_sub(ONES) & !backslash;
        let found = (control | quote | backslash) & HIGH;
        if found != 0 {
            return index + (found.trailing_zeros() / 8) as usize;
        }
        index += 8;
    }
    index
        + rest
            .iter()
            .position(|&byte| byte == b'"' || byte == b'\\' || byte < 0x20)
            .unwrap_or(rest.len())
}

#[cfg(test)]
mod tests;
