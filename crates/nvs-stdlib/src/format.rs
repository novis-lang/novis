//! `Core\Str::format`'s `printf` grammar — the one `Core` member that reads a
//! template rather than an option.
//!
//! # Why the grammar is `printf`'s, and why it is written here
//!
//! Novis keeps PHP's `printf` grammar deliberately, so this is a *closed*
//! conversion list (`%s %d %u %f %e %g %x %X %o %b %%`) with `printf`'s flag,
//! width, precision and `%1$s` positional syntax, minus everything that reads
//! ambient state. Nothing outside is accepted: an unknown conversion character
//! throws rather than being copied through, which is the difference between a
//! typo that is reported and one that silently ships.
//!
//! No crate implements it. The `printf`-alike crates on crates.io format Rust
//! values or re-expose C's `vsnprintf`; what is needed here is the grammar
//! applied to a [`Value`] — a tagged union whose conversion rows are
//! `rule:types/conversion`'s and
//! `rule:types/conversion`'s, not
//! Rust's `Display`. Adapting one would be more code than the grammar, so
//! `rule:packaging/a-c-dependency-answers-two-questions`'s first
//! question answers itself: this is Novis's own semantics, not an external
//! specification someone else maintains.
//!
//! # What it refuses, and why each is a throw
//!
//! `rule:core-api/shape-rules` R4 makes
//! failure a throw, and PHP's `printf` argument-mismatch bug family is exactly
//! what this turns into an error:
//!
//! * **Too few arguments** for the placeholders written — PHP 8 throws here
//!   too, so this is compatibility rather than divergence.
//! * **An argument no placeholder consumed.** PHP ignores a trailing extra;
//!   here it throws, because an unread argument is a mismatch in the same
//!   family and silence is what made that family a bug family. Positional
//!   placeholders make it a real rule rather than a count: every argument must
//!   be named by at least one placeholder, in any order and any number of
//!   times.
//! * **A value with no reading for its conversion** — an array for `%d`, a
//!   non-integral `decimal` for `%x`. The `decimal` row is `rule:types/conversion`'s,
//!   which makes rounding something the program says out loud.
//!
//! # One parse, two entry points
//!
//! `format` is an
//! `rule:expressions/intrinsic-constant-arguments`
//! intrinsic: a **literal** template has its placeholder count and types
//! checked while compiling. That rule is why the checker does not get a
//! parser of its own — [`Pieces`] is the one walk over the grammar, [`format`]
//! drives it to render and [`placeholders`] drives it to describe, so a
//! template the compiler accepts is exactly one the runtime accepts and a
//! diagnostic quotes the message the throw would have carried.
//!
//! What the two do *not* share is the argument reading: [`format`] has values
//! and reads them, while the checker has only static types and refuses a
//! placeholder no value of that type could ever satisfy
//! (`nvs_types::intrinsics`). Anything narrower than that stays a throw.
//!
//! # Width and precision count what `Core\Str::length` counts
//!
//! `%10s` pads to ten of [`crate::granularity::DEFAULT`]'s units and `%.2s`
//! truncates to two of them, so `format` measures a `string` the way
//! `length`, `at` and `padStart` do rather than the way C counts bytes.
//! `rule:types/string-is-utf8` owns that unit; restating it per member is what
//! would let two members drift apart.

use nvs_runtime::{Fault, NvsStr, StrWriter, Tag, ThrownClass, Value};

use crate::granularity::DEFAULT;

/// `template` with every placeholder replaced from `arguments` — the whole of
/// `Core\Str::format`, and the only entry point.
///
/// # Errors
///
/// A `LogicError` for every refusal this module's own docs list: a malformed
/// or unknown placeholder, an argument count that does not match the
/// placeholders, or a value with no reading for the conversion it reached.
/// Every one is a bug in the program — the template is the
/// program's own grammar and the arguments its own call, never input — which
/// is the side `Core\Time::parse` puts a bad *pattern* on, as against the
/// `ParseError` it keeps for text.
///
/// # Allocation
///
/// One per call: the result, written straight into the `string` the member
/// returns. A `string` argument is borrowed, an integer is written on the
/// stack, and only a float conversion builds a `String` of its own. The
/// capacity is the template plus each `string` argument's length and 20 bytes
/// for any other argument, so most calls never grow. A `%.Ns` over a long
/// argument spends that argument's length once, for the duration of the call.
/// On an error, the allocation already made is freed with the result.
pub(crate) fn format(template: &str, arguments: &[Value]) -> Result<NvsStr, Fault> {
    let capacity = arguments.iter().fold(template.len(), |sum, argument| {
        sum.saturating_add(argument.as_str_bytes().map_or(20, <[u8]>::len))
    });
    let mut outcome = Ok(());
    let built = NvsStr::build(capacity, |out| outcome = render(template, arguments, out));
    outcome.map(|()| built)
}

