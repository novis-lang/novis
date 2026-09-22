//! `Core\IO` — the first `Core` class that reaches the operating system, and
//! so the first one written entirely behind
//! `rule:security/capability-check-at-the-door` and `rule:security/capability-declaration-is-one-table`
//! 's doors.
//!
//! **There is no capability check in this file, and that is the design.** Every
//! body below calls a door in `nvs_runtime::capability`, which
//! asks before it acts; a member here that forgot to ask would have to name
//! `std::fs` to perform the effect at all, and
//! `nvs_stdlib_reaches_the_os_only_through_the_gate` reads this crate's sources
//! and fails on one. So the thing a reviewer would otherwise have to check —
//! did the author remember — is not a property of this module's care. What each
//! member needs is declared once in [`crate::registry::CAPABILITIES`], which
//! nothing consults at run time.
//!
//! **A path is a `string` and not a `Core\Path`.** § 8's class is a pure
//! lexical splitter — it never touches a filesystem — so making it this
//! signature's type would force every caller to build one for a value they
//! already have, and would suggest a validation that class does not perform.
//! What decides whether a path is acceptable is the grant, checked at the door
//! against the canonical spelling.
//!
//! **Every path parameter in this class is a `Qual::Sink`, and `within` is the
//! one launderer that gets past them.** `rule:security/sink-predicate`'s table classifies a
//! filesystem path component as an instruction — `..` and the separators
//! *direct the resolver* — so spec § 14's opening sentence, which calls every
//! member of this class a path sink, is that predicate applied rather than a
//! second rule. The whole class carries the mark together: half of it marked
//! would be a class where the refusal a developer met at `read` did not
//! arrive at `remove`, which teaches the wrong rule more effectively than no
//! rule at all. `Core\Path` is deliberately *not* marked the same way — its
//! own module doc says why, and the difference is that nothing in § 8 resolves
//! anything.
//!
//! **`read` answers with `string` rather than `bytes`.** Novis text is a byte
//! string, so the two would carry the same content, and every caller of a
//! whole-file read wants the thing it can concatenate. The other half — a
//! bounded read, a stream — is [`FILE`], reached through `open`, which is why
//! `open_read` hands back a handle rather than a `Vec`.
//!
//! **An open file is an object, and this class has two of them.** Spec § 14's
//! R14 is that a `resource` is never exposed, so `open` answers [`FILE`] and
//! the mode it takes is [`FILE_MODE`] rather than `fopen`'s string (R11). Both
//! of those constants own their own reasoning; what belongs here is why the
//! handle members sit in this file at all rather than in one of their own,
//! which is that they are the same door: `Core\IO::read` and
//! `$file->read($n)` differ in how much they take and in nothing else, and a
//! second module would be a second place to look for that answer.
//!
//! **`readText` is that same read with § 7's exact decode after it**, and not a
//! second reader: one door, one buffer, one ceiling, and the conversion is
//! `Core\Encoding`'s own, refusal included. `read` hands back the octets and
//! asks nothing about them; `readText` is where a caller says what they are and
//! gets a throw naming the first offset that is not, rather than a string with
//! U+FFFD in it. PHP's pair is `file_get_contents` plus a hand-written
//! `mb_convert_encoding`, and the failure is the half it leaves out.
//!
//! **`lines` is that same read split by `Core\Str::lines`' own rule**, and
//! answers [`LINES`] — the class whose docs own the one decision this member
//! rests on, that the lines are held and not streamed off the open file. Three
//! members, one door, one buffer: what differs between them is what happens to
//! the octets afterwards, which is the shape this class keeps rather than
//! growing a reader per question.

