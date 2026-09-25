//! `Core\Storage` — `rule:programs/framework-core-half`'s
//! object storage: four members that put, get, delete and enumerate **named objects on a disk an
//! operator configured**, over the `fs.*` capabilities `rule:core-api/tier-roster` already grants and no capability of its own.
//!
//! `rule:programs/framework-core-half`'s row is one sentence — "local-filesystem object storage over `rule:core-api/tier-placement`'s existing
//! `fs.*` capabilities. Remote backends (S3 and friends) are a package or an extension, never
//! Core" — and the decisions it does not write are below.
//!
//! # It declares no capability, and that is the design rather than an omission
//!
//! Every other class in this stage's half of the framework brought a `Cap` with it:
//! [`crate::mail`] brought `mail.send`, [`crate::cache`]'s shared tier brought `cache.shared`. This
//! one brings nothing. A `storage.read` would be a *second* answer to a question `fs.read`
//! already answers about the same byte on the same disk — an operator who granted
//! `fs.write = ["./var/objects"]` has said everything there is to say about what a program may
//! write there, and a second grant beside it could only disagree. Two grants over one door is the
//! shape where a deployment is tightened in one of them and stays open through the other.
//!
//! So the door is [`nvs_runtime::capability`]'s own `create`, `open_read`, `exists` and
//! `remove_file`, exactly as [`crate::io`] reaches them, and the resolved path is what the check
//! is made against. `storage_over_local_disk_is_gated_on_the_same_fs_capability` is that claim
//! asserted over the registry, because it is the kind of rule a later member can break while
//! every behavioural test stays green.
//!
//! # A key is not a path, and this is what keeps the grant exact
//!
//! `$key` names an object; it does not name a file. It is one flat segment of ASCII letters,
//! digits, `.`, `-` and `_`, at most [`MAX_KEY`] bytes, not beginning with a `.` — and every
//! other spelling is **refused** rather than escaped or trimmed
//! (`rule:errors/ambiguous-input-refused`). A `/`,
//! a `\`, a `..` and a NUL are each that refusal.
//!
//! The flatness is the load-bearing half. Every path this class can construct is
//! `<root>/<one segment>`, so a grant of `fs.write` over the disk's root covers precisely the set
//! of objects and nothing under it — there is no key that names a directory, and therefore none
//! that asks this class to *create* one the operator never granted. A key space with separators
//! in it would need a `mkdir` door, and a `mkdir` door is a program building structure inside a
//! grant that was written about files. Object stores are flat underneath anyway: a `/` in an S3
//! key is a listing convention, not a container, and a package that wants that convention over
//! this class encodes it in the key it hands us.
//!
//! Because the grammar refuses rather than repairs, `$key` accepts a `tainted` argument freely —
//! it is data in `rule:security/log-is-not-a-sink`
//! 's sense, and storing a file a user named is the ordinary case. `$disk` is the opposite and
//! is the one [`Qual::Sink`] here, for [`crate::mail`]'s `$endpoint` reason: it selects between
//! deployments an operator wrote, so it belongs at the call site and never in input. *Which*
//! objects a caller may reach among those the grant covers is authorization, which is the
//! application's and not this class's.
//!
//! # What `list` answers over, in one order, with the prefix as an option
//!
//! Three decisions, none of them written in `rule:programs/framework-core-half`, which says only that the storage is
//! local.
//!
//! **Every key it answers is one [`get`](CLASS) hands octets back for.** The root is an operator's
//! directory and may hold whatever an operator put there — a `README`, a stray `.DS_Store`, a
//! subdirectory no key of this class could have made, a symlink to somewhere else entirely. None
//! of those is an object, so none of them is listed: an entry is a key here only when it is a
//! **regular file** whose name [`is_key`] accepts. That is an *omission* rather than a refusal,
//! and the difference is who wrote the bytes — `rule:errors/ambiguous-input-refused` refuses ambiguous **input**, and a disk's
//! own contents are not the caller's input. The alternative is a disk that one stray file makes
//! permanently unlistable.
//!
//! The one direction this does not run is the symlink: `get` follows one and the listing does not,
//! so a linked object is readable by a caller that already knows its key and is not enumerable.
//! That is the safe half of the asymmetry — what a link names is not on this disk, and listing it
//! would make the answer depend on a path the operator's grant was never written about.
//!
//! **The answer is sorted, byte-ascending.** `readdir` order is the filesystem's own and is stable
//! across neither hosts nor runs, so an unsorted answer is a result no program may depend on and
//! no test can freeze. The sort is over names already in hand, and it buys every caller a total
//! order each of them would otherwise have to impose.
//!
//! **`prefix` is an option, and it is empty or itself an object key.** Being optional, `rule:core-api/shape-rules` R3
//! puts it in the one trailing shape rather than in a second positional slot. Every non-empty
//! prefix of a key *is* a key — the grammar bounds length from above only, and no byte it admits
//! is one a longer name may not carry — so any other spelling names nothing this disk can hold,
//! and it is refused with the key grammar's own sentence rather than answered with an empty array
//! (`rule:errors/ambiguous-input-refused` again). Filtering here rather than at the call site is what keeps a large disk's
//! answer proportional to what was asked about.
//!
//! # There is no `exists`, and R17 is why rather than an omission
//!
//! [`get`](CLASS) answers absence as `null` under `rule:core-api/shape-rules` R4 — failure throws
//! and absence is `?T` — so a second member
//! asking the same question would be the one operation reachable two ways that R17 forbids. The
//! cost is real and named: `get` on a large object reads it to answer a question about its
//! existence. [`list`](CLASS) under a `prefix` is the cheap half of that — it reads the directory
//! and never the object — but it is a listing rather than a test, so a caller with one key in mind
//! compares the element it got back.