/// [`format`]'s rendering loop, writing into the result's allocation.
fn render(template: &str, arguments: &[Value], out: &mut StrWriter<'_>) -> Result<(), Fault> {
    let mut read = Read::new(arguments.len());
    let mut next = 0usize;
    for piece in Pieces::new(template) {
        match piece.map_err(|why| Fault::thrown_as(ThrownClass::Logic, why.message))? {
            Piece::Text(text) => out.push_str(text),
            Piece::Spec(spec, _) => {
                let index = spec.index(&mut next);
                let argument = arguments.get(index).ok_or_else(|| {
                    Fault::thrown_as(ThrownClass::Logic, reads_missing(index, arguments.len()))
                })?;
                read.mark(index);
                spec.afford()?;
                let body = spec.convert(argument)?;
                spec.pad_into(spec.truncated(body.as_str()), out);
            }
        }
    }
    if let Some(unused) = read.first_unread() {
        return Err(Fault::thrown_as(ThrownClass::Logic, never_read(unused)));
    }
    Ok(())
}

/// The symbol one placeholder of a literal `Core\Str::format` template lowers
/// to, where the checker could not fold the placeholder into plain text
/// (`nvs_types::expr_table::FormatPiece::Converted` and `::Rendered`). With
/// it, no literal template is parsed at run time.
///
/// Row-less for `crate::html::MARKUP_SYMBOL`'s reason: the call's own
/// lowering is the only thing that reaches it, and a row would make it
/// callable by name.
pub const PIECE_SYMBOL: &str = "nvs_core_str_format_piece";

nvs_runtime::nvs_helper! {
    /// One placeholder of a literal template, rendered by [`format`] exactly
    /// as `Core\Str::format` renders it. Argument 0 is the placeholder as it
    /// was written, such as `%05d`, and argument 1 is the value it reads.
    ///
    /// One allocation, the result. The placeholder is parsed again on every
    /// call, which is a few bytes of a constant.
    fn nvs_core_str_format_piece(_ctx, args: [2]) {
        // Unreachable from source: no row names this symbol, and the one
        // lowering that calls it passes the placeholder as a string literal.
        let spec = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Str::format expected a placeholder, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let rendered = format(spec, &args[1..])?;
        #[expect(
            clippy::disallowed_methods,
            reason = "`format` writes only `&str` pieces, through `StrWriter::push_str`"
        )]
        let piece = Value::str(rendered);
        Ok(piece)
    }
}

/// The address of this module's one symbol, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    (symbol == PIECE_SYMBOL).then(|| (nvs_core_str_format_piece as *const ()).cast())
}

/// Which arguments a placeholder has read: one bit per argument, and a list
/// only for a call with more than 64 of them.
struct Read {
    bits: u64,
    wide: Vec<bool>,
    count: usize,
}

impl Read {
    fn new(count: usize) -> Self {
        let wide = match count > 64 {
            true => vec![false; count],
            false => Vec::new(),
        };
        Self {
            bits: 0,
            wide,
            count,
        }
    }

    /// Records `index` as read. The caller has already checked it against
    /// the argument count, so a narrow call's index is below 64.
    fn mark(&mut self, index: usize) {
        match self.wide.get_mut(index) {
            Some(seen) => *seen = true,
            None => self.bits |= 1 << index,
        }
    }

    /// The first argument no placeholder read.
    fn first_unread(&self) -> Option<usize> {
        if !self.wide.is_empty() {
            return self.wide.iter().position(|seen| !seen);
        }
        let all = match self.count {
            64 => u64::MAX,
            count => (1 << count) - 1,
        };
        let unread = all & !self.bits;
        (unread != 0).then(|| unread.trailing_zeros() as usize)
    }
}

/// One placeholder's argument as text, before truncation and padding.
enum Body<'a> {
    /// The argument is a `string`, used as it is.
    Borrowed(&'a str),
    /// An integer, written on the stack.
    Digits(Digits),
    /// A float conversion, or a `%s` of a value that is not a string or an
    /// integer.
    Owned(String),
}

impl Body<'_> {
    fn as_str(&self) -> &str {
        match self {
            Self::Borrowed(text) => text,
            Self::Digits(digits) => digits.as_str(),
            Self::Owned(text) => text,
        }
    }
}

/// An integer's text in a buffer on the stack. The widest integer rendering
/// is a `%b` of `u64::MAX` with a `+` in front, 65 bytes, so it always fits.
struct Digits {
    bytes: [u8; 72],
    len: usize,
}

impl Digits {
    fn of(arguments: std::fmt::Arguments<'_>) -> Self {
        let mut digits = Self {
            bytes: [0; 72],
            len: 0,
        };
        // Every caller writes one integer, which always fits, so this
        // write cannot fail.
        let _ = std::fmt::Write::write_fmt(&mut digits, arguments);
        digits
    }

    fn as_str(&self) -> &str {
        // Integer formatting writes ASCII only, so this is always valid.
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }
}

