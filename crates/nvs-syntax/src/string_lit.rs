//! Cooks the double-quoted escape grammar shared by a `"…"` string literal
//! and an interpolated-heredoc's own text runs — [`crate::ast::ExprKind`]'s
//! `Str` (double-quoted case only) and each `Text` piece of `Interpolated`'s
//! [`crate::ast::StringPart`] vector — into the runtime bytes they name.
//!
//! This crate's lexer already recognizes *which* backslash sequences exist
//! (`Lexer::lex_escape_in_place`) and rejects a syntactically malformed
//! `\u{...}` (missing hex digits, or no closing `}`) at lex time via
//! `code::E_INVALID_ESCAPE`. It does not evaluate what a `\xHH`, octal, or
//! `\u{...}` escape actually *produces* — cooking needs the whole literal
//! assembled first, the same reason `nvs_types::expr::infer`'s `ExprKind::Int`
//! arm (not the lexer) is where an integer literal's *magnitude* gets
//! checked. This module is that arm's sibling for string content: it cooks
//! the escapes and reports the two ways cooking can still fail — an
//! out-of-range/surrogate `\u{...}` codepoint, or a `\xHH`/octal byte escape
//! sequence that does not decode as valid UTF-8 (`rule:types/bytes`'s "`string` is
//! guaranteed-valid UTF-8" invariant) — before `nvs-ir` ever lowers the
//! literal.
//!
//! **One escape grammar, and it lives at the bottom.** Three crates above this
//! one have to agree byte for byte about what a written literal denotes:
//! `nvs-hir`'s `require` resolution reads a path out of one before any
//! checking happens, `nvs-types`'s `ExprKind::Str` arm diagnoses it and
//! interns its single-value type, and `nvs-ir`'s lowering emits the actual bytes.
//! `nvs-syntax` is the only crate all three already depend on, so this is
//! where the routine sits and `nvs_types::string_lit` is a re-export of it
//! rather than a second implementation. The duplicate that direction *does*
//! tolerate is `nvs_types::expr`'s `int_literal_digits` beside
//! `nvs_ir::lower`'s: a ~15-line digit splitter either crate can get wrong in
//! isolation without the other noticing. This is a much larger, subtler
//! routine where a checker's diagnosis, a resolved `require` path and a
//! lowered value diverging silently is the whole failure mode, and sharing one
//! implementation is what forecloses it.
//!
//! Only the double-quoted escape grammar lives here. A single-quoted literal
//! (`\\`/`\'` only) can never produce invalid UTF-8 — it copies source
//! characters through unchanged aside from those two escapes, both already
//! ASCII — so it has no cooking routine of its own to share; `nvs-ir`'s
//! `cook_str_literal` keeps that tiny case inline.
//!
//! This module also cooks a heredoc/nowdoc's body: PHP 7.3's "flexible
//! heredoc" rule, which lets the closing marker itself be indented and
//! strips that same indentation off every line of the body
//! ([`heredoc_shape`], [`dedent_heredoc_run`]). A heredoc's interpolated
//! text runs still cook through this module's own double-quoted escape
//! grammar afterward ([`cook_double_quoted_text_str`], the owned-`&str`
//! sibling of [`cook_double_quoted_text`] needed once dedenting has already
//! broken the byte-for-byte correspondence to a single source span); a
//! nowdoc's body applies no escapes at all, exactly like `nvs-ir`'s own
//! single-quoted case — dedenting is the *only* transformation it gets.
//!
//! **Cooking has a second mode, and only a diagnostic asks for it.**
//! [`cook_string_literal_positions`] returns the same value
//! [`cook_string_literal`] does, alongside the map from each decoded byte back
//! to the file offset the character that produced it was written at. That map
//! is what lets a checker underline the one placeholder inside a template
//! rather than the whole literal ([`decoded_range_span`]), which a span
//! covering the quotes and the escapes cannot do. It holds four bytes per
//! decoded byte, for as long as the diagnostic is being built, and is filled
//! only where a caller passes somewhere to put it — so cooking for a *value*,
//! which is every cook on the success path, allocates none of it.

use nvs_diagnostics::{SourceFile, Span};

/// One way [`cook_double_quoted_text`] failed to fully cook its input.
/// Cooking still returns a best-effort `String` alongside these (lossy UTF-8
/// substitution for [`CookIssue::InvalidUtf8`], the escape dropped entirely
/// for [`CookIssue::InvalidUnicodeEscape`]) so a caller that reports and
/// keeps going — every checker diagnostic in this crate does — has something
/// to keep checking against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookIssue {
    /// A `\u{...}` escape named a value outside Unicode's valid scalar range
    /// (`> 0x10FFFF`) or inside the UTF-16 surrogate range
    /// (`0xD800..=0xDFFF`) — neither has a UTF-8 encoding. The span covers
    /// just that one escape.
    InvalidUnicodeEscape(Span),
    /// One or more `\xHH`/octal byte escapes combined into a sequence that is
    /// not valid UTF-8 once assembled with the rest of the text. Attributed
    /// to the whole cooked span rather than one escape, since a UTF-8
    /// continuation byte is only invalid in the context of the bytes around
    /// it.
    InvalidUtf8(Span),
}