use std::io::{Read as _, Write as _};
use std::path::PathBuf;

use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, as `registry::CAPABILITIES` and every refusal both spell it.
pub(crate) const NAME: &str = r"Core\Storage";

/// `Core\Storage::put`'s member name, in one place — three refusals name it.
const PUT: &str = r"Core\Storage::put";
/// See [`PUT`].
const GET: &str = r"Core\Storage::get";
/// See [`PUT`].
const DELETE: &str = r"Core\Storage::delete";
/// See [`PUT`].
const LIST: &str = r"Core\Storage::list";

/// The ABI slot each parameter and each flattened option lands in — the bag
/// expands to one argument per option, in declaration order, after the
/// positionals.
///
/// The slots are per row, so two names can share an index: [`LIST`] takes no
/// key, and its one option lands where the other three rows carry [`KEY`].
const DISK: usize = 0;
/// See [`DISK`].
const KEY: usize = 1;
/// See [`DISK`].
const CONTENTS: usize = 2;
/// See [`DISK`].
const OVERWRITE: usize = 3;
/// See [`DISK`].
const PREFIX: usize = 1;

/// The longest key this class will resolve to a file name.
///
/// 255 because that is the byte limit every filesystem worth naming imposes on
/// one path component — ext4, APFS, NTFS and every FAT descendant. Refusing at
/// this bound is refusing *here*, with a sentence naming the rule, rather than
/// at the `create` with whatever `ENAMETOOLONG` renders as on the host: the two
/// are the same refusal, and only one of them is portable enough to freeze in a
/// test.
const MAX_KEY: usize = 255;

/// [`put`](CLASS)'s trailing shape — the one question about an existing object
/// that a caller can answer differently.
///
/// `true` by default because that is the object-store contract: a key names one
/// object, and putting it again replaces it. The option exists for the caller
/// who is claiming a key rather than updating one, and it is an *option* rather
/// than a read-then-write at the call site because the door does it atomically —
/// `create_new` is one syscall, and the check-then-write it replaces is a race
/// two requests can both win.
const PUT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "overwrite",
    ty: CoreTy::Bool,
    default: Const::Bool(true),
}];

/// [`list`](CLASS)'s trailing shape — which of the disk's objects are being
/// asked about.
///
/// Empty means all of them, which is why the default is a value rather than
/// [`Const::Null`]: "no prefix" and "the prefix every key starts with" are the
/// same question, and a not-given path in the body would have been a second
/// spelling of one answer.
const LIST_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "prefix",
    ty: CoreTy::Text(Qual::Neutral),
    default: Const::Str(""),
}];