impl std::fmt::Write for Digits {
    fn write_str(&mut self, piece: &str) -> std::fmt::Result {
        let end = self.len + piece.len();
        let room = self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?;
        room.copy_from_slice(piece.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// One placeholder as the *checker* sees it: which argument it reads and what
/// conversion it applies, with everything only a rendering needs — the flags,
/// the width, the precision — left behind.
///
/// `rule:expressions/intrinsic-list-is-closed`'s `Core\Str::format` row is checked against exactly this, so
/// [`crate::format`]'s grammar stays the one thing that says what a
/// placeholder is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placeholder {
    /// The **0-based** argument this placeholder reads, `%1$s`'s own numbering
    /// already resolved against the running position.
    pub index: usize,
    /// The conversion character, already one of the closed list.
    pub conversion: char,
    /// Whether the placeholder is the conversion alone, with no argument
    /// number, flag, width or precision: `%d`, never `%1$d`, `%+d` or `%5d`.
    pub plain: bool,
    /// Where in the template this one was written, so a refusal about it can
    /// underline it rather than the literal that carried it.
    pub written: Written,
}

/// Where something the template grammar read was written in the template: the
/// offset of its opening `%` and how far from there it runs.
///
/// Offsets are into the template *as a value* — the cooked string, escapes
/// already applied — which is the only form this module ever sees. Mapping one
/// back to a column in the source a literal was written at is the caller's
/// half, and `nvs_syntax::string_lit::cook_string_literal_positions` is what
/// answers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Written {
    /// Byte offset of the opening `%`.
    pub at: usize,
    /// Bytes from [`Self::at`] this covers — the whole placeholder, or for a
    /// refusal exactly the text its message quotes back.
    pub len: usize,
}

/// A placeholder this grammar cannot read, with the words [`format`] would
/// have thrown and where the offending text sits in the template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Malformed {
    /// The text of the `LogicError` [`format`] raises on the same template, so
    /// a diagnostic quotes the runtime's own words rather than a second
    /// wording of one refusal.
    pub message: String,
    /// The text [`Self::message`] quotes, located in the template.
    pub written: Written,
}

/// Every placeholder in `template`, in written order, or the message the
/// runtime would have thrown — `rule:expressions/intrinsic-list-is-closed`'s validation half of this module.
///
/// The count and the argument types are *not* checked here: this answers what
/// the template asks for, and `nvs_types::intrinsics` is what holds the call's
/// argument list beside it. A `Vec` rather than the iterator itself because
/// the checker asks the whole template two questions at once — the highest
/// index it reads, and whether every argument is read — and neither is
/// answerable one placeholder at a time.
///
/// # Errors
///
/// [`Malformed`], carrying the words [`format`] would have thrown on the same
/// template and where in it the offending text was written.
pub fn placeholders(template: &str) -> Result<Vec<Placeholder>, Malformed> {
    let mut found = Vec::new();
    let mut next = 0usize;
    for piece in Pieces::new(template) {
        if let Piece::Spec(spec, written) = piece? {
            found.push(Placeholder {
                index: spec.index(&mut next),
                conversion: spec.conversion,
                plain: spec.argnum.is_none()
                    && !spec.left
                    && !spec.plus
                    && spec.pad == ' '
                    && spec.width == 0
                    && spec.precision.is_none(),
                written,
            });
        }
    }
    Ok(found)
}

/// What a template the caller has too few arguments for throws — written once
/// because [`placeholders`]' caller reports the same fact while checking.
#[must_use]
pub fn reads_missing(index: usize, given: usize) -> String {
    format!(
        "Core\\Str::format(): the template reads argument {} of {given}",
        index + 1
    )
}

/// What an argument no placeholder consumed throws — [`reads_missing`]'s
/// sibling, shared for its reason.
#[must_use]
pub fn never_read(index: usize) -> String {
    format!(
        "Core\\Str::format(): argument {} is never read by the template",
        index + 1
    )
}

/// One run of a template: literal text, or a placeholder.
///
/// A `%%` arrives as [`Self::Text`] carrying the single `%` it stands for, so
/// nothing downstream has to know the escape exists.
enum Piece<'a> {
    Text(&'a str),
    Spec(Spec, Written),
}

/// The template grammar's one walk, borrowing the template and allocating
/// nothing — [`format`] renders each piece, [`placeholders`] describes them.
///
/// An error ends the iteration: the parser cannot know where the next
/// placeholder starts once one is malformed, and the runtime threw on the
/// first refusal anyway.
struct Pieces<'a> {
    rest: &'a str,
    /// Where `rest` begins in the template it was made from — the running
    /// total that lets a piece say where it was written without the template
    /// being threaded alongside it.
    at: usize,
}

impl<'a> Pieces<'a> {
    const fn new(template: &'a str) -> Self {
        Self {
            rest: template,
            at: 0,
        }
    }
}

impl<'a> Iterator for Pieces<'a> {
    type Item = Result<Piece<'a>, Malformed>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let start = self.at;
        let Some(at) = self.rest.find('%') else {
            self.at += self.rest.len();
            return Some(Ok(Piece::Text(std::mem::take(&mut self.rest))));
        };
        if at > 0 {
            let (text, rest) = self.rest.split_at(at);
            self.rest = rest;
            self.at += at;
            return Some(Ok(Piece::Text(text)));
        }
        let after = &self.rest[1..];
        if let Some(tail) = after.strip_prefix('%') {
            self.rest = tail;
            self.at += 2;
            // The `%` this escape stands for, sliced out of the template so the
            // piece borrows rather than owning.
            return Some(Ok(Piece::Text(&after[..1])));
        }
        match Spec::parse(after) {
            Ok((spec, tail)) => {
                let len = self.rest.len() - tail.len();
                self.rest = tail;
                self.at += len;
                Some(Ok(Piece::Spec(spec, Written { at: start, len })))
            }
            Err(mut why) => {
                self.rest = "";
                // `Spec::parse` measures from the `%` it was handed the text
                // after, so the offset it reports is relative to this piece.
                why.written.at += start;
                Some(Err(why))
            }
        }
    }
}