use std::collections::VecDeque;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use nvs_runtime::capability::Access;
use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc, ErrorDoc,
    MethodDoc, ParamDoc, Qual,
};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\IO";

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[
        CoreMethod {
            name: "read",
            names: &["path"],
            // A sink in the path, like every other path in this class: `rule:security/sink-predicate`
            // classifies a path component as an instruction, because `..`
            // and the separators direct the resolver. The module doc above owns
            // why the whole class carries the mark together, and `within` is
            // the row that makes it usable.
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Text(Qual::Neutral),
            symbol: "nvs_core_io_read",
            doc: Some(&READ_DOC),
        },
        CoreMethod {
            name: "write",
            names: &["path", "content"],
            // The path is a sink and `$content` is not: `rule:security/sink-predicate`'s table
            // puts a file's *contents* on the data side, so bytes that arrived
            // from outside may be written to a path this program chose.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_write",
            doc: Some(&WRITE_DOC),
        },
        CoreMethod {
            name: "append",
            names: &["path", "content"],
            // `write`'s pair exactly, and for its reason: the path directs a
            // resolver and the content does not, so `rule:security/sink-predicate`'s table marks
            // the first and leaves the second alone.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_append",
            doc: Some(&APPEND_DOC),
        },
        CoreMethod {
            name: "writeStream",
            names: &["path", "src"],
            // The path is a sink and the chunks are not, exactly as `write`'s
            // pair is and for the same reason: what arrives from outside here
            // is the *content*, which is what this member exists to accept.
            // `Iterated` is spec § 14's `Iterable<bytes>` — `rule:iteration/foreach-subjects`'s
            // three shapes, so an `array<bytes>` a program already holds is
            // one of them and no caller has to build a generator to write.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Iterated(&CoreTy::Blob(Qual::Neutral)),
                CoreTy::Options(WRITE_STREAM_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_write_stream",
            doc: Some(&WRITE_STREAM_DOC),
        },
        CoreMethod {
            name: "exists",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_io_exists",
            doc: Some(&EXISTS_DOC),
        },
        CoreMethod {
            name: "isFile",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_io_is_file",
            doc: Some(&IS_FILE_DOC),
        },
        CoreMethod {
            name: "isDir",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_io_is_dir",
            doc: Some(&IS_DIR_DOC),
        },
        CoreMethod {
            name: "isReadable",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_io_is_readable",
            doc: Some(&IS_READABLE_DOC),
        },
        CoreMethod {
            name: "isWritable",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_io_is_writable",
            doc: Some(&IS_WRITABLE_DOC),
        },
        CoreMethod {
            name: "size",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            // `uint` rather than `int`, like `Core\Arr::count`: a byte count has
            // no negative half, and the spec's answer for "how big is it" is the
            // same shape wherever it is asked.
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_io_size",
            doc: Some(&SIZE_DOC),
        },
        CoreMethod {
            name: "modifiedAt",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            // `Core\Time\Instant` and never an epoch `int`: R12's "units are
            // types", and the one place PHP's `filemtime` leaks a number a
            // caller then has to remember the unit of.
            return_ty: CoreTy::Instance(crate::time::INSTANT_NAME),
            symbol: "nvs_core_io_modified_at",
            doc: Some(&MODIFIED_AT_DOC),
        },
        CoreMethod {
            name: "stat",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(METADATA_NAME),
            symbol: "nvs_core_io_stat",
            doc: Some(&STAT_DOC),
        },
        CoreMethod {
            name: "copy",
            names: &["from", "to"],
            // Both paths are sinks: this class marks every path parameter it
            // has, and the module doc above owns why it is the whole class
            // together rather than the ones that happen to resolve.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_copy",
            doc: Some(&COPY_DOC),
        },
        CoreMethod {
            name: "move",
            names: &["from", "to"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_move",
            doc: Some(&MOVE_DOC),
        },
        CoreMethod {
            name: "remove",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_remove",
            doc: Some(&REMOVE_DOC),
        },
        CoreMethod {
            name: "makeDir",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_make_dir",
            doc: Some(&MAKE_DIR_DOC),
        },
        CoreMethod {
            name: "removeDir",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_remove_dir",
            doc: Some(&REMOVE_DIR_DOC),
        },
        CoreMethod {
            name: "list",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            // `array<string>` and not `array<tainted string>`, which is spec
            // § 14's own spelling and agrees with `stdin`'s row comment below
            // that standard input is this class's one tainted answer. An entry
            // name is what the operating system reports for a directory this
            // program named and was granted; it did not cross a request
            // boundary, and the question a caller has about composing one back
            // into a path is containment, which is `within`'s.
            return_ty: CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
            symbol: "nvs_core_io_list",
            doc: Some(&LIST_DOC),
        },
        CoreMethod {
            name: "walk",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            // `Iterable<string>` spelled the way `lines` above spells one — a
            // named class, because `CoreTy::Iterated` is parameter position
            // only. [`WALK`]'s own docs are the home of why this is a second
            // *question* rather than a second spelling of `list`'s: that row
            // reads one directory and this one reads the tree under it, which
            // is the only reading of the two that `rule:core-api/shape-rules` R6 admits.
            return_ty: CoreTy::Instance(WALK_NAME),
            symbol: "nvs_core_io_walk",
            doc: Some(&WALK_DOC),
        },
        CoreMethod {
            name: "temporaryDir",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Text(Qual::Neutral),
            symbol: "nvs_core_io_temporary_dir",
            doc: Some(&TEMPORARY_DIR_DOC),
        },
        CoreMethod {
            name: "canonicalize",
            names: &["path"],
            // A sink like every other path here, and its answer is
            // `CoreTy::Text(Qual::Neutral)` rather than `CoreTy::Str`: this
            // member resolves and does not *prove*, so it launders nothing.
            // `within` below is the row that removes a qualifier, and the two
            // sitting next to each other is the whole reason this comment is
            // here — reaching for the resolver when the question was
            // containment is the mistake `rule:security/launderers-are-sink-named` exists to prevent.
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Text(Qual::Neutral),
            symbol: "nvs_core_io_canonicalize",
            doc: Some(&CANONICALIZE_DOC),
        },
        CoreMethod {
            name: "within",
            names: &["base", "path"],
            // The class's one laundering row, and the two marks are different
            // on purpose. `$base` is a path this program chose, so it is a
            // sink like every other; `$path` is the untrusted half the member
            // exists to accept, and `rule:security/launderers-are-sink-named` makes a launderer's answer the
            // plain, unqualified type.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_io_within",
            doc: Some(&WITHIN_DOC),
        },
        CoreMethod {
            name: "readText",
            names: &["path"],
            // The path is a sink like every other in this class, and the
            // charset is the one trailing options shape `rule:core-api/shape-rules` R3 allows —
            // which is why `names` carries one entry and `params` two.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Options(READ_TEXT_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Text(Qual::Neutral),
            symbol: "nvs_core_io_read_text",
            doc: Some(&READ_TEXT_DOC),
        },
        CoreMethod {
            name: "lines",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            // Spec § 14 writes `Iterable<string>`, and a *named class* is how
            // the registry spells one: `CoreTy::Iterated` is parameter
            // position only, by its own docs. [`LINES`] is that name, and
            // `crate::registry::ITERABLES` is where its element type is
            // declared.
            return_ty: CoreTy::Instance(LINES_NAME),
            symbol: "nvs_core_io_lines",
            doc: Some(&LINES_DOC),
        },
        CoreMethod {
            name: "open",
            names: &["path", "mode"],
            // The path is a sink like every other in this class. The mode is
            // [`FILE_MODE`] and never a string, which is R11 and the whole of
            // what replaces `fopen`'s `"r+b"` grammar.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Enum(FILE_MODE_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(FILE_NAME),
            symbol: "nvs_core_io_open",
            doc: Some(&OPEN_DOC),
        },
        CoreMethod {
            name: "stdin",
            names: &[],
            params: &[],
            // The one member of this class whose answer is `tainted`, and the
            // only one whose bytes the program did not name a source for: they
            // are whatever the invoker piped in. [`nvs_core_io_stdin`] is the
            // home of that reading and of why the writing half is not here.
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_io_stdin",
            doc: Some(&STDIN_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\IO::readText`'s `{charset?}` — the one option, and the whole of what
/// separates that member from [`nvs_core_io_read`].
///
/// UTF-8 by default because that is what Novis text already is
/// (`rule:types/bytes`): a caller who
/// says nothing gets the decode that is the identity on a file written the way
/// the language spells strings, and every other encoding has to name itself. A
/// default of "whatever the file looks like" is the guess this member exists to
/// refuse.
const READ_TEXT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "charset",
    ty: CoreTy::Enum(crate::encoding::CHARSET_NAME),
    default: Const::EnumCase(crate::encoding::CHARSET_NAME, "Utf8"),
}];

/// `Core\IO::writeStream`'s `{max?, overwrite?}` — `rule:core-classes/io-write-stream`'s two rules,
/// and the whole of what this member decides that [`nvs_core_io_write`] does
/// not.
///
/// **`overwrite` defaults to `false`** because the destination is normally a
/// name a client claimed, which is the case the member was written for: a
/// default that replaced would make the safe spelling the one a caller has to
/// remember. `Core\IO::write` replaces without asking and keeps that default,
/// because a whole-buffer write is a path the program itself chose.
///
/// **`max` defaults to no ceiling at all**, unlike every other bound in this
/// class, because what is behind it bounds the other direction:
/// [`nvs_core_io_read`]'s doc holds a *read* to `[limits] max_output`, and this
/// member writes a stream a client is sending rather than filling a buffer the
/// request will hold. So a caller who means a limit is the only one who can say
/// what it is, and one who says nothing has said unbounded rather than
/// inherited a number.
///
/// **`Core\Request\Part::saveTo` is this same bag**, by naming this constant
/// rather than declaring a second one beside it: `rule:core-classes/io-write-stream` makes that member
/// a delegation to this one, and two spellings of one default are two things
/// that can disagree.
pub(crate) const WRITE_STREAM_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "max",
        ty: CoreTy::Uint,
        default: Const::Uint(u64::MAX),
    },
    CoreOption {
        name: "overwrite",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `Core\IO::read`'s reference card — `rule:core-api/reference-card`.
const READ_DOC: MethodDoc = MethodDoc {
    short: "The whole content of a file, as text — `file_get_contents`. Needs the `fs.read` \
            capability for the path, which is checked against its canonical spelling, so a \
            symlink or a `..` that leaves the granted roots is refused.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file to read, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "The file's bytes as a `string`, with nothing stripped and no encoding assumed.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path; the message names \
                   the capability in the spelling `nvs.toml` grants it under. Or the file is \
                   larger than `[limits] max_output`, the one ceiling a request holds a single \
                   read to — the same directive that bounds a captured child's output.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — the file does \
                   not exist, is a directory, or could not be read.",
        },
    ],
};

/// `Core\IO::write`'s reference card — `rule:core-api/reference-card`.
const WRITE_DOC: MethodDoc = MethodDoc {
    short: "Replaces a file's whole content, creating it if it does not exist — \
            `file_put_contents`. Needs the `fs.write` capability for the path; being allowed to \
            read a root is not permission to write in it.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The file to write, absolute or relative to the working directory.",
            shape: &[],
        },
        ParamDoc {
            name: "content",
            desc: "The bytes to write. They become the file's entire content; `append` is the \
                   member that adds to what is already there.",
            shape: &[],
        },
    ],
    ret: "Nothing. A refusal throws rather than answering `false`, so a caller that ignores the \
          result has not ignored a failure.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for this path; checked before \
                   anything is created, so a refused write leaves no file behind.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — a missing \
                   directory, a read-only filesystem, a permission the process lacks.",
        },
    ],
};

/// `Core\IO::append`'s reference card — `rule:core-api/reference-card`.
const APPEND_DOC: MethodDoc = MethodDoc {
    short: "Adds to the end of a file, creating it if it is not there — `file_put_contents` with \
            `FILE_APPEND`, which is a member here rather than a flag on the member that \
            replaces. Needs the `fs.write` capability for the path.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The file to add to, absolute or relative to the working directory.",
            shape: &[],
        },
        ParamDoc {
            name: "content",
            desc: "The bytes to add. Whatever the file already holds is kept and these follow \
                   it; the end is found by the operating system at the write, not read \
                   beforehand.",
            shape: &[],
        },
    ],
    ret: "Nothing. A refusal throws rather than answering `false`, so a caller that ignores the \
          result has not ignored a failure.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for this path; checked before \
                   anything is created, so a refused append leaves no file behind.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — a missing \
                   directory, a read-only filesystem, a path that is a directory.",
        },
    ],
};

/// `Core\IO::writeStream`'s reference card — `rule:core-api/reference-card`.
const WRITE_STREAM_DOC: MethodDoc = MethodDoc {
    short: "Writes a sequence of `bytes` chunks to `$path` as they arrive, holding no more than one \
            of them — an upload part, a response body, a decompressed archive. Needs the `fs.write` \
            capability for the path.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The file to write. It must not already exist unless `overwrite` says otherwise.",
            shape: &[],
        },
        ParamDoc {
            name: "src",
            desc: "The chunks, in order — an `array<bytes>`, an `Iterable<bytes>` or an \
                   `Iterator<bytes>`. Each is written and then dropped, so the source's length is \
                   not what this holds.",
            shape: &[],
        },
        ParamDoc {
            name: "max",
            desc: "The most bytes to accept across the whole stream. Unbounded when it is not \
                   given: nothing else bounds a file on disk, so this is the only ceiling there is.",
            shape: &[],
        },
        ParamDoc {
            name: "overwrite",
            desc: "Whether an existing file may be replaced. `false` by default, because the \
                   destination is usually a name a client claimed.",
            shape: &[],
        },
    ],
    ret: "Nothing. A failure part-way through removes the partial file before it throws, so a later \
          reader never finds a truncated write the program believes it completed.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for this path, or the stream ran \
                   past `max` bytes.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "Something is already at the path and `overwrite` is `false`, or the operating \
                   system refused the create or one of the writes.",
        },
    ],
};

/// `Core\IO::exists`'s reference card — `rule:core-api/reference-card`.
const EXISTS_DOC: MethodDoc = MethodDoc {
    short: "Reports whether anything is at `$path` — `file_exists`, and true for a directory as \
            well as a file. Needs the `fs.read` capability, which is asked before the path is \
            touched, so an ungranted path throws rather than answering `false`.",
    params: &[ParamDoc {
        name: "path",
        desc: "The name to look for, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "`true` if the name resolves to something, `false` if it resolves to nothing. Absence is \
          an answer here and not a failure, which is what separates this from `size`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path. A refusal and a \
                   `false` are deliberately distinguishable: a program that was never granted the \
                   root cannot use this member to learn what is in it.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system could answer neither yes nor no — a parent directory it \
                   will not traverse, which is not the same as the name being absent.",
        },
    ],
};

/// `Core\IO::isFile`'s reference card — `rule:core-api/reference-card`.
const IS_FILE_DOC: MethodDoc = MethodDoc {
    short: "Reports whether `$path` names a regular file — `is_file`. Symbolic links are followed, \
            so a link to a file answers `true`. Needs the `fs.read` capability, which is asked \
            before the path is touched.",
    params: &[ParamDoc {
        name: "path",
        desc: "The name to ask about, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "`true` for a regular file, `false` for a directory, for anything else the operating \
          system holds at that name, and for a name that is not there at all. Absence answers \
          `false` here rather than throwing, because the question is what kind of thing is at the \
          name and *nothing* is a complete answer to it.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path. A refusal and a \
                   `false` stay distinguishable, exactly as they do for `exists`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system could answer neither yes nor no — a parent directory it \
                   will not traverse, which is not the same as the name naming no file.",
        },
    ],
};

/// `Core\IO::isDir`'s reference card — `rule:core-api/reference-card`.
const IS_DIR_DOC: MethodDoc = MethodDoc {
    short: "Reports whether `$path` names a directory — `is_dir`. Symbolic links are followed, so \
            a link to a directory answers `true`. Needs the `fs.read` capability.",
    params: &[ParamDoc {
        name: "path",
        desc: "The name to ask about, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "`true` for a directory, `false` for a file, for anything else, and for a name that is \
          not there. With `isFile` it partitions what `exists` answers `true` for into the two \
          kinds this class has separate members for, and a name can satisfy neither.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system could answer neither yes nor no — a parent directory it \
                   will not traverse.",
        },
    ],
};

/// `Core\IO::isReadable`'s reference card — `rule:core-api/reference-card`.
const IS_READABLE_DOC: MethodDoc = MethodDoc {
    short: "Whether this process could read what is at `$path` right now — `is_readable`. Needs \
            the `fs.read` capability, which is a separate and earlier gate: a path outside the \
            grant is refused rather than reported as unreadable.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file or directory to ask about, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "`true` if the operating system would allow a read, `false` if it would not — including \
          for a name that is not there. The answer is about the instant it was asked and nothing \
          holds it still, so a read that follows it can still fail.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The configuration does not grant `fs.read` for this path.",
    }],
};

/// `Core\IO::isWritable`'s reference card — `rule:core-api/reference-card`.
const IS_WRITABLE_DOC: MethodDoc = MethodDoc {
    short: "Whether this process could write what is at `$path` right now — `is_writable`. Needs \
            the `fs.write` capability, because the whole question is about writing: a program \
            granted only reads cannot ask where it could write.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file or directory to ask about, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "`true` if the operating system would allow a write, `false` if it would not — including \
          for a name that is not there. A snapshot, exactly as `isReadable` is.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The configuration does not grant `fs.write` for this path.",
    }],
};

/// `Core\IO::size`'s reference card — `rule:core-api/reference-card`.
const SIZE_DOC: MethodDoc = MethodDoc {
    short: "The size of the file at `$path` in bytes, as the operating system reports it — \
            `filesize`. Needs the `fs.read` capability: measuring a file is reading it.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file to measure, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "The byte count as a `uint`. For a text file this is bytes and not characters — a \
          `string`'s own length is `Core\\Str::length`, which counts what § 1 says it counts.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — there is nothing \
                   at the path, or its metadata could not be read. A missing file has no size, so \
                   it throws here where `exists` answers `false`.",
        },
    ],
};

/// `Core\IO::modifiedAt`'s reference card — `rule:core-api/reference-card`.
const MODIFIED_AT_DOC: MethodDoc = MethodDoc {
    short: "When the file at `$path` was last written, as a `Core\\Time\\Instant` — `filemtime`, \
            with the unit in the type instead of in the caller's memory. Needs the `fs.read` \
            capability: asking when a file changed is reading it.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file or directory to ask about, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "The modification time as an absolute point on the timeline, with no zone of its own — \
          `->in($zone)` is what gives it a calendar.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "There is nothing at the path, its metadata could not be read, the platform \
                   does not record a modification time, or the one it recorded falls outside \
                   the range a `Core\\Time\\Instant` can name.",
        },
    ],
};

/// `Core\IO::stat`'s reference card — `rule:core-api/reference-card`.
const STAT_DOC: MethodDoc = MethodDoc {
    short: "Everything one `stat` answers about `$path`, as a `Core\\IO\\Metadata` — `stat`, \
            `lstat` and `filemtime` in one call, so a program asking more than one question \
            about a file pays for one syscall rather than one per question. Needs the \
            `fs.read` capability.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file or directory to measure, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "A `Core\\IO\\Metadata` — a snapshot, not a live view: it answers about the moment the \
          call was made, and says nothing about the file afterwards.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "There is nothing at the path, or its metadata could not be read. A missing \
                   file has no metadata, so it throws here where `exists` answers `false`.",
        },
    ],
};

/// `Core\IO::copy`'s reference card — `rule:core-api/reference-card`.
const COPY_DOC: MethodDoc = MethodDoc {
    short: "Duplicates a file — `copy`. Needs `fs.read` for the source and `fs.write` for the \
            destination, which are two grants and not one: reading a directory is never permission \
            to fill it.",
    params: &[
        ParamDoc {
            name: "from",
            desc: "The file to read. It is left exactly as it was.",
            shape: &[],
        },
        ParamDoc {
            name: "to",
            desc: "The file to create. Anything already at this path is replaced, as `write` \
                   replaces — this destination is one the program named beside a source it already \
                   holds.",
            shape: &[],
        },
    ],
    ret: "Nothing. A refusal throws rather than answering `false`, so a caller that ignores the \
          result has not ignored a failure.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for the source or `fs.write` for the \
                   destination; both are checked before either is used, so a refusal copies \
                   nothing.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — nothing at the \
                   source, a destination directory that is not there, or a permission the process \
                   lacks. The message names both ends.",
        },
    ],
};

/// `Core\IO::move`'s reference card — `rule:core-api/reference-card`.
const MOVE_DOC: MethodDoc = MethodDoc {
    short: "Renames a file, which is how it is moved — `rename`. Needs `fs.write` for **both** \
            paths, and not `copy`'s read for the source: a move takes the source away, and taking a \
            file away is destroying it.",
    params: &[
        ParamDoc {
            name: "from",
            desc: "The path that stops existing.",
            shape: &[],
        },
        ParamDoc {
            name: "to",
            desc: "The path that ends up holding the file. Anything already there is replaced.",
            shape: &[],
        },
    ],
    ret: "Nothing. The rename is the operating system's own, so it is atomic: the destination is \
          the whole file or the file it was before. A move between filesystems fails rather than \
          becoming a copy and a delete, which would be neither atomic nor the two capability checks \
          the program would have chosen — `copy` then `remove` is that spelling.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for one of the two paths; both are \
                   checked before either is used.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — nothing at the \
                   source, a destination directory that is not there, or the two paths on \
                   different filesystems. The message names both ends.",
        },
    ],
};

/// `Core\IO::makeDir`'s reference card — `rule:core-api/reference-card`.
const MAKE_DIR_DOC: MethodDoc = MethodDoc {
    short: "Makes sure a directory exists at `$path`, creating any missing parent along the way — \
            `mkdir` with `$recursive` true, which is a parameter there and the only behaviour \
            here. Needs the `fs.write` capability.",
    params: &[ParamDoc {
        name: "path",
        desc: "The directory that is to exist. Every missing component above it is created too; \
               all of them are under the path the capability was asked about.",
        shape: &[],
    }],
    ret: "Nothing. A directory that is already there is success rather than a refusal — the \
          contract is that it exists afterwards, and refusing would leave every caller writing an \
          `exists` check in front of this one. `removeDir` is deliberately not the mirror of this: \
          it refuses to recurse, because what it would recurse over is destruction.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for this path; checked before \
                   anything is created, so a refused call leaves no directory behind.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — a component of the \
                   path exists and is a file, or the process may not create there.",
        },
    ],
};

/// `Core\IO::remove`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Deletes the file at `$path` — `unlink`. Needs the `fs.write` capability: removal is a \
            write, because an account that may replace a file's whole content can already destroy \
            it.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file to delete. A directory is `removeDir`'s argument, not this one.",
        shape: &[],
    }],
    ret: "Nothing. Removing a name that is not there throws rather than answering quietly, so a \
          program that deleted nothing has not been told it succeeded.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for this path; being allowed to \
                   read a root is not permission to empty it.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — nothing is at the \
                   path, it is a directory, or the process may not unlink it.",
        },
    ],
};

/// `Core\IO::removeDir`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DIR_DOC: MethodDoc = MethodDoc {
    short: "Deletes the **empty** directory at `$path` — `rmdir`. Needs the `fs.write` capability. \
            There is no recursive form: a tree deleted by one grant check is one wrong argument \
            away from deleting everything under it, so a program that means to empty a directory \
            walks it and removes each entry the capability was asked about.",
    params: &[ParamDoc {
        name: "path",
        desc: "The directory to delete, which must hold no entries.",
        shape: &[],
    }],
    ret: "Nothing. A refusal throws rather than answering `false`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — nothing is at the \
                   path, it is not a directory, or it still has entries in it.",
        },
    ],
};

/// `Core\IO::list`'s reference card — `rule:core-api/reference-card`.
const LIST_DOC: MethodDoc = MethodDoc {
    short: "The entries of the directory at `$path`, as an `array<string>` of bare names — \
            replacing `scandir`, `glob` and the whole `opendir`/`readdir`/`closedir` sequence. \
            `.` and `..` are not entries: they are the two names every `scandir` caller filters \
            out, so they are never handed over. Needs the `fs.read` capability. The order is the \
            operating system's own and nothing here sorts it — `Core\\Arr::sort` is one call and \
            a member that sorted by default would charge every caller for a guarantee most do not \
            need.",
    params: &[ParamDoc {
        name: "path",
        desc: "The directory to read. A file throws rather than answering a one-element array.",
        shape: &[],
    }],
    ret: "One `string` per entry, each a name and not a path: joining it back onto `$path` is the \
          caller's own step, and `within` is what makes that join safe when the name reached this \
          program from outside. The whole directory is held at once, which is what makes this a \
          member for a directory a program expects to fit in memory; `walk` is the member for the \
          tree underneath it.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — nothing is at the \
                   path, it is not a directory, or an entry could not be read partway through the \
                   walk.",
        },
    ],
};

/// `Core\IO::walk`'s reference card — `rule:core-api/reference-card`.
const WALK_DOC: MethodDoc = MethodDoc {
    short: "Every entry of the tree under `$path`, as an `Iterable<string>` of paths relative to it \
            — replacing `RecursiveDirectoryIterator`, `RecursiveIteratorIterator` and a recursive \
            `glob`. `list` is the one-directory member and this is the whole-tree one; needs the \
            `fs.read` capability, which is asked for **every** directory the walk enters and not \
            only for the root. A symbolic link is an entry and is never descended into, so the walk \
            is finite whatever the links say.",
    params: &[ParamDoc {
        name: "path",
        desc: "The directory the walk starts at. A file throws, exactly as `list` does.",
        shape: &[],
    }],
    ret: "One `string` per entry found anywhere beneath `$path`, each a path *relative to* `$path` \
          and spelled with `Core\\Path::SEPARATOR` — joining it back on is the caller's own step, \
          and `within` is what makes that join safe. Every entry of a directory is answered before \
          any entry beneath it, and within one directory the order is the operating system's own.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for a directory the walk asked to \
                   read; the message names the one it stopped at, which is the root unless a grant \
                   covers less than a whole subtree.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — nothing is at the \
                   path, it is not a directory, or a directory the walk had entered could not be \
                   read to the end.",
        },
    ],
};

/// `Core\IO::temporaryDir`'s reference card — `rule:core-api/reference-card`.
const TEMPORARY_DIR_DOC: MethodDoc = MethodDoc {
    short: "Creates a new, empty, private directory under the root Novis owns — `[io] temp_root`, \
            or a `novis` subdirectory of the platform temporary directory — and answers its path. \
            `sys_get_temp_dir` and `tempnam` in one member, and the directory is made rather than \
            merely named, so there is no window between choosing a name and owning it. Needs the \
            `fs.write` capability **for the path it creates**: the name is chosen first and asked \
            about second, so a configuration granting only the working directory does not reach \
            the temporary root.",
    params: &[],
    ret: "The absolute path of a directory that exists, holds nothing, and belongs to this script. \
          The runtime deletes it, and everything in it, when the script ends — after the last user \
          code and whatever the ending was — so a program never has to remember and can never leak \
          one. Removing it early is allowed and is not an error; a file that must outlive its \
          script is storage, not a temporary.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for the temporary root; the message \
                   names the path a grant would have to cover.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and no directory could be created — the root is full, \
                   read-only, or absent.",
        },
    ],
};

/// `Core\IO::canonicalize`'s reference card — `rule:core-api/reference-card`.
const CANONICALIZE_DOC: MethodDoc = MethodDoc {
    short: "The absolute path `$path` resolves to, with every `.`, `..` and symbolic link followed \
            by the operating system — `realpath`. Needs the `fs.read` capability: resolving a name \
            reads the directories on the way to it. **This is not the traversal check.** It \
            resolves and stops there; `within` is the member that resolves and then proves \
            containment, and it is the one an untrusted path has to pass through.",
    params: &[ParamDoc {
        name: "path",
        desc: "The name to resolve, absolute or relative to the working directory. Every \
               component must exist, including the last one.",
        shape: &[],
    }],
    ret: "The resolved absolute path. Passing the answer back in resolves to itself, so the result \
          is a fixed point and a program may compare two of them for equality — which is the one \
          use this member has that `Core\\Path::normalize` cannot serve, since two different \
          spellings of one file normalize differently and canonicalize the same.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — a component of \
                   the path is not there, or is not a directory, or a symbolic link loops. A name \
                   that does not exist has no resolution, so it throws here where `exists` \
                   answers `false`.",
        },
    ],
};

/// `Core\IO::within`'s reference card — `rule:core-api/reference-card`.
const WITHIN_DOC: MethodDoc = MethodDoc {
    short: "Resolves `$path` against `$base` and then **proves** the answer is still under it — the \
            path-traversal launderer, so its result is accepted where a `tainted` string is not. \
            `Core\\Path::normalize` cannot make this check: collapsing `..` textually says nothing \
            about where a name ended up once a symlink is on the way. Needs the `fs.read` \
            capability for both paths, because resolving one reads the directories above it.",
    params: &[
        ParamDoc {
            name: "base",
            desc: "The directory the answer must stay under. It has to exist, since containment is \
                   proved against its canonical spelling.",
            shape: &[],
        },
        ParamDoc {
            name: "path",
            desc: "The name to resolve against `$base` — the untrusted half, which is the whole \
                   point of the member. An absolute path is no escape hatch: it is resolved and \
                   then fails the same containment check.",
            shape: &[],
        },
    ],
    ret: "The resolved absolute path, as a plain `string`. Every `..`, every symlink and every \
          separator is already gone, so what the caller holds is a name the operating system \
          agrees with rather than one it still has to be trusted about.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The resolved path is not inside `$base`. The message names the base and the \
                   argument and never where the argument led, so a refusal discloses nothing about \
                   a symlink's target. A configuration that does not grant `fs.read` for either \
                   path refuses earlier, in the same class.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and nothing could be resolved — `$base` is not there, \
                   or no ancestor of the joined path is.",
        },
    ],
};

/// `Core\IO::readText`'s reference card — `rule:core-api/reference-card`.
const READ_TEXT_DOC: MethodDoc = MethodDoc {
    short: "The whole content of a file, decoded from the charset it is written in — \
            `file_get_contents` and the `mb_convert_encoding` a caller writes after it, with the \
            failure that pair does not have. Needs the `fs.read` capability for the path, exactly \
            as `read` does.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The file to read, absolute or relative to the working directory.",
            shape: &[],
        },
        ParamDoc {
            name: "charset",
            desc: "The encoding the file's octets are in. `Core\\Charset::Utf8` when it is not \
                   given, which is the decode that is the identity on text already written the \
                   way Novis spells it.",
            shape: &[],
        },
    ],
    ret: "The file's content as a `string`, converted from `$charset` — never with a replacement \
          character in it, because a conversion that cannot be exact throws instead.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path, or the file's bytes \
                   are not `$charset` — the second message names the offset of the first sequence \
                   that is not.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — the file does not \
                   exist, is a directory, or could not be read.",
        },
    ],
};

/// `Core\IO::lines`'s reference card — `rule:core-api/reference-card`.
const LINES_DOC: MethodDoc = MethodDoc {
    short: "Every line of a file, without its terminator — `file()` and the `fgets` loop that \
            replaces it, over the one definition of a line `Core\\Str::lines` already uses. Needs \
            the `fs.read` capability for the path, exactly as `read` does.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file to read, absolute or relative to the working directory.",
        shape: &[],
    }],
    ret: "An `Iterable<string>` a `foreach` walks in file order, and walks again as often as it \
          is asked. `\\n`, `\\r\\n` and a lone `\\r` each end a line; a trailing terminator does \
          not open an empty last one, and an empty file has no lines at all.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — the file does not \
                   exist, is a directory, or could not be read.",
        },
    ],
};

/// `Core\IO::open`'s reference card — `rule:core-api/reference-card`.
const OPEN_DOC: MethodDoc = MethodDoc {
    short: "Opens a file and answers the handle every later read and write goes through — `fopen`, \
            with the mode string replaced by an enum. Needs `fs.read` for `Read`, `fs.write` for \
            `Write` and `Append`, and both for `ReadWrite`.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The file to open, absolute or relative to the working directory.",
            shape: &[],
        },
        ParamDoc {
            name: "mode",
            desc: "What the handle may do, as a `Core\\IO\\FileMode` case.",
            shape: &[],
        },
    ],
    ret: "An open `Core\\IO\\File`. It is closed by `close`, and by the end of the request if the \
          program never calls it.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant a capability this mode needs for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — `Read` on a path \
                   that is not there, a directory, or a path this process may not open.",
        },
    ],
};

/// `Core\IO::stdin`'s reference card — `rule:core-api/reference-card`.
const STDIN_DOC: MethodDoc = MethodDoc {
    short: "Reads everything the program's standard input will produce, in one call — \
            the `fgets(STDIN)` loop and every wrapper spelling of the same stream, with no wrapper \
            grammar in front of either. The result is `tainted`: the bytes are the invoker's, not \
            the program's. Needs no capability, because the descriptor is a grant the program \
            was started with.",
    params: &[],
    ret: "Every byte until end of input, as one `tainted string` — the empty string when input \
          is already closed, which is what a program started with no input sees. Input ends when \
          its writer ends it, so at a terminal this waits for the person there; a program that \
          means to ask someone a question uses `Core\\Cli`'s prompts, which have a deadline.",
    errors: &[ErrorDoc {
        error: "IOError",
        desc: "The operating system failed the read — the pipe's writer died, or the \
                   descriptor was not open for reading.",
    }],
};

/// `Core\IO\FileMode`'s fully-qualified name, in one place for the same reason
/// [`NAME`] is.
pub(crate) const FILE_MODE_NAME: &str = r"Core\IO\FileMode";

/// Spec § 14's `FileMode` — R11's "an enum, never a mode string".
///
/// Four cases where `fopen` has twelve spellings of six things, because the two
/// axes PHP's grammar crosses are not independent in any way a caller benefits
/// from: `b` is meaningless on every platform Novis targets, and `t` is a
/// translation this language does not perform, so the mode string's whole
/// remaining content is the access. What is left after that is `r`/`w`/`a` plus
/// one read-write case, and the `+` forms that differ only in whether they
/// truncate or create are folded into it — [`nvs_runtime::capability::Access`]
/// is the one home of what each does to the file.
///
/// The values are ordinals rather than anything meaningful, unlike
/// [`crate::log::LEVEL`]'s severities: nothing outside this language reads
/// them, so a number that looked like a `libc` flag would be a coincidence a
/// reader would then have to check.
pub(crate) const FILE_MODE: CoreEnum = CoreEnum {
    name: FILE_MODE_NAME,
    cases: &[("Read", 0), ("Write", 1), ("Append", 2), ("ReadWrite", 3)],
    doc: Some(&FILE_MODE_DOC),
};

/// [`FILE_MODE`]'s reference card — `rule:core-api/reference-card`.
const FILE_MODE_DOC: EnumDoc = EnumDoc {
    short: "What an open handle may do, replacing `fopen`'s mode string. There is no binary or text \
            flag: Novis text is octets, so every mode is what PHP would call binary.",
    cases: &[
        CaseDoc {
            name: "Read",
            desc: "Reading only, from the start of a file that must already exist — `fopen`'s `r`.",
        },
        CaseDoc {
            name: "Write",
            desc: "Writing only, emptying the file first and creating it if it is not there — \
                   `fopen`'s `w`.",
        },
        CaseDoc {
            name: "Append",
            desc: "Writing only, always at the end whatever else has written since, creating the \
                   file if it is not there — `fopen`'s `a`.",
        },
        CaseDoc {
            name: "ReadWrite",
            desc: "Both, creating the file if it is not there and emptying nothing — `fopen`'s \
                   `c+`, which is the one of its four `+` forms that surprises nobody.",
        },
    ],
};

/// `Core\IO::open`'s answer, as [`CoreTy::Instance`] spells it.
pub(crate) const FILE_NAME: &str = r"Core\IO\File";

/// [`FILE`]'s first slot: the key its open file is filed under in the request.
const FILE_SLOT: usize = 0;

/// [`FILE`]'s second slot: the path it was opened on, kept so that every
/// refusal a member of this class raises can name the file the way the program
/// wrote it.
///
/// **What it spends:** one `Value` and a shared reference to the string the
/// caller already passed, per open handle. The alternative is an `IOError`
/// saying only which member failed, which is the diagnostic a developer with
/// four open handles cannot act on.
const FILE_PATH_SLOT: usize = 1;

/// How much [`nvs_core_io_file_read_line`] reads at a time before it looks for
/// a terminator. A line longer than this costs one more read and nothing else;
/// the buffer is on the stack, so the number is a syscall-count choice and not
/// a footprint one.
const LINE_CHUNK: usize = 8 * 1024;

/// Spec § 14's `File` — R14's "an open file is an object and never a
/// `resource`".
///
/// # Decision: the slot holds a key, and the request holds the descriptor
///
/// A `Core` instance's slots hold Novis values, so a `std::fs::File` cannot go
/// in one ([`crate::instance`]'s first decision is the home of why). The slot
/// holds an `int` instead — the key
/// [`Ctx::hold_open_file`](nvs_runtime::Ctx::hold_open_file) filed the
/// descriptor under, whose own doc comment owns the accounting and the reason
/// the table is the *request's*: a `Core` instance has no native drop, so a
/// table this module kept would never learn that the last handle to a file had
/// gone, and its footprint would be O(files opened by the process).
/// `Core\Script\Handle` is the same shape for the same reason and landed first.
///
/// **This is what R14 buys over a `resource`.** A `resource` is an integer with
/// a type tag that every function accepting one has to re-check, and PHP's
/// whole `fread`/`fgets`/`fwrite`/`fseek`/`flock` family is that check written
/// out ninety times. Here the *type* is the check: `nvs_types` refuses a
/// `Core\IO\File` where a `Core\Regex\Match` is wanted before the program runs,
/// and the only thing left for a body to ask is whether the handle is still
/// open — which is one question, answered in [`open_file`], and is the one thing
/// a static type genuinely cannot know.
///
/// # § 14's handle roster is complete, and the standard streams are not on it
///
/// `read`, `readLine`, `write`, `seek`, `tell`, `truncate`, `flush`, `lock`
/// and `close`. `seek` and `tell` arrived when the handle became
/// random-access, since a position a caller can set is the whole difference
/// between this and a stream; `truncate` is the length half of the same idea,
/// and the only way a program shortens a file it is already holding open.
/// `lock` is the one member here that is not about this program's own view of
/// the file at all, and [`nvs_core_io_file_lock`] is the home of what it does
/// and does not promise.
///
/// **No instance of this class is ever a standard stream.** § 14's last line is
/// [`nvs_core_io_stdin`] alone — a member answering a `tainted string` rather
/// than a handle — and that member's doc comment is the home of why the writing
/// half does not exist and why the reading half is not one of these.
pub(crate) const FILE: CoreClass = CoreClass {
    name: FILE_NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "read",
            names: &["max"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_io_file_read",
            doc: Some(&FILE_READ_DOC),
        },
        CoreMethod {
            name: "readLine",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_io_file_read_line",
            doc: Some(&FILE_READ_LINE_DOC),
        },
        CoreMethod {
            name: "write",
            names: &["data"],
            // `$data` is not a path, so it is not a sink — the same reading
            // `Core\IO::write`'s own `$content` carries.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_io_file_write",
            doc: Some(&FILE_WRITE_DOC),
        },
        CoreMethod {
            name: "seek",
            // No `whence`: R3 refuses a mode argument, and `$offset` is from the
            // start because that is the only origin a caller can name without
            // first asking where the handle already is.
            names: &["offset"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_file_seek",
            doc: Some(&FILE_SEEK_DOC),
        },
        CoreMethod {
            name: "tell",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_io_file_tell",
            doc: Some(&FILE_TELL_DOC),
        },
        CoreMethod {
            name: "truncate",
            // A length is measured from the start of the file and from nowhere
            // else, so there is no origin to name here for `seek`'s reason.
            names: &["size"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_file_truncate",
            doc: Some(&FILE_TRUNCATE_DOC),
        },
        CoreMethod {
            name: "flush",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_file_flush",
            doc: Some(&FILE_FLUSH_DOC),
        },
        CoreMethod {
            name: "lock",
            // No `LOCK_SH`/`LOCK_EX`/`LOCK_UN`: R3 refuses the mode argument,
            // `close` is what `LOCK_UN` was, and exclusive is the one a caller
            // reaching for a lock at all means.
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_file_lock",
            doc: Some(&FILE_LOCK_DOC),
        },
        CoreMethod {
            name: "close",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_file_close",
            doc: Some(&FILE_CLOSE_DOC),
        },
    ],
    slots: &["handle", "path"],
    constants: &[],
};

/// `Core\IO\File::read`'s reference card — `rule:core-api/reference-card`.
const FILE_READ_DOC: MethodDoc = MethodDoc {
    short: "Reads up to `$max` bytes from where the handle is, and moves it past them — `fread`. \
            Needs no capability of its own: the descriptor was checked when `open` produced it.",
    params: &[ParamDoc {
        name: "max",
        desc: "The most bytes to read. Fewer are returned when the file ends first.",
        shape: &[],
    }],
    ret: "The bytes read, as a `string`. An empty string means the end of the file, which is the \
          one answer `read` gives that is not an error and not data.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The handle has already been closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The read itself failed, or the handle was not opened for reading.",
        },
    ],
};

/// `Core\IO\File::readLine`'s reference card — `rule:core-api/reference-card`.
const FILE_READ_LINE_DOC: MethodDoc = MethodDoc {
    short: "Reads the next line and moves the handle past it — `fgets`. The terminator is consumed \
            and never returned, and `\\n`, `\\r\\n` and `\\r` all end a line, exactly as \
            `Core\\Str::lines` and `Core\\IO::lines` divide one.",
    params: &[],
    ret: "The line without its terminator, or `null` at the end of the file — which is R5's \
          spelling of an absence, and the reason this member needs no separate `eof`. A last line \
          with no terminator on it is still a line.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The handle has already been closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The read itself failed, or the handle was not opened for reading.",
        },
    ],
};

/// `Core\IO\File::write`'s reference card — `rule:core-api/reference-card`.
const FILE_WRITE_DOC: MethodDoc = MethodDoc {
    short: "Writes `$data` at the handle's position and moves it past what went out — `fwrite`. \
            Needs no capability of its own: the descriptor was checked when `open` produced it.",
    params: &[ParamDoc {
        name: "data",
        desc: "The bytes to write.",
        shape: &[],
    }],
    ret: "How many bytes were written, which is every byte of `$data` — a short write is retried \
          rather than reported, so a caller never has to loop.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The handle has already been closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The write itself failed, or the handle was not opened for writing.",
        },
    ],
};

/// `Core\IO\File::seek`'s reference card — `rule:core-api/reference-card`.
const FILE_SEEK_DOC: MethodDoc = MethodDoc {
    short: "Moves the handle to `$offset` bytes from the start of the file — `fseek`, with no \
            `whence`. Seeking past the end is allowed and is how a sparse file is written: the \
            gap becomes zeroes when something is written after it.",
    params: &[ParamDoc {
        name: "offset",
        desc: "How many bytes from the start of the file the next read or write happens at.",
        shape: &[],
    }],
    ret: "Nothing. `tell` is how a program reads the position back, so this member has no answer \
          of its own to give.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The handle has already been closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The seek itself failed — a pipe or a terminal, which has no position to move \
                   to.",
        },
    ],
};

/// `Core\IO\File::tell`'s reference card — `rule:core-api/reference-card`.
const FILE_TELL_DOC: MethodDoc = MethodDoc {
    short: "Answers where the handle is, in bytes from the start of the file — `ftell`. It is the \
            position the next `read` or `write` acts at, which every member of this class leaves \
            just past what it touched.",
    params: &[],
    ret: "The position, as a `uint`. Zero on a handle nothing has read or written yet, unless \
          `FileMode::Append` put it at the end.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The handle has already been closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The query itself failed — a pipe or a terminal, which has no position to \
                   report.",
        },
    ],
};

/// `Core\IO\File::truncate`'s reference card — `rule:core-api/reference-card`.
const FILE_TRUNCATE_DOC: MethodDoc = MethodDoc {
    short: "Sets the file's length to `$size` bytes — `ftruncate`. A smaller size drops \
            everything past it; a larger one extends the file with zeroes, which is the same \
            hole a write past the end leaves.",
    params: &[ParamDoc {
        name: "size",
        desc: "How long the file is to be afterwards, in bytes from its start.",
        shape: &[],
    }],
    ret: "Nothing. The handle's own position does not move, so shortening a file can leave the \
          handle past its new end — `tell` still answers where it was, and a write there lands \
          over a hole rather than at the end.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The handle has already been closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The resize itself failed — most often a handle opened `FileMode::Read`, which \
                   the operating system will not resize.",
        },
    ],
};

/// `Core\IO\File::flush`'s reference card — `rule:core-api/reference-card`.
const FILE_FLUSH_DOC: MethodDoc = MethodDoc {
    short: "Hands everything written on this handle to the operating system — `fflush`. Novis \
            writes straight to the descriptor, so there is nothing of its own left to push, and \
            this member is the promise that a program never has to know that.",
    params: &[],
    ret: "Nothing. It is **not** `fsync`: reaching the operating system is not reaching the disk, \
          and durability is not something this member promises.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The handle has already been closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The flush itself failed. Nothing this class does can provoke one today, and \
                   the row is here because the answer belongs to the operating system rather than \
                   to this member.",
        },
    ],
};

/// `Core\IO\File::lock`'s reference card — `rule:core-api/reference-card`.
const FILE_LOCK_DOC: MethodDoc = MethodDoc {
    short: "Takes an exclusive lock on the file and holds it until the handle closes — `flock` \
            with `LOCK_EX`. It never waits: a lock another handle holds is refused rather than \
            queued for, so there is no `LOCK_NB` to remember and no unbounded wait to forget.",
    params: &[],
    ret: "Nothing, and there is no `unlock` — the lock's lifetime is the handle's, so `close` \
          releases it and so does the end of the request.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The handle has already been closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "Another handle already holds the lock, or the operating system refused it. \
                   Whether a lock stops a *non-holder's* own reads and writes is the platform's \
                   answer rather than this member's.",
        },
    ],
};

/// `Core\IO\File::close`'s reference card — `rule:core-api/reference-card`.
const FILE_CLOSE_DOC: MethodDoc = MethodDoc {
    short: "Closes the handle and releases the descriptor — `fclose`. Calling it is optional: a \
            handle the program never closes is closed when the request ends.",
    params: &[],
    ret: "Nothing.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The handle has already been closed. Closing twice is a bug in the program rather \
               than a state of the file, so it is reported rather than ignored.",
    }],
};

/// `Core\IO::lines`'s answer, as [`CoreTy::Instance`] spells it.
pub(crate) const LINES_NAME: &str = r"Core\IO\Lines";

/// The symbol behind `Iterable<string>::iterate()`, reached by name through
/// this class's method table rather than as a registered member — see
/// [`crate::cursor`] and [`crate::instance`]'s dispatch roster.
pub(crate) const LINES_ITERATE_SYMBOL: &str = "nvs_core_io_lines_iterate";

/// [`LINES`]'s one slot: the lines themselves, as an `array<string>`.
const LINES_SLOT: usize = 0;

/// The class `lines` answers with — spec § 14's `Iterable<string>`, given the
/// name the registry needs to write it.
///
/// # Decision: the lines are read and held, not streamed off the open file
///
/// `lines` is [`slurp`] plus [`crate::str::line_pieces`], exactly as `readText`
/// is `slurp` plus a decode: the door, the buffer and the ceiling are `read`'s,
/// and the split is `Core\Str::lines`' own so that the two members cannot come
/// to disagree about what a line is. **What it spends:** the file's octets once
/// more, as one `string` per line plus the list holding them, for as long as
/// the program holds the value — charged to the request that asked, and bounded
/// by the request's memory limit like `read`'s single buffer.
///
/// The alternative — a cursor over the open handle, reading a line at a time —
/// is what `fgets` is, and it is still not this member: [`FILE`] is now the
/// machinery an earlier version of this paragraph said did not exist, so a
/// `readLine` over an open handle is a signature away, but `lines` keeps
/// answering a walk that can be taken twice and cannot fail halfway. Those are
/// different promises, and a program that wants the streaming one asks `open`
/// for it. `AGENTS.md`'s ordering is what makes holding the octets acceptable:
/// footprint is last, and simplicity is bought with it.
///
/// **The type is still `Iterable<string>` and not `array<string>`**, because
/// what the spec promises a caller is a forward walk and nothing more — no
/// count, no index, no second meaning for a key. That is the surface a
/// streaming implementation would land behind unchanged.
///
/// # Why it has no members
///
/// Everything it does is `iterate()`, which is dispatched by name through
/// [`crate::instance`]'s roster rather than registered — so it is a *handle* in
/// the sense `registry`'s `a_class_with_slots_has_instance_members_and_the_reverse`
/// names, beside `Core\Script\Handle`: its slot is read, just not through a
/// member of its own.
pub(crate) const LINES: CoreClass = CoreClass {
    name: LINES_NAME,
    doc: None,
    methods: &[],
    instance: &[],
    slots: &["lines"],
    constants: &[],
};

/// `Core\IO::walk`'s answer, as [`CoreTy::Instance`] spells it.
pub(crate) const WALK_NAME: &str = r"Core\IO\Walk";

/// The symbol behind this class's `iterate()`, reached by name exactly as
/// [`LINES_ITERATE_SYMBOL`] is.
pub(crate) const WALK_ITERATE_SYMBOL: &str = "nvs_core_io_walk_iterate";

/// [`WALK`]'s one slot: the entries themselves, as an `array<string>`.
const WALK_SLOT: usize = 0;

/// The class `walk` answers with — spec § 14's second `Iterable<string>`.
///
/// # Decision: a tree, where `list` is a directory
///
/// Spec § 14 writes `list(string $path): array<string>` and
/// `walk(string $path): Iterable<string>` next to each other, and the only
/// reading of that pair `rule:core-api/shape-rules`
/// R6 admits is two *questions*: a member that answered the same entries in a
/// second container would be one operation reachable two ways, which is the
/// shape that rule closes. So `list` reads one directory and this reads the
/// whole tree under it — which is also what makes the pair cover everything
/// § 14's row names, `RecursiveDirectoryIterator` and the rest of the
/// `DirectoryIterator` family included, rather than `scandir` twice.
///
/// **An entry is a path relative to the root, not a bare name.** `list`'s
/// bare name is unambiguous because there is one directory; here there is a
/// tree, and a name alone could not say which of two `config.toml`s it found.
/// The join back onto the root is still the caller's own step for the reason
/// [`nvs_core_io_list`] gives, and `within` is still the member for it.
///
/// **A symbolic link is an entry and is never descended into.** `std::fs`'s
/// `DirEntry::file_type` reports the link rather than what it points at, which
/// is what this body asks — so a link that points back up its own tree makes
/// the walk finite instead of endless. That is a deliberate divergence from
/// [`nvs_core_io_is_dir`], whose whole partition follows links: a predicate
/// about one name is answering a question about what the name leads to, where
/// a walk that followed one would be enumerating a graph and calling it a
/// tree.
///
/// **Every directory the walk enters goes through the door**, rather than the
/// root alone standing for all of them. With today's path grants that cannot
/// refuse partway — `nvs_config`'s `Scope::Path` is a canonicalized prefix
/// match, so a granted root implies everything under it — and the check is per
/// directory anyway, because the door is where a `read_dir` is authorized and
/// a member that authorized one syscall on the strength of having authorized a
/// different one would be encoding that prefix rule into `Core\IO`. What it
/// spends is one capability check per directory, which is a string comparison
/// beside a `readdir` syscall.
///
/// # Decision: the entries are held, not streamed
///
/// The same decision [`LINES`] makes and the same reasoning, over a bigger
/// subject: the walk is taken whole when the member is called, so the value
/// can be walked twice and cannot fail halfway through a `foreach`. **What it
/// spends** is one `string` per entry in the tree plus the list holding them,
/// for as long as the program holds the value — charged to the request that
/// asked and bounded by its memory limit. That is a larger bill than `lines`'
/// and it is the same trade: `AGENTS.md`'s ordering puts footprint last, and a
/// streaming implementation lands behind `Iterable<string>` unchanged if a
/// tree that does not fit ever turns up.
///
/// # Why it has no members
///
/// [`LINES`]'s answer, for [`LINES`]'s reason: everything it does is
/// `iterate()`, dispatched by name through [`crate::instance`]'s roster.
pub(crate) const WALK: CoreClass = CoreClass {
    name: WALK_NAME,
    doc: None,
    methods: &[],
    instance: &[],
    slots: &["entries"],
    constants: &[],
};

/// `Core\IO::stat`'s answer, as [`CoreTy::Instance`] spells it.
pub(crate) const METADATA_NAME: &str = r"Core\IO\Metadata";

/// [`METADATA`]'s slots, in declaration order — the layout
/// [`nvs_core_io_stat`] fills and every member of that class reads back.
const METADATA_SIZE_SLOT: usize = 0;
/// See [`METADATA_SIZE_SLOT`].
const METADATA_MODIFIED_AT_SLOT: usize = 1;
/// See [`METADATA_SIZE_SLOT`].
const METADATA_IS_FILE_SLOT: usize = 2;
/// See [`METADATA_SIZE_SLOT`].
const METADATA_IS_DIR_SLOT: usize = 3;

/// Spec § 14's `stat`, as the value it answers with: one `stat` call's whole
/// answer about one path, frozen at the moment it was asked.
///
/// # Decision: an instance, not a fixed-key shape
///
/// [`crate::instance`] can build either — an `rule:types/object-top` shape is an anonymous
/// methodless object whose fields a program reads with `->size`, and a
/// [`CoreClass`] is one with members and no reachable field. The shape reads
/// better at the call site and is the wrong one here for one reason:
/// [`CoreTy`] has no spelling for a shape *return*, so a member answering one
/// would need a new registry variant, a lowering for it in `nvs-types` and a
/// second way for the checker to learn a `Core` member's result type. `Core`
/// already answers with instances in 141 rows, and `Core\Time\Instant` — which
/// this class holds one of — is the same shape. A record is not worth a second
/// return-type mechanism.
///
/// # Decision: it answers the questions the single-question members answer
///
/// `size`, `modifiedAt`, `isFile` and `isDir` are all members of `Core\IO`
/// too, and spec § 14's metadata bullet names them beside `stat` on purpose.
/// **`rule:core-api/shape-rules` R17/R18 are not what that collides with**: R18 forbids a static
/// that *mirrors an object's own method* — `Time::format($instant, $fmt)`
/// beside `$instant->format($fmt)`, where the object is the static's own
/// subject. Here the subject of `Core\IO::size` is a `string` path and the
/// operation is a syscall; the subject of `$m->size()` is a snapshot already
/// taken and the operation is a slot read. They are not two spellings of one
/// operation and their answers can differ — a file that grew between the two
/// calls says so — which is exactly why a program that asks more than one
/// question asks `stat` once instead.
///
/// # What it does not carry
///
/// **No permission member.** `fileperms` has no portable content: Windows
/// records one read-only attribute and Unix records nine mode bits, so a
/// member over `std::fs::Permissions::readonly` would answer the platform's
/// question rather than the language's, and no `Core` member sets permissions,
/// so no case could ever pin its `true` side. The question a program actually
/// asks is answered by § 14's `isReadable` and `isWritable`, which are access
/// checks over the process, the path and the mount rather than a bit on the
/// inode.
///
/// **No created or accessed time.** Neither is recorded by every filesystem
/// Novis runs on, and a member whose answer is "this platform does not know"
/// is one every caller has to write a branch for. § 14 names `modifiedAt`
/// alone, and that is the one every filesystem has.
///
/// **What it spends:** two allocations per `stat` — this instance and the
/// `Core\Time\Instant` in its second slot — charged to the request that asked,
/// released with it. The `Instant` is built eagerly rather than from a stored
/// pair of `int` slots so that its representation stays inside
/// [`crate::time`], which is where [`crate::time::instant_at_system_time`]'s
/// own doc argues it belongs.
pub(crate) const METADATA: CoreClass = CoreClass {
    name: METADATA_NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "size",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_io_metadata_size",
            doc: Some(&METADATA_SIZE_DOC),
        },
        CoreMethod {
            name: "modifiedAt",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(crate::time::INSTANT_NAME),
            symbol: "nvs_core_io_metadata_modified_at",
            doc: Some(&METADATA_MODIFIED_AT_DOC),
        },
        CoreMethod {
            name: "isFile",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_io_metadata_is_file",
            doc: Some(&METADATA_IS_FILE_DOC),
        },
        CoreMethod {
            name: "isDir",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_io_metadata_is_dir",
            doc: Some(&METADATA_IS_DIR_DOC),
        },
    ],
    slots: &["size", "modifiedAt", "isFile", "isDir"],
    constants: &[],
};

/// `Core\IO\Metadata::size`'s reference card — `rule:core-api/reference-card`.
const METADATA_SIZE_DOC: MethodDoc = MethodDoc {
    short: "The file's size in bytes at the moment `stat` was called. Needs no capability of its \
            own: the path was checked when `stat` produced this value.",
    params: &[],
    ret: "The byte count as a `uint`. A directory's is the platform's own number for a directory \
          entry and means nothing portable.",
    errors: &[],
};

/// `Core\IO\Metadata::modifiedAt`'s reference card — `rule:core-api/reference-card`.
const METADATA_MODIFIED_AT_DOC: MethodDoc = MethodDoc {
    short: "When the file was last written, as a `Core\\Time\\Instant` — the same answer \
            `Core\\IO::modifiedAt` gives, out of the `stat` this value already holds.",
    params: &[],
    ret: "The modification time as an absolute point on the timeline, with no zone of its own.",
    errors: &[],
};

/// `Core\IO\Metadata::isFile`'s reference card — `rule:core-api/reference-card`.
const METADATA_IS_FILE_DOC: MethodDoc = MethodDoc {
    short: "Whether the path was a regular file. Together with `isDir` this partitions most of \
            what exists and does not cover it — a socket, a device node and a named pipe answer \
            `false` to both.",
    params: &[],
    ret: "`true` for a regular file, `false` for anything else that was there.",
    errors: &[],
};

/// `Core\IO\Metadata::isDir`'s reference card — `rule:core-api/reference-card`.
const METADATA_IS_DIR_DOC: MethodDoc = MethodDoc {
    short: "Whether the path was a directory — the other half of the partition `isFile` \
            describes.",
    params: &[],
    ret: "`true` for a directory, `false` for anything else that was there.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_io_read" => (nvs_core_io_read as *const ()).cast(),
        "nvs_core_io_read_text" => (nvs_core_io_read_text as *const ()).cast(),
        "nvs_core_io_write" => (nvs_core_io_write as *const ()).cast(),
        "nvs_core_io_append" => (nvs_core_io_append as *const ()).cast(),
        "nvs_core_io_write_stream" => (nvs_core_io_write_stream as *const ()).cast(),
        "nvs_core_io_exists" => (nvs_core_io_exists as *const ()).cast(),
        "nvs_core_io_is_file" => (nvs_core_io_is_file as *const ()).cast(),
        "nvs_core_io_is_dir" => (nvs_core_io_is_dir as *const ()).cast(),
        "nvs_core_io_is_readable" => (nvs_core_io_is_readable as *const ()).cast(),
        "nvs_core_io_is_writable" => (nvs_core_io_is_writable as *const ()).cast(),
        "nvs_core_io_list" => (nvs_core_io_list as *const ()).cast(),
        "nvs_core_io_canonicalize" => (nvs_core_io_canonicalize as *const ()).cast(),
        "nvs_core_io_size" => (nvs_core_io_size as *const ()).cast(),
        "nvs_core_io_modified_at" => (nvs_core_io_modified_at as *const ()).cast(),
        "nvs_core_io_stat" => (nvs_core_io_stat as *const ()).cast(),
        "nvs_core_io_metadata_size" => (nvs_core_io_metadata_size as *const ()).cast(),
        "nvs_core_io_metadata_modified_at" => {
            (nvs_core_io_metadata_modified_at as *const ()).cast()
        }
        "nvs_core_io_metadata_is_file" => (nvs_core_io_metadata_is_file as *const ()).cast(),
        "nvs_core_io_metadata_is_dir" => (nvs_core_io_metadata_is_dir as *const ()).cast(),
        "nvs_core_io_copy" => (nvs_core_io_copy as *const ()).cast(),
        "nvs_core_io_move" => (nvs_core_io_move as *const ()).cast(),
        "nvs_core_io_make_dir" => (nvs_core_io_make_dir as *const ()).cast(),
        "nvs_core_io_remove" => (nvs_core_io_remove as *const ()).cast(),
        "nvs_core_io_remove_dir" => (nvs_core_io_remove_dir as *const ()).cast(),
        "nvs_core_io_temporary_dir" => (nvs_core_io_temporary_dir as *const ()).cast(),
        "nvs_core_io_within" => (nvs_core_io_within as *const ()).cast(),
        "nvs_core_io_lines" => (nvs_core_io_lines as *const ()).cast(),
        "nvs_core_io_walk" => (nvs_core_io_walk as *const ()).cast(),
        "nvs_core_io_open" => (nvs_core_io_open as *const ()).cast(),
        "nvs_core_io_stdin" => (nvs_core_io_stdin as *const ()).cast(),
        "nvs_core_io_file_read" => (nvs_core_io_file_read as *const ()).cast(),
        "nvs_core_io_file_read_line" => (nvs_core_io_file_read_line as *const ()).cast(),
        "nvs_core_io_file_write" => (nvs_core_io_file_write as *const ()).cast(),
        "nvs_core_io_file_seek" => (nvs_core_io_file_seek as *const ()).cast(),
        "nvs_core_io_file_tell" => (nvs_core_io_file_tell as *const ()).cast(),
        "nvs_core_io_file_truncate" => (nvs_core_io_file_truncate as *const ()).cast(),
        "nvs_core_io_file_flush" => (nvs_core_io_file_flush as *const ()).cast(),
        "nvs_core_io_file_lock" => (nvs_core_io_file_lock as *const ()).cast(),
        "nvs_core_io_file_close" => (nvs_core_io_file_close as *const ()).cast(),
        LINES_ITERATE_SYMBOL => (nvs_core_io_lines_iterate as *const ()).cast(),
        WALK_ITERATE_SYMBOL => (nvs_core_io_walk_iterate as *const ()).cast(),
        _ => return None,
    })
}

/// One `string` argument, or the engine fault a wrongly-tagged one is: the
/// signature is checked at compile time, so a bad tag here is a lowering bug
/// and not something a program can provoke.
fn text<'a>(value: &'a Value, member: &str, what: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\IO::{member} expected {:?} for its {what}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::read(string $path): string` — replacing
    /// `file_get_contents`.
    ///
    /// The whole file into one buffer, with no size ceiling **of its own**:
    /// what bounds it is `[limits] max_output`, which
    /// `rule:core-classes/process-run` already reuses for the other member that
    /// fills a buffer from outside the request. A ceiling of this member's own
    /// would be a second number an operator has to keep in step with that one.
    /// [`slurp`] is where it is asked and
    /// [`nvs_runtime::Ctx::intake_limit`] owns the unit it is read in.
    fn nvs_core_io_read(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "read", "path")?);
        Ok(Value::str(NvsStr::new(&slurp(ctx, path, "Core\\IO::read")?)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::readText(string $path, {charset?: Core\Charset}): string` —
    /// replacing `file_get_contents` followed by `mb_convert_encoding`.
    ///
    /// [`slurp`] and then [`crate::encoding::decode_argument`], which is the
    /// whole member: the door, the buffer and the ceiling are `read`'s, and § 7
    /// already owns what an exact conversion is. The decode runs *after* the
    /// capability check for the ordinary reason — a file this program was never
    /// granted must refuse the same way whatever its bytes are.
    fn nvs_core_io_read_text(ctx, args: [2]) {
        let path = Path::new(text(&args[0], "readText", "path")?);
        let raw = slurp(ctx, path, "Core\\IO::readText")?;
        let text = crate::encoding::decode_argument(args, "Core\\IO::readText", &raw)?;
        Ok(Value::str(NvsStr::new(text.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::lines(string $path): Iterable<string>` — replacing `file()`
    /// and the `while (fgets(…))` loop.
    ///
    /// [`slurp`] and then [`crate::str::line_pieces`], which is the whole
    /// member. [`LINES`]'s own docs own the decision it rests on — the lines
    /// are held rather than streamed — and why the answer is still spelled
    /// `Iterable<string>`.
    ///
    /// **No decode**, deliberately: this is `read`'s treatment of a file's
    /// octets and not `readText`'s, so a caller who knows what charset the
    /// file is in reads it with `readText` and splits it with
    /// `Core\Str::lines`. The three terminators are ASCII, so the split itself
    /// asks nothing of the bytes around them.
    fn nvs_core_io_lines(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "lines", "path")?);
        let raw = slurp(ctx, path, "Core\\IO::lines")?;
        let mut out = NvsArray::new();
        for line in crate::str::line_pieces(&raw) {
            out.append(Value::str(NvsStr::new(line)));
        }
        Ok(crate::instance::build(&LINES, [Value::array(out)]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<string>::iterate(): Iterator<string>` — a cursor over the
    /// lines this value is already holding.
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed —
    /// [`crate::cursor`]'s module docs own both halves of that. The cursor
    /// shares the list rather than copying it, because nothing can change it:
    /// [`LINES`] has no member at all, let alone a mutating one, so the
    /// snapshot every other `iterate()` has to take was taken once already
    /// when `lines` built the value.
    fn nvs_core_io_lines_iterate(_ctx, args: [1]) {
        let cursor = held_strings(args[0], &LINES, LINES_SLOT, nvs_runtime::sequence::ITERATE)
            .map(crate::cursor::over);
        crate::cursor::consume(args[0]);
        cursor
    }
}

/// The `array<string>` a [`LINES`] or [`WALK`] receiver holds, **retained** —
/// the caller takes over the reference this answers with.
///
/// One function for both because the two classes are one shape: a slot holding
/// the walk that was taken when the member was called, read by an `iterate()`
/// that is reached by name. `class` and `slot` are what tell them apart, and
/// the message names the class it was handed.
///
/// # Errors
///
/// A `Fault::fatal` if the receiver is not one of that class's instances, on
/// the same reading as [`text`]: the signature was checked at compile time and
/// the slot is written by the member that built the value and by nothing else,
/// so either mismatch is a bug in this crate rather than something a program
/// can reach.
fn held_strings(
    value: Value,
    class: &'static CoreClass,
    slot: usize,
    member: &str,
) -> Result<NvsArray, Fault> {
    let receiver = crate::instance::receiver(value, class, member)?;
    let held = crate::instance::slot(receiver, slot);
    let ptr = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{}::{member} expected {:?} in its `{}` slot, got tag {}",
            class.name,
            Tag::Array,
            class.slots[slot],
            held.tag_byte()
        ))
    })?;
    let borrowed = crate::arr::borrowed(ptr);
    Ok((*borrowed).clone())
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::stdin(): tainted string` — spec § 14's standard streams, which
    /// are one member and not three.
    ///
    /// # Decision: the reading half is a value, and there is no writing half
    ///
    /// **Standard output and standard error are not here at all.**
    /// `rule:tooling/terminal-output-is-a-sink`
    /// makes both a sink whose substitution is uniform — not qualifier-dependent
    /// and, in that section's own words, not tty-dependent either, because a CI
    /// log is written to a pipe and read by a human afterwards. A
    /// `Core\IO\File` over descriptor 1 whose `write` is a `write_all` would be
    /// a raw door onto that stream, so the sink would hold only for the program
    /// that did not take the other door; and it would be a second spelling of
    /// `Core\Cli::write`, which
    /// `rule:core-api/shape-rules`'s "no
    /// operation is reachable two ways" refuses on its own. The consequence is
    /// recorded rather than hidden: a program cannot emit byte-exact binary on
    /// its standard output, and one that must emit bytes names a file.
    ///
    /// **The reading half is a `string` and not a handle**, unlike everything
    /// else this class opens. Six of [`FILE`]'s nine members are meaningless or
    /// destructive on a descriptor the process was handed rather than opened:
    /// `seek` and `tell` want a position a pipe does not have, `truncate` and
    /// `lock` want a file, and `close` would shut the *process's* standard
    /// input — which the request-scoped table
    /// ([`Ctx::hold_open_file`](nvs_runtime::Ctx::hold_open_file)) would then do
    /// again at the end of every request that read one, taking descriptor 0
    /// away from every later request sharing the process. A member that answers
    /// the bytes owns none of that.
    ///
    /// **A terminal is waited on, and this member does not pretend otherwise.**
    /// Input ends when the writer ends it, and at a terminal the writer is a
    /// person pressing Ctrl-D — the platform's contract, which a member that
    /// reads to end of input does not get to overrule. A refusal was written and
    /// taken back out: the condition can only be met by a program run at a
    /// terminal, so no conformance case could ever reach it, and
    /// `conformance_coverage`'s error-path gate is right that a message no case
    /// can provoke is the state to avoid. What that refusal was reaching for is
    /// a real hazard and belongs where it can be asserted: a *served request*
    /// calling this member parks its core on a descriptor the request never
    /// opened. Goal `server` is where a case can serve a request, and so where that
    /// decision is answerable rather than guessed at. The interactive half is
    /// `Core\Cli`'s prompts, which ask a question under a deadline
    /// (`rule:tooling/a-prompt-is-a-core-member`) and are what a program at a terminal actually wants.
    ///
    /// **What it spends:** one buffer the size of the input, charged to the
    /// request's memory limit like [`nvs_core_io_read`]'s and bounded by the
    /// same number rather than by a second one.
    fn nvs_core_io_stdin(_ctx, _args: [0]) {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin().lock(), &mut bytes).map_err(|err| {
            nvs_runtime::capability::io_failure("Core\\IO::stdin", Path::new("<stdin>"), &err)
        })?;
        Ok(Value::str(NvsStr::new(&bytes)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::open(string $path, Core\IO\FileMode $mode): Core\IO\File` —
    /// replacing `fopen`.
    ///
    /// The door decides which capability the mode needs, this decides nothing,
    /// and what comes back is filed against the request before the object that
    /// names it exists — so a failure between the two closes the descriptor by
    /// dropping it rather than stranding it in a table nothing points at.
    fn nvs_core_io_open(ctx, args: [2]) {
        let spelling = text(&args[0], "open", "path")?;
        let access = access_of(&args[1])?;
        let file = nvs_runtime::capability::open(ctx, Path::new(spelling), access, "Core\\IO::open")?;
        let key = ctx.hold_open_file(file);
        #[expect(
            unsafe_code,
            reason = "the path is owned by the caller's argument, which outlives \
                      this call, so the copy this handle keeps for its refusals \
                      needs a reference of its own"
        )]
        unsafe {
            args[0].retain();
        }
        Ok(crate::instance::build(&FILE, [Value::uint(key), args[0]]))
    }
}

/// One `Core\IO\FileMode` case, as the access
/// [`nvs_runtime::capability::open`] asks for.
///
/// # Errors
///
/// A `Fault::fatal` for anything that is not one of [`FILE_MODE`]'s four
/// values, on [`crate::log`]'s `level_of` reading: the row's parameter is
/// `CoreTy::Enum`, so `E0401` refused every other spelling at the call site and
/// what is left is a lowering bug.
fn access_of(value: &Value) -> Result<Access, Fault> {
    match value.as_int() {
        Some(0) => Ok(Access::Read),
        Some(1) => Ok(Access::Write),
        Some(2) => Ok(Access::Append),
        Some(3) => Ok(Access::ReadWrite),
        // Unreachable from source: the row's parameter is `CoreTy::Enum`, so
        // `E0401` refuses anything that is not a case of this enum at the call
        // site, and a case of it is one of the four integers above. Fatal
        // rather than thrown for `crate::log`'s `level_of` reason.
        _ => Err(Fault::fatal(format!(
            "Core\\IO::open expected a `{FILE_MODE_NAME}` case, got tag {} value {:?}",
            value.tag_byte(),
            value.as_int()
        ))),
    }
}

/// The key and the path a [`FILE`] receiver holds — everything a member reads
/// out of that receiver, so that none of them spells the slot indices itself.
///
/// The path is **borrowed** from the receiver, which the caller owns for the
/// length of the call.
///
/// `pub(crate)` because a handle is an argument as well as a receiver:
/// [`crate::csv`]'s `rows` opens its walk on a file this class already has
/// open, and reads the descriptor back through the same table these members do.
/// A caller outside this module names its own member in `member`, since that is
/// what the messages below are for.
///
/// # Errors
///
/// A `Fault::fatal` if the receiver is not one of this class's instances or a
/// slot holds the wrong tag, on [`held_lines`]' reading: both slots are written
/// by [`nvs_core_io_open`] and by nothing else.
pub(crate) fn handle_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
    let receiver = crate::instance::receiver(value, &FILE, member)?;
    let key = crate::instance::slot(receiver, FILE_SLOT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{FILE_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                FILE.slots[FILE_SLOT]
            ))
        })?;
    Ok((key, crate::instance::slot(receiver, FILE_PATH_SLOT)))
}

/// The catchable `RuntimeError` every member of [`FILE`] raises on a handle
/// that has already been closed.
///
/// A `RuntimeError` and not an `IOError` because nothing about the file went
/// wrong: the program asked a question of something it had already given up,
/// which is a mistake in the program (`rule:errors/on-uncaught-throw`'s split).
fn already_closed(member: &str, path: &Value) -> Fault {
    Fault::thrown(format!(
        "{FILE_NAME}::{member}: this handle is closed — {}",
        path.as_text().unwrap_or("?")
    ))
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::read(uint $max): string` — replacing `fread`.
    ///
    /// The buffer grows to what the file actually had, not to `$max`: a caller
    /// asking for a megabyte from a file with nine bytes left in it should not
    /// charge a megabyte to the request, and `Read::take` is what makes the
    /// argument a ceiling rather than a size.
    fn nvs_core_io_file_read(ctx, args: [2]) {
        let (key, path) = handle_of(args[0], "read")?;
        let max = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{FILE_NAME}::read expected {:?} for its max, got tag {}",
                Tag::Uint,
                args[1].tag_byte()
            ))
        })?;
        let mut octets = Vec::new();
        {
            let file = ctx.open_file_mut(key).ok_or_else(|| already_closed("read", &path))?;
            file.take(max).read_to_end(&mut octets)
        }
        .map_err(|err| {
            nvs_runtime::capability::io_failure(
                "Core\\IO\\File::read",
                Path::new(path.as_text().unwrap_or("?")),
                &err,
            )
        })?;
        Ok(Value::str(NvsStr::new(&octets)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::readLine(): ?string` — replacing `fgets`.
    ///
    /// # Decision: a chunk is read and the tail is given back, and what a line
    /// is comes from one place
    ///
    /// The descriptor lives in the request's table
    /// ([`nvs_runtime::Ctx::hold_open_file`]) and nothing wraps it, so there is
    /// no `BufReader` to hold a line's worth of look-ahead between two calls.
    /// The two ways to read a line off a bare descriptor are a byte at a time —
    /// a syscall per byte, which prices a member the whole point of which is
    /// walking a file — or a chunk at a time with the bytes past the terminator
    /// handed back by seeking. This does the second: two syscalls a line rather
    /// than one a byte, and the position afterwards is exactly where a caller
    /// interleaving [`nvs_core_io_file_read`] would need it to be. **What it
    /// spends:** one [`LINE_CHUNK`]-byte stack buffer for the length of the
    /// call, plus the line itself, charged to the request that asked.
    ///
    /// The terminator is **not** returned, which is where this parts company
    /// with `fgets` and joins [`crate::str::line_pieces`] — the one place that
    /// decides what a line is, so that `readLine` and `Core\IO::lines` cannot
    /// come to disagree about a `\r\n`. PHP's answer keeps the newline and every
    /// correct caller then writes the same `rtrim`, which is the shape `rule:core-api/shape-rules`
    /// R5 refuses: the end of the file is `null` and nothing else has to be
    /// looked for.
    fn nvs_core_io_file_read_line(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "readLine")?;
        let failed = |err: &std::io::Error| {
            nvs_runtime::capability::io_failure(
                "Core\\IO\\File::readLine",
                Path::new(path.as_text().unwrap_or("?")),
                err,
            )
        };
        let file = ctx
            .open_file_mut(key)
            .ok_or_else(|| already_closed("readLine", &path))?;

        let mut line = Vec::new();
        let mut buffer = [0u8; LINE_CHUNK];
        loop {
            let read = file.read(&mut buffer).map_err(|err| failed(&err))?;
            if read == 0 {
                // The end of the file. A last line with no terminator is still
                // a line; nothing at all is the absence R5 spells `null`.
                return Ok(if line.is_empty() {
                    Value::null()
                } else {
                    Value::str(NvsStr::new(&line))
                });
            }
            let chunk = &buffer[..read];
            let Some(at) = chunk.iter().position(|byte| *byte == b'\n' || *byte == b'\r') else {
                line.extend_from_slice(chunk);
                continue;
            };
            line.extend_from_slice(&chunk[..at]);
            let mut consumed = at + 1;
            if chunk[at] == b'\r' {
                if at + 1 < read {
                    consumed += usize::from(chunk[at + 1] == b'\n');
                } else {
                    // A `\r` that ended the buffer: one more byte decides
                    // whether this is a `\r\n` cluster or a lone `\r`, and the
                    // byte is given back when it is neither.
                    let mut next = [0u8; 1];
                    if file.read(&mut next).map_err(|err| failed(&err))? == 1 && next[0] != b'\n' {
                        file.seek(std::io::SeekFrom::Current(-1))
                            .map_err(|err| failed(&err))?;
                    }
                }
            }
            // What of the chunk lies past the terminator, given back so that the
            // handle ends this call exactly where the line ended. It is at most
            // `LINE_CHUNK`, which is why the conversion cannot fail.
            let tail = i64::try_from(read - consumed)
                .expect("a chunk is `LINE_CHUNK` bytes, which fits an `i64` on every target");
            if tail > 0 {
                file.seek(std::io::SeekFrom::Current(-tail))
                    .map_err(|err| failed(&err))?;
            }
            return Ok(Value::str(NvsStr::new(&line)));
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::write(string $data): uint` — replacing `fwrite`.
    ///
    /// `write_all` rather than one `write`, so the answer is always every byte
    /// of `$data`: PHP's short-write return is a number every correct caller
    /// turns into the same loop, and a member that returns it is a member whose
    /// contract is "you finish this".
    fn nvs_core_io_file_write(ctx, args: [2]) {
        let (key, path) = handle_of(args[0], "write")?;
        let data = text(&args[1], "write", "data")?;
        {
            let file = ctx.open_file_mut(key).ok_or_else(|| already_closed("write", &path))?;
            file.write_all(data.as_bytes())
        }
        .map_err(|err| {
            nvs_runtime::capability::io_failure(
                "Core\\IO\\File::write",
                Path::new(path.as_text().unwrap_or("?")),
                &err,
            )
        })?;
        Ok(Value::uint(data.len() as u64))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::seek(uint $offset): void` — replacing `fseek`, and half of
    /// what makes a handle random-access rather than a stream.
    ///
    /// # Decision: one origin, and it is the start of the file
    ///
    /// PHP's `fseek` takes a `$whence` of `SEEK_SET`, `SEEK_CUR` or `SEEK_END`,
    /// which is exactly the `int` mode argument `rule:core-api/shape-rules` R3 refuses: three
    /// unrelated operations reached through one member, chosen by a constant the
    /// signature cannot check. `SEEK_SET` is the one a caller can name on its
    /// own; the other two are `seek($file->tell() + $n)` and a size read, both
    /// spelled out of members this class already has, and both of which say at
    /// the call site which origin was meant. So the parameter is a `uint` and
    /// there is no negative offset to validate — the type refuses the seek
    /// before the file is asked.
    ///
    /// Seeking past the end is **not** an error, here or in the operating
    /// system: it is how a sparse file is written, and the hole becomes zeroes
    /// when a later write lands past it. `tell` will answer that position, and a
    /// read there returns nothing, which is `read`'s ordinary end-of-file.
    fn nvs_core_io_file_seek(ctx, args: [2]) {
        let (key, path) = handle_of(args[0], "seek")?;
        let offset = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{FILE_NAME}::seek expected {:?} for its offset, got tag {}",
                Tag::Uint,
                args[1].tag_byte()
            ))
        })?;
        {
            let file = ctx.open_file_mut(key).ok_or_else(|| already_closed("seek", &path))?;
            file.seek(std::io::SeekFrom::Start(offset))
        }
        .map_err(|err| {
            nvs_runtime::capability::io_failure(
                "Core\\IO\\File::seek",
                Path::new(path.as_text().unwrap_or("?")),
                &err,
            )
        })?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::tell(): uint` — replacing `ftell`, and the other half of a
    /// random-access handle.
    ///
    /// `stream_position` rather than a zero-length `seek(Current(0))`: the two
    /// are the same syscall and the first says what it is for. The answer is a
    /// `uint` because a position is a distance from the start and cannot be
    /// negative — the same reading that gives [`nvs_core_io_file_seek`] its
    /// parameter type, so the two agree on what a position is and a round trip
    /// through them cannot lose a value.
    fn nvs_core_io_file_tell(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "tell")?;
        let at = {
            let file = ctx.open_file_mut(key).ok_or_else(|| already_closed("tell", &path))?;
            file.stream_position()
        }
        .map_err(|err| {
            nvs_runtime::capability::io_failure(
                "Core\\IO\\File::tell",
                Path::new(path.as_text().unwrap_or("?")),
                &err,
            )
        })?;
        Ok(Value::uint(at))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::truncate(uint $size): void` — replacing `ftruncate`.
    ///
    /// # Decision: no capability of its own, because the handle is the grant
    ///
    /// Every other door in this module opens on a *path*, and
    /// [`nvs_runtime::capability`] has no member that takes a descriptor —
    /// deliberately, because by the time one exists the question has already
    /// been answered. [`nvs_core_io_open`] asked for `fs.write` when the mode
    /// was `Write`, `Append` or `ReadWrite`, and the handle is the proof that
    /// it was granted. Re-checking the path here would be checking a *second*
    /// path: the name may have been renamed or replaced since the descriptor
    /// was opened, and the file this member resizes is the one the descriptor
    /// holds either way. `Core\Storage` reuses the same reading over the same
    /// `fs.*` grants, and [`crate::storage`]'s module doc is the home of it.
    ///
    /// A handle opened `Read` carries no such proof, and nothing in this body
    /// looks for one: the operating system refuses the resize, and the program
    /// sees the `IOError` every other failure of the file itself arrives as.
    /// The alternative — reading the mode back out of a third slot and raising
    /// a refusal of our own — spends a slot per open handle to restate an
    /// answer the descriptor already has.
    ///
    /// # Decision: the position is left where it was
    ///
    /// POSIX `ftruncate` does not move the file offset and neither does
    /// [`std::fs::File::set_len`], so a handle at byte 10 of a file cut to 4
    /// stays at 10, and the next write lands there over a zero hole. That is
    /// the state [`nvs_core_io_file_seek`] can already produce past the end, so
    /// a program has one rule to learn rather than two, and this member gains
    /// no answer of its own that a caller would have to check.
    fn nvs_core_io_file_truncate(ctx, args: [2]) {
        let (key, path) = handle_of(args[0], "truncate")?;
        let size = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{FILE_NAME}::truncate expected {:?} for its size, got tag {}",
                Tag::Uint,
                args[1].tag_byte()
            ))
        })?;
        {
            let file = ctx.open_file_mut(key).ok_or_else(|| already_closed("truncate", &path))?;
            file.set_len(size)
        }
        .map_err(|err| {
            nvs_runtime::capability::io_failure(
                "Core\\IO\\File::truncate",
                Path::new(path.as_text().unwrap_or("?")),
                &err,
            )
        })?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::flush(): void` — replacing `fflush`.
    ///
    /// Novis holds no buffer of its own in front of the descriptor: every
    /// [`nvs_core_io_file_write`] is a `write_all` that has already reached the
    /// operating system by the time it returns. So this is `Write::flush` and
    /// nothing more, and the member exists for two reasons that outlive that —
    /// a program porting from `fflush` should not have to know which layer
    /// buffered, and if a buffer is ever put in front of a handle this is
    /// already the place that empties it.
    ///
    /// **It is deliberately not `sync_data`.** Durability is a much stronger and
    /// much more expensive promise than the one `fflush` makes, and a member
    /// that quietly upgraded to it would price every port from PHP at an
    /// `fsync` per call. A member that means durability can be added when
    /// something asks for one; it will not be spelled `flush`.
    fn nvs_core_io_file_flush(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "flush")?;
        {
            let file = ctx.open_file_mut(key).ok_or_else(|| already_closed("flush", &path))?;
            file.flush()
        }
        .map_err(|err| {
            nvs_runtime::capability::io_failure(
                "Core\\IO\\File::flush",
                Path::new(path.as_text().unwrap_or("?")),
                &err,
            )
        })?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::lock(): void` — replacing `flock`, and the whole of it.
    ///
    /// # Decision: exclusive, and there is no mode argument
    ///
    /// PHP's `flock($h, LOCK_SH|LOCK_EX|LOCK_UN)` is three unrelated
    /// operations behind one `int`, which is exactly `rule:core-api/shape-rules` R3's refusal —
    /// and one of the three is not an operation on the lock at all. `LOCK_UN`
    /// is this class's [`nvs_core_io_file_close`]: the lock is the
    /// descriptor's, so the operating system drops it when the descriptor
    /// goes, and a release member would be a second spelling for something
    /// that already happens. What is left is shared against exclusive, and
    /// this is the exclusive one because it is what a caller reaching for a
    /// lock at all almost always means. A shared lock coordinates readers
    /// against a writer, which is a second question; it would arrive as R11's
    /// enum — a row in [`crate::registry::ENUMS`] and a parameter — rather
    /// than as the `bool` that would make this member two members.
    ///
    /// # Decision: it never waits, and contention throws
    ///
    /// `flock` without `LOCK_NB` waits for as long as the other holder cares
    /// to hold it, which on a request path is an unbounded wait governed by no
    /// timeout in this process. Every other member of this class blocks its
    /// core too, and that is not the same thing: a read finishes because the
    /// disk finishes, while a contended lock finishes when another *program*
    /// decides. That is the wait `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` leaves no spelling for on the
    /// outbound side, and the reading carries over, so this is `try_lock` and
    /// there is no waiting form of it anywhere.
    ///
    /// Contention is therefore an `IOError` and never a `false`: `rule:core-api/shape-rules`
    /// leaves no room for a falsy return, and `rule:errors/on-uncaught-throw`'s split puts this
    /// on the file's side of the line — nothing about the program is wrong,
    /// the file is held. A caller that wants to wait writes the wait it
    /// actually means, out of a task that sleeps between attempts and hands
    /// the core back while it does.
    ///
    /// # What this does not promise
    ///
    /// Whether the lock stops a *non-holder's* `read` or `write` is the
    /// operating system's answer and not this member's — advisory on Unix,
    /// mandatory on Windows, as [`std::fs::File::try_lock`] states — so no
    /// case in this tree freezes either behaviour. What is portable, and what
    /// the member is for, is that two handles cannot hold it at once. Locking
    /// a handle that already holds one is unspecified for the same reason, and
    /// the only thing that keeps that from being a hazard is that this member
    /// never blocks: the worst it can do is refuse.
    fn nvs_core_io_file_lock(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "lock")?;
        let taken = {
            let file = ctx.open_file_mut(key).ok_or_else(|| already_closed("lock", &path))?;
            file.try_lock()
        };
        // Through `io::Error` rather than matching the two arms of the lock's
        // own error type: naming that type means naming `std::fs`, which the
        // scan in `tests/capability.rs` forbids this crate outright, and the
        // conversion is the one std defines — contention becomes
        // `ErrorKind::WouldBlock` and nothing else does.
        match taken {
            Ok(()) => Ok(Value::null()),
            Err(refused) => {
                let refused = std::io::Error::from(refused);
                if refused.kind() == std::io::ErrorKind::WouldBlock {
                    Err(Fault::thrown_as(
                        ThrownClass::Io,
                        format!(
                            "Core\\IO\\File::lock: another handle already holds the lock on {}",
                            path.as_text().unwrap_or("?")
                        ),
                    ))
                } else {
                    Err(nvs_runtime::capability::io_failure(
                        "Core\\IO\\File::lock",
                        Path::new(path.as_text().unwrap_or("?")),
                        &refused,
                    ))
                }
            }
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO\File::close(): void` — replacing `fclose`.
    ///
    /// Taking the descriptor out of the request's table *is* the close: nothing
    /// else holds one, so dropping it here is the syscall. A second `close`
    /// finds the slot empty and throws, rather than succeeding quietly — the
    /// key is never reused, so this can only be the same handle twice, and that
    /// is a bug worth naming.
    fn nvs_core_io_file_close(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "close")?;
        let file = ctx.take_open_file(key).ok_or_else(|| already_closed("close", &path))?;
        drop(file);
        Ok(Value::null())
    }
}

/// A whole file's octets, from behind [`nvs_runtime::capability::open_read`]'s
/// door — what [`nvs_core_io_read`] and [`nvs_core_io_read_text`] share, so
/// that the second is the first plus a conversion rather than a second reader
/// with its own idea of what a whole-file read is.
///
/// `member` is the fully-qualified spelling every refusal here names.
///
/// **Held to `[limits] max_output`**, through the pair
/// [`nvs_runtime::Ctx::intake_bound`] and [`nvs_runtime::Ctx::intake_breach`] —
/// the same two calls, in the same order, that `Core\Process::run` bounds a
/// child's capture with. That is the whole of the sharing: one directive, one
/// unit, one refusal, so a file and a child that are each too big to hold
/// answer a program the same way. A request under no ceiling reads the file
/// whole, exactly as this did before there was one.
///
/// # Errors
///
/// The door's catchable `RuntimeError` when `fs.read` does not cover `path`;
/// [`nvs_runtime::capability::io_failure`]'s `IOError` when the open or the read
/// itself fails; and `intake_breach`'s `RuntimeError` for a file past the
/// ceiling, which is a refusal to hold it rather than a failure to read it.
fn slurp(ctx: &mut nvs_runtime::Ctx, path: &Path, member: &str) -> Result<Vec<u8>, Fault> {
    let file = nvs_runtime::capability::open_read(ctx, path, member)?;
    let mut bytes = Vec::new();
    file.take(ctx.intake_bound())
        .read_to_end(&mut bytes)
        .map_err(|err| nvs_runtime::capability::io_failure(member, path, &err))?;
    if let Some(over) = ctx.intake_breach(member, bytes.len()) {
        return Err(over);
    }
    Ok(bytes)
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::write(string $path, string $content): void` — replacing
    /// `file_put_contents`.
    fn nvs_core_io_write(ctx, args: [2]) {
        let path = Path::new(text(&args[0], "write", "path")?);
        let content = text(&args[1], "write", "content")?;
        nvs_runtime::capability::write(ctx, path, content.as_bytes(), "Core\\IO::write")?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::append(string $path, string $content): void` — replacing
    /// `file_put_contents`'s `FILE_APPEND` flag, which is a member here for
    /// `rule:core-api/shape-rules` R6's reason: an option that changes *what a member does* is a
    /// second member.
    ///
    /// The handle door rather than [`nvs_runtime::capability::write`]'s
    /// finished effect, because this is the one whole-file write with no
    /// finished effect to hand across: [`Access::Append`] is `O_APPEND`, so
    /// the operating system places every write at the end **as it happens**,
    /// and a member that read the length first and wrote at it would race
    /// anything else appending to the same file for the bytes in between. The
    /// handle is dropped at the end of this call rather than filed against the
    /// request the way [`nvs_core_io_open`]'s is, so a program that appends
    /// twice has appended twice and holds nothing open between them.
    ///
    /// **What it spends:** one descriptor for the duration of the call, and no
    /// copy of the content — it is written straight out of the caller's
    /// argument.
    fn nvs_core_io_append(ctx, args: [2]) {
        let path = Path::new(text(&args[0], "append", "path")?);
        let content = text(&args[1], "append", "content")?;
        let mut file = nvs_runtime::capability::open(ctx, path, Access::Append, "Core\\IO::append")?;
        file.write_all(content.as_bytes())
            .map_err(|err| nvs_runtime::capability::io_failure("Core\\IO::append", path, &err))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::copy(string $from, string $to): void` — replacing `copy`.
    ///
    /// Two paths and two capabilities, both decided by
    /// [`nvs_runtime::capability::copy`] and neither by this: what is here is
    /// the argument reading, which is the whole of every body in this module.
    fn nvs_core_io_copy(ctx, args: [2]) {
        let from = Path::new(text(&args[0], "copy", "from")?);
        let to = Path::new(text(&args[1], "copy", "to")?);
        nvs_runtime::capability::copy(ctx, from, to, "Core\\IO::copy")?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::move(string $from, string $to): void` — replacing `rename`.
    ///
    /// `move` and not `rename`, because moving is what the caller is doing and
    /// renaming is how the operating system spells it: spec § 14 names the
    /// member for the effect, and the same call moves a file across a directory
    /// and gives it another name in place.
    fn nvs_core_io_move(ctx, args: [2]) {
        let from = Path::new(text(&args[0], "move", "from")?);
        let to = Path::new(text(&args[1], "move", "to")?);
        nvs_runtime::capability::rename(ctx, from, to, "Core\\IO::move")?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::makeDir(string $path): void` — replacing `mkdir`, whose
    /// `$recursive` parameter is this member's only behaviour.
    ///
    /// [`nvs_runtime::capability::create_dir`]'s doc owns both decisions —
    /// why the parents come with it where `removeDir` refuses to recurse, and
    /// why a directory that is already there is success.
    fn nvs_core_io_make_dir(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "makeDir", "path")?);
        nvs_runtime::capability::create_dir(ctx, path, "Core\\IO::makeDir")?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::writeStream(string $path, Iterable<bytes> $src, {max?, overwrite?}): void` —
    /// `rule:core-classes/io-write-stream`'s one way a stream reaches disk.
    ///
    /// Argument reading only; [`stream_to_disk`] is the member. The two
    /// options arrive already defaulted, so there is nothing here that decides
    /// what `overwrite` means when a caller says nothing — that is
    /// [`WRITE_STREAM_OPTIONS`], which is the one place it is written.
    fn nvs_core_io_write_stream(ctx, args: [4]) {
        let path = Path::new(text(&args[0], "writeStream", "path")?);
        // Both option reads below are unreachable from source, on `access_of`'s
        // reading: the rows in [`WRITE_STREAM_OPTIONS`] are `CoreTy::Uint` and
        // `CoreTy::Bool`, so `E0401` refuses anything else at the call site and
        // an absent option arrives as the row's own default. Fatal rather than
        // thrown, because what is left is a lowering bug.
        let max = args[2].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\IO::writeStream expected {:?} for its max, got tag {}",
                Tag::Uint,
                args[2].tag_byte()
            ))
        })?;
        // Unreachable from source for the reason above: `E0401`.
        let overwrite = args[3].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\IO::writeStream expected {:?} for its overwrite, got tag {}",
                Tag::Bool,
                args[3].tag_byte()
            ))
        })?;
        stream_to_disk(ctx, path, args[1], max, overwrite, WRITE_STREAM)?;
        Ok(Value::null())
    }
}

/// `Core\IO::writeStream`'s member name, in one place: five refusals name it
/// and one of them is raised from a closure two frames down.
///
/// It is *handed* to [`stream_to_disk`] rather than read there, because
/// `Core\Request\Part::saveTo` drives the same function under `rule:core-classes/io-write-stream`'s
/// delegation and a program that called `saveTo` must not be told about a
/// member it never named.
const WRITE_STREAM: &str = "Core\\IO::writeStream";

/// `rule:core-classes/io-write-stream`'s member: the door, the drive, and the two rules the ADR gives
/// this member of its own.
///
/// A plain function rather than the helper's body so that this module's own
/// cases can drive it with a source they built — what those cases assert is
/// the failure path, and reading four argument slots has nothing to do with
/// it.
///
/// **Nothing is materialised.** `nvs_runtime::sequence::for_each` hands over
/// one chunk at a time and this writes it and drops it, so what the call holds
/// is the largest single chunk rather than the length of the stream. That is
/// the whole difference from [`nvs_core_io_write`], which already has every
/// byte before it starts.
///
/// **A failure removes the partial file**, which the ADR asks for because a
/// truncated file the application believes it wrote is worse than a throw: the
/// caller learns from the error rather than from a later reader. The removal
/// is best effort and its own failure is swallowed — the fault that reaches
/// the program is the one that stopped the write, never a second one about the
/// cleanup. The handle is dropped first, because Windows refuses to remove a
/// file that is still open.
///
/// A sibling temporary renamed into place was the rejected alternative. It
/// makes the cleanup unnecessary, and buys that with two names where the
/// program chose one: the capability was granted for `path`, so the temporary
/// would need a grant of its own, and `overwrite: false` would become a check
/// followed by a rename — a window another process can create the file in.
/// `create_new` closes that window in the kernel, and the door's own docs say
/// so.
///
/// **`what` is the member the *program* called** — [`WRITE_STREAM`] here, and
/// `Core\Request\Part::saveTo` where `rule:core-classes/io-write-stream`'s delegation drives it. Every
/// refusal below quotes it, so the delegation is as invisible in a message as
/// the ADR makes it in the language.
///
/// **`src` is borrowed, not consumed.** `for_each` retains the cursor it drives
/// and releases that reference itself, so the reference the caller handed in is
/// still the caller's when this returns — by either arm.
///
/// # Errors
///
/// The door's catchable `RuntimeError` when `fs.write` does not cover `path`,
/// a `RuntimeError` when the stream runs past `max`, or
/// [`nvs_runtime::capability::io_failure`]'s `IOError` when the create or a
/// write fails. Whatever the source's own `advance`/`current` threw reaches
/// the caller unchanged.
pub(crate) fn stream_to_disk(
    ctx: &mut nvs_runtime::Ctx,
    path: &Path,
    src: Value,
    max: u64,
    overwrite: bool,
    what: &'static str,
) -> Result<(), Fault> {
    let mut file = nvs_runtime::capability::create(ctx, path, overwrite, what)?;
    let mut written = 0_u64;
    let mut write = |chunk: Value| {
        let outcome = write_chunk(&mut file, &mut written, max, chunk, path, what);
        #[expect(
            unsafe_code,
            reason = "`for_each`'s sink owns every value handed to it, on the \
                      failing call as much as on any other, and this one is \
                      written to the file rather than kept"
        )]
        unsafe {
            chunk.release();
        }
        outcome
    };
    let result = nvs_runtime::sequence::for_each(ctx, src, what, &mut write);
    // Before the removal below, and explicitly: Windows refuses to unlink a
    // file that is still open, so a handle left alive until the end of the
    // function would turn the cleanup into a no-op on one platform only.
    drop(file);
    if let Err(fault) = result {
        let _ = nvs_runtime::capability::remove_file(ctx, path, what);
        return Err(fault);
    }
    Ok(())
}