/// `Core\Storage`'s four rows — `rule:programs/framework-core-half`'s object storage.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[
        CoreMethod {
            name: "put",
            names: &["disk", "key", "contents"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Options(PUT_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_storage_put",
            doc: Some(&PUT_DOC),
        },
        CoreMethod {
            name: "get",
            names: &["disk", "key"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Bytes),
            symbol: "nvs_core_storage_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "delete",
            names: &["disk", "key"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_storage_delete",
            doc: Some(&DELETE_DOC),
        },
        CoreMethod {
            name: "list",
            names: &["disk"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Options(LIST_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_storage_list",
            doc: Some(&LIST_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The `$disk` parameter's card, which all three rows share word for word.
const DISK_DOC: ParamDoc = ParamDoc {
    name: "disk",
    desc: "Which `[storage.<name>]` block in `nvs.toml` the object lives on. Refuses a `tainted` \
           argument: it selects a deployment, so it is written at the call site and never read \
           from input.",
    shape: &[],
};

/// The `$key` parameter's card, likewise shared.
const KEY_DOC: ParamDoc = ParamDoc {
    name: "key",
    desc: "The object's name on that disk: one segment of ASCII letters, digits, `.`, `-` and \
           `_`, at most 255 bytes, not beginning with a `.`. A separator is refused rather than \
           resolved, so no key names anything but an object directly on the disk.",
    shape: &[],
};

/// The refusal all three rows share, in the one wording the three cards use.
const REFUSAL_DOC: ErrorDoc = ErrorDoc {
    error: "RuntimeError",
    desc: "No `[storage.<name>]` block of that name sets a `root`; or `$key` is not an object \
           key; or the `fs.read`/`fs.write` capability does not cover the object's path. Each is \
           a deployment or a call that was written wrong.",
};

/// `Core\Storage::put`'s reference card — `rule:core-api/reference-card`.
const PUT_DOC: MethodDoc = MethodDoc {
    short: "Writes `$contents` as the object `$key` on `$disk`, replacing whatever was there \
            unless `overwrite` says not to.",
    params: &[
        DISK_DOC,
        KEY_DOC,
        ParamDoc {
            name: "contents",
            desc: "The object's octets, written whole.",
            shape: &[],
        },
        ParamDoc {
            name: "overwrite",
            desc: "Whether an object already at `$key` may be replaced. `true` by default, which \
                   is the object-store contract; `false` claims the key instead, and fails if \
                   another writer already holds it.",
            shape: &[],
        },
    ],
    ret: "Nothing. The object is on the disk once this returns.",
    errors: &[
        REFUSAL_DOC,
        ErrorDoc {
            error: "IOError",
            desc: "The object could not be written — including `overwrite: false` against a key \
                   that already exists, which is what a refused claim is.",
        },
    ],
};

/// `Core\Storage::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Reads the object `$key` on `$disk`, or answers `null` where the disk holds no object \
            of that name.",
    params: &[DISK_DOC, KEY_DOC],
    ret: "The object's octets, or `null` for a key nothing was ever put at — absence is the \
          return type's answer here, not an error.",
    errors: &[
        REFUSAL_DOC,
        ErrorDoc {
            error: "IOError",
            desc: "The object is there and could not be read.",
        },
    ],
};

/// `Core\Storage::delete`'s reference card — `rule:core-api/reference-card`.
const DELETE_DOC: MethodDoc = MethodDoc {
    short: "Removes the object `$key` from `$disk`.",
    params: &[DISK_DOC, KEY_DOC],
    ret: "Nothing. The object is gone once this returns.",
    errors: &[
        REFUSAL_DOC,
        ErrorDoc {
            error: "IOError",
            desc: "There is no object at `$key`, or it could not be removed. Deleting what was \
                   never there is a failure rather than a silent success: the key was computed \
                   by the caller, and a typo that succeeds is one nothing reports.",
        },
    ],
};

/// `Core\Storage::list`'s reference card — `rule:core-api/reference-card`.
const LIST_DOC: MethodDoc = MethodDoc {
    short: "Answers the keys of the objects on `$disk`, sorted byte-ascending — every entry one \
            that `get` hands octets back for.",
    params: &[
        DISK_DOC,
        ParamDoc {
            name: "prefix",
            desc: "Which of the disk's keys to answer about: the ones beginning with this text. \
                   Empty by default, which is all of them. A prefix that is neither empty nor \
                   itself an object key is refused rather than answered with nothing, since no \
                   key the disk can hold could have begun with it.",
            shape: &[],
        },
    ],
    ret: "The matching keys, sorted byte-ascending; an empty array where the disk holds no object \
          that matches. Only a regular file whose name is an object key is listed, so a \
          subdirectory, a symlink and a name this class has no key for are all absent.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "No `[storage.<name>]` block of that name sets a `root`; or `prefix` is neither \
                   empty nor an object key; or the `fs.read` capability does not cover the disk's \
                   own root.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The disk's root could not be read — it is not there, or it is not a directory.",
        },
    ],
};

/// The `Core` symbol table's arm for this module — see [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_storage_put" => (nvs_core_storage_put as *const ()).cast(),
        "nvs_core_storage_get" => (nvs_core_storage_get as *const ()).cast(),
        "nvs_core_storage_delete" => (nvs_core_storage_delete as *const ()).cast(),
        "nvs_core_storage_list" => (nvs_core_storage_list as *const ()).cast(),
        _ => return None,
    })
}

/// The directory `[storage.<disk>] root` names, with an empty value read as
/// absent.
///
/// Absent and blank are one answer for [`crate::mail`]'s reason: `root = ""` is
/// an operator clearing a setting, and reading it as a directory would produce a
/// refusal about the current working directory rather than about the
/// configuration.
///
/// # Errors
///
/// A catchable `RuntimeError` naming the block that would have to exist. The
/// capability is **not** asked here, because there is nothing yet to ask it
/// about: the path this class touches is built out of this answer, and asking
/// `fs.*` about the root instead would be a check over a directory no member
/// opens.
fn root_of(ctx: &Ctx, disk: &str, member: &str) -> Result<PathBuf, Fault> {
    ctx.config()
        .and_then(|config| config.get(&format!("storage.{disk}.root")))
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{member}: no `[storage.{disk}]` block sets `root`, so there is no disk of that \
                 name to store on"
            ))
        })
}