/// Cooks `span` — the text strictly between a double-quoted literal's own
/// quote characters, or one [`crate::ast::StringPart::Text`] run inside
/// an `Interpolated` literal (which never includes a quote character at all)
/// — into the `string` value it denotes.
///
/// Handles every escape `nvs-syntax`'s lexer recognizes when interpolation is
/// active (`Lexer::lex_quoted_body`'s `interpolation` branch, shared
/// verbatim between a double-quoted literal and a heredoc opened without
/// `'quotes'`, which is exactly why this same routine cooks both): `\\`,
/// `\"`, `\$`, `\n`, `\t`, `\r`, `\v`, `\f`, `\e`, a 1-3-digit octal escape
/// (`\0`-`\377`, silently wrapping mod 256 the same way PHP's does), a
/// 1-2-digit hex escape (`\xHH`), and a `\u{...}` Unicode codepoint escape
/// (encoded to its UTF-8 bytes). Any other backslash sequence — including a
/// syntactically malformed `\u{...}` the lexer already flagged separately —
/// is passed through literally, matching PHP and avoiding a second report of
/// something `nvs-syntax` already diagnosed.
#[must_use]
pub fn cook_double_quoted_text(src: &SourceFile, span: Span) -> (String, Vec<CookIssue>) {
    let text = src.span_text(span).unwrap_or_default();
    cook_double_quoted_chars(text, span, span_of(span), &[], None)
}

/// The two escapes an html template adds to the double-quoted grammar, and the
/// whole of what it adds: `` \` `` for a backtick in the body, and `\{` for the
/// one case that wants a literal `{$` (`rule:core-classes/html-template`).
///
/// Neither can be a row of the shared table: in a double-quoted string both are
/// an unknown escape, which PHP passes through as the two characters written,
/// and that pass-through is itself a rule the table states.
const HTML_TEMPLATE_ESCAPES: &[char] = &['`', '{'];

/// Cooks one [`crate::ast::StringPart::Text`] run of an html template —
/// ``html`…` ``'s segment grammar, which is [`cook_double_quoted_text`]'s plus
/// [`HTML_TEMPLATE_ESCAPES`].
///
/// A segment is trusted bytes and nothing about it is escaped *for* HTML here:
/// `rule:core-classes/html-template` trusts a segment because the author wrote
/// it, so what runs over one is the same source-escape grammar every other
/// literal's text runs, and `<` stays `<`.
#[must_use]
pub fn cook_html_template_text(src: &SourceFile, span: Span) -> (String, Vec<CookIssue>) {
    let text = src.span_text(span).unwrap_or_default();
    cook_double_quoted_chars(text, span, span_of(span), HTML_TEMPLATE_ESCAPES, None)
}

/// Maps a cooked run's own byte offsets back to spans in the file, for a run
/// whose text is `span`'s verbatim — every caller but the heredoc one, whose
/// text no longer corresponds byte-for-byte to any span at all
/// ([`cook_double_quoted_text_str`]).
fn span_of(span: Span) -> impl Fn(usize, usize) -> Span {
    move |s, e| {
        Span::new(
            span.file,
            span.start + off_as_u32(s),
            span.start + off_as_u32(e),
        )
    }
}

/// [`cook_double_quoted_text`]'s sibling for text that no longer corresponds
/// byte-for-byte to any single contiguous span in the source file — a
/// heredoc/nowdoc body run once [`dedent_heredoc_run`] has stripped its
/// per-line indentation, which shifts every offset past the first stripped
/// line. Cooks the identical escape grammar, but every [`CookIssue`] it
/// finds is attributed to the whole `attribute_to` span rather than a
/// precise sub-span — a deliberate, narrow loss of diagnostic precision
/// (never of the cooked *value*, which is exact either way) that only
/// applies once a heredoc/nowdoc's closing marker is itself indented; a
/// marker with no indentation never reaches this function at all, see
/// `nvs-ir`'s `cook_str_literal`/`Lowering::lower_interpolated_parts`.
#[must_use]
pub fn cook_double_quoted_text_str(text: &str, attribute_to: Span) -> (String, Vec<CookIssue>) {
    cook_double_quoted_chars(text, attribute_to, |_, _| attribute_to, &[], None)
}

