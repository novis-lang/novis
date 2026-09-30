//! `Core\Csv` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 12's second table: `parse`, replacing PHP's `str_getcsv` and the parsing
//! half of `fgetcsv`; `format`, replacing `fputcsv`'s formatting half; and
//! `rows`, which walks a file `Core\IO` has open a record at a time and is the
//! `while (fgetcsv($handle))` loop. `parse` and `format` take and answer a
//! whole document as a `string`; `rows` is the member here that reads octets
//! itself, through a handle whose door was passed at `Core\IO::open`.
//!
//! # The reader is a dependency and the writer is not
//!
//! `csv-core` is bound for [`nvs_core_csv_parse`] and for the walk
//! [`nvs_core_csv_rows`] opens, and nothing is bound for
//! [`nvs_core_csv_format`]. That asymmetry is
//! [ground-rules.md](/docs/ground-rules.md)'s "an external
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
//! `rule:packaging/a-c-dependency-answers-two-questions`'s first
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
//! `rule:classes/an-encoder-ends-a-cycle-by-identity` has nothing to carry
//! here, because this writer cannot reach an object graph at all: its walk is
//! exactly two levels and neither level recurses — [`nvs_core_csv_format`]
//! walks the rows, [`write_record`] walks one record's cells, and a cell is
//! required to be a `string` by its tag alone, which reads no member and
//! renders no object. There is therefore no path from the root long enough to
//! meet a value twice, and a value that is not a field is refused where it
//! sits rather than descended into.
//!
//! # `rows` walks an open handle, and holds one record
//!
//! [`nvs_core_csv_rows`] takes the same dialect and the same `{header: true}`
//! as `parse` and answers a [`ROWS`] the program walks with `foreach`. What it
//! reads is a `Core\IO\File` — the descriptor stays in the request's own table
//! where `Core\IO::open` filed it, so this member opens nothing, asks for no
//! capability of its own, and leaves `$file->tell()` where the walk has got to.
//! A record is keyed exactly as `parse` keys one, ragged rows and repeated
//! column names included, because [`row`] is the one function that decides so.
//!
//! # Decision: the walk takes the handle, and R14 admits exactly this shape
//!
//! `rule:core-api/a-lifetime-is-an-object` refuses the `$link`-first calling
//! convention — `fread($handle, 8)` as a shape — and this member takes a
//! handle as parameter 1. What that rule is protecting is named in it: a
//! handle-first free function has nowhere to enforce a capability and nothing
//! to hang an API on. Neither applies here. The capability was enforced at the
//! door this handle came through, and the API hangs on the class that owns the
//! grammar, because what `rows` knows is CSV and not files.
//!
//! The alternative is the one the rule's own reasoning points at, and it is
//! worse: `Core\IO\File::csvRows()` puts a format's grammar on the class whose
//! subject is a descriptor, and then every format that can be read
//! incrementally asks for a member there too. `Core\Csv::rows(string $path)`
//! is worse still — it would be a second door onto the filesystem beside
//! `Core\IO::open`, and `rule:security/capability-check-at-the-door` has one.
//! So the member takes the handle and **neither opens nor closes it**: it
//! reads forward and leaves the position where it got to, which is the whole
//! of what it does to something it does not own.
//!
//! **A walk is taken once.** Each `advance()` reads forward, so the walk ends
//! where the file does and a second `foreach` over the same value answers no
//! records — the difference from `Core\IO::lines`, which holds a list it can
//! hand out again, and the difference that buys the footprint below. A program
//! that wants the document twice asks `$file->seek(0)` for a second walk or
//! reads it with `parse`.
//!
//! The parse state a record boundary cannot live without — `csv-core`'s DFA,
//! the chunk last read, the field in progress — is native, so it is parked on
//! the request through [`nvs_runtime::Ctx::hold_open_reader`] and the walk's
//! slot holds the key ([`crate::instance`]'s first decision is why a `Core`
//! slot cannot hold it directly). It is released when the walk reaches the end
//! of the file, and with the request otherwise.
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
//! `rows` spends [`READ_CHUNK`] plus [`CELL`] bytes per open walk — the chunk
//! last read off the file and the buffer a field is unescaped into — plus the
//! header's names, the record being assembled and the one record the walk is
//! standing on. That is O(1) in the document's size and O(1) in its record
//! count, charged to the request that opened the walk and released at the end
//! of the file; a field or a record longer than a buffer costs another pass
//! and not another buffer. **The document is never held**, which is the whole
//! difference from composing `Core\IO::read` with `parse`.
//!
//! # Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-stdlib/src/csv.rs` lists them.