/// One parsed placeholder: `%[argnum$][flags][width][.precision]conversion`.
struct Spec {
    /// The 1-based argument `%1$s` names, or `None` for the next unnamed one.
    argnum: Option<usize>,
    /// The `-` flag: pad on the right rather than the left.
    left: bool,
    /// The `+` flag: write a `+` before a non-negative number.
    plus: bool,
    /// What padding is made of — a space, `0` for the `0` flag, or the
    /// character after `'`.
    pad: char,
    /// The minimum rendered width, in [`DEFAULT`]'s units.
    width: usize,
    /// `.n`: significant digits for `%g`, digits after the point for
    /// `%f`/`%e`, a maximum length for `%s`, and nothing at all for the
    /// integer conversions.
    precision: Option<usize>,
    /// The conversion character, already checked against the closed list.
    conversion: char,
}

impl Spec {
    /// Parses one placeholder off the front of `after` — the text following
    /// the `%`, with `%%` already handled by the caller — and returns it with
    /// whatever follows it.
    ///
    /// Refuses with a message rather than a [`Fault`]: this is the half of the
    /// module both entry points share, and only [`format`] is in a position to
    /// throw. See the module docs' *One parse, two entry points*.
    fn parse(after: &str) -> Result<(Self, &str), Malformed> {
        let bytes = after.as_bytes();
        let mut at = 0usize;

        // `1$` is an argument number only when the `$` is actually there —
        // `%12d` is a width, so the digits are left for the width scan below
        // when it is not. Reading ahead rather than committing is the whole
        // reason this is written over byte indices.
        let mut argnum = None;
        let mut digits = 0usize;
        while bytes.get(digits).is_some_and(u8::is_ascii_digit) {
            digits += 1;
        }
        if digits > 0 && bytes.get(digits) == Some(&b'$') {
            let parsed = after[..digits].parse::<usize>().ok().filter(|n| *n > 0);
            argnum = Some(parsed.ok_or_else(|| malformed(after))?);
            at = digits + 1;
        }

        let mut left = false;
        let mut plus = false;
        let mut pad = ' ';
        loop {
            match bytes.get(at) {
                Some(b'-') => left = true,
                Some(b'+') => plus = true,
                // PHP's space flag asks for the padding that is already the
                // default, so it is accepted and means nothing.
                Some(b' ') => {}
                Some(b'0') => pad = '0',
                Some(b'\'') => {
                    let c = after[at + 1..]
                        .chars()
                        .next()
                        .ok_or_else(|| malformed(after))?;
                    pad = c;
                    at += 1 + c.len_utf8();
                    continue;
                }
                _ => break,
            }
            at += 1;
        }

        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        let width = match at == start {
            true => 0,
            false => after[start..at]
                .parse::<usize>()
                .map_err(|_| malformed(after))?,
        };

        let mut precision = None;
        if bytes.get(at) == Some(&b'.') {
            at += 1;
            let start = at;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            // A bare `.` is `.0`, which is C's own reading.
            precision = Some(match at == start {
                true => 0,
                false => after[start..at]
                    .parse::<usize>()
                    .map_err(|_| malformed(after))?,
            });
        }

        let conversion = after[at..].chars().next().ok_or_else(|| malformed(after))?;
        at += conversion.len_utf8();
        if !matches!(
            conversion,
            's' | 'd' | 'u' | 'f' | 'e' | 'g' | 'x' | 'X' | 'o' | 'b'
        ) {
            return Err(Malformed {
                message: format!(
                    "Core\\Str::format(): `%{conversion}` is not one of the conversions the \
                     template grammar allows (`%s %d %u %f %e %g %x %X %o %b %%`)"
                ),
                // The whole placeholder, `%` through the conversion that ended
                // it: the flags and width ahead of it read without complaint,
                // but they are part of the one placeholder being refused and
                // underlining the conversion alone points inside a construct
                // rather than at it.
                written: Written { at: 0, len: at + 1 },
            });
        }
        Ok((
            Self {
                argnum,
                left,
                plus,
                pad,
                width,
                precision,
                conversion,
            },
            &after[at..],
        ))
    }

    /// Which argument this placeholder reads, 0-based, advancing `next` when
    /// it is an unnumbered one.
    ///
    /// The one place `%1$s`'s numbering is resolved, because the running
    /// position it interacts with is state a single placeholder cannot see:
    /// a numbered placeholder does not consume a position, so `%2$s %s` reads
    /// arguments 2 and 1 in that order.
    fn index(&self, next: &mut usize) -> usize {
        match self.argnum {
            Some(argnum) => argnum - 1,
            None => {
                let index = *next;
                *next += 1;
                index
            }
        }
    }