/// Whether `key` is an object key — the grammar the module doc's second
/// section states, in the one place both readers of it agree from.
///
/// A predicate beside [`key_of`] rather than inside it because
/// [`list`](CLASS) asks the question about names it is *filtering* and not
/// about an argument: a refusal there would be a `Fault` built, formatted and
/// dropped once per file the operator's directory happens to hold.
fn is_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY
        && !key.starts_with('.')
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// `key` as an object key, or the one refusal that says why it is not one.
///
/// One refusal rather than five, and the sentence carries the whole grammar
/// because that is what a caller needs to see: the five ways to fail it are one
/// question — is this a name, or is it a path? — and a message per way would
/// have said the same thing five times.
///
/// # Errors
///
/// A catchable `RuntimeError` naming the key and the rule. `rule:errors/ambiguous-input-refused` is why none
/// of it is stripped instead: a key a program did not mean is an object stored
/// where it did not mean, and only the caller can say which was intended.
fn key_of<'a>(key: &'a str, member: &str) -> Result<&'a str, Fault> {
    if is_key(key) {
        return Ok(key);
    }
    Err(Fault::thrown(format!(
        "{member}: `{key}` is not an object key — one to {MAX_KEY} bytes of ASCII letters, \
         digits, `.`, `-` and `_`, not beginning with a `.`. A key names an object on the disk \
         and never a path through it, so a separator is refused rather than resolved"
    )))
}

/// `prefix` as a key prefix — empty, or an object key.
///
/// The refusal is [`key_of`]'s own sentence and that is exact rather than
/// approximate: every non-empty prefix of a key is itself a key, so a prefix
/// the grammar refuses is one no key on the disk can begin with, and "this is
/// not an object key" is precisely what is wrong with it. The module doc's
/// third section is why it is refused rather than answered with nothing.
///
/// # Errors
///
/// [`key_of`]'s catchable `RuntimeError`, for a non-empty prefix that is not a
/// key.
fn prefix_of<'a>(prefix: &'a str, member: &str) -> Result<&'a str, Fault> {
    if prefix.is_empty() {
        return Ok(prefix);
    }
    key_of(prefix, member)
}

/// Where the object named by this call's `$disk` and `$key` lives.
///
/// The join is the whole of the mapping, and it is safe because [`key_of`] has
/// already refused every spelling that could have made it something other than
/// one entry directly under the root.
///
/// # Errors
///
/// [`root_of`]'s and [`key_of`]'s refusals, in that order — a program naming a
/// disk that does not exist learns that before anything about its key.
fn object_of(ctx: &Ctx, args: &[Value], member: &str) -> Result<PathBuf, Fault> {
    let disk = text_of(args, DISK, "disk", member)?;
    let key = text_of(args, KEY, "key", member)?;
    let root = root_of(ctx, disk, member)?;
    Ok(root.join(key_of(key, member)?))
}