use std::io::Read;

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Csv`'s fully-qualified name, written once so the registry row and
/// every diagnostic naming the class cannot drift apart.
pub(crate) const NAME: &str = r"Core\Csv";

/// `Core\Csv`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads CSV text into rows of strings, and writes rows back as CSV. It can use a header \
            row, and a different separator or quote character.",
};

/// `Core\Csv\Rows`'s class card — `rule:core-api/reference-card`.
const ROWS_CARD: ClassDoc = ClassDoc {
    short: "The rows of a CSV file, read one at a time while a `foreach` loop runs. The whole \
            file is never in memory at once.",
};

/// `Core\Csv`'s registry rows — spec § 12's second table, whole.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
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
        CoreMethod {
            name: "rows",
            names: &["file"],
            // The subject is the handle — R1 — and the options are `parse`'s
            // own slice rather than a copy of it, because a dialect that could
            // differ between the two members would be two grammars for one
            // format.
            params: &[
                CoreTy::Instance(crate::io::FILE_NAME),
                CoreTy::Options(PARSE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(ROWS_NAME),
            symbol: "nvs_core_csv_rows",
            doc: Some(&ROWS_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Csv\Rows`'s fully-qualified name, written once for the reason
/// [`NAME`] is.
pub(crate) const ROWS_NAME: &str = r"Core\Csv\Rows";

/// The symbols behind this walk's `iterate()`, `advance()` and `current()`,
/// reached by name through [`crate::instance`]'s dispatch roster rather than
/// registered as members — `Core\Db\Stream`'s three, for its reason.
pub(crate) const ROWS_ITERATE_SYMBOL: &str = "nvs_core_csv_rows_iterate";
/// See [`ROWS_ITERATE_SYMBOL`].
pub(crate) const ROWS_ADVANCE_SYMBOL: &str = "nvs_core_csv_rows_advance";
/// See [`ROWS_ITERATE_SYMBOL`].
pub(crate) const ROWS_CURRENT_SYMBOL: &str = "nvs_core_csv_rows_current";

/// [`ROWS`]'s first slot: the key [`nvs_runtime::Ctx::hold_open_reader`] filed
/// this walk's parse state under, and `0` once the walk has reached the end of
/// the file.
const ROWS_READER_AT: usize = 0;
/// [`ROWS`]'s second slot: the path the handle was opened on, so that a read
/// that fails mid-walk names the file the way the program wrote it —
/// `Core\IO\File`'s own second slot, for the same reason and at the same cost.
const ROWS_PATH_AT: usize = 1;
/// [`ROWS`]'s third slot: the record the last `advance()` read, `null` before
/// the first one and after the last. This is where the member's promise is
/// kept — one record, whatever the document's size.
const ROWS_RECORD_AT: usize = 2;

/// What [`nvs_core_csv_rows`] answers with: spec § 12's streaming read, as a
/// class a return type can name.
///
/// # Why it carries `advance()` and `current()` itself
///
/// Every `Core` collection that answers `iterate()` with a [`crate::cursor`]
/// is walking a snapshot it already holds. A streamed record is not in one —
/// it does not exist until the file is read that far — so this class *is* its
/// own iterator, exactly as `Core\Db\Stream` and `Core\Request\BodyStream`
/// are and for the same reason: naming the walk is not reading it. Nothing is
/// read by `rows` itself, so a program that names a walk and never takes it
/// has read no records at all.
///
/// # Why it has no members
///
/// Everything it does is the iteration trio, dispatched by name — so it is a
/// *handle* in the sense `registry`'s
/// `a_class_with_slots_has_instance_members_and_the_reverse` names, beside
/// `Core\IO\Lines`: its slots are read, just not through a member of its own.
pub(crate) const ROWS: CoreClass = CoreClass {
    name: ROWS_NAME,
    doc: Some(&ROWS_CARD),
    methods: &[],
    instance: &[],
    slots: &["reader", "path", "record"],
    constants: &[],
};