/// One chunk of [`stream_to_disk`]'s source, written and counted.
///
/// The ceiling is checked against the running total *before* the write rather
/// than after it, so a stream that would pass `max` never puts the byte that
/// passes it on disk — the file the cleanup removes is the one the earlier
/// chunks made, and nothing beyond the limit was ever there.
///
/// # Errors
///
/// A catchable `RuntimeError` when the total would pass `max`, or
/// [`nvs_runtime::capability::io_failure`]'s `IOError` when the write fails. A
/// `Fault::fatal` for a chunk that is not `bytes`, on [`text`]'s reading: the
/// row's parameter is an `Iterable<bytes>`, so anything else was refused at the
/// call site.
fn write_chunk<W: Write>(
    file: &mut W,
    written: &mut u64,
    max: u64,
    chunk: Value,
    path: &Path,
    what: &'static str,
) -> Result<(), Fault> {
    // Unreachable from source: the row's parameter is an `Iterable<bytes>`, so
    // `E0401` refuses a source of anything else at the call site.
    let octets = chunk.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{what} expected {:?} from its src, got tag {}",
            Tag::Bytes,
            chunk.tag_byte()
        ))
    })?;
    let total = written.saturating_add(octets.len() as u64);
    if total > max {
        return Err(Fault::thrown(format!(
            "{what}: {} would pass the `max` of {max} bytes at {total}",
            path.display()
        )));
    }
    *written = total;
    file.write_all(octets)
        .map_err(|err| nvs_runtime::capability::io_failure(what, path, &err))
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::exists(string $path): bool` — replacing `file_exists`.
    ///
    /// One member for a file and a directory alike, where PHP has three
    /// (`file_exists`, `is_file`, `is_dir`) and the difference between them is
    /// the first thing a caller of the wrong one gets wrong. The kind of the
    /// thing found is a separate question, answered by the members § 14 names
    /// beside this one, and asking it is never the same as asking whether the
    /// name resolves at all.
    ///
    /// The door refuses before it looks, so a `false` here means *not there*
    /// and never *not allowed* — the two are distinguishable to a program, and
    /// deliberately so: an ungranted path throws.
    fn nvs_core_io_exists(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "exists", "path")?);
        let found = nvs_runtime::capability::exists(ctx, path, "Core\\IO::exists")?;
        Ok(Value::bool(found))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::isReadable(string $path): bool` — replacing `is_readable`.
    ///
    /// **A guard and never a gate.** What it answers is a snapshot of the
    /// operating system's opinion, which anything may invalidate before the
    /// caller acts on it — so the shape a program should reach for is still a
    /// `read` inside a `try`, and this member is for the case where "cannot"
    /// is an ordinary branch rather than an error: a config file that may be
    /// absent, a directory a tool offers to use if it can.
    ///
    /// Absence is `false` here where `size` throws for it, and the difference
    /// is the question: a name that is not there is genuinely not readable,
    /// while it has no size for any answer to be about.
    fn nvs_core_io_is_readable(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "isReadable", "path")?);
        let allowed =
            nvs_runtime::capability::readable(ctx, path, "Core\\IO::isReadable")?;
        Ok(Value::bool(allowed))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::isWritable(string $path): bool` — replacing `is_writable`.
    ///
    /// [`nvs_core_io_is_readable`]'s doc is the home of what this pair is for,
    /// and the door [`nvs_runtime::capability::writable`] owns why the
    /// capability is `fs.write`.
    ///
    /// **A name that is not there is not writable, even where creating it
    /// would succeed.** The question is about the path as it is, not about
    /// what `write` would do with it — `is_writable` answers the same way, and
    /// matching it is priority 2. A program asking "can I create this file"
    /// is asking about the *directory*, and that is the path to hand over.
    fn nvs_core_io_is_writable(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "isWritable", "path")?);
        let allowed =
            nvs_runtime::capability::writable(ctx, path, "Core\\IO::isWritable")?;
        Ok(Value::bool(allowed))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::size(string $path): uint` — replacing `filesize`.
    ///
    /// The operating system's number rather than the length of anything the
    /// program is holding: that is the whole reason a member exists for a
    /// question a caller could otherwise answer by reading the file and
    /// measuring the string, and the two differ the moment anything else is
    /// writing.
    fn nvs_core_io_size(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "size", "path")?);
        let stat = nvs_runtime::capability::metadata(ctx, path, "Core\\IO::size")?;
        Ok(Value::uint(stat.len()))
    }
}

