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
//! whole-file read wants the thing it can concatenate. A member for the other
//! half — a bounded read, a stream — is a later signature over the same door,
//! which is why `open_read` hands back a handle rather than a `Vec`.

use std::io::Read;
use std::path::Path;

use nvs_runtime::{Fault, NvsStr, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

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
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

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

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_io_read" => (nvs_core_io_read as *const ()).cast(),
        "nvs_core_io_write" => (nvs_core_io_write as *const ()).cast(),
        "nvs_core_io_exists" => (nvs_core_io_exists as *const ()).cast(),
        "nvs_core_io_size" => (nvs_core_io_size as *const ()).cast(),
        "nvs_core_io_remove" => (nvs_core_io_remove as *const ()).cast(),
        "nvs_core_io_remove_dir" => (nvs_core_io_remove_dir as *const ()).cast(),
        "nvs_core_io_temporary_dir" => (nvs_core_io_temporary_dir as *const ()).cast(),
        "nvs_core_io_within" => (nvs_core_io_within as *const ()).cast(),
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
        let mut file = nvs_runtime::capability::open_read(ctx, path, "Core\\IO::read")?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|err| nvs_runtime::capability::io_failure("Core\\IO::read", path, &err))?;
        Ok(Value::str(NvsStr::new(&bytes)))
    }
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