    /// Refuses a width or a precision the request cannot afford, before
    /// anything is built.
    ///
    /// Both are counts written into the template, so `%99999999999s` asks
    /// for that many bytes of padding and `%.99999999999f` for that many
    /// digits. Without this the allocator meets them first and aborts the
    /// process instead of the request. The bound is the padding at the pad
    /// character's width, plus the digits of a float conversion and 400 bytes
    /// for the widest `f64` in front of them; `%s` only truncates, so its
    /// precision costs nothing.
    fn afford(&self) -> Result<(), Fault> {
        let digits = match self.conversion {
            'f' | 'e' | 'g' => self.precision.unwrap_or(6).checked_add(400),
            _ => Some(0),
        };
        let bytes = self
            .width
            .checked_mul(self.pad.len_utf8())
            .zip(digits)
            .and_then(|(padding, digits)| padding.checked_add(digits));
        nvs_runtime::affordable(bytes, "Core\\Str::format()").map(drop)
    }

    /// This placeholder's argument rendered, before truncation and padding.
    fn convert<'a>(&self, argument: &'a Value) -> Result<Body<'a>, Fault> {
        match self.conversion {
            's' => text_of(argument),
            'd' => {
                let value = integer(argument, 'd')?;
                Ok(Body::Digits(match self.plus && value >= 0 {
                    true => Digits::of(format_args!("+{value}")),
                    false => Digits::of(format_args!("{value}")),
                }))
            }
            // PHP's `%u` prints the two's-complement reading of a negative
            // `int`, which is the whole point of having it beside `%d`.
            #[expect(
                clippy::cast_sign_loss,
                reason = "the reinterpretation is what `%u` asks for"
            )]
            'u' => Ok(Body::Digits(Digits::of(format_args!(
                "{}",
                integer(argument, 'u')? as u64
            )))),
            #[expect(
                clippy::cast_sign_loss,
                reason = "a base conversion is over the bit pattern, as PHP's is"
            )]
            'x' | 'X' | 'o' | 'b' => {
                let bits = integer(argument, self.conversion)? as u64;
                Ok(Body::Digits(match self.conversion {
                    'x' => Digits::of(format_args!("{bits:x}")),
                    'X' => Digits::of(format_args!("{bits:X}")),
                    'o' => Digits::of(format_args!("{bits:o}")),
                    _ => Digits::of(format_args!("{bits:b}")),
                }))
            }
            'f' => Ok(Body::Owned(self.signed_text(fixed(
                floating(argument, 'f')?,
                self.precision.unwrap_or(6),
            )))),
            'e' => Ok(Body::Owned(self.signed_text(scientific(
                floating(argument, 'e')?,
                self.precision.unwrap_or(6),
            )))),
            _ => Ok(Body::Owned(self.signed_text(shortest(
                floating(argument, 'g')?,
                self.precision.unwrap_or(6),
            )))),
        }
    }

    /// `body` cut to a `%s` precision, or `body` unchanged for every other
    /// placeholder.
    fn truncated<'b>(&self, body: &'b str) -> &'b str {
        match self.precision {
            Some(precision) if self.conversion == 's' && precision < DEFAULT.length(body) => {
                &body[..DEFAULT.byte_of_index(body, precision)]
            }
            _ => body,
        }
    }

    /// The same, for a float already rendered — its own sign is already in the
    /// text, so only a missing `+` is added.
    fn signed_text(&self, text: String) -> String {
        match self.plus && !text.starts_with('-') {
            true => format!("+{text}"),
            false => text,
        }
    }

    /// `body` padded to [`Self::width`] and appended to `out`.
    ///
    /// Zero padding goes *after* the sign — `%08.3f` of `-3.14159` is
    /// `-003.142`, not `00-3.142` — which is the one place the pad character
    /// changes where the padding lands.
    ///
    /// A placeholder with no width writes `body` without measuring it.
    fn pad_into(&self, body: &str, out: &mut StrWriter<'_>) {
        if self.width == 0 {
            out.push_str(body);
            return;
        }
        let length = DEFAULT.length(body);
        let Some(short) = self.width.checked_sub(length).filter(|short| *short > 0) else {
            out.push_str(body);
            return;
        };
        if self.left {
            out.push_str(body);
            self.pad_run(short, out);
            return;
        }
        let sign = match self.pad == '0' && body.starts_with(['-', '+']) {
            true => 1,
            false => 0,
        };
        out.push_str(&body[..sign]);
        self.pad_run(short, out);
        out.push_str(&body[sign..]);
    }

    /// `count` pad characters, appended to `out`.
    fn pad_run(&self, count: usize, out: &mut StrWriter<'_>) {
        let mut encoded = [0u8; 4];
        let pad = self.pad.encode_utf8(&mut encoded);
        for _ in 0..count {
            out.push_str(pad);
        }
    }
}

/// One `%s` argument's text. A `string` is used as it is, an `int` or a
/// `uint` is written on the stack, and every other value goes through
/// [`rendered`]. Each fast arm gives the same text `rendered` gives.
fn text_of(argument: &Value) -> Result<Body<'_>, Fault> {
    if let Some(text) = argument.as_text() {
        return Ok(Body::Borrowed(text));
    }
    match argument.tag() {
        Some(Tag::Int) => {
            if let Some(value) = argument.as_int() {
                return Ok(Body::Digits(Digits::of(format_args!("{value}"))));
            }
        }
        Some(Tag::Uint) => {
            if let Some(value) = argument.as_uint() {
                return Ok(Body::Digits(Digits::of(format_args!("{value}"))));
            }
        }
        _ => {}
    }
    rendered(argument).map(Body::Owned)
}