/// The modification time in `stat` as a `Core\Time\Instant`, or the `IOError`
/// saying why there is not one.
///
/// One function because [`nvs_core_io_modified_at`] and [`nvs_core_io_stat`]
/// ask the same thing of the same `stat` and must fail the same way: a
/// platform that does not record a modification time and one that recorded an
/// unnameable one are both `IOError`s here rather than a `null` either caller
/// would have to branch on. [`METADATA`]'s docs own why the second of those
/// fails the whole `stat` rather than only the member that would have read it.
///
/// # Errors
///
/// An `IOError` naming the member and the path, for either failure above.
fn modified_at(
    stat: &nvs_runtime::capability::Metadata,
    path: &Path,
    member: &str,
) -> Result<Value, Fault> {
    let at = stat
        .modified()
        .map_err(|err| nvs_runtime::capability::io_failure(member, path, &err))?;
    crate::time::instant_at_system_time(at).ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member} failed on {}: its modification time is outside the range a \
                 `Core\\Time\\Instant` can name",
                path.display()
            ),
        )
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::modifiedAt(string $path): Core\Time\Instant` — replacing
    /// `filemtime`.
    ///
    /// An `Instant` and not an epoch `int`, which is `rule:core-api/shape-rules` R12: `filemtime`
    /// hands back a number whose unit the caller has to remember, and every
    /// comparison against one is a chance to remember it wrong. What the type
    /// buys is that the answer can only be compared with another point on the
    /// timeline and can only be rendered through a zone the program names.
    fn nvs_core_io_modified_at(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "modifiedAt", "path")?);
        let stat = nvs_runtime::capability::metadata(ctx, path, "Core\\IO::modifiedAt")?;
        modified_at(&stat, path, "Core\\IO::modifiedAt")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::stat(string $path): Core\IO\Metadata` — replacing `stat`,
    /// `lstat` and the whole `file*` family of one-question members.
    ///
    /// **One syscall, four answers.** That is the only reason this member
    /// exists beside `size`, `modifiedAt`, `isFile` and `isDir`: a program
    /// asking two of those questions about one path pays for two `stat`s, and
    /// the two can disagree because anything else may have written between
    /// them. [`METADATA`]'s own docs own why that is not the two-spellings
    /// `rule:core-api/shape-rules` R17 forbids.
    ///
    /// The result is a **snapshot** and never a live view — the door hands
    /// back a whole `std::fs::Metadata` for exactly this, and its doc comment
    /// is the home of why a second door per field would have been a second
    /// syscall for the same permission.
    fn nvs_core_io_stat(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "stat", "path")?);
        let stat = nvs_runtime::capability::metadata(ctx, path, "Core\\IO::stat")?;
        let modified = modified_at(&stat, path, "Core\\IO::stat")?;
        Ok(crate::instance::build(
            &METADATA,
            [
                Value::uint(stat.len()),
                modified,
                Value::bool(stat.is_file()),
                Value::bool(stat.is_dir()),
            ],
        ))
    }
}

