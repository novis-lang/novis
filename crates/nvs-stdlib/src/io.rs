//! `Core\IO` — the first `Core` class that reaches the operating system, and
//! so the first one written entirely behind
//! [ADR 0118](../../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md)
//! §§ 2-3's doors.
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
//! one launderer that gets past them.** ADR 0088 § 1's table classifies a
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

use std::io::{Read, Write};
use std::path::Path;

use nvs_runtime::capability::Access;
use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, Value};

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
    methods: &[
        CoreMethod {
            name: "read",
            names: &["path"],
            // A sink in the path, like every other path in this class: ADR 0088
            // § 1 classifies a path component as an instruction, because `..`
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
            // The path is a sink and `$content` is not: ADR 0088 § 1's table
            // puts a file's *contents* on the data side, so bytes that arrived
            // from outside may be written to a path this program chose.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_write",
            doc: Some(&WRITE_DOC),
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
            name: "remove",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_io_remove",
            doc: Some(&REMOVE_DOC),
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
            name: "temporaryDir",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Text(Qual::Neutral),
            symbol: "nvs_core_io_temporary_dir",
            doc: Some(&TEMPORARY_DIR_DOC),
        },
        CoreMethod {
            name: "within",
            names: &["base", "path"],
            // The class's one laundering row, and the two marks are different
            // on purpose. `$base` is a path this program chose, so it is a
            // sink like every other; `$path` is the untrusted half the member
            // exists to accept, and ADR 0024 § 3 makes a launderer's answer the
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
            // charset is the one trailing options shape ADR 0063 R3 allows —
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
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\IO::readText`'s `{charset?}` — the one option, and the whole of what
/// separates that member from [`nvs_core_io_read`].
///
/// UTF-8 by default because that is what Novis text already is
/// ([ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) § 1): a caller who
/// says nothing gets the decode that is the identity on a file written the way
/// the language spells strings, and every other encoding has to name itself. A
/// default of "whatever the file looks like" is the guess this member exists to
/// refuse.
const READ_TEXT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "charset",
    ty: CoreTy::Enum(crate::encoding::CHARSET_NAME),
    default: Const::EnumCase(crate::encoding::CHARSET_NAME, "Utf8"),
}];

/// `Core\IO::read`'s reference card — ADR 0117.
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
                   the capability in the spelling `nvs.toml` grants it under.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — the file does \
                   not exist, is a directory, or could not be read.",
        },
    ],
};

/// `Core\IO::write`'s reference card — ADR 0117.
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
            desc: "The bytes to write. They become the file's entire content; there is no \
                   append in this signature.",
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

/// `Core\IO::exists`'s reference card — ADR 0117.
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

/// `Core\IO::size`'s reference card — ADR 0117.
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

/// `Core\IO::remove`'s reference card — ADR 0117.
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

/// `Core\IO::removeDir`'s reference card — ADR 0117.
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

/// `Core\IO::temporaryDir`'s reference card — ADR 0117.
const TEMPORARY_DIR_DOC: MethodDoc = MethodDoc {
    short: "Creates a new, empty, private directory under the system temporary root and answers its \
            path — `sys_get_temp_dir` and `tempnam` in one member, and the directory is made rather \
            than merely named, so there is no window between choosing a name and owning it. Needs \
            the `fs.write` capability **for the path it creates**: the name is chosen first and \
            asked about second, so a configuration granting only the working directory does not \
            reach the temporary root.",
    params: &[],
    ret: "The absolute path of a directory that exists, holds nothing, and belongs to this process. \
          Removing it is the program's own job — `remove` each entry, then `removeDir` — because a \
          runtime that swept it would be deciding the lifetime of data it knows nothing about.",
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

/// `Core\IO::within`'s reference card — ADR 0117.
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

/// `Core\IO::readText`'s reference card — ADR 0117.
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

/// `Core\IO::lines`'s reference card — ADR 0117.
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

/// `Core\IO::open`'s reference card — ADR 0117.
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

/// [`FILE_MODE`]'s reference card — ADR 0117.
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
/// # Three members so far, and the rest of § 14's roster is owed
///
/// `read`, `write` and `close`. The spec also names `readLine`, `seek`, `tell`,
/// `truncate`, `flush` and `lock`, and each is a signature over the same slot
/// with nothing new to decide — they are absent because nothing has needed one
/// yet, which is the same test every row in [`crate::registry::ENUMS`] passes.
/// `Core\IO::stdin`/`stdout`/`stderr` answer this class too and are owed with
/// them.
pub(crate) const FILE: CoreClass = CoreClass {
    name: FILE_NAME,
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

/// `Core\IO\File::read`'s reference card — ADR 0117.
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

/// `Core\IO\File::write`'s reference card — ADR 0117.
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

/// `Core\IO\File::close`'s reference card — ADR 0117.
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
    methods: &[],
    instance: &[],
    slots: &["lines"],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_io_read" => (nvs_core_io_read as *const ()).cast(),
        "nvs_core_io_read_text" => (nvs_core_io_read_text as *const ()).cast(),
        "nvs_core_io_write" => (nvs_core_io_write as *const ()).cast(),
        "nvs_core_io_exists" => (nvs_core_io_exists as *const ()).cast(),
        "nvs_core_io_size" => (nvs_core_io_size as *const ()).cast(),
        "nvs_core_io_remove" => (nvs_core_io_remove as *const ()).cast(),
        "nvs_core_io_remove_dir" => (nvs_core_io_remove_dir as *const ()).cast(),
        "nvs_core_io_temporary_dir" => (nvs_core_io_temporary_dir as *const ()).cast(),
        "nvs_core_io_within" => (nvs_core_io_within as *const ()).cast(),
        "nvs_core_io_lines" => (nvs_core_io_lines as *const ()).cast(),
        "nvs_core_io_open" => (nvs_core_io_open as *const ()).cast(),
        "nvs_core_io_file_read" => (nvs_core_io_file_read as *const ()).cast(),
        "nvs_core_io_file_write" => (nvs_core_io_file_write as *const ()).cast(),
        "nvs_core_io_file_close" => (nvs_core_io_file_close as *const ()).cast(),
        LINES_ITERATE_SYMBOL => (nvs_core_io_lines_iterate as *const ()).cast(),
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
    /// The whole file into one buffer, with no size ceiling of its own: what
    /// bounds it is the request's memory limit, which a buffer this size is
    /// charged against like any other allocation. A second ceiling here would
    /// be a number an operator has to keep in step with that one.
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
        let cursor = held_lines(args[0], nvs_runtime::sequence::ITERATE).map(crate::cursor::over);
        crate::cursor::consume(args[0]);
        cursor
    }
}

/// The `array<string>` a [`LINES`] receiver holds, **retained** — the caller
/// takes over the reference this answers with.
///
/// # Errors
///
/// A `Fault::fatal` if the receiver is not one of this class's instances, on
/// the same reading as [`text`]: the signature was checked at compile time and
/// the slot is written by [`nvs_core_io_lines`] and by nothing else, so either
/// mismatch is a bug in this crate rather than something a program can reach.
fn held_lines(value: Value, member: &str) -> Result<NvsArray, Fault> {
    let receiver = crate::instance::receiver(value, &LINES, member)?;
    let held = crate::instance::slot(receiver, LINES_SLOT);
    let ptr = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{LINES_NAME}::{member} expected {:?} in its `{}` slot, got tag {}",
            Tag::Array,
            LINES.slots[LINES_SLOT],
            held.tag_byte()
        ))
    })?;
    let borrowed = crate::arr::borrowed(ptr);
    Ok((*borrowed).clone())
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

/// The key and the path a [`FILE`] receiver holds — everything its three
/// members read out of their receiver, so that none of them spells the slot
/// indices itself.
///
/// The path is **borrowed** from the receiver, which the caller owns for the
/// length of the call.
///
/// # Errors
///
/// A `Fault::fatal` if the receiver is not one of this class's instances or a
/// slot holds the wrong tag, on [`held_lines`]' reading: both slots are written
/// by [`nvs_core_io_open`] and by nothing else.
fn handle_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
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
/// which is a mistake in the program (ADR 0020 § 2's split).
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
/// `member` is the fully-qualified spelling both refusals name.
///
/// # Errors
///
/// The door's catchable `RuntimeError` when `fs.read` does not cover `path`, or
/// [`nvs_runtime::capability::io_failure`]'s `IOError` when the open or the
/// read itself fails.
fn slurp(ctx: &mut nvs_runtime::Ctx, path: &Path, member: &str) -> Result<Vec<u8>, Fault> {
    let mut file = nvs_runtime::capability::open_read(ctx, path, member)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|err| nvs_runtime::capability::io_failure(member, path, &err))?;
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
    fn nvs_core_io_temporary_dir(ctx, _args: [0]) {
        let made = nvs_runtime::capability::temp_dir(ctx, "Core\\IO::temporaryDir")?;
        // Lossy only where a path is not UTF-8, which a Novis `string` cannot
        // hold at all (ADR 0009): the alternative is refusing to answer for a
        // temporary root this process did not choose.
        Ok(Value::str(NvsStr::new(made.to_string_lossy().as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\IO::within(string $base, tainted string $path): string` — ADR 0024
    /// § 3's path-traversal launderer, and the one member of this class that
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
