//! `Core\Csv` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 12's second table, both members of it: `parse`, replacing PHP's
//! `str_getcsv` and the parsing half of `fgetcsv`, and `format`, replacing
//! `fputcsv`'s formatting half. Neither touches a file — a whole CSV document
//! is a `string` here, and the streaming half belongs to `Core\IO` at § 14.
//!
//! # The reader is a dependency and the writer is not
//!
//! `csv-core` is bound for [`nvs_core_csv_parse`] and nothing is bound for
//! [`nvs_core_csv_format`]. That asymmetry is
//! [ground-rules.md](/docs/adr/ground-rules.md)'s "an external
//! specification is a dependency rather than a hand-written parser" applied
//! where it actually bites: reading. RFC 4180's corners — a quoted field
//! holding the separator, a quoted field holding a bare `CRLF`, a doubled
//! quote inside one, a document that ends in the middle of a quoted field, a
//! `CRLF` terminator against a lone `LF` one — are each individually obvious
//! and are collectively what a hand-written scanner gets wrong one release at
//! a time. Writing has no such corners: the whole rule is *quote a field when
//! a byte in it would otherwise change the parse*, it is stated in full three
//! paragraphs down, and a writer cannot silently accept a document that means
//! something else.
//!
//! [ADR 0051](/docs/adr/0051-standard-library-tiers.md) § 4's first
//! question is **yes** — an uploaded spreadsheet is attacker-controlled text
//! reaching this member directly — so under that ADR a C dependency here would
//! need question 2's exceptional verification record and would not have one.
//! It does not arise: `csv-core` is pure Rust, `no_std`, has no build script,
//! and its one dependency is `memchr`, which this tree already carries under
//! `regex`. It is the parsing core of the `csv` crate the Rust ecosystem
//! reads for this, split out precisely so the caller owns the buffers — which
//! is what lets this module copy each field once, into the [`NvsStr`] it is
//! going to answer with, instead of through a `String` it would then copy
//! again.
//!
//! # The dialect: three bytes, and each one is the caller's
//!
//! `{separator?, quote?, escape?}` are the three knobs every real CSV dialect
//! turns, and each is a **single ASCII byte** written as a one-character
//! `string`. A multi-byte separator has no meaning to a byte-oriented reader,
//! and a non-ASCII one would cut a UTF-8 sequence in half, so both throw
//! rather than being silently truncated to a first byte. `CR` and `LF` are
//! refused for the same reason in reverse: they already terminate a record,
//! and a dialect that made one a separator would have no records at all. The
//! three must also name three different bytes.
//!
//! The defaults are RFC 4180's: `,` and `"`, and **no escape character at
//! all** — `escape` defaults to the empty string, which means a quote inside a
//! quoted field is written by doubling it and nothing else is special. That is
//! PHP 8.5's default too, after PHP spent a major version deprecating the `\`
//! it had inherited from nowhere in particular; `{escape: "\\"}` asks for the
//! old behaviour explicitly, and doubling keeps working alongside it.
//!
//! # `{header: true}` keys the rows and is not one of them
//!
//! The first record becomes the column names, every later record is keyed by
//! them, and the header record itself is not returned. The return type does
//! not change — it is `array<array<string>>` either way — because every Novis
//! array key is a `string` already, which is the whole reason this is an
//! option rather than a second member with a second return type.
//!
//! Two things a ragged document forces, both decided in favour of answering
//! rather than throwing, because this member sits at the edge of a request
//! where the caller is going to validate the rows anyway:
//!
//! - **A record with more fields than the header keys the surplus by its
//!   column index** — `"3"` for the fourth field of a three-column document —
//!   which is exactly the key that field would have had with no header at all.
//!   Nothing is dropped, and the row still says where each value came from.
//! - **A record with fewer fields than the header simply has fewer keys.** It
//!   is not padded with empty strings, because an absent column and a column
//!   holding `""` are different facts and a reader can tell them apart.
//!
//! A repeated column name keeps the **last** field under that name, which is
//! what any keyed structure over duplicate keys must do and what PHP's own
//! `array_combine` would answer.
//!
//! A blank line is not a record. `csv-core` discards a terminator seen at the
//! start of a record, so `a\n\nb\n` is two rows rather than three, and the
//! trailing newline every generator writes does not produce a phantom row of
//! one empty field.
//!
//! # Writing: one rule, four bytes
//!
//! [`nvs_core_csv_format`] quotes a field exactly when it holds the separator,
//! the quote, `CR` or `LF` — the four bytes whose presence would otherwise
//! change how the field parses back — and doubles the quote inside one. Every
//! record ends with `LF`, including the last, so `parse(format($rows))` is
//! `$rows` again.
//!
//! This deliberately does **not** reproduce `fputcsv`'s habit of quoting a
//! field that merely contains a space or a tab. Neither byte changes a parse,
//! RFC 4180 § 2.4 keeps the spaces inside a field either way, and quoting on
//! them makes a diff of two exports noisy for no reader's benefit. A caller
//! who needs every field quoted is asking for a different member, not for this
//! one's default to be looser.
//!
//! `{header: [...]}` writes those names as the first record and is otherwise
//! independent of the rows — a row is written by its **values in order**, its
//! keys ignored, which is what makes `format(parse($t, {header: true}),
//! {header: $names})` a round trip rather than a rekeying.
//!
//! # What it spends
//!
//! `parse` holds one output buffer the size of its input for the whole call,
//! plus the answer: one array per record, and one [`NvsStr`] per cell. Both
//! are freed with the request that produced them, and nothing is retained
//! between calls. `csv-core`'s DFA is a single forward pass with no
//! backtracking, so a hostile document costs O(n) in time and O(n) in memory
//! and cannot be made to cost more — there is no input that makes the reader
//! re-scan. Unescaping only ever removes bytes, so the one buffer is sized
//! once and never grows.
//!
//! `format` spends the string it answers and one column-name copy per cell it
//! keys, walking the rows with a borrowed handle apiece rather than retaining
//! them.
//!
//! # Known gaps
//!
//! 1. **No streaming.** Both members take and answer a whole document, so a
//!    file larger than memory has no reader here. That is `fgetcsv`'s other
//!    half and it lands with `Core\IO`'s file handles at spec § 14, over the
//!    same `csv-core` reader — this module's parse loop is already written as
//!    a fed-buffer loop rather than a whole-slice scan, so the incremental
//!    caller is the same code with a different feeder.

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Csv`'s fully-qualified name, written once so the registry row and
/// every diagnostic naming the class cannot drift apart.
pub(crate) const NAME: &str = r"Core\Csv";

/// `Core\Csv`'s registry rows — spec § 12's second table, whole.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "parse",
            names: &["text"],
            params: &[CoreTy::Str, CoreTy::Options(PARSE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Array(&CoreTy::Str)),
            symbol: "nvs_core_csv_parse",
            doc: Some(&PARSE_DOC),
        },
        CoreMethod {
            name: "format",
            names: &["rows"],
            params: &[
                CoreTy::Array(&CoreTy::Array(&CoreTy::Str)),
                CoreTy::Options(FORMAT_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_csv_format",
            doc: Some(&FORMAT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Csv::parse`'s reference card — ADR 0117.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Parses the whole CSV document `$text` into its records, as `str_getcsv` and the \
            parsing half of `fgetcsv` do, by RFC 4180's grammar: a field is quoted or it is not, \
            a doubled quote inside a quoted field is one quote, a record ends at `LF` or `CRLF`, \
            and a blank line is not a record.",
    params: &[
        ParamDoc {
            name: "text",
            desc: "The CSV document, whole.",
            shape: &[],
        },
        ParamDoc {
            name: "separator",
            desc: "The single ASCII byte between fields; `,` by default.",
            shape: &[],
        },
        ParamDoc {
            name: "quote",
            desc: "The single ASCII byte that quotes a field; `\"` by default.",
            shape: &[],
        },
        ParamDoc {
            name: "escape",
            desc: "The single ASCII byte that escapes a quote inside a quoted field, or the \
                   empty string for none, which is the default — a doubled quote is the RFC's \
                   own escape.",
            shape: &[],
        },
        ParamDoc {
            name: "header",
            desc: "Consume the first record as column names and key every returned row by \
                   them; the default keys each field by its column index.",
            shape: &[],
        },
    ],
    ret: "One array per record, every field a `string`, keyed by column index or — with \
          `header` — by the header's names, a field past the header's last name keeping its \
          index; the header row itself is not returned. The document itself never fails to \
          parse, and an empty document is the empty array.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A dialect option is empty, longer than one ASCII character, or `CR` or `LF`; or \
               two of `separator`, `quote` and `escape` name the same byte.",
    }],
};