/// One `%s` argument's text — `rule:types/conversion`'s rows, reached through the one implementation of them.
///
/// `nvs_runtime::value_to_string` answers a `Tag::Str` carrying exactly one
/// fresh reference, so the handle below owns it and releases it when the
/// borrowed text has been copied out.
fn rendered(argument: &Value) -> Result<String, Fault> {
    let value = nvs_runtime::value_to_string(*argument)?;
    // The tag is `rule:types/bytes`'s UTF-8 guarantee, so reading the payload as text is
    // the same check as reading it at all — `nvs_runtime`'s `string` module owns
    // that argument in its § *Reading the payload as text*. Re-deriving it here
    // was an O(n) pass per rendered argument, on `format`'s own hot path.
    //
    // Both halves are a post-condition of the call above rather than a
    // boundary, and so unreachable from source with no diagnostic to name:
    // every `Ok` arm of `value_to_string` builds a `Value::str`, `rule:security/capture-answers-the-carrier`'s
    // carrier arm included, so this is a `Tag::Str` or it is the `Err` the `?`
    // above already took.
    let (Some(ptr), Some(text)) = (value.str_ptr(), value.as_text()) else {
        return Err(Fault::fatal(
            "`value_to_string` answered something that is not a string",
        ));
    };
    let text = text.to_owned();
    #[expect(
        unsafe_code,
        reason = "`value_to_string` hands back exactly one fresh reference, and this handle is \
                  the thing that releases it"
    )]
    let _owned = unsafe { NvsStr::from_raw(ptr) };
    Ok(text)
}

/// One argument as the `int` an integer conversion needs.
///
/// A `float` truncates toward zero, as PHP's own `%d` does. A `decimal`
/// follows `rule:types/conversion` instead: integral and in range, or a throw naming the
/// rounding member, because that rule allows no silent rounding.
#[expect(
    clippy::cast_possible_truncation,
    reason = "truncating toward zero is what `%d` over a float means"
)]
fn integer(argument: &Value, conversion: char) -> Result<i64, Fault> {
    match argument.tag() {
        Some(Tag::Int) => argument.as_int().ok_or_else(|| unreadable(conversion)),
        #[expect(
            clippy::cast_possible_wrap,
            reason = "`%u` and the base conversions read the same bit pattern back"
        )]
        Some(Tag::Uint) => argument
            .as_uint()
            .map(|n| n as i64)
            .ok_or_else(|| unreadable(conversion)),
        Some(Tag::Bool) => Ok(i64::from(argument.as_bool() == Some(true))),
        Some(Tag::Float) => argument
            .as_float()
            .map(|n| n as i64)
            .ok_or_else(|| unreadable(conversion)),
        Some(Tag::Decimal) => argument
            .as_decimal()
            .and_then(nvs_runtime::Decimal::to_i64)
            .ok_or_else(|| {
                Fault::thrown_as(
                    ThrownClass::Logic,
                    format!(
                        "Core\\Str::format(): `%{conversion}` needs a whole number, and this \
                     `decimal` is not one — round it with `Core\\Math::floor`, `::ceil` or \
                     `::round` first"
                    ),
                )
            }),
        _ => Err(unreadable(conversion)),
    }
}

/// One argument as the `float` a `%f`/`%e`/`%g` needs. A `decimal` converts by
/// `rule:types/conversion`'s `decimal → float` row — nearest `f64`, lossy, and asked for
/// explicitly by writing a float conversion.
#[expect(
    clippy::cast_precision_loss,
    reason = "an integer wider than 2^53 loses precision on the way to `%f`, which is what asking \
              for a float conversion means"
)]
fn floating(argument: &Value, conversion: char) -> Result<f64, Fault> {
    match argument.tag() {
        Some(Tag::Float) => argument.as_float().ok_or_else(|| unreadable(conversion)),
        Some(Tag::Int) => argument
            .as_int()
            .map(|n| n as f64)
            .ok_or_else(|| unreadable(conversion)),
        Some(Tag::Uint) => argument
            .as_uint()
            .map(|n| n as f64)
            .ok_or_else(|| unreadable(conversion)),
        Some(Tag::Bool) => Ok(f64::from(u8::from(argument.as_bool() == Some(true)))),
        Some(Tag::Decimal) => argument
            .as_decimal()
            .map(nvs_runtime::Decimal::to_f64)
            .ok_or_else(|| unreadable(conversion)),
        _ => Err(unreadable(conversion)),
    }
}

/// `%f`: fixed point, with `precision` digits after the point.
fn fixed(value: f64, precision: usize) -> String {
    match value.is_finite() {
        true => format!("{value:.precision$}"),
        false => nonfinite(value),
    }
}

/// `%e`: one digit, a point, `precision` digits, then `e` and a signed
/// exponent of as few digits as it takes — PHP's shape (`1.234500e+3`), not
/// C's two-digit minimum.
fn scientific(value: f64, precision: usize) -> String {
    if !value.is_finite() {
        return nonfinite(value);
    }
    let rendered = format!("{value:.precision$e}");
    // Rust writes the exponent without a `+`; everything else already agrees.
    match rendered.split_once('e') {
        Some((mantissa, exponent)) if !exponent.starts_with('-') => {
            format!("{mantissa}e+{exponent}")
        }
        _ => rendered,
    }
}