/// Cooks a whole [`crate::ast::ExprKind::Str`] literal — delimiters
/// included — into the `string` it denotes, dispatching on which of the three
/// spellings `span` opens with.
///
/// * A **single-quoted** literal has exactly two escapes (`\\` and `\'`),
///   cooked inline below: both are ASCII and every other character copies
///   straight through from an already-valid-UTF-8 source file, so there is no
///   failure mode to report and nothing worth a routine of its own.
/// * A **double-quoted** literal delegates its inner span to
///   [`cook_double_quoted_text`].
/// * A **heredoc/nowdoc** body with no interpolation site anywhere in it (its
///   span opens `<<<`) runs [`heredoc_shape`]/[`dedent_heredoc_run`]'s
///   flexible-indentation strip first, then the same double-quoted escape
///   grammar — unless [`heredoc_is_nowdoc`], which applies no escapes at all.
///
/// Every [`CookIssue`] and [`HeredocIndentIssue`] is discarded: this is the
/// *value*, and `nvs_types::expr`'s own `ExprKind::Str` arm already reported both
/// against the same span. That is the standing "the checker diagnoses,
/// everything downstream trusts" split, and it is why one routine can serve
/// both `nvs-ir`'s lowering and `nvs_types::defaults`'s parameter-default
/// evaluation without either growing a second escape grammar.
///
/// * A **bareword** span — no delimiters at all — is PHP's simple-syntax array
///   offset, `"$row[key]"`. Its own text is its value; see the branch itself.
///
/// # Panics
///
/// Panics if `span` is empty — a lexer bug, since nothing else produces an
/// `ExprKind::Str`.
#[must_use]
pub fn cook_string_literal(src: &SourceFile, span: Span) -> String {
    let raw = src.span_text(span).unwrap_or_default();
    if raw.starts_with("<<<") {
        return cook_heredoc_literal(src, span, raw);
    }
    let quote = raw
        .chars()
        .next()
        .unwrap_or_else(|| panic!("an empty string literal span at {span:?} — lexer bug?"));
    if quote != '\'' && quote != '"' {
        // A **bareword** offset inside PHP's simple interpolation syntax —
        // the `key` of `"$row[key]"`, which this crate's parser turns into
        // an `ExprKind::Str` over the unquoted span (see its
        // `parse_simple_interp_variable`). It is the one spelling that reaches
        // here without delimiters, and it carries no escape grammar at all:
        // the lexer only ever spans identifier characters, so its own text
        // *is* its value. A digit run is left a string on purpose — an array
        // subscript normalises a numeric string key to an integer one
        // (`rule:types/arrays`), which is exactly what PHP does with `"$n[0]"`.
        return raw.to_owned();
    }
    let inner_span = Span::new(span.file, span.start + 1, span.end - 1);
    if quote == '"' {
        return cook_double_quoted_text(src, inner_span).0;
    }
    let inner = src.span_text(inner_span).unwrap_or_default();
    cook_single_quoted(inner, quote, None)
}