/// `Core\Csv::format`'s reference card — ADR 0117.
const FORMAT_DOC: MethodDoc = MethodDoc {
    short: "Writes `$rows` as a CSV document, as `fputcsv`'s formatting half does over a whole \
            document: a field is quoted exactly when it holds the separator, the quote, `CR` \
            or `LF`, a quote inside one is doubled, and every record ends with `LF`, so \
            `parse(format($rows))` answers `$rows`.",
    params: &[
        ParamDoc {
            name: "rows",
            desc: "The records to write, each an array of `string` fields taken in order; a \
                   row's keys are ignored.",
            shape: &[],
        },
        ParamDoc {
            name: "separator",
            desc: "The single ASCII byte written between fields; `,` by default.",
            shape: &[],
        },
        ParamDoc {
            name: "quote",
            desc: "The single ASCII byte that quotes a field; `\"` by default.",
            shape: &[],
        },
        ParamDoc {
            name: "header",
            desc: "Column names written as the first record; `null`, the default, writes no \
                   header row.",
            shape: &[],
        },
    ],
    ret: "The document; the empty string for no rows and no header.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A dialect option is empty, longer than one ASCII character, or `CR` or `LF`; or \
               `separator` and `quote` name the same byte.",
    }],
};

/// `Core\Csv::parse`'s dialect and its one structural option.
///
/// The three byte options are RFC 4180's defaults; this module's own docs own
/// why `escape` defaults to *none* rather than to PHP's historical `\`.
const PARSE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "separator",
        ty: CoreTy::Str,
        default: Const::Str(","),
    },
    CoreOption {
        name: "quote",
        ty: CoreTy::Str,
        default: Const::Str("\""),
    },
    CoreOption {
        name: "escape",
        ty: CoreTy::Str,
        default: Const::Str(""),
    },
    CoreOption {
        name: "header",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `Core\Csv::format`'s two dialect bytes and its column names.
///
/// There is no `escape` here: a writer that doubles the quote never needs one,
/// and offering the choice would let a caller emit a document this crate's own
/// `parse` reads back differently.
///
/// `header` defaults to [`Const::Null`] rather than to an empty array, since
/// "no header row" and "a header row of no columns" are different documents —
/// the same reason `Core\Arr::sort`'s `{by?: callable}` defaults that way.
const FORMAT_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "separator",
        ty: CoreTy::Str,
        default: Const::Str(","),
    },
    CoreOption {
        name: "quote",
        ty: CoreTy::Str,
        default: Const::Str("\""),
    },
    CoreOption {
        name: "header",
        ty: CoreTy::Array(&CoreTy::Str),
        default: Const::Null,
    },
];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_csv_parse" => (nvs_core_csv_parse as *const ()).cast(),
        "nvs_core_csv_format" => (nvs_core_csv_format as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Arguments — the dialect, read the same way by both members
// ============================================================================

/// One `string` argument as text.
///
/// # Errors
///
/// One failure, and it is a [`Fault::fatal`]: a value that is not a `string`
/// at all, which the checker has already refused — this is the ABI's own
/// assertion, not a program-visible outcome. There is no *encoding* failure to
/// report, because the tag [`Value::as_text`] checks is itself ADR 0009's
/// UTF-8 guarantee; `crate::str`'s own `text` states why re-deriving it here
/// would be an O(n) pass over a buffer the runtime already knows the answer
/// for.
fn text_of<'a>(value: &'a Value, member: &str, position: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Csv::{member} expected {:?} for {position}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// One `bool` argument.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not a `bool`, as [`text_of`].
fn boolean(value: &Value, member: &str, position: &str) -> Result<bool, Fault> {
    value.as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Csv::{member} expected {:?} for {position}, got tag {}",
            Tag::Bool,
            value.tag_byte()
        ))
    })
}