/// Slot `index` of the `Core\IO\Metadata` receiver in argument slot 0,
/// retained for the caller.
///
/// Every member of that class is this and nothing else, which is
/// [`METADATA`]'s decision written out: the questions were all answered by the
/// one `stat` that built the value, so a member here computes nothing and can
/// fail at nothing.
///
/// # Errors
///
/// The [`crate::instance::receiver`] fault a wrongly-tagged receiver is, which
/// compiled code cannot produce.
fn metadata_slot(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let object = crate::instance::receiver(args[0], &METADATA, member)?;
    let held = crate::instance::slot(object, index);
    #[expect(
        unsafe_code,
        reason = "the slot is owned by the receiver, which the argument slot holds a \
                  reference to for the length of the call, so the copy handed back to \
                  Novis code needs a reference of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `$m->size(): uint` — the byte count the `stat` behind this value read.
    fn nvs_core_io_metadata_size(_ctx, args: [1]) {
        metadata_slot(args, METADATA_SIZE_SLOT, "size")
    }
}

nvs_runtime::nvs_helper! {
    /// `$m->modifiedAt(): Core\Time\Instant` — the modification time the
    /// `stat` behind this value read, built when it was.
    fn nvs_core_io_metadata_modified_at(_ctx, args: [1]) {
        metadata_slot(args, METADATA_MODIFIED_AT_SLOT, "modifiedAt")
    }
}