/// [`cook_string_literal`]'s single-quoted branch: the two escapes that
/// spelling has, over the text strictly between its quotes.
///
/// `positions` is filled exactly as [`cook_double_quoted_chars`] fills its
/// own — `inner`'s offsets, one per decoded byte, plus the past-the-end
/// sentinel — so [`cook_string_literal_positions`] rebases either the same
/// way.
fn cook_single_quoted(inner: &str, quote: char, mut positions: Option<&mut Vec<u32>>) -> String {
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.char_indices();
    let mut mark = 0usize;
    while let Some((at, c)) = chars.next() {
        if let Some(map) = positions.as_deref_mut() {
            map.resize(out.len(), off_as_u32(mark));
            mark = at;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some((_, '\\')) => out.push('\\'),
            Some((_, next)) if next == quote => out.push(quote),
            Some((_, other)) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    if let Some(map) = positions {
        map.resize(out.len(), off_as_u32(mark));
        map.push(off_as_u32(inner.len()));
    }
    out
}

/// [`cook_string_literal`]'s second mode: the same value, and the map from
/// each decoded byte back to the file offset the character that produced it
/// was written at.
///
/// `positions[k]` is that offset for decoded byte `k`, and the map carries one
/// entry more than the string is long — the offset just past the last
/// character consumed — so a decoded range always has an exclusive end to
/// read. [`decoded_range_span`] is what reads it.
///
/// **An empty map means no offset is knowable**, and two spellings return one.
/// A heredoc/nowdoc's flexible-indentation strip shifts every offset past the
/// first stripped line, which is the same correspondence
/// [`cook_double_quoted_text_str`] already gives up. A literal whose byte
/// escapes did not assemble into valid UTF-8 has had the bytes the map indexes
/// replaced by the lossy substitution. A caller reading an empty map
/// underlines the whole literal, which is what every caller did before this
/// mode existed.
///
/// This mode exists for a diagnostic and runs only when one is being emitted:
/// cooking for a *value* is [`cook_string_literal`] and allocates none of it.
///
/// # Panics
///
/// Panics if `span` is empty, exactly as [`cook_string_literal`] does.
#[must_use]
pub fn cook_string_literal_positions(src: &SourceFile, span: Span) -> (String, Vec<u32>) {
    let raw = src.span_text(span).unwrap_or_default();
    if raw.starts_with("<<<") {
        return (cook_heredoc_literal(src, span, raw), Vec::new());
    }
    let quote = raw
        .chars()
        .next()
        .unwrap_or_else(|| panic!("an empty string literal span at {span:?} — lexer bug?"));
    if quote != '\'' && quote != '"' {
        // A bareword offset carries no escape grammar at all — see
        // [`cook_string_literal`]'s own branch — so its map is the identity.
        let map = (0..=raw.len())
            .map(|at| span.start + off_as_u32(at))
            .collect();
        return (raw.to_owned(), map);
    }
    let inner_span = Span::new(span.file, span.start + 1, span.end - 1);
    let inner = src.span_text(inner_span).unwrap_or_default();
    let mut map = Vec::with_capacity(inner.len() + 1);
    let cooked = if quote == '"' {
        cook_double_quoted_chars(inner, inner_span, span_of(inner_span), &[], Some(&mut map)).0
    } else {
        cook_single_quoted(inner, quote, Some(&mut map))
    };
    for at in &mut map {
        *at += inner_span.start;
    }
    (cooked, map)
}

/// The source span of a decoded byte range, read back through
/// [`cook_string_literal_positions`]'s map.
///
/// `None` when the map is empty — the spellings that have no byte-for-byte
/// correspondence — and when the range runs past what was decoded, which is a
/// caller holding a different string than it cooked. Either way the caller
/// falls back to `literal` itself.
#[must_use]
pub fn decoded_range_span(literal: Span, positions: &[u32], at: usize, len: usize) -> Option<Span> {
    let start = *positions.get(at)?;
    let end = *positions.get(at.checked_add(len)?)?;
    Some(Span::new(literal.file, start, end.max(start)))
}

/// [`cook_string_literal`]'s heredoc/nowdoc branch — `raw` is `span`'s own
/// text, already confirmed to start with `<<<` by the caller.
fn cook_heredoc_literal(src: &SourceFile, span: Span, raw: &str) -> String {
    let (shape, _issues) = heredoc_shape(src, span);
    let mut issues = Vec::new();
    let dedented = dedent_heredoc_run(src, &shape.indent, shape.body, true, true, &mut issues);
    if heredoc_is_nowdoc(raw) {
        dedented
    } else {
        cook_double_quoted_text_str(&dedented, span).0
    }
}

/// `positions`, when a caller asks for one, is filled with `text`'s own
/// offsets — one entry per decoded byte, plus the past-the-end sentinel — and
/// the caller rebases them onto the file. It is padded at the *top* of each
/// iteration from the offset the previous one consumed, rather than beside
/// every append below: the escape table pushes bytes at more than a dozen
/// sites and takes three different `continue`s out of the loop, so a mapping
/// written per-append is one a later escape row silently forgets.
fn cook_double_quoted_chars(
    text: &str,
    whole_span_for_utf8_issue: Span,
    escape_span: impl Fn(usize, usize) -> Span,
    extra: &[char],
    mut positions: Option<&mut Vec<u32>>,
) -> (String, Vec<CookIssue>) {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let mut bytes: Vec<u8> = Vec::with_capacity(text.len());
    let mut issues = Vec::new();
    let mut buf = [0u8; 4];

    // The offset every byte appended but not yet mapped was written at.
    let mut mark = 0usize;
    let mut i = 0usize;
    while i < n {
        let (at, c) = chars[i];
        if let Some(map) = positions.as_deref_mut() {
            map.resize(bytes.len(), off_as_u32(mark));
            mark = at;
        }
        if c != '\\' {
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            i += 1;
            continue;
        }
        // `c` is the backslash itself; a lone trailing `\` (only possible at
        // the very end of a `StringPart::Text` run immediately followed by
        // an interpolation site, e.g. `"\` right before `{$x}`) has nothing
        // to escape — keep it literal rather than indexing past the end.
        let Some(&(_, next)) = chars.get(i + 1) else {
            bytes.push(b'\\');
            i += 1;
            continue;
        };
        match next {
            '\\' | '"' | '$' => {
                bytes.push(u8::try_from(next).expect("\\, \", $ are all ASCII"));
                i += 2;
            }
            'n' => {
                bytes.push(b'\n');
                i += 2;
            }
            't' => {
                bytes.push(b'\t');
                i += 2;
            }
            'r' => {
                bytes.push(b'\r');
                i += 2;
            }
            'v' => {
                bytes.push(0x0B);
                i += 2;
            }
            'f' => {
                bytes.push(0x0C);
                i += 2;
            }
            'e' => {
                bytes.push(0x1B);
                i += 2;
            }
            '0'..='7' => {
                let mut val: u32 = 0;
                let mut j = i + 1;
                let mut consumed = 0;
                while consumed < 3
                    && let Some(&(_, d)) = chars.get(j)
                    && ('0'..='7').contains(&d)
                {
                    val = val * 8 + d.to_digit(8).expect("guarded to '0'..='7' above");
                    j += 1;
                    consumed += 1;
                }
                bytes.push(u8::try_from(val % 256).expect("reduced mod 256 above"));
                i = j;
            }
            'x' => {
                let mut val: u32 = 0;
                let mut j = i + 2;
                let mut consumed = 0;
                while consumed < 2
                    && let Some(&(_, d)) = chars.get(j)
                    && let Some(hex) = d.to_digit(16)
                {
                    val = val * 16 + hex;
                    j += 1;
                    consumed += 1;
                }
                if consumed == 0 {
                    // PHP itself leaves a bare `\x` (no hex digit follows)
                    // literal rather than erroring — the lexer never flags
                    // this shape either, so mirror that instead of inventing
                    // a diagnostic for it here.
                    bytes.extend_from_slice(b"\\x");
                    i += 2;
                } else {
                    bytes.push(u8::try_from(val).expect("at most 2 hex digits always fits a u8"));
                    i = j;
                }
            }
            'u' if chars.get(i + 2).map(|&(_, c)| c) == Some('{') => {
                let hex_start = i + 3;
                let mut j = hex_start;
                while chars.get(j).is_some_and(|&(_, d)| d.is_ascii_hexdigit()) {
                    j += 1;
                }
                if j > hex_start && chars.get(j).map(|&(_, c)| c) == Some('}') {
                    let hex: String = chars[hex_start..j].iter().map(|&(_, c)| c).collect();
                    let value = u32::from_str_radix(&hex, 16).unwrap_or(u32::MAX);
                    let escape_end_off = chars.get(j + 1).map_or(text.len(), |&(off, _)| off);
                    match char::from_u32(value) {
                        Some(ch) => bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes()),
                        None => issues.push(CookIssue::InvalidUnicodeEscape(escape_span(
                            chars[i].0,
                            escape_end_off,
                        ))),
                    }
                    i = j + 1;
                } else {
                    // A syntactically malformed `\u{...}` (no digits, or no
                    // closing `}`) — `nvs-syntax` already reported
                    // `E_INVALID_ESCAPE` for this at lex time. Pass the two
                    // characters through literally rather than double-report.
                    bytes.extend_from_slice(b"\\u");
                    i += 2;
                }
            }
            // A delimiter only this literal form has — the caller's own
            // additions to the grammar, escaped to themselves so the backslash
            // that kept the lexer from reading them structurally is not left in
            // the value.
            other if extra.contains(&other) => {
                bytes.extend_from_slice(other.encode_utf8(&mut buf).as_bytes());
                i += 2;
            }
            other => {
                bytes.push(b'\\');
                bytes.extend_from_slice(other.encode_utf8(&mut buf).as_bytes());
                i += 2;
            }
        }
    }

    if let Some(map) = positions.as_deref_mut() {
        map.resize(bytes.len(), off_as_u32(mark));
        // One entry more than the decoded text is long, so a decoded range
        // ending at the last byte still has an exclusive end to read.
        map.push(off_as_u32(text.len()));
    }

    match String::from_utf8(bytes) {
        Ok(s) => (s, issues),
        Err(e) => {
            issues.push(CookIssue::InvalidUtf8(whole_span_for_utf8_issue));
            // Lossy substitution rewrites the very bytes the map indexes, so
            // the map no longer describes this string at all. Empty is how a
            // caller is told no offset is knowable here.
            if let Some(map) = positions {
                map.clear();
            }
            (String::from_utf8_lossy(e.as_bytes()).into_owned(), issues)
        }
    }
}

/// A source file caps out at 4 GiB (`nvs_diagnostics::span::BytePos` is
/// `u32`), so a byte offset within one always fits back into a `u32`.
fn off_as_u32(off: usize) -> u32 {
    u32::try_from(off).expect("a byte offset within one source file fits u32 (BytePos's own type)")
}

// --- heredoc/nowdoc flexible-indentation stripping (PHP 7.3+) --------------

/// One way validating a heredoc/nowdoc's flexible-indentation strip went
/// wrong — [`heredoc_shape`]'s closing-marker check
/// (`MixedIndentWhitespace`, reported at most once per literal), or
/// [`dedent_heredoc_run`]'s per-line check (`InsufficientIndent`, reported
/// once per offending line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeredocIndentIssue {
    /// The closing marker's own leading whitespace mixes spaces and tabs —
    /// PHP requires it to be one or the other so a body line's leading
    /// whitespace can be compared to it byte-for-byte.
    MixedIndentWhitespace(Span),
    /// A non-blank body line's leading whitespace does not start with the
    /// closing marker's own indentation. A line that is entirely empty is
    /// exempt (PHP's own carve-out): there is nothing to check, and nothing
    /// to strip either.
    InsufficientIndent(Span),
}