/// One dialect option as the single ASCII byte it names.
///
/// # Errors
///
/// A [`Fault::thrown`] where the option is not exactly one ASCII character, or
/// is `CR` or `LF`. This is caller-supplied data rather than a compiler
/// oversight — the type is `string` and only its *value* can be wrong — so it
/// throws where [`text_of`]'s tag mismatch aborts.
fn dialect_byte(value: &Value, member: &str, option: &str) -> Result<u8, Fault> {
    let text = text_of(value, member, option)?;
    let refuse = |why: &str| {
        Fault::thrown(format!(
            "Core\\Csv::{member}(): the `{option}` option is {why}. A CSV dialect is written in \
             single ASCII bytes, since the separator, the quote and the escape are each compared \
             against one byte of the document"
        ))
    };
    match text.as_bytes() {
        [byte] if byte.is_ascii() && *byte != b'\r' && *byte != b'\n' => Ok(*byte),
        [b'\r' | b'\n'] => Err(refuse(
            "a carriage return or a line feed, which already ends a record",
        )),
        [] => Err(refuse("empty")),
        _ => Err(refuse("longer than one ASCII character")),
    }
}

/// `Core\Csv::parse`'s `{escape}`, where the empty string means *no escape
/// character*.
///
/// # Errors
///
/// [`dialect_byte`]'s, for a non-empty option that is not one ASCII byte.
fn optional_dialect_byte(value: &Value, member: &str, option: &str) -> Result<Option<u8>, Fault> {
    if text_of(value, member, option)?.is_empty() {
        return Ok(None);
    }
    dialect_byte(value, member, option).map(Some)
}