/// One `string` argument's text.
///
/// # Errors
///
/// A [`Fault::fatal`], for [`crate::mail`]'s `text_of` reason: the row declares
/// the slot, so another tag is compiled-code damage rather than anything a
/// program can write.
fn text_of<'a>(args: &'a [Value], slot: usize, what: &str, member: &str) -> Result<&'a str, Fault> {
    args[slot].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `string` for `{what}`, got tag {}",
            args[slot].tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Storage::put(string $disk, string $key, bytes $contents, {overwrite?: bool}): void` —
    /// `rule:programs/framework-core-half`'s object storage, writing half.
    ///
    /// The door decides the capability and this decides nothing: the path is
    /// built, `fs.write` is shown for it, and the octets go down in one write.
    fn nvs_core_storage_put(ctx, args: [4]) {
        let path = object_of(ctx, args, PUT)?;
        // Unreachable from source on `crate::io`'s `access_of` reading: the row
        // declares `bytes`, so `E0401` refuses every other spelling at the call
        // site and what is left is a lowering bug.
        let contents = args[CONTENTS].as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{PUT} expected {:?} for `contents`, got tag {}",
                Tag::Bytes,
                args[CONTENTS].tag_byte()
            ))
        })?;
        // Unreachable from source for the same reason: `PUT_OPTIONS` declares
        // `CoreTy::Bool`, and an omitting call site passes the row's default.
        let overwrite = args[OVERWRITE].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "{PUT} expected {:?} for `overwrite`, got tag {}",
                Tag::Bool,
                args[OVERWRITE].tag_byte()
            ))
        })?;
        let mut file = nvs_runtime::capability::create(ctx, &path, overwrite, PUT)?;
        file.write_all(contents)
            .map_err(|err| nvs_runtime::capability::io_failure(PUT, &path, &err))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Storage::get(string $disk, string $key): ?bytes` — `rule:programs/framework-core-half`'s
    /// reading half.
    ///
    /// Two doors rather than one, and both are `fs.read`:
    /// [`nvs_runtime::capability::exists`] is what turns a missing object into
    /// this row's `null` instead of an `IOError`, and its own doc is the reason
    /// that leaks nothing — the grant is asked before the question, so a path
    /// outside it is refused whether or not anything is there.
    fn nvs_core_storage_get(ctx, args: [2]) {
        let path = object_of(ctx, args, GET)?;
        if !nvs_runtime::capability::exists(ctx, &path, GET)? {
            return Ok(Value::null());
        }
        let mut file = nvs_runtime::capability::open_read(ctx, &path, GET)?;
        let mut octets = Vec::new();
        file.read_to_end(&mut octets)
            .map_err(|err| nvs_runtime::capability::io_failure(GET, &path, &err))?;
        Ok(Value::bytes(NvsStr::new(&octets)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Storage::delete(string $disk, string $key): void` — `rule:programs/framework-core-half`'s
    /// third row.
    ///
    /// [`nvs_runtime::capability::remove_file`] and not `remove_dir`: a key
    /// cannot name a directory, so the door that unlinks one file is the only
    /// one this class ever needs.
    fn nvs_core_storage_delete(ctx, args: [2]) {
        let path = object_of(ctx, args, DELETE)?;
        nvs_runtime::capability::remove_file(ctx, &path, DELETE)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Storage::list(string $disk, {prefix?: string}): array<string>` —
    /// `rule:programs/framework-core-half`'s fourth row, and the only one that reads the disk itself
    /// rather than one object on it.
    ///
    /// The door is [`nvs_runtime::capability::read_dir`] over the root, and it
    /// is `fs.read` about the directory the other three rows' objects already
    /// sit in — so a deployment that granted the disk at all has granted this,
    /// which is what keeps the class's "no capability of its own" true for a
    /// member that asks a new question. Everything after the door is filtering,
    /// and the module doc's third section is the whole of what it filters on.
    fn nvs_core_storage_list(ctx, args: [2]) {
        let disk = text_of(args, DISK, "disk", LIST)?;
        let prefix = prefix_of(text_of(args, PREFIX, "prefix", LIST)?, LIST)?;
        let root = root_of(ctx, disk, LIST)?;

        let mut keys: Vec<String> = Vec::new();
        for entry in nvs_runtime::capability::read_dir(ctx, &root, LIST)? {
            // An entry that cannot be read or typed is not an object this call
            // can promise anything about, so it joins the names the module doc
            // omits rather than failing the whole listing. A name that is not
            // UTF-8 is not a key by the same reading: the grammar is ASCII.
            let Ok(entry) = entry else { continue };
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !name.starts_with(prefix) || !is_key(name) {
                continue;
            }
            // A regular file and nothing else. `file_type` does not follow a
            // symlink, which is the answer this member wants: what a link names
            // is not on this disk, and following one would make a listing
            // depend on a path the operator's grant was not written about.
            if entry.file_type().is_ok_and(|kind| kind.is_file()) {
                keys.push(name.to_owned());
            }
        }
        keys.sort_unstable();

        let mut out = NvsArray::new();
        for key in &keys {
            out.append(Value::str(NvsStr::new(key.as_bytes())));
        }
        Ok(Value::array(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:programs/framework-core-half`'s row, as the property a later member can quietly break:
    /// every door this class reaches is an `fs.*` one, so the class declares no
    /// capability of its own and every row says which half of `fs` it needs.
    ///
    /// An acceptance check in the goal records under `data/goals/`. It asserts over the registry
    /// rather than over a temporary directory on purpose — a `storage.read`
    /// added beside these rows would leave every behavioural test in this file
    /// green while splitting one door's authority in two.
    #[test]
    fn storage_over_local_disk_is_gated_on_the_same_fs_capability() {
        let rows: Vec<(&str, Option<nvs_config::Cap>)> = crate::registry::CAPABILITIES
            .iter()
            .filter(|(class, _, _)| *class == NAME)
            .map(|(_, member, cap)| (*member, *cap))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("put", Some(nvs_config::Cap::FsWrite)),
                ("get", Some(nvs_config::Cap::FsRead)),
                ("delete", Some(nvs_config::Cap::FsWrite)),
                ("list", Some(nvs_config::Cap::FsRead)),
            ],
            "every Core\\Storage member is gated on the fs capability its door already asks for"
        );

        // The same capabilities `Core\IO` declares, named from its own rows so
        // that this cannot drift into asserting a constant against itself.
        let io: Vec<nvs_config::Cap> = crate::registry::CAPABILITIES
            .iter()
            .filter(|(class, _, _)| *class == crate::io::NAME)
            .filter_map(|(_, _, cap)| *cap)
            .collect();
        for (member, cap) in &rows {
            let cap = cap.expect("a Core\\Storage member declares a capability");
            assert!(
                io.contains(&cap),
                "Core\\Storage::{member} declares {cap:?}, which Core\\IO does not — the disk is \
                 reached through a door of its own"
            );
        }

        // And no member names a path, which is what makes the grant exact.
        for row in CLASS.methods {
            assert_eq!(row.names[0], "disk");
            assert!(
                !row.names.contains(&"path"),
                "Core\\Storage::{} takes a path, so the key is no longer a name",
                row.name
            );
            assert!(
                matches!(row.params[DISK], CoreTy::Text(Qual::Sink)),
                "the disk name must be a sink: it chooses between operator blocks"
            );
        }
    }

    /// The module doc's second section, as the property it claims: a key names
    /// an object and can never name a path through the disk.
    #[test]
    fn a_key_names_an_object_and_never_a_path() {
        for good in ["note.txt", "a", "receipt-2026_09.pdf", "x.y.z"] {
            assert!(key_of(good, PUT).is_ok(), "`{good}` was refused as a key");
        }
        for bad in [
            "",
            "..",
            ".",
            ".hidden",
            "a/b",
            "a\\b",
            "../escape",
            "/absolute",
            "with space",
            "café",
        ] {
            assert!(key_of(bad, PUT).is_err(), "`{bad}` was accepted as a key");
        }
        let long = "a".repeat(MAX_KEY);
        assert!(key_of(&long, PUT).is_ok(), "the bound itself was refused");
        assert!(
            key_of(&format!("{long}a"), PUT).is_err(),
            "one byte past the bound was accepted"
        );
    }
}