/// The parts of a heredoc/nowdoc literal PHP 7.3's "flexible heredoc" rule
/// needs: the closing marker's own indentation text (spaces or tabs, never
/// both — see [`HeredocIndentIssue::MixedIndentWhitespace`]; empty when the
/// marker isn't indented at all, the common case, which makes every
/// [`dedent_heredoc_run`] call downstream a no-op) and the body's own span
/// — the text strictly between the opening line's terminating `\n` and the
/// closing marker's leading whitespace, *inclusive* of the one trailing
/// newline PHP always drops rather than treats as content.
#[derive(Debug, Clone)]
pub struct HeredocShape {
    /// The closing marker's own leading indentation, copied verbatim off its
    /// source line.
    pub indent: String,
    /// The body's span, opening-line newline through the closing marker's
    /// own leading whitespace (exclusive) — see the struct's own doc
    /// comment for why its *trailing* edge still includes one newline
    /// character (`dedent_heredoc_run` is what actually drops it).
    pub body: Span,
}

/// Computes a heredoc/nowdoc literal's [`HeredocShape`] from `whole_span` —
/// the entire literal exactly as [`crate::ast::ExprKind::Str`]/
/// `Interpolated` carry it: `<<<LABEL` (or `<<<'LABEL'`/`<<<"LABEL"`)
/// through the closing marker's own last character, inclusive. No separate
/// span for the closing marker needs to be threaded through from the parser
/// for this: this crate's own `Parser::parse_heredoc_string` never leaves
/// anything after the marker inside this span, so the marker's own line is
/// always exactly the text after `whole_span`'s *last* `\n` — even in the
/// degenerate empty-body case, where that last `\n` is the same one that
/// ends the opening line.
///
/// Returns `(shape, issues)` rather than reporting through `Diagnostics`
/// directly, the same shape [`cook_double_quoted_text`] already uses, so a
/// checker call site can attribute a code/message to each issue and
/// `nvs-ir` can discard them, trusting the checker already ran.
#[must_use]
pub fn heredoc_shape(
    src: &SourceFile,
    whole_span: Span,
) -> (HeredocShape, Vec<HeredocIndentIssue>) {
    let raw = src.span_text(whole_span).unwrap_or_default();
    let Some(last_nl) = raw.rfind('\n') else {
        // No newline at all inside the whole literal -- a malformed heredoc
        // header `nvs-syntax` already reported `E_BAD_HEREDOC` for. Fall
        // back to an empty body rather than guessing at a shape.
        let empty = Span::new(whole_span.file, whole_span.end, whole_span.end);
        return (
            HeredocShape {
                indent: String::new(),
                body: empty,
            },
            Vec::new(),
        );
    };
    let first_nl = raw.find('\n').expect("rfind above already found one");
    let marker = &raw[last_nl + 1..];
    let indent_len = marker
        .chars()
        .take_while(|&c| c == ' ' || c == '\t')
        .count();
    let indent = &marker[..indent_len];
    let mut issues = Vec::new();
    let indent = if indent.contains(' ') && indent.contains('\t') {
        let marker_start = whole_span.start + off_as_u32(last_nl + 1);
        issues.push(HeredocIndentIssue::MixedIndentWhitespace(Span::new(
            whole_span.file,
            marker_start,
            marker_start + off_as_u32(indent_len),
        )));
        String::new()
    } else {
        indent.to_owned()
    };
    let body = Span::new(
        whole_span.file,
        whole_span.start + off_as_u32(first_nl + 1),
        whole_span.start + off_as_u32(last_nl + 1),
    );
    (HeredocShape { indent, body }, issues)
}