/// The three dialect bytes, checked against each other.
///
/// # Errors
///
/// A [`Fault::thrown`] where two of them are the same byte, which would make
/// the document's grammar ambiguous rather than merely unusual.
fn distinct(bytes: &[u8], member: &str) -> Result<(), Fault> {
    for (at, byte) in bytes.iter().enumerate() {
        if bytes[..at].contains(byte) {
            return Err(Fault::thrown(format!(
                "Core\\Csv::{member}(): two of the dialect options name the same byte, {:?}. A \
                 separator that is also the quote — or the escape — has no reading that recovers \
                 the fields",
                *byte as char
            )));
        }
    }
    Ok(())
}

/// One argument's array, borrowed for the length of the call.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not an array, as [`text_of`].
fn array_of(
    value: &Value,
    member: &str,
    position: &str,
) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let raw = value.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Csv::{member} expected {:?} for {position}, got tag {}",
            Tag::Array,
            value.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(raw))
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Csv::parse(string $text, {separator?, quote?, escape?, header?: bool}):
    /// array<array<string>>` — replacing PHP's `str_getcsv` and the parsing
    /// half of `fgetcsv`, over a whole document rather than one line.
    ///
    /// The grammar is RFC 4180's, read by `csv-core`: a field is quoted or it
    /// is not, a doubled quote inside a quoted field is one quote, a record
    /// ends at `LF` or `CRLF`, and a blank line is not a record. This module's
    /// own docs own the dialect defaults, what `{header: true}` does to the
    /// keys, and what a ragged record answers.
    ///
    /// # Errors
    ///
    /// [`dialect_byte`]'s and [`distinct`]'s, for a dialect that is not three
    /// distinct single ASCII bytes. The document itself never throws: this
    /// reader prefers *a* parse over *no* parse, which is the right side to
    /// err on at the edge of a request, where a malformed upload should be
    /// rejected by the code that knows what the columns mean rather than by
    /// the decoder.
    fn nvs_core_csv_parse(_ctx, args: [5]) {
        let text = text_of(&args[0], "parse", "the text")?;
        let separator = dialect_byte(&args[1], "parse", "separator")?;
        let quote = dialect_byte(&args[2], "parse", "quote")?;
        let escape = optional_dialect_byte(&args[3], "parse", "escape")?;
        let header = boolean(&args[4], "parse", "the `header` option")?;
        distinct(
            &[Some(separator), Some(quote), escape]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            "parse",
        )?;

        let mut reader = csv_core::ReaderBuilder::new()
            .delimiter(separator)
            .quote(quote)
            .escape(escape)
            .build();

        // Unescaping only ever removes bytes, so one buffer the size of the
        // whole document holds any single field this reader can produce and
        // never has to grow. `max(1)` because `csv-core` reads an empty output
        // buffer as `OutputFull` rather than as "nothing to write".
        let mut cell = vec![0_u8; text.len().max(1)];
        let mut field: Vec<u8> = Vec::new();
        let mut record: Vec<Vec<u8>> = Vec::new();
        let mut names: Vec<Vec<u8>> = Vec::new();
        let mut rows = NvsArray::new();
        let mut read = 0;

        loop {
            let (result, taken, written) = reader.read_field(&text.as_bytes()[read..], &mut cell);
            read += taken;
            field.extend_from_slice(&cell[..written]);
            match result {
                // The document is exhausted mid-field, or one field is longer
                // than the buffer: feed again, keeping what has been written.
                // Reading past the end is how the reader is told the document
                // has ended, which is what closes the final record.
                csv_core::ReadFieldResult::InputEmpty
                | csv_core::ReadFieldResult::OutputFull => {}
                csv_core::ReadFieldResult::Field { record_end } => {
                    record.push(std::mem::take(&mut field));
                    if record_end {
                        if header && names.is_empty() {
                            names = std::mem::take(&mut record);
                        } else {
                            rows.append(Value::array(row(&record, &names)));
                            record.clear();
                        }
                    }
                }
                csv_core::ReadFieldResult::End => break,
            }
        }

        Ok(Value::array(rows))
    }
}