/// `Core\Csv::parse`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Csv::format`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Csv::rows`'s reference card — `rule:core-api/reference-card`.
const ROWS_DOC: MethodDoc = MethodDoc {
    short: "Walks an open file one record at a time, as `while ($r = fgetcsv($h))` does, by the \
            same RFC 4180 grammar and the same dialect `parse` reads. The walk holds one record \
            and never the document, so a file larger than memory reads fine; it reads forward \
            from wherever the handle is and is taken once.",
    params: &[
        ParamDoc {
            name: "file",
            desc: "The open handle to read from, positioned where the walk should start.",
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
            desc: "Consume the first record as column names and key every walked record by \
                   them; the default keys each field by its column index.",
            shape: &[],
        },
    ],
    ret: "A walk over the records, each an array of `string` fields keyed as `parse` keys one. \
          Nothing is read until the walk is taken.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "A dialect option is empty, longer than one ASCII character, or `CR` or `LF`; \
                   two of `separator`, `quote` and `escape` name the same byte; or the handle \
                   was closed before or during the walk.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system refused a read part way through the walk.",
        },
    ],
};

/// The reading dialect and its one structural option, shared by `parse` and
/// `rows` — one slice rather than two rosters, because the two members read
/// one grammar and an option either of them did not have would be the other's
/// document read a second way.
///
/// The three byte options are RFC 4180's defaults; this module's own docs own
/// why `escape` defaults to *none* rather than to PHP's historical `\`.
///
/// Each of them is [`Qual::Neutral`] under
/// `rule:security/unclassified-parameter-refuses-tainted`: a dialect byte is
/// compared against the document and never written into what either member
/// answers, so the records carry the *document's* qualifier and never a
/// separator's.
const PARSE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "separator",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Str(","),
    },
    CoreOption {
        name: "quote",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Str("\""),
    },
    CoreOption {
        name: "escape",
        ty: CoreTy::Text(Qual::Neutral),
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
        "nvs_core_csv_rows" => (nvs_core_csv_rows as *const ()).cast(),
        // The walk's three, which no registry row names: they are reached by
        // name through the dispatch roster, which resolves them through this
        // same function.
        ROWS_ITERATE_SYMBOL => (nvs_core_csv_rows_iterate as *const ()).cast(),
        ROWS_ADVANCE_SYMBOL => (nvs_core_csv_rows_advance as *const ()).cast(),
        ROWS_CURRENT_SYMBOL => (nvs_core_csv_rows_current as *const ()).cast(),
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
/// report, because the tag [`Value::as_text`] checks is itself `rule:types/bytes`'s
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
        // by `rule:types/bytes` — or the separator, the quote or an `LF`, each of which
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

// ============================================================================
// The walk — `rows`, and the three names a `foreach` reaches it by
// ============================================================================

/// How much of the file one read asks for, and so the most of it a walk holds
/// at once. A record spanning the end of a chunk costs one more read and
/// nothing else, so this is a syscall-count choice rather than a limit on what
/// a document may hold.
const READ_CHUNK: usize = 8 * 1024;

/// The buffer one field is unescaped through. A field longer than this is read
/// in several turns — `csv-core` answers `OutputFull` and the bytes already
/// written are kept — so it bounds the copy and not the field.
const CELL: usize = 8 * 1024;

/// The parse state one open walk parks on the request: everything a record
/// boundary needs that outlives a single `advance()` and that a `Core` slot
/// cannot hold.
#[derive(Debug)]
struct RowReader {
    /// The handle the records are read from, as `Core\IO`'s own table keys it.
    file: u64,
    /// `csv-core`'s DFA, left mid-document between two `advance()` calls.
    reader: csv_core::Reader,
    /// The chunk last read off the file.
    pending: Vec<u8>,
    /// How much of [`RowReader::pending`] the DFA has taken.
    at: usize,
    /// The field being unescaped, across as many turns as it takes.
    field: Vec<u8>,
    /// The buffer [`RowReader::field`] is unescaped through, [`CELL`] wide.
    cell: Vec<u8>,
    /// The record being assembled, one field per column read so far.
    record: Vec<Vec<u8>>,
    /// The header's names, where `{header: true}` asked for them.
    names: Vec<Vec<u8>>,
    /// Whether the first record is the header rather than a row.
    header: bool,
    /// Whether the file has answered a read with nothing.
    eof: bool,
    /// Whether the DFA has answered its last record, after which this reader
    /// holds no buffers at all.
    done: bool,
}

/// What one turn of [`RowReader::step`] produced.
enum Step {
    /// A whole record, keyed as [`row`] keys one.
    Record(NvsArray),
    /// Everything fed so far is taken, and the file has not ended yet.
    NeedInput,
    /// The document is over.
    End,
}

impl RowReader {
    /// The next record out of the bytes this reader already holds, or what it
    /// needs in order to answer one.
    ///
    /// A header record is consumed here rather than answered, so the first
    /// record a `{header: true}` walk hands out is the first data row — which
    /// is `parse`'s rule, kept in the one place either member decides it.
    fn step(&mut self) -> Step {
        loop {
            if self.done {
                return Step::End;
            }
            if self.at >= self.pending.len() && !self.eof {
                return Step::NeedInput;
            }
            let (result, taken, written) = self
                .reader
                .read_field(&self.pending[self.at..], &mut self.cell);
            self.at += taken;
            self.field.extend_from_slice(&self.cell[..written]);
            match result {
                // The chunk ended mid-field, or the field is longer than the
                // buffer: keep what has been written and go round for more.
                // Reading past the end is how the reader is told the document
                // has ended, which is what closes the final record.
                csv_core::ReadFieldResult::InputEmpty | csv_core::ReadFieldResult::OutputFull => {}
                csv_core::ReadFieldResult::Field { record_end } => {
                    self.record.push(std::mem::take(&mut self.field));
                    if record_end {
                        if self.header && self.names.is_empty() {
                            self.names = std::mem::take(&mut self.record);
                        } else {
                            let built = row(&self.record, &self.names);
                            // Cleared rather than taken, so the next record
                            // reuses the fields' own allocation.
                            self.record.clear();
                            return Step::Record(built);
                        }
                    }
                }
                csv_core::ReadFieldResult::End => {
                    self.finish();
                    return Step::End;
                }
            }
        }
    }

    /// Takes the bytes just read off the file — or, where `chunk` is empty,
    /// marks the end of it, which is how `csv-core` is told to flush the record
    /// it is part way through.
    fn feed(&mut self, chunk: &[u8]) {
        self.pending.clear();
        self.pending.extend_from_slice(chunk);
        self.at = 0;
        self.eof = chunk.is_empty();
    }

    /// Gives up every buffer at the end of the walk, leaving a reader that
    /// answers [`Step::End`] to anything asked of it.
    ///
    /// The entry stays in the request's table rather than being taken out of
    /// it, so a `foreach` over a drained walk answers no records where a
    /// missing reader would be a fault. What that costs is the table's own
    /// `Option<Box<…>>`, which a key that is never reused costs anyway.
    fn finish(&mut self) {
        self.done = true;
        self.pending = Vec::new();
        self.cell = Vec::new();
        self.field = Vec::new();
        self.record = Vec::new();
        self.names = Vec::new();
    }
}

impl nvs_runtime::HeldReader for RowReader {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The catchable `RuntimeError` a walk raises for a handle the program has
/// given up.
///
/// A `RuntimeError` and not an `IOError` for `Core\IO\File`'s own reason:
/// nothing about the file went wrong, the program asked a question of
/// something it had already closed.
fn closed(path: &Value) -> Fault {
    Fault::thrown(format!(
        "Core\\Csv::rows: this handle is closed — {}",
        path.as_text().unwrap_or("?")
    ))
}

/// The walk's parked parse state, as the type this module filed it as.
///
/// # Errors
///
/// A [`Fault::fatal`] where the request holds no such reader. No program can
/// reach it: the key is written by [`nvs_core_csv_rows`] and by nothing else,
/// and a drained walk keeps its entry rather than giving it up.
fn reader_at(ctx: &mut nvs_runtime::Ctx, key: u64) -> Result<&mut RowReader, Fault> {
    ctx.open_reader_mut(key)
        .and_then(|reader| reader.as_any_mut().downcast_mut::<RowReader>())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{ROWS_NAME}::{} found no reader under the key in its `{}` slot",
                nvs_runtime::sequence::ADVANCE,
                ROWS.slots[ROWS_READER_AT]
            ))
        })
}