/// Whether a heredoc/nowdoc literal's own opening (`raw`, starting with
/// `<<<`) is a nowdoc (`<<<'LABEL'`) rather than a heredoc (`<<<LABEL` or
/// `<<<"LABEL"`) — the one distinction that decides whether its body runs
/// any escape grammar at all (a nowdoc runs none, exactly like a
/// single-quoted literal minus even `\\`/`\'`; see this module's own docs).
/// Shared between the checker and `nvs-ir` for the same reason every other
/// function in this module is: getting this wrong silently would mean the
/// two disagree on whether an escape sequence is even live.
#[must_use]
pub fn heredoc_is_nowdoc(raw: &str) -> bool {
    raw.strip_prefix("<<<")
        .map(str::trim_start)
        .is_some_and(|rest| rest.starts_with('\''))
}

/// Strips `indent` from the start of one heredoc/nowdoc body line's raw text
/// (`line`, containing no `\n` of its own). A blank line (zero characters)
/// is exempt from the check and returned unchanged, matching PHP; anything
/// else must start with `indent` exactly, or the mismatch is reported
/// against `line_span` and `line` itself is returned unstripped — a
/// best-effort recovery value so a caller keeps cooking the rest of the
/// literal rather than aborting on the first bad line.
fn strip_line_indent<'a>(
    indent: &str,
    line: &'a str,
    line_span: Span,
    issues: &mut Vec<HeredocIndentIssue>,
) -> &'a str {
    if line.is_empty() {
        return line;
    }
    match line.strip_prefix(indent) {
        Some(rest) => rest,
        None => {
            issues.push(HeredocIndentIssue::InsufficientIndent(line_span));
            line
        }
    }
}