/// One parsed record as an array, keyed by `names` where it has them.
///
/// A field past the end of `names` is keyed by its own column index, which is
/// the key it would have had with no header at all; this module's docs own why
/// that is answered rather than thrown.
fn row(record: &[Vec<u8>], names: &[Vec<u8>]) -> NvsArray {
    let mut out = NvsArray::new();
    for (column, field) in record.iter().enumerate() {
        let value = Value::str(NvsStr::new(field));
        match names.get(column) {
            Some(name) => out.set(NvsStr::new(name), value),
            None if names.is_empty() => out.append(value),
            None => out.set(NvsStr::new(column.to_string().as_bytes()), value),
        }
    }
    out
}

nvs_runtime::nvs_helper! {
    /// `Core\Csv::format(array<array<string>> $rows, {separator?, quote?,
    /// header?: array<string>}): string` — replacing `fputcsv`'s formatting
    /// half, over a whole document rather than one line.
    ///
    /// A field is quoted exactly when it holds the separator, the quote, `CR`
    /// or `LF`, and a quote inside one is doubled. Every record ends with
    /// `LF`, so `parse(format($rows))` answers `$rows`. A row is written by
    /// its values in order and its keys are ignored — the module docs own why,
    /// and own why there is no `escape` option here.
    ///
    /// # Errors
    ///
    /// [`dialect_byte`]'s and [`distinct`]'s for the dialect, and a
    /// [`Fault::thrown`] for a row that is not an array of `string`s — which
    /// [`write_record`] holds no source program can produce, and says why.
    fn nvs_core_csv_format(_ctx, args: [4]) {
        let rows = array_of(&args[0], "format", "the rows")?;
        let separator = dialect_byte(&args[1], "format", "separator")?;
        let quote = dialect_byte(&args[2], "format", "quote")?;
        distinct(&[separator, quote], "format")?;

        let mut out: Vec<u8> = Vec::new();
        if args[3].tag() != Some(Tag::Null) {
            let names = array_of(&args[3], "format", "the `header` option")?;
            write_record(&mut out, &names, separator, quote)?;
        }

        let mut slot = 0;
        while let Some(live) = rows.next_slot(slot) {
            slot = live + 1;
            let value = rows.value_at(live).expect("a live slot has a value");
            let record = array_of(&value, "format", "a row")?;
            write_record(&mut out, &record, separator, quote)?;
        }

        // Every byte written is either one of the argument's — which are UTF-8
        // by ADR 0009 — or the separator, the quote or an `LF`, each of which
        // `dialect_byte` has already held to ASCII.
        Ok(Value::str(NvsStr::new(&out)))
    }
}