/// The next record of `key`'s walk, reading the file wherever the parked state
/// has run out — `None` at the end of the document.
///
/// **The descriptor and the parse state are read in turn and never together**,
/// because they are two tables of one context: a frame holding both borrows at
/// once would not compile, and the chunk between them is a stack buffer rather
/// than a third place either could be copied to.
///
/// # Errors
///
/// [`closed`]'s `RuntimeError` for a handle given up under the walk, an
/// `IOError` for a read the operating system refused, and [`reader_at`]'s
/// fatal for a key no walk filed.
fn next_record(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    path: &Value,
) -> Result<Option<NvsArray>, Fault> {
    loop {
        let reader = reader_at(ctx, key)?;
        let file = reader.file;
        match reader.step() {
            Step::Record(record) => return Ok(Some(record)),
            Step::End => return Ok(None),
            Step::NeedInput => {}
        }
        let mut buffer = [0_u8; READ_CHUNK];
        let read = {
            let open = ctx.open_file_mut(file).ok_or_else(|| closed(path))?;
            open.read(&mut buffer)
        }
        .map_err(|err| {
            nvs_runtime::capability::io_failure(
                "Core\\Csv::rows",
                std::path::Path::new(path.as_text().unwrap_or("?")),
                &err,
            )
        })?;
        reader_at(ctx, key)?.feed(&buffer[..read]);
    }
}