/// Dedents one raw heredoc/nowdoc body run — [`HeredocShape::body`]'s whole
/// span for a heredoc/nowdoc that collapsed to `ExprKind::Str` (no
/// interpolation site anywhere in the body), or one
/// `StringPart::Text` span for an `ExprKind::Interpolated` heredoc — against
/// `indent` (from [`heredoc_shape`]), applying [`strip_line_indent`] to
/// every line this run *starts*.
///
/// `body_start` must be `true` only for the very first run of the whole
/// body: its own first line is a fresh line needing a dedent check only
/// then, since every other run picks up wherever the previous run or
/// interpolation site left off, which is never a line start on its own (an
/// interpolation site never itself carries leading whitespace — any
/// indentation before one is always literal text already captured by the
/// preceding run, see `nvs-ir`'s own module docs on this point). Every
/// *embedded* `\n` inside `line`'s own text always starts a fresh line
/// regardless of `body_start`, since a literal newline can only ever appear
/// inside a `Text` run.
///
/// `is_last_run` must be `true` only for the run immediately before the
/// closing marker: it strips the exact one trailing newline (`\r\n` or
/// `\n`) PHP always drops rather than keeps as content, before any line
/// splitting happens. Every *other* embedded newline in the body — CRLF or
/// LF — is left as literal content unchanged; a heredoc does not otherwise
/// normalize line endings.
#[must_use]
pub fn dedent_heredoc_run(
    src: &SourceFile,
    indent: &str,
    span: Span,
    body_start: bool,
    is_last_run: bool,
    issues: &mut Vec<HeredocIndentIssue>,
) -> String {
    let raw = src.span_text(span).unwrap_or_default();
    let text = if is_last_run {
        raw.strip_suffix("\r\n")
            .or_else(|| raw.strip_suffix('\n'))
            .unwrap_or(raw)
    } else {
        raw
    };
    if indent.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut offset: u32 = 0;
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let line_span = Span::new(
            span.file,
            span.start + offset,
            span.start + offset + off_as_u32(line.len()),
        );
        if i > 0 || body_start {
            out.push_str(strip_line_indent(indent, line, line_span, issues));
        } else {
            out.push_str(line);
        }
        offset += off_as_u32(line.len()) + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{SourceMap, Span};

    use super::*;

    fn cook(src_text: &str) -> (String, Vec<CookIssue>) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src_text);
        let span = Span::new(file, 0, u32::try_from(src_text.len()).unwrap());
        cook_double_quoted_text(map.file(file), span)
    }

    #[test]
    fn named_escapes_cook_to_their_control_characters() {
        let (s, issues) = cook(r#"a\nb\tc\rd\ve\ff\eg\\h\$i\"j"#);
        assert!(issues.is_empty());
        assert_eq!(s, "a\nb\tc\rd\x0Be\x0Cf\x1Bg\\h$i\"j");
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        let (s, issues) = cook("hello, world");
        assert!(issues.is_empty());
        assert_eq!(s, "hello, world");
    }

    #[test]
    fn octal_escape_cooks_and_wraps_mod_256() {
        let (s, issues) = cook(r"\101\102");
        assert!(issues.is_empty());
        assert_eq!(s, "AB");

        // \400 is 256 decimal, which wraps to 0 mod 256 -- PHP's own
        // documented overflow behavior for this escape.
        let (s, issues) = cook(r"\400");
        assert!(issues.is_empty());
        assert_eq!(s, "\0");
    }

    #[test]
    fn octal_escape_stops_at_three_digits() {
        let (s, issues) = cook(r"\1019");
        assert!(issues.is_empty());
        // \101 is 'A'; the trailing '9' is not part of the escape.
        assert_eq!(s, "A9");
    }

    #[test]
    fn hex_escape_cooks_one_or_two_digits() {
        let (s, issues) = cook(r"\x41\x4");
        assert!(issues.is_empty());
        assert_eq!(s, "A\x04");
    }

    #[test]
    fn bare_hex_escape_with_no_digit_is_left_literal() {
        let (s, issues) = cook(r"\xZ");
        assert!(issues.is_empty());
        assert_eq!(s, "\\xZ");
    }

    #[test]
    fn unicode_escape_cooks_to_utf8() {
        let (s, issues) = cook(r"\u{48}\u{65}\u{1F600}");
        assert!(issues.is_empty());
        assert_eq!(s, "He\u{1F600}");
    }

    #[test]
    fn unicode_escape_out_of_range_is_diagnosed() {
        let (s, issues) = cook(r"a\u{110000}b");
        assert_eq!(s, "ab");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0], CookIssue::InvalidUnicodeEscape(_)));
    }

    #[test]
    fn unicode_escape_surrogate_is_diagnosed() {
        let (s, issues) = cook(r"a\u{D800}b");
        assert_eq!(s, "ab");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0], CookIssue::InvalidUnicodeEscape(_)));
    }

    #[test]
    fn malformed_unicode_escape_passes_through_without_a_second_diagnostic() {
        // No digits at all -- nvs-syntax's lexer already reported
        // `E_INVALID_ESCAPE` for this at lex time; cooking must not panic or
        // double-report.
        let (s, issues) = cook(r"\u{}");
        assert!(issues.is_empty());
        assert_eq!(s, "\\u{}");
    }

    #[test]
    fn byte_escapes_combining_into_invalid_utf8_are_diagnosed() {
        // \xFF alone is never a valid UTF-8 byte on its own.
        let (s, issues) = cook(r"\xFF");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0], CookIssue::InvalidUtf8(_)));
        assert_eq!(s, "\u{FFFD}");
    }

    #[test]
    fn byte_escapes_combining_into_valid_utf8_cook_cleanly() {
        // \xC3\xA9 is 'é' (U+00E9) encoded as UTF-8.
        let (s, issues) = cook(r"\xC3\xA9");
        assert!(issues.is_empty());
        assert_eq!(s, "\u{E9}");
    }

    #[test]
    fn unrecognized_escape_is_left_literal() {
        let (s, issues) = cook(r"\q");
        assert!(issues.is_empty());
        assert_eq!(s, "\\q");
    }

    // --- heredoc/nowdoc flexible-indentation stripping ---------------------

    fn shape_of(src_text: &str) -> (HeredocShape, Vec<HeredocIndentIssue>) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src_text);
        let span = Span::new(file, 0, u32::try_from(src_text.len()).unwrap());
        heredoc_shape(map.file(file), span)
    }

    #[test]
    fn heredoc_shape_finds_no_indent_on_a_flush_left_marker() {
        let (shape, issues) = shape_of("<<<EOT\nhello\nEOT");
        assert!(issues.is_empty());
        assert_eq!(shape.indent, "");
    }

    #[test]
    fn heredoc_shape_extracts_the_closing_markers_indentation() {
        let (shape, issues) = shape_of("<<<EOT\n    hello\n    EOT");
        assert!(issues.is_empty());
        assert_eq!(shape.indent, "    ");
    }

    #[test]
    fn heredoc_shape_flags_mixed_space_and_tab_indentation() {
        let (shape, issues) = shape_of("<<<EOT\n\t hello\n\t EOT");
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            HeredocIndentIssue::MixedIndentWhitespace(_)
        ));
        // Recovery: treat the marker as unindented rather than guess.
        assert_eq!(shape.indent, "");
    }

    #[test]
    fn heredoc_shape_handles_an_empty_body() {
        // The closing marker line immediately follows the opening line, so
        // the single '\n' in the whole span both ends the header and starts
        // the marker line.
        let (shape, issues) = shape_of("<<<EOT\nEOT");
        assert!(issues.is_empty());
        assert_eq!(shape.indent, "");
    }

    fn dedent(
        indent: &str,
        src_text: &str,
        body_start: bool,
        is_last_run: bool,
    ) -> (String, Vec<HeredocIndentIssue>) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src_text);
        let span = Span::new(file, 0, u32::try_from(src_text.len()).unwrap());
        let mut issues = Vec::new();
        let s = dedent_heredoc_run(
            map.file(file),
            indent,
            span,
            body_start,
            is_last_run,
            &mut issues,
        );
        (s, issues)
    }

    #[test]
    fn dedent_is_a_no_op_with_no_indent() {
        let (s, issues) = dedent("", "  line one\n  line two\n", true, false);
        assert!(issues.is_empty());
        assert_eq!(s, "  line one\n  line two\n");
    }

    #[test]
    fn dedent_strips_matching_indentation_from_every_line() {
        let (s, issues) = dedent("    ", "    line one\n    line two\n", true, false);
        assert!(issues.is_empty());
        assert_eq!(s, "line one\nline two\n");
    }

    #[test]
    fn dedent_strips_the_one_trailing_newline_before_the_closing_marker() {
        let (s, issues) = dedent("    ", "    hello\n", true, true);
        assert!(issues.is_empty());
        assert_eq!(s, "hello");
    }

    #[test]
    fn dedent_strips_a_trailing_crlf_before_the_closing_marker() {
        let (s, issues) = dedent("    ", "    hello\r\n", true, true);
        assert!(issues.is_empty());
        assert_eq!(s, "hello");
    }

    #[test]
    fn dedent_exempts_a_truly_blank_line_from_the_indentation_check() {
        let (s, issues) = dedent("    ", "    a\n\n    b\n", true, false);
        assert!(issues.is_empty());
        assert_eq!(s, "a\n\nb\n");
    }

    #[test]
    fn dedent_flags_a_non_blank_line_with_insufficient_indentation() {
        let (s, issues) = dedent("    ", "    a\n  b\n", true, false);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            HeredocIndentIssue::InsufficientIndent(_)
        ));
        // Recovery: the offending line is left unstripped.
        assert_eq!(s, "a\n  b\n");
    }

    #[test]
    fn dedent_does_not_check_the_very_first_line_of_a_non_body_start_run() {
        // Simulates the run right after an interpolation site: its own
        // first "line" is really a continuation of the previous run's line,
        // so it must not be dedented even though it has no indentation.
        let (s, issues) = dedent("    ", "rest of line\n    next line\n", false, false);
        assert!(issues.is_empty());
        assert_eq!(s, "rest of line\nnext line\n");
    }

    #[test]
    fn heredoc_is_nowdoc_detects_the_single_quoted_label() {
        assert!(heredoc_is_nowdoc("<<<'EOT'\nraw\nEOT"));
        assert!(!heredoc_is_nowdoc("<<<EOT\nplain\nEOT"));
        assert!(!heredoc_is_nowdoc("<<<\"EOT\"\nplain\nEOT"));
    }

    #[test]
    fn cook_double_quoted_text_str_cooks_the_same_grammar_from_an_owned_string() {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", "whatever");
        let attribute_to = Span::new(file, 0, 8);
        let (s, issues) = cook_double_quoted_text_str(r"a\nb", attribute_to);
        assert!(issues.is_empty());
        assert_eq!(s, "a\nb");
    }
}