/// One record's fields, in slot order, terminated with `LF`.
///
/// # Errors
///
/// A [`Fault::thrown`] naming the column, for a cell that is not a `string` —
/// which no source program produces, for the reason the guard itself states.
fn write_record(
    out: &mut Vec<u8>,
    record: &NvsArray,
    separator: u8,
    quote: u8,
) -> Result<(), Fault> {
    let mut slot = 0;
    let mut column = 0;
    while let Some(live) = record.next_slot(slot) {
        slot = live + 1;
        let value = record.value_at(live).expect("a live slot has a value");
        // Both call sites are `array<string>` — `format`'s rows are
        // `array<array<string>>` and its `header` option is `array<string>` —
        // so a `mixed` cell is `E0401` at the argument, and the one spelling
        // that fills such a binding from untyped data, `Core\Json::decode(…)
        // as array<string>`, refuses per *element* at the conversion. Probed
        // with `nvs run` at one level and at two: unreachable from source, and
        // kept because it is what makes `as_str_bytes` safe to unwrap here.
        let field = value.as_str_bytes().ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Csv::format(): column {column} of a row holds a value that is not a \
                 `string`, so there is no field a CSV record could write it as"
            ))
        })?;
        if column > 0 {
            out.push(separator);
        }
        write_field(out, field, separator, quote);
        column += 1;
    }
    out.push(b'\n');
    Ok(())
}