/// One step of the walk: the next record parked into [`ROWS_RECORD_AT`], and
/// whether there was one.
///
/// # Errors
///
/// [`next_record`]'s, and a [`Fault::fatal`] for a receiver whose slots hold
/// the wrong shape — this crate's paste error rather than a program's.
fn rows_step(ctx: &mut nvs_runtime::Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = crate::instance::receiver(value, &ROWS, member)?;
    let key = crate::instance::slot(receiver, ROWS_READER_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{ROWS_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                ROWS.slots[ROWS_READER_AT]
            ))
        })?;
    let path = crate::instance::slot(receiver, ROWS_PATH_AT);
    // `set_slot` releases what it displaces, so the record the previous
    // `advance()` parked is freed right here unless the loop body is still
    // holding it — and the end of the walk clears the slot rather than leaving
    // the last record in it, which is what makes the footprint one record
    // whatever the document held.
    match next_record(ctx, key, &path)? {
        None => {
            crate::instance::set_slot(receiver, ROWS_RECORD_AT, Value::null());
            Ok(Value::bool(false))
        }
        Some(record) => {
            crate::instance::set_slot(receiver, ROWS_RECORD_AT, Value::array(record));
            Ok(Value::bool(true))
        }
    }
}

/// The record the last `advance()` read, with a reference of its own: the slot
/// keeps its until the next step, so a loop body that keeps a record keeps a
/// value nothing can invalidate.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not one of this class's
/// instances, which compiled code cannot produce.
fn rows_record(value: Value, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, &ROWS, member)?;
    let held = crate::instance::slot(receiver, ROWS_RECORD_AT);
    #[expect(
        unsafe_code,
        reason = "the slot keeps its reference until the next `advance`, so the \
                  value handed back needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Core\Csv::rows(Core\IO\File $file, {separator?, quote?, escape?,
    /// header?: bool}): Core\Csv\Rows` — `fgetcsv`'s other half, as a value a
    /// `foreach` walks.
    ///
    /// **Nothing is read here.** The dialect is checked, the parse state is
    /// parked on the request, and the walk is answered standing on no record —
    /// so the first read is the first `advance()`, and a program that names a
    /// walk and never takes it has read nothing. This module's docs own what
    /// the walk holds, why it is taken once, and why it needs no capability of
    /// its own: the door is `Core\IO::open`'s and this handle has been through
    /// it.
    ///
    /// # Errors
    ///
    /// [`dialect_byte`]'s and [`distinct`]'s, for a dialect that is not
    /// distinct single ASCII bytes, and [`closed`]'s `RuntimeError` for a
    /// handle that is already closed.
    fn nvs_core_csv_rows(ctx, args: [5]) {
        let (file, path) = crate::io::handle_of(args[0], "rows")?;
        let separator = dialect_byte(&args[1], "rows", "separator")?;
        let quote = dialect_byte(&args[2], "rows", "quote")?;
        let escape = optional_dialect_byte(&args[3], "rows", "escape")?;
        let header = boolean(&args[4], "rows", "the `header` option")?;
        distinct(
            &[Some(separator), Some(quote), escape]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            "rows",
        )?;
        // Refused where the program can still see which call was wrong, rather
        // than at the first `advance()` inside a `foreach` header.
        if ctx.open_file_mut(file).is_none() {
            return Err(closed(&path));
        }

        #[expect(
            unsafe_code,
            reason = "the path is borrowed off the handle, which is live for the \
                      length of this call, and the walk built below owns the \
                      reference this takes"
        )]
        unsafe {
            path.retain();
        }
        let key = ctx.hold_open_reader(Box::new(RowReader {
            file,
            reader: csv_core::ReaderBuilder::new()
                .delimiter(separator)
                .quote(quote)
                .escape(escape)
                .build(),
            pending: Vec::new(),
            at: 0,
            field: Vec::new(),
            cell: vec![0_u8; CELL],
            record: Vec::new(),
            names: Vec::new(),
            header,
            eof: false,
            done: false,
        }));
        Ok(crate::instance::build(
            &ROWS,
            [Value::uint(key), path, Value::null()],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<array<string>>::iterate(): Iterator<array<string>>` — the walk
    /// itself, because the next record does not exist when the walk is named.
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed,
    /// and handing that reference straight back out is what makes the cursor
    /// the open walk rather than a copy of it.
    fn nvs_core_csv_rows_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &ROWS, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<array<string>>::advance(): bool` — reads the next record off
    /// the file, answering `false` at the end of the document and giving up
    /// the walk's buffers there.
    fn nvs_core_csv_rows_advance(ctx, args: [1]) {
        let stepped = rows_step(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<array<string>>::current(): array<string>` — the record the
    /// last `advance()` read, and the only one this walk is holding.
    fn nvs_core_csv_rows_current(_ctx, args: [1]) {
        let read = rows_record(args[0], nvs_runtime::sequence::CURRENT);
        crate::cursor::consume(args[0]);
        read
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, OutputSink, call};

    use super::{NvsArray, NvsStr, Tag, Value, dialect_byte, distinct, write_field};

    /// Runs one member through the `rule:errors/propagation` boundary compiled code reaches it
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

    /// One record's key/value pairs, in slot order — the shape both members
    /// that answer records are read through, so a walked record and a parsed
    /// one are compared as they are rather than as two shapes.
    fn cells(record: &Value) -> Vec<(String, String)> {
        let record = crate::arr::borrowed(record.array_ptr().expect("every record is an array"));
        let mut out = Vec::new();
        let mut slot = 0;
        while let Some(live) = record.next_slot(slot) {
            slot = live + 1;
            let key = record.key_at(live).expect("a live slot has a key");
            let value = record.value_at(live).expect("a live slot has a value");
            out.push((
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
        out
    }

    /// Every row of a `parse` answer, as its key/value pairs in order.
    fn read(answer: &Value) -> Vec<Vec<(String, String)>> {
        let rows = crate::arr::borrowed(answer.array_ptr().expect("`parse` answers an array"));
        let mut out = Vec::new();
        let mut slot = 0;
        while let Some(live) = rows.next_slot(slot) {
            slot = live + 1;
            out.push(cells(
                &rows.value_at(live).expect("a live slot has a value"),
            ));
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
    // covers: Core\Csv::parse
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

    // covers: Core\Csv::parse
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

    /// One record as a list of fields, keyed by position.
    fn list(fields: &[&str]) -> Value {
        let mut out = NvsArray::new();
        for field in fields {
            out.append(s(field));
        }
        Value::array(out)
    }

    /// A `string` answer as text.
    fn text(value: &Value) -> String {
        String::from_utf8(
            value
                .as_str_bytes()
                .expect("`format` answers a string")
                .to_vec(),
        )
        .expect("the document is text")
    }

    /// `Core\Csv::format` as a program reaches it — through the symbol the
    /// registry row names, with the rows in slot 0, the dialect in slots 1 and
    /// 2 and the column names in slot 3.
    ///
    /// The quoting rule itself is pinned above over [`write_field`], and the
    /// round trip is pinned from Novis. What the member can get wrong is the
    /// seam: a dialect byte read out of the other slot, a header written as a
    /// row rather than before them, a record rendered by its keys instead of
    /// its values, and a refusal that answers a document anyway. Neither
    /// dialect byte here is the default, so any of those renders a document
    /// that is visibly not this one.
    // covers: Core\Csv::format
    #[test]
    fn the_member_writes_the_header_first_and_takes_every_row_by_its_values() {
        // A record keyed by column names, as `parse({header: true})` answers
        // one: its values are written in slot order and its keys nowhere,
        // which is what makes a parsed document safe to write straight back.
        let mut keyed = NvsArray::new();
        keyed.set(NvsStr::new(b"qty"), s("2"));
        keyed.set(NvsStr::new(b"name"), s("a'b"));
        let mut rows = NvsArray::new();
        rows.append(Value::array(keyed));
        rows.append(list(&["10", "plain"]));

        let document = run(
            super::nvs_core_csv_format,
            &[Value::array(rows), s(";"), s("'"), list(&["qty", "name"])],
        )
        .expect("two distinct ASCII bytes are a dialect");
        assert_eq!(text(&document), "qty;name\n2;'a''b'\n10;plain\n");
        released(document);

        // No `header` option writes no header record, and no rows at all is
        // the empty document rather than a bare terminator.
        let empty = run(
            super::nvs_core_csv_format,
            &[
                Value::array(NvsArray::new()),
                s(","),
                s("\""),
                Value::null(),
            ],
        )
        .expect("an empty table writes");
        assert_eq!(text(&empty), "");
        released(empty);

        // The separator and the quote must name two bytes, which is the one
        // refusal a source program reaches this member's own guard for.
        assert!(
            run(
                super::nvs_core_csv_format,
                &[Value::array(NvsArray::new()), s(";"), s(";"), Value::null(),],
            )
            .is_err(),
            "one byte cannot be both the separator and the quote"
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

    /// A path under the host's temporary directory that one case owns, with
    /// anything a previous run left there removed.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("nvs-csv-rows");
        std::fs::create_dir_all(&dir).expect("a temporary directory the tests own");
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        path
    }

    /// A `Core\IO\File` over `path`, filed on `ctx` exactly as `Core\IO::open`
    /// files one, and the key it went in under.
    ///
    /// The door is that member's and not this one's, which is why a case here
    /// opens a descriptor without a capability: `rows` reads a handle that has
    /// already been through it.
    fn handle(ctx: &mut Ctx, path: &std::path::Path) -> (Value, u64) {
        let file = std::fs::File::open(path).expect("the case wrote this file");
        let key = ctx.hold_open_file(file);
        let opened = crate::instance::build(
            &crate::io::FILE,
            [Value::uint(key), s(&path.display().to_string())],
        );
        (opened, key)
    }

    /// The walk `rows` answers over an open handle, at the default dialect —
    /// taking over the handle's reference, as the member's caller does.
    fn walk(ctx: &mut Ctx, file: Value, header: bool) -> Value {
        let args = [file, s(","), s("\""), s(""), Value::bool(header)];
        let answer = call(super::nvs_core_csv_rows, ctx, &args)
            .expect("the default dialect over an open handle never throws");
        for arg in &args {
            #[expect(
                unsafe_code,
                reason = "this case owns the one reference it built for each \
                          argument, and the helper borrowed rather than \
                          consumed it"
            )]
            unsafe {
                arg.release();
            }
        }
        answer
    }

    /// One `advance()` of a walk.
    fn advanced(ctx: &mut Ctx, rows: Value) -> bool {
        super::rows_step(ctx, rows)
            .expect("a walk over a readable file answers every step")
            .as_bool()
            .expect("`advance` answers a `bool`")
    }

    /// The record a walk is standing on, with the reference `current()` hands
    /// its caller.
    fn standing(rows: Value) -> Value {
        super::rows_record(rows, nvs_runtime::sequence::CURRENT)
            .expect("a walk answers the record it is standing on")
    }

    /// Drops the one reference this frame owns, exactly as a member's caller
    /// would.
    fn released(value: Value) {
        #[expect(
            unsafe_code,
            reason = "the reference released here is the one the case owns, and \
                      every other one is accounted for where it was taken"
        )]
        unsafe {
            value.release();
        }
    }

    /// How many owners hold `record` — the case's own reference plus whatever
    /// the walk is still holding.
    fn owners(record: &Value) -> usize {
        let ptr = record.array_ptr().expect("every record is an array");
        #[expect(
            unsafe_code,
            reason = "the case keeps a reference of its own to every record it \
                      asks about, so each one is live for the whole test"
        )]
        unsafe {
            nvs_runtime::NvsArray::refcount_of(ptr)
        }
    }

    /// The walk is a read rather than a parse of something already read:
    /// naming it takes no octets at all, each `advance()` answers the next
    /// record, and what it answers is `parse`'s reading of the same document —
    /// quoted field holding a separator and a terminator included, which is
    /// what a reader fed a chunk at a time is liable to get wrong.
    // covers: Core\Csv::rows
    #[test]
    fn csv_rows_reads_a_file_one_record_at_a_time() {
        const DOCUMENT: &str = "name,note,qty\nfig,\"a, b\nc\",2\nplum,plain,7\n";

        let path = scratch("orders.csv");
        std::fs::write(&path, DOCUMENT).expect("the document this case walks");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let (file, _) = handle(&mut ctx, &path);
        let rows = walk(&mut ctx, file, true);
        let receiver = rows.obj_ptr().expect("`rows` answers an object");
        let key = crate::instance::slot(receiver, super::ROWS_READER_AT)
            .as_uint()
            .expect("the walk files its parse state under a key");

        // Nothing is read by `rows` itself: no chunk taken, no end of file
        // seen, and no header consumed.
        let parked = super::reader_at(&mut ctx, key).expect("the walk parks its state");
        assert!(
            parked.pending.is_empty() && !parked.eof && parked.names.is_empty(),
            "naming a walk reads no octets — the first read is the first `advance`"
        );

        let mut seen = Vec::new();
        while advanced(&mut ctx, rows) {
            let record = standing(rows);
            seen.push(cells(&record));
            released(record);
        }

        assert_eq!(
            seen,
            parse(DOCUMENT, true),
            "a walked document and a parsed one are one reading of the same bytes"
        );
        assert_eq!(seen.len(), 2, "the header is consumed and is not a record");
        assert_eq!(keys(&seen[0]), ["name", "note", "qty"]);
        assert_eq!(fields(&seen[0]), ["fig", "a, b\nc", "2"]);
        assert_eq!(fields(&seen[1]), ["plum", "plain", "7"]);

        assert_eq!(
            crate::instance::slot(receiver, super::ROWS_RECORD_AT).tag(),
            Some(Tag::Null),
            "the end of the walk clears the record rather than leaving the last one in it"
        );
        released(rows);
    }

    /// The member's promise, measured where [`super::rows_step`] keeps it: a
    /// walk over a document of any size holds one record and a bounded read
    /// buffer, never the document.
    ///
    /// **Asserted by weighing the parked reader after every step and by
    /// counting the records the walk still owns**, rather than by reading the
    /// slot: a member that read the file in and parsed it would answer every
    /// record correctly and fail the first weighing, and one that cleared
    /// nothing at the end would pass both and fail the line after them. A
    /// document many reads long, because the number the answer must not depend
    /// on is the document's size.
    // covers: Core\Csv::rows
    #[test]
    fn csv_rows_holds_one_record_and_never_the_document() {
        const RECORDS: usize = 4_000;

        let mut document = String::from("id,amount\n");
        for n in 0..RECORDS {
            document.push_str(&format!("{n},{}\n", n * 3));
        }
        assert!(
            document.len() > super::READ_CHUNK * 4,
            "the document has to be several reads long for the bound below to mean anything"
        );
        let path = scratch("ledger.csv");
        std::fs::write(&path, &document).expect("the document this case walks");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let (file, _) = handle(&mut ctx, &path);
        let rows = walk(&mut ctx, file, true);
        let receiver = rows.obj_ptr().expect("`rows` answers an object");
        let key = crate::instance::slot(receiver, super::ROWS_READER_AT)
            .as_uint()
            .expect("the walk files its parse state under a key");

        // One reference of the case's own per record, so that a record the
        // walk has let go of is still live enough to be counted.
        let mut held: Vec<Value> = Vec::new();
        let mut heaviest = 0;
        while advanced(&mut ctx, rows) {
            let record = standing(rows);
            assert_eq!(
                owners(&record),
                2,
                "the slot and the caller of `current` are a record's two owners"
            );
            if let Some(previous) = held.last() {
                assert_eq!(
                    owners(previous),
                    1,
                    "parking a record releases the one before it"
                );
            }
            held.push(record);

            let parked = super::reader_at(&mut ctx, key).expect("the walk parks its state");
            heaviest = heaviest.max(
                parked.pending.len()
                    + parked.cell.len()
                    + parked.field.len()
                    + parked.record.iter().map(Vec::len).sum::<usize>()
                    + parked.names.iter().map(Vec::len).sum::<usize>(),
            );
        }

        assert_eq!(
            held.len(),
            RECORDS,
            "every record of the document is walked"
        );
        assert!(
            heaviest <= super::READ_CHUNK + super::CELL + 128,
            "a walk holds one chunk, one field's buffer and one record — it held {heaviest} \
             bytes over a document of {}",
            document.len()
        );
        assert!(
            held.iter().all(|record| owners(record) == 1),
            "the end of the walk holds no record at all"
        );

        let parked = super::reader_at(&mut ctx, key).expect("the entry outlives the walk");
        assert!(
            parked.done && parked.pending.is_empty() && parked.cell.is_empty(),
            "the end of the walk gives up its buffers rather than holding them for the request"
        );
        assert!(
            !advanced(&mut ctx, rows),
            "a drained walk answers no records rather than faulting"
        );

        for record in held {
            released(record);
        }
        released(rows);
    }
}