/// `%g`: `precision` *significant* digits, rendered as `%f` or `%e` by C's own
/// rule, then with the trailing zeros that rule leaves behind removed.
///
/// The one PHP-specific part is what "removed" means in the `%e` form: PHP
/// keeps a digit after the point (`1.0e+6`, never `1e+6`), while the `%f` form
/// drops the point along with the zeros (`999999`, `0.5`).
fn shortest(value: f64, precision: usize) -> String {
    if !value.is_finite() {
        return nonfinite(value);
    }
    let significant = precision.max(1);
    // The decimal exponent *after* rounding to `significant` digits, read back
    // off the scientific form so that 999999.5 at six digits reports 6.
    let scientific = format!("{value:.*e}", significant - 1);
    let exponent: i32 = scientific
        .split_once('e')
        .and_then(|(_, exponent)| exponent.parse().ok())
        .unwrap_or(0);
    if exponent < -4 || exponent >= i32::try_from(significant).unwrap_or(i32::MAX) {
        let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
        let mantissa = match mantissa.contains('.') {
            true => {
                let trimmed = mantissa.trim_end_matches('0');
                match trimmed.ends_with('.') {
                    true => format!("{trimmed}0"),
                    false => trimmed.to_owned(),
                }
            }
            false => format!("{mantissa}.0"),
        };
        let sign = match exponent.starts_with('-') {
            true => "",
            false => "+",
        };
        return format!("{mantissa}e{sign}{exponent}");
    }
    let after = usize::try_from(i32::try_from(significant).unwrap_or(i32::MAX) - 1 - exponent)
        .unwrap_or_default();
    let fixed = format!("{value:.after$}");
    match fixed.contains('.') {
        true => fixed.trim_end_matches('0').trim_end_matches('.').to_owned(),
        false => fixed,
    }
}

/// An infinity or a NaN, in C's spelling — the one shape every `%f`/`%e`/`%g`
/// shares, so the three cannot disagree about it.
fn nonfinite(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    match value.is_sign_negative() {
        true => "-inf".to_owned(),
        false => "inf".to_owned(),
    }
}

/// A placeholder this grammar cannot read at all, measured from the `%` that
/// `after` follows: the message and the span it underlines are built from one
/// cut of the text, so what is quoted and what is underlined cannot come
/// apart.
fn malformed(after: &str) -> Malformed {
    let shown: String = after.chars().take(8).collect();
    Malformed {
        message: format!(
            "Core\\Str::format(): `%{shown}` is not a placeholder this template grammar allows"
        ),
        written: Written {
            at: 0,
            len: 1 + shown.len(),
        },
    }
}

/// A value with no reading for the conversion it reached.
fn unreadable(conversion: char) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!("Core\\Str::format(): `%{conversion}` has no reading for this value"),
    )
}

#[cfg(test)]
mod tests {
    use super::format;
    use nvs_runtime::{Decimal, NvsArray, Value};

    /// One `string` argument, and the reference this test owes for it.
    fn s(text: &str) -> Value {
        Value::text(text)
    }