/// One field, quoted only where a byte in it would otherwise change the parse.
fn write_field(out: &mut Vec<u8>, field: &[u8], separator: u8, quote: u8) {
    let structural =
        |byte: u8| byte == separator || byte == quote || byte == b'\r' || byte == b'\n';
    if !field.iter().copied().any(structural) {
        out.extend_from_slice(field);
        return;
    }
    out.push(quote);
    for byte in field.iter().copied() {
        if byte == quote {
            out.push(quote);
        }
        out.push(byte);
    }
    out.push(quote);
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, OutputSink, call};

    use super::{NvsStr, Value, dialect_byte, distinct, write_field};

    /// Runs one member through the ADR 0002 boundary compiled code reaches it
    /// at, releasing every value this test built afterwards — the helper
    /// convention borrows, so the caller still owns them.
    fn run(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        args: &[Value],
    ) -> Result<Value, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(member, &mut ctx, args);
        for arg in args {
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference it built for each \
                          argument, and the helper borrowed rather than \
                          consumed it"
            )]
            unsafe {
                arg.release();
            }
        }
        result
    }

    /// A `string` argument.
    fn s(text: &str) -> Value {
        Value::str(NvsStr::new(text.as_bytes()))
    }

    /// `parse` over the default dialect, as the key/value pairs of each row.
    fn parse(text: &str, header: bool) -> Vec<Vec<(String, String)>> {
        let answer = run(
            super::nvs_core_csv_parse,
            &[s(text), s(","), s("\""), s(""), Value::bool(header)],
        )
        .expect("the default dialect never throws");
        let out = read(&answer);
        #[expect(
            unsafe_code,
            reason = "a returned heap value carries one fresh reference, which \
                      the caller owns"
        )]
        unsafe {
            answer.release();
        }
        out
    }

    /// Every row of a `parse` answer, as its key/value pairs in order.
    fn read(answer: &Value) -> Vec<Vec<(String, String)>> {
        let rows = crate::arr::borrowed(answer.array_ptr().expect("`parse` answers an array"));
        let mut out = Vec::new();
        let mut slot = 0;
        while let Some(live) = rows.next_slot(slot) {
            slot = live + 1;
            let record = crate::arr::borrowed(
                rows.value_at(live)
                    .expect("a live slot has a value")
                    .array_ptr()
                    .expect("every row is an array"),
            );
            let mut cells = Vec::new();
            let mut inner = 0;
            while let Some(cell) = record.next_slot(inner) {
                inner = cell + 1;
                let key = record.key_at(cell).expect("a live slot has a key");
                let value = record.value_at(cell).expect("a live slot has a value");
                cells.push((
                    String::from_utf8(key.as_bytes().to_vec()).expect("a key is text"),
                    String::from_utf8(
                        value
                            .as_str_bytes()
                            .expect("every cell is a string")
                            .to_vec(),
                    )
                    .expect("a cell is text"),
                ));
            }
            out.push(cells);
        }
        out
    }

    /// The fields a row holds, without their keys.
    fn fields(row: &[(String, String)]) -> Vec<&str> {
        row.iter().map(|(_, value)| value.as_str()).collect()
    }

    /// The keys a row holds.
    fn keys(row: &[(String, String)]) -> Vec<&str> {
        row.iter().map(|(key, _)| key.as_str()).collect()
    }

    #[test]
    fn a_trailing_newline_does_not_add_a_row() {
        let rows = parse("a,b\nc,d\n", false);
        assert_eq!(rows.len(), 2);
        assert_eq!(fields(&rows[1]), ["c", "d"]);
        assert_eq!(keys(&rows[0]), ["0", "1"]);
    }

    #[test]
    fn a_document_without_a_trailing_newline_still_ends_its_record() {
        let rows = parse("a,b", false);
        assert_eq!(rows.len(), 1);
        assert_eq!(fields(&rows[0]), ["a", "b"]);
    }

    #[test]
    fn a_blank_line_is_not_a_record() {
        assert_eq!(parse("a\n\nb\n", false).len(), 2);
    }

    #[test]
    fn empty_text_is_no_rows_at_all() {
        assert!(parse("", false).is_empty());
    }

    /// RFC 4180 § 2.5-2.7, the three things a quoted field is for.
    #[test]
    fn a_quoted_field_holds_the_separator_a_newline_and_a_doubled_quote() {
        let rows = parse("\"a,b\",\"c\r\nd\",\"e\"\"f\"\n", false);
        assert_eq!(fields(&rows[0]), ["a,b", "c\r\nd", "e\"f"]);
    }

    #[test]
    fn crlf_terminates_a_record_as_lf_does() {
        let rows = parse("a,b\r\nc,d\r\n", false);
        assert_eq!(rows.len(), 2);
        assert_eq!(fields(&rows[1]), ["c", "d"]);
    }

    #[test]
    fn a_header_keys_every_later_row_and_is_not_one() {
        let rows = parse("name,qty\nfig,2\n", true);
        assert_eq!(rows.len(), 1);
        assert_eq!(keys(&rows[0]), ["name", "qty"]);
        assert_eq!(fields(&rows[0]), ["fig", "2"]);
    }

    /// This module's docs own both halves of the ragged rule.
    #[test]
    fn a_ragged_row_keys_its_surplus_by_column_and_pads_nothing() {
        let rows = parse("a,b\n1,2,3\n4\n", true);
        assert_eq!(keys(&rows[0]), ["a", "b", "2"]);
        assert_eq!(keys(&rows[1]), ["a"]);
        assert_eq!(fields(&rows[1]), ["4"]);
    }

    #[test]
    fn a_repeated_column_name_keeps_the_last_field() {
        let rows = parse("a,a\n1,2\n", true);
        assert_eq!(keys(&rows[0]), ["a"]);
        assert_eq!(fields(&rows[0]), ["2"]);
    }

    /// The writer's one rule, over every byte that triggers it.
    #[test]
    fn a_field_is_quoted_only_where_a_byte_would_change_the_parse() {
        let mut out = Vec::new();
        write_field(&mut out, b"plain text", b',', b'"');
        write_field(&mut out, b"|", b',', b'"');
        write_field(&mut out, b"a,b", b',', b'"');
        write_field(&mut out, b"a\"b", b',', b'"');
        write_field(&mut out, b"a\nb", b',', b'"');
        assert_eq!(
            String::from_utf8(out).expect("ascii"),
            "plain text|\"a,b\"\"a\"\"b\"\"a\nb\""
        );
    }

    /// A dialect option is one ASCII byte, and every other spelling throws
    /// rather than being truncated to a first byte.
    #[test]
    fn a_multi_byte_or_empty_separator_throws() {
        for spelling in ["", ";;", "€", "\n"] {
            let value = Value::str(NvsStr::new(spelling.as_bytes()));
            assert!(dialect_byte(&value, "parse", "separator").is_err());
        }
    }

    #[test]
    fn two_dialect_options_naming_one_byte_throw() {
        assert!(distinct(b",,", "parse").is_err());
        assert!(distinct(b",\"\\", "parse").is_ok());
    }
}