nvs_runtime::nvs_helper! {
    /// `$m->isFile(): bool` — whether the path was a regular file.
    fn nvs_core_io_metadata_is_file(_ctx, args: [1]) {
        metadata_slot(args, METADATA_IS_FILE_SLOT, "isFile")
    }
}

nvs_runtime::nvs_helper! {
    /// `$m->isDir(): bool` — whether the path was a directory.
    fn nvs_core_io_metadata_is_dir(_ctx, args: [1]) {
        metadata_slot(args, METADATA_IS_DIR_SLOT, "isDir")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::isFile(string $path): bool` — replacing `is_file`.
    ///
    /// One `stat` and one question of it. The pair `isFile`/`isDir` partitions
    /// what [`nvs_core_io_exists`] answers `true` for — and does not cover it,
    /// because a socket, a device node and a named pipe are all names that
    /// exist and are neither. That is why this is not spelled as the negation
    /// of `isDir` anywhere: two members that each answer `false` for the same
    /// path is the honest shape, and one derived from the other would have to
    /// invent an answer for the third kind.
    ///
    /// Absence answers `false` rather than throwing, which is the door's
    /// choice and not this body's: `metadata_if_present` is the door precisely
    /// so that a missing name and an ungranted one stay distinguishable, the
    /// second still throwing.
    fn nvs_core_io_is_file(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "isFile", "path")?);
        let found = nvs_runtime::capability::metadata_if_present(ctx, path, "Core\\IO::isFile")?;
        Ok(Value::bool(found.is_some_and(|stat| stat.is_file())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::isDir(string $path): bool` — replacing `is_dir`.
    ///
    /// The other half of [`nvs_core_io_is_file`]'s partition, over the same
    /// door and the same `stat`; that member's doc comment owns why the two are
    /// written separately rather than one as the negation of the other.
    ///
    /// Symbolic links are followed, because `std::fs::metadata` follows them
    /// and this class has no member that does not: a program asking whether it
    /// may `list` a name wants the answer about what the name leads to, and the
    /// one member that cares where a link points is `within`, whose whole job
    /// is that it resolves first.
    fn nvs_core_io_is_dir(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "isDir", "path")?);
        let found = nvs_runtime::capability::metadata_if_present(ctx, path, "Core\\IO::isDir")?;
        Ok(Value::bool(found.is_some_and(|stat| stat.is_dir())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::list(string $path): array<string>` — replacing `scandir`,
    /// `glob` and the `opendir`/`readdir`/`closedir` sequence.
    ///
    /// **`.` and `..` are not entries.** Every `scandir` caller in every PHP
    /// codebase filters those two out, and a member that hands them over is
    /// handing over the one thing nobody wanted: `std::fs::read_dir` omits
    /// them already, so what this body does is decline to add them back.
    ///
    /// **A bare name per entry, not a path.** Joining it onto `$path` is the
    /// caller's step, and it is the caller's step on purpose — the join is
    /// where an entry name that arrived from outside becomes a path this
    /// program is about to open, and `within` is the member for that. A
    /// listing that answered whole paths would read as though the composition
    /// had already been checked.
    ///
    /// Nothing is sorted. The operating system's order is what a directory
    /// has, and `Core\Arr::sort` is one call for the caller who needs another.
    fn nvs_core_io_list(ctx, args: [1]) {
        const MEMBER: &str = "Core\\IO::list";

        let path = Path::new(text(&args[0], "list", "path")?);
        let entries = nvs_runtime::capability::read_dir(ctx, path, MEMBER)?;
        let mut names = NvsArray::new();
        for entry in entries {
            // A failure partway through the walk is this member's and not the
            // caller's, unlike the door's own reading: `list` promised the
            // whole directory, so half of it is not an answer.
            let entry =
                entry.map_err(|err| nvs_runtime::capability::io_failure(MEMBER, path, &err))?;
            // Lossy only where a name is not UTF-8, which a Novis `string`
            // cannot hold at all (`rule:types/bytes`) — the same reading as
            // [`nvs_core_io_temporary_dir`]'s.
            names.append(Value::str(NvsStr::new(
                entry.file_name().to_string_lossy().as_bytes(),
            )));
        }
        Ok(Value::array(names))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::walk(string $path): Iterable<string>` — replacing
    /// `RecursiveDirectoryIterator`, `RecursiveIteratorIterator` and the
    /// recursive half of `glob`.
    ///
    /// [`nvs_core_io_list`] over one directory, and then over every directory
    /// it found: the door, the entry names and the treatment of `.` and `..`
    /// are all that member's. [`WALK`]'s own docs own the four decisions that
    /// are this member's alone — a tree rather than a directory, a relative
    /// path rather than a bare name, a link that is an entry and not a
    /// descent, and a capability asked per directory.
    ///
    /// The frontier is a queue and not recursion, so a tree deep enough to
    /// exhaust the native stack is a slow answer rather than a crash. It holds
    /// one entry per directory *not yet read*, which is bounded by the tree's
    /// width and not by the count this member is building.
    fn nvs_core_io_walk(ctx, args: [1]) {
        const MEMBER: &str = "Core\\IO::walk";

        let root = Path::new(text(&args[0], "walk", "path")?);
        let mut found = NvsArray::new();
        // `(directory, its path relative to the root)`. The root's own prefix
        // is empty, which is what makes every entry below it relative to the
        // path the caller named.
        let mut frontier: VecDeque<(PathBuf, PathBuf)> =
            VecDeque::from([(root.to_path_buf(), PathBuf::new())]);
        while let Some((directory, prefix)) = frontier.pop_front() {
            let entries = nvs_runtime::capability::read_dir(ctx, &directory, MEMBER)?;
            for entry in entries {
                // A failure partway through is this member's and not the
                // caller's, on `list`'s own reading: `walk` promised the tree,
                // so part of it is not an answer.
                let entry = entry
                    .map_err(|err| nvs_runtime::capability::io_failure(MEMBER, &directory, &err))?;
                let descend = entry
                    .file_type()
                    .map_err(|err| nvs_runtime::capability::io_failure(MEMBER, &directory, &err))?
                    .is_dir();
                let relative = prefix.join(entry.file_name());
                // Lossy only where a name is not UTF-8, exactly as `list` is.
                found.append(Value::str(NvsStr::new(
                    relative.to_string_lossy().as_bytes(),
                )));
                if descend {
                    frontier.push_back((entry.path(), relative));
                }
            }
        }
        Ok(crate::instance::build(&WALK, [Value::array(found)]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<string>::iterate(): Iterator<string>` — a cursor over the
    /// entries this value is already holding.
    ///
    /// [`nvs_core_io_lines_iterate`]'s body over [`WALK`]'s slot, and that
    /// member's doc comment owns both halves of the convention it keeps: the
    /// receiver is transferred rather than borrowed, and the cursor shares the
    /// list because no member can change it.
    fn nvs_core_io_walk_iterate(_ctx, args: [1]) {
        let cursor = held_strings(args[0], &WALK, WALK_SLOT, nvs_runtime::sequence::ITERATE)
            .map(crate::cursor::over);
        crate::cursor::consume(args[0]);
        cursor
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::canonicalize(string $path): string` — replacing `realpath`.
    ///
    /// **It resolves and it does not prove.** `within` is the launderer and
    /// this is not, which is why the row's parameter is a sink and its answer
    /// is an ordinary `string` rather than `rule:security/launderers-are-sink-named`'s plain one. Reaching
    /// for this member when the question was containment is the mistake that
    /// ADR exists to prevent, and the two rows sit next to each other in the
    /// registry so that the difference is read rather than remembered.
    ///
    /// The door is [`nvs_runtime::capability::resolve_existing`] rather than
    /// the `capability::canonicalize` [`nvs_core_io_within`] uses, and the two
    /// differ in exactly one thing: this one refuses a path that is not there,
    /// where that one answers by pinning the deepest existing ancestor. The
    /// *walk* is the same walk on purpose — this class would otherwise have two
    /// resolving members answering two different spellings of one file, which
    /// is the wrong answer for the one use this member has that
    /// `Core\Path::normalize` cannot serve: comparing two resolutions for
    /// equality. The door's own doc comment is the home of that reasoning.
    fn nvs_core_io_canonicalize(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "canonicalize", "path")?);
        let resolved =
            nvs_runtime::capability::resolve_existing(ctx, path, "Core\\IO::canonicalize")?;
        Ok(Value::str(NvsStr::new(
            resolved.to_string_lossy().as_bytes(),
        )))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::remove(string $path): void` — replacing `unlink`.
    ///
    /// A file only. `removeDir` is the other half, and the split is the
    /// operating system's own rather than this class's invention: the two are
    /// different syscalls with different failure modes, and one member covering
    /// both would have to guess which the caller meant for a name that is not
    /// there at all.
    fn nvs_core_io_remove(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "remove", "path")?);
        nvs_runtime::capability::remove_file(ctx, path, "Core\\IO::remove")?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::removeDir(string $path): void` — replacing `rmdir`.
    ///
    /// Empty directories only; the door's own doc comment carries why there is
    /// no recursive form, which is a security decision and not an omission.
    fn nvs_core_io_remove_dir(ctx, args: [1]) {
        let path = Path::new(text(&args[0], "removeDir", "path")?);
        nvs_runtime::capability::remove_dir(ctx, path, "Core\\IO::removeDir")?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::temporaryDir(): string` — replacing `sys_get_temp_dir`,
    /// `tempnam` and `tmpfile`, none of which survives on its own.
    ///
    /// One member instead of three, because the two PHP splits into are the
    /// bug: `sys_get_temp_dir` hands back a directory everything else on the
    /// machine is also writing into, and every use of it then races to pick a
    /// name nobody has taken. What a program wants is a place of its own, and
    /// the door creates it rather than proposing it.
    ///
    /// A directory rather than a file, for the same reason: a program that
    /// needs one temporary file needs somewhere to put the second one, and a
    /// directory it owns is removable in one call once both are gone.
    ///
    /// Removable, and removed without being asked:
    /// [`nvs_runtime::capability::temp_dir`] records what it created on the
    /// context and `rule:core-classes/temporary-dir-sweep`'s sweep deletes it when the script ends. So this member has no
    /// counterpart to call, no persist option to pass and nothing for the
    /// program to remember — which is why there is no `temporaryFile` either:
    /// a file that must outlive its script is storage.
    fn nvs_core_io_temporary_dir(ctx, _args: [0]) {
        let made = nvs_runtime::capability::temp_dir(ctx, "Core\\IO::temporaryDir")?;
        // Lossy only where a path is not UTF-8, which a Novis `string` cannot
        // hold at all (`rule:types/bytes`): the alternative is refusing to answer for a
        // temporary root this process did not choose.
        Ok(Value::str(NvsStr::new(made.to_string_lossy().as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::within(string $base, tainted string $path): string` — `rule:security/launderers-are-sink-named`
    /// 's path-traversal launderer, and the one member of this class that
    /// removes a qualifier rather than refusing one.
    ///
    /// **It resolves and *then* proves.** That order is the whole member.
    /// `Core\Path::normalize` collapses `..` lexically, which is the answer a
    /// symlink makes wrong — `base/link/../..` is outside the base exactly when
    /// `link` points somewhere else, and no amount of string algebra can see
    /// that. So containment is decided against what the operating system says
    /// the name resolves to and never against the string that came in.
    ///
    /// **No operating system call happens here.**
    /// `nvs_runtime::capability::canonicalize` is the door, asked once per
    /// path, and what this body owns is one comparison: `Path::starts_with`,
    /// which compares whole components and so does not accept `/base-more`
    /// under `/base` the way a byte-wise prefix test would.
    ///
    /// The refusal names the base and the caller's own argument and never the
    /// path it resolved to. A symlink under the base pointing out of it is
    /// refused, and saying where it pointed would answer with one message the
    /// question the refusal exists to prevent being asked.
    fn nvs_core_io_within(ctx, args: [2]) {
        const MEMBER: &str = "Core\\IO::within";

        let given = Path::new(text(&args[0], "within", "base")?);
        let candidate = text(&args[1], "within", "path")?;
        let base = nvs_runtime::capability::canonicalize(ctx, given, MEMBER)?;
        let resolved =
            nvs_runtime::capability::canonicalize(ctx, &base.join(candidate), MEMBER)?;
        if !resolved.starts_with(&base) {
            return Err(Fault::thrown(format!(
                "a path must stay inside the base it is resolved against: {candidate:?} does not \
                 stay inside {} ({MEMBER})",
                base.display()
            )));
        }
        Ok(Value::str(NvsStr::new(
            resolved.to_string_lossy().as_bytes(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context granting `fs.write` everywhere and nothing else.
    ///
    /// Everywhere rather than under a root because what these cases assert is
    /// on the far side of the door: `rule:security/capability-check-at-the-door`'s own suite
    /// (`crates/nvs-stdlib/tests/capability.rs`) is where the grant decides
    /// anything, and a root here would only add a way for them to fail for a
    /// reason they are not about.
    fn writing() -> nvs_runtime::Ctx {
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_config(std::sync::Arc::new(nvs_config::Snapshot {
            config: nvs_config::tree::Config {
                capabilities: Some(nvs_config::tree::Capabilities {
                    fs: Some(nvs_config::tree::CapFs {
                        read: None,
                        write: Some(nvs_config::tree::Setting::Bool(true)),
                    }),
                    ..nvs_config::tree::Capabilities::default()
                }),
                ..nvs_config::tree::Config::default()
            },
            ..nvs_config::Snapshot::default()
        }));
        ctx
    }

    /// `chunks` as an `array<bytes>` — one of `rule:iteration/foreach-subjects`'s three iterable
    /// shapes, and the one a case can build with no compiler in front of it.
    /// The caller owns what this answers with.
    fn source(chunks: &[&[u8]]) -> Value {
        let mut array = NvsArray::new();
        for chunk in chunks {
            array.append(Value::bytes(NvsStr::new(chunk)));
        }
        Value::array(array)
    }

    /// Releases what [`source`] built, so a case leaves the counters where it
    /// found them.
    fn discard(src: Value) {
        #[expect(
            unsafe_code,
            reason = "the case owns the one reference `source` produced and \
                      has handed it nowhere: `for_each` retains and releases \
                      each element around the sink"
        )]
        unsafe {
            src.release();
        }
    }

    /// A path under the host's temporary directory that one case owns, with
    /// anything a previous run left there removed.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("nvs-write-stream");
        std::fs::create_dir_all(&dir).expect("a temporary directory the tests own");
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn write_stream_defaults_to_no_overwrite() {
        // `rule:core-classes/io-write-stream`'s first rule, in both places it has to hold. The row's
        // default alone would pass over a body that never read the option, and
        // the behaviour alone would pass over a row whose default was `true` —
        // it is the pair that says a caller who writes nothing is safe.
        let overwrite = WRITE_STREAM_OPTIONS
            .iter()
            .find(|option| option.name == "overwrite")
            .expect("§ 4's second option is a row");
        assert!(
            matches!(overwrite.default, Const::Bool(false)),
            "a caller who says nothing must not replace what is there"
        );

        let mut ctx = writing();
        let path = scratch("existing.bin");
        std::fs::write(&path, b"already here").expect("the file the write must not replace");
        let src = source(&[b"one", b"two"]);

        let refused = stream_to_disk(&mut ctx, &path, src, u64::MAX, false, WRITE_STREAM)
            .expect_err("the default refuses a destination that already exists");
        let Fault::Thrown(_, message) = refused else {
            panic!("a destination that is already there is catchable, not a fatal");
        };
        assert!(
            message.contains("writeStream"),
            "the refusal names the member that raised it: {message}"
        );
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            b"already here",
            "and the refusal happened before anything was written"
        );

        // The same call with the rule turned off, which is what makes the
        // refusal above a *default* rather than the only thing this member does.
        stream_to_disk(&mut ctx, &path, src, u64::MAX, true, WRITE_STREAM)
            .expect("`overwrite: true` replaces what is there");
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            b"onetwo",
            "the chunks arrive in order and are the whole content"
        );

        discard(src);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_write_stream_that_fails_midway_removes_the_partial_file() {
        // `rule:core-classes/io-write-stream`'s second rule. `max` is the failure the case reaches
        // for because it is the one this member can be made to raise part-way
        // through on any host and without a source that throws: two chunks
        // land, the third passes the ceiling, and the claim is that the two
        // that landed are gone.
        let mut ctx = writing();
        let path = scratch("partial.bin");
        let src = source(&[b"aaaa", b"bbbb", b"cccc"]);

        let refused = stream_to_disk(&mut ctx, &path, src, 10, false, WRITE_STREAM)
            .expect_err("twelve bytes do not fit under a ceiling of ten");
        let Fault::Thrown(_, message) = refused else {
            panic!("passing `max` is the program's own limit and so is catchable");
        };
        assert!(
            message.contains("max"),
            "the throw names the ceiling it passed: {message}"
        );
        assert!(
            !path.exists(),
            "a write that stopped part-way leaves nothing behind: {}",
            path.display()
        );

        // And the removal is the failure path and not something the member
        // does every time: the same stream under a ceiling that fits is on
        // disk afterwards, whole.
        stream_to_disk(&mut ctx, &path, src, 12, false, WRITE_STREAM)
            .expect("twelve bytes fit under twelve");
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            b"aaaabbbbcccc",
            "the bound is asserted on both sides, and the accepted side is complete"
        );

        discard(src);
        let _ = std::fs::remove_file(&path);
    }

    /// A context granting `fs.read` everywhere under one `[limits] max_output`, written as the
    /// TOML an operator writes rather than as the tree it parses into — the ceiling is what
    /// these cases are about, so the spelling that arms it is worth going through.
    ///
    /// Everywhere rather than under a root for [`writing`]'s reason, unchanged.
    fn reading(ceiling: &str) -> nvs_runtime::Ctx {
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_config(crate::tests::granting(&format!(
            "[capabilities.fs]\nread = true\n\n[limits]\nmax_output = \"{ceiling}\"\n"
        )));
        ctx
    }

    /// `Core\IO::read(path)` driven as a program drives it, answering how many bytes came back
    /// or the message the refusal left in `ctx`.
    fn read_under(ctx: &mut nvs_runtime::Ctx, path: &std::path::Path) -> Result<usize, String> {
        let written = path
            .to_str()
            .expect("a scratch path this suite spelled itself");
        let args = [Value::str(NvsStr::new(written.as_bytes()))];
        let answered = nvs_runtime::call(nvs_core_io_read, ctx, &args);
        let refusal = ctx.take_pending().map(std::borrow::Cow::into_owned);
        for argument in args {
            #[expect(unsafe_code, reason = "the list holds the one reference it built")]
            unsafe {
                argument.release();
            }
        }
        match answered {
            Ok(value) => {
                let read = value.as_text().expect("`read` answers a string").len();
                #[expect(unsafe_code, reason = "the member handed back a reference of its own")]
                unsafe {
                    value.release();
                }
                Ok(read)
            }
            Err(_) => Err(refusal.expect("a non-zero status leaves its message in the context")),
        }
    }

    /// `rule:core-classes/process-run`'s reuse of `[limits] max_output`, read at *this* class's
    /// buffer — all three halves of what "the same directive and the same signature" is.
    ///
    /// The directive first: one key arms both the response ceiling and the per-call one, so an
    /// operator who raises what a read may hold cannot be raising a second number they never
    /// wrote. Then the bound on both sides over one file, since a member that refused everything
    /// passes the refusing half alone. Then the signature: both members refuse out of
    /// `Ctx::intake_breach`, so their two messages differ in the member they name and in nothing
    /// else, and a member that grew a message of its own fails here while still refusing.
    #[test]
    fn core_io_read_is_bounded_by_the_same_directive_and_the_same_signature() {
        let path = scratch("bounded-by-max-output.txt");
        std::fs::write(&path, vec![b'x'; 4096]).expect("the file the ceilings are chosen around");

        let mut tight = reading("64");
        assert_eq!(
            tight.intake_limit(),
            tight.output_limit(),
            "the per-call ceiling and the response ceiling came off different keys, so an \
             operator now has two numbers to keep in step"
        );
        assert_eq!(
            tight.intake_limit(),
            64,
            "`[limits] max_output` never arrived"
        );

        let refused = read_under(&mut tight, &path).expect_err("4096 bytes do not fit under 64");
        assert!(
            refused.contains("max_output") && refused.contains(r"Core\IO::read"),
            "the refusal names neither the directive an operator would raise nor the member that \
             hit it: {refused}"
        );

        let mut roomy = reading("8M");
        assert_eq!(
            read_under(&mut roomy, &path).expect("4096 bytes fit under 8M"),
            4096,
            "the accepted side is the whole file, so the ceiling truncates nothing on its way \
             past"
        );

        let (Some(Fault::Thrown(here, mine)), Some(Fault::Thrown(there, theirs))) = (
            tight.intake_breach(r"Core\IO::read", 4096),
            tight.intake_breach(r"Core\Process::run", 4096),
        ) else {
            panic!(
                "a read past the ceiling is catchable, not a fatal — it is a refusal to hold \
                    a buffer and not a limit the request exceeded"
            );
        };
        assert_eq!(
            here, there,
            "the two members refuse as two different classes"
        );
        assert_eq!(
            mine.replace(r"Core\IO::read", "<member>"),
            theirs.replace(r"Core\Process::run", "<member>"),
            "a file too large to hold and a child too chatty to capture read differently to a \
             program, though one pair of methods answers both"
        );

        let _ = std::fs::remove_file(&path);
    }
}