    /// Releases every reference the arguments of one case carried — `format`
    /// borrows them, so this test still owns them.
    fn released(arguments: Vec<Value>) {
        for argument in arguments {
            #[expect(
                unsafe_code,
                reason = "this test built exactly one reference per argument, and `format` \
                          borrowed rather than consumed it"
            )]
            unsafe {
                argument.release();
            }
        }
    }

    /// Asserts one template's answer and releases its arguments.
    ///
    /// **Every expected string here was produced by PHP 8.5's own `vsprintf`
    /// before it was written down.** This module's whole job is to answer what
    /// those six functions answer, so a hand-derived expectation would be
    /// testing the implementation against itself.
    #[track_caller]
    fn same(template: &str, arguments: Vec<Value>, expected: &str) {
        let answer = format(template, &arguments);
        released(arguments);
        assert_eq!(
            answer.expect("no failure").as_bytes(),
            expected.as_bytes(),
            "for `{template}`"
        );
    }

    #[test]
    fn the_conversions_answer_what_php_answers() {
        same("%s has %d", vec![s("fox"), Value::int(2)], "fox has 2");
        same(
            "%s|%s|%s|",
            vec![Value::bool(true), Value::bool(false), Value::null()],
            "1|||",
        );
        same("%d|", vec![Value::float(3.99)], "3|");
        same("%u|", vec![Value::int(-1)], "18446744073709551615|");
        same(
            "%x|%X|%o|%b|",
            vec![
                Value::int(255),
                Value::int(255),
                Value::int(8),
                Value::int(5),
            ],
            "ff|FF|10|101|",
        );
        same("%%|", Vec::new(), "%|");
    }

    #[test]
    fn the_float_conversions_answer_what_php_answers() {
        same(
            "%.3f|%.0f|%f|",
            vec![Value::float(1.23456), Value::float(2.5), Value::float(1.5)],
            "1.235|2|1.500000|",
        );
        same(
            "%e|%e|",
            vec![Value::float(1234.5), Value::float(0.000_123)],
            "1.234500e+3|1.230000e-4|",
        );
        same(
            "%g|%g|%g|",
            vec![
                Value::float(0.0001),
                Value::float(0.000_01),
                Value::float(123_456_789.0),
            ],
            "0.0001|1.0e-5|1.23457e+8|",
        );
        same(
            "%g|%g|%.3g|%.0g|",
            vec![
                Value::float(999_999.0),
                Value::float(1_000_000.0),
                Value::float(123_456.0),
                Value::float(-2.5),
            ],
            "999999|1.0e+6|1.23e+5|-2|",
        );
    }

    #[test]
    fn the_flags_width_and_precision_answer_what_php_answers() {
        same("%5s|%-5s|", vec![s("ab"), s("ab")], "   ab|ab   |");
        same(
            "%05d|%+d|%+d|",
            vec![Value::int(42), Value::int(42), Value::int(-42)],
            "00042|+42|-42|",
        );
        same("%'*8s|", vec![s("hi")], "******hi|");
        same("%'x-8d|", vec![Value::int(42)], "42xxxxxx|");
        same(
            "%10.3f|%-8.2f|",
            vec![Value::float(1.23456), Value::float(1.23456)],
            "     1.235|1.23    |",
        );
        same("%08.3f|", vec![Value::float(-1.23456)], "-001.235|");
        same("%.2s|%5.2s|", vec![s("abcdef"), s("abcdef")], "ab|   ab|");
    }

    /// `%1$s` names an argument rather than consuming the next one, so it can
    /// read the same one twice — PHP's own behaviour, and the reason
    /// "every argument is read" is a rule per argument rather than a count.
    #[test]
    fn a_positional_placeholder_names_its_argument() {
        same("%1$s-%1$s-%2$s", vec![s("a"), s("b")], "a-a-b");
    }

    /// Width and precision measure `Core\Str::length`'s unit, not C's bytes —
    /// this module's own docs own why.
    #[test]
    fn width_measures_what_length_measures() {
        same(
            "%4s|",
            vec![s("\u{1f1e6}\u{1f1f9}")],
            "   \u{1f1e6}\u{1f1f9}|",
        );
        same(
            "%.1s|",
            vec![s("\u{1f1e6}\u{1f1f9}x")],
            "\u{1f1e6}\u{1f1f9}|",
        );
    }

    /// Each refusal this module's docs list, at the boundary that reports it.
    #[test]
    fn every_mismatch_throws_rather_than_producing_something() {
        for (template, arguments) in [
            ("%s %s", vec![s("only one")]),
            ("%s", vec![s("read"), s("unread")]),
            ("%q", Vec::new()),
            ("%", Vec::new()),
            ("%d", vec![Value::array(NvsArray::new())]),
            (
                "%d",
                vec![Value::decimal(Decimal::parse("1.5").expect("a decimal"))],
            ),
        ] {
            let answer = format(template, &arguments);
            released(arguments);
            assert!(
                answer.is_err(),
                "`{template}` produced {:?} rather than throwing",
                answer.ok()
            );
        }
    }

    /// `rule:expressions/preparation-preserves-behaviour`, asserted rather than described: the checker's entry point
    /// and the runtime's are one implementation, so neither can drift into
    /// accepting a template the other refuses.
    ///
    /// Asserted by **agreement**, not by two expectation lists — a second list
    /// of what [`placeholders`] answers would pass while both halves drifted
    /// together. The malformed half compares the refusal's own words, and the
    /// well-formed half hands the runtime exactly the argument count the
    /// checker derived and asserts the boundary on both sides of it: that many
    /// renders, one fewer throws.
    #[test]
    fn a_prepared_template_and_its_runtime_twin_share_one_implementation() {
        for template in ["%q", "%1$", "%", "%'", "%.2z"] {
            let checked = super::placeholders(template).expect_err("a refusal");
            let thrown = format(template, &[]).expect_err("a refusal");
            let nvs_runtime::Fault::Thrown(_, message) = thrown else {
                panic!("`{template}` refused as something other than a throw");
            };
            assert_eq!(checked.message, message, "for `{template}`");
            // What the refusal says it is refusing has to be inside the
            // template it was refusing, or a caller mapping it back to a
            // column underlines somewhere else entirely.
            assert!(
                checked.written.at + checked.written.len <= template.len(),
                "`{template}` located its refusal outside itself: {:?}",
                checked.written
            );
        }
        for template in ["%s", "%s %d", "%2$s %1$s %2$s", "100%% of %s", "%08.3f"] {
            let read = super::placeholders(template).expect("a template");
            let wanted = read.iter().map(|p| p.index).max().map_or(0, |i| i + 1);
            let arguments: Vec<Value> = (0..wanted).map(|_| Value::int(1)).collect();
            assert!(
                format(template, &arguments).is_ok(),
                "`{template}` refused the {wanted} argument(s) the checker derived"
            );
            assert!(
                format(template, &arguments[..wanted - 1]).is_err(),
                "`{template}` accepted one argument fewer than the checker derived"
            );
        }
    }

    /// An integral `decimal` reaches `%d` unchanged, and any `decimal` renders
    /// through `%s` at its own scale — `rule:types/conversion`'s two rows, side by side.
    #[test]
    fn a_decimal_follows_the_conversion_rows() {
        same(
            "%d|%s|",
            vec![
                Value::decimal(Decimal::parse("42").expect("a decimal")),
                Value::decimal(Decimal::parse("19.90").expect("a decimal")),
            ],
            "42|19.90|",
        );
    }
}
