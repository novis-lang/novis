//! `Core\Path` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 8, pure string algebra over paths.
//!
//! That section is authoritative for every signature. **No member touches the
//! disk**, so this module has no dependency at all — not `std::path`, whose
//! `Path`/`PathBuf` are a *host* abstraction that resolves differently on each
//! target and would drag `OsStr`'s non-UTF-8 question into a type `rule:types/bytes`
//! guarantees is text. Everything here is `&str` arithmetic.
//!
//! # One grammar on every platform
//!
//! Spec § 8 says every member accepts `/` and `\` alike on every platform and
//! emits [`SEPARATOR`]. This module reads that as the stronger statement it
//! has to be: **a path is parsed by one grammar everywhere, and only the
//! rendering is platform-dependent.** So `Path::isAbsolute('C:/log')` is
//! `true` on Linux and `Path::isAbsolute('\\tmp')` is `true` on Windows, in
//! both directions.
//!
//! The alternative — a grammar that follows the host — was rejected for a
//! reason the test suite makes concrete: this repository runs its conformance
//! cases on a native Windows leg *and* a WSL leg, so a member whose **answer**
//! depends on the platform cannot be pinned by a case at all. Only the
//! separator a member *emits* differs, which a case normalizes away
//! (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`).
//!
//! # Where this diverges from PHP, and why
//!
//! Three rows, each one a place PHP's answer cannot be relied on:
//!
//! * **A leading dot is not an extension separator.** `extension('.gitignore')`
//!   is `null` here and `'gitignore'` in PHP's `pathinfo`. PHP's reading makes
//!   `withExtension` non-invertible on every dotfile — it would rename
//!   `.gitignore` to `.txt` — and a dotfile's whole name *is* its name.
//! * **A trailing dot is not an extension either.** `extension('report.')` is
//!   `null`, not `''`: an empty extension is an absence, and R5 makes `null`
//!   the one spelling of that.
//! * **`dirname('')` is `'.'`**, not PHP's `''`. The empty path names the
//!   current directory's contents, so its parent is the current directory;
//!   PHP's answer is the one input where `dirname` hands back something that
//!   is not a usable directory.
//!
//! [`nvs_core_path_with_extension`] takes exactly what [`nvs_core_path_extension`]
//! returns — undotted, or `null` for none. That inverse is the whole reason
//! the first two rows are worth diverging for.
//!
//! Neither gap below is a missing *member*: spec § 8's roster is whole here.
//!
//! # Known gaps
//!
//! 1. **A UNC path is not modelled.** `\\server\share\f` parses as an ordinary
//!    absolute path whose components are `server`, `share` and `f`, so
//!    re-rendering it loses the doubled separator that makes it UNC. Nothing
//!    on the path to `examples/collect.nvs` writes one; the fix is a third
//!    root shape beside [`Parts::drive`], not a change of interface.
//!    Decided: Yes: add a third root shape beside the drive letter — Round-trips correctly; one more
//!    root case in the path parser and its tests.
//!    — owner: unowned-closures
//! 2. **A drive-*relative* path is not modelled.** `C:log` — Windows' "the
//!    current directory *on* drive C" — has no separator after the colon, so
//!    [`split_drive`] declines it and the whole thing is one component named
//!    `C:log`. That is the shape that round-trips; treating `C:` as a root
//!    would make `Path::split('a:b')` answer `['a:', 'b']` for an ordinary
//!    relative path, which is worse.
//!    Decided: No: keep it one relative component and state it as the grammar — Round-trips and never
//!    splits a:b wrongly; loses the Windows-specific meaning.
//!    — owner: unowned-closures
//!
//! # What these members do with a qualifier
//!
//! `rule:security/unclassified-parameter-refuses-tainted`'s classification, and the one judgement in it worth writing
//! down: **`normalize` is not a launderer.** It is the member most likely to be
//! read as one — resolving `..` is exactly what stops a path climbing out of a
//! directory, so it *looks* like the thing that makes a `tainted` path safe —
//! and spec § 8 says outright that it is not. Its answer is still the caller's
//! bytes rearranged, and the sink it would have to launder for is a filesystem
//! nothing in this module opens. So it is `Qual::Contagious` with the other
//! seven text-answering members, and `isAbsolute` is the class's only
//! `Qual::Neutral` row because a `bool` carries no byte of its subject.

use nvs_runtime::{Fault, HelperResult, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    Const, CoreClass, CoreConst, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
    Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Path`'s registry rows, in the spec's own order — all nine of § 8's
/// members, plus the `SEPARATOR` constant on [`CONSTANTS`].
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Path",
    methods: &[
        CoreMethod {
            name: "basename",
            names: &["path"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(BASENAME_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_path_basename",
            doc: Some(&BASENAME_DOC),
        },
        CoreMethod {
            name: "dirname",
            names: &["path"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(DIRNAME_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_path_dirname",
            doc: Some(&DIRNAME_DOC),
        },
        CoreMethod {
            name: "extension",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_path_extension",
            doc: Some(&EXTENSION_DOC),
        },
        CoreMethod {
            name: "withExtension",
            names: &["path", "extension"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Nullable(&CoreTy::Text(Qual::Contagious)),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_path_with_extension",
            doc: Some(&WITH_EXTENSION_DOC),
        },
        CoreMethod {
            name: "join",
            names: &["base", "segments"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Variadic(&CoreTy::Text(Qual::Contagious)),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_path_join",
            doc: Some(&JOIN_DOC),
        },
        CoreMethod {
            name: "split",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_path_split",
            doc: Some(&SPLIT_DOC),
        },
        CoreMethod {
            name: "normalize",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_path_normalize",
            doc: Some(&NORMALIZE_DOC),
        },
        CoreMethod {
            name: "isAbsolute",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_path_is_absolute",
            doc: Some(&IS_ABSOLUTE_DOC),
        },
        CoreMethod {
            name: "relativeTo",
            names: &["path", "base"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Text(Qual::Contagious),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_path_relative_to",
            doc: Some(&RELATIVE_TO_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: CONSTANTS,
};

/// `Core\Path::basename`'s reference card — `rule:core-api/reference-card`.
const BASENAME_DOC: MethodDoc = MethodDoc {
    short: "Answers the name of `$path`'s last component, as `basename` and \
            `pathinfo(…, PATHINFO_BASENAME)` do; a trailing separator is ignored, so `/a/b/` \
            is `b`.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The path, with `/` and `\\` both read as separators.",
            shape: &[],
        },
        ParamDoc {
            name: "withoutExtension",
            desc: "Drop the extension from the name — `pathinfo`'s `PATHINFO_FILENAME`; the \
                   default keeps it.",
            shape: &[],
        },
    ],
    ret: "The last component's name; the empty string for a path that is nothing but a root, \
          such as `/`.",
    errors: &[],
};

/// `Core\Path::dirname`'s reference card — `rule:core-api/reference-card`.
const DIRNAME_DOC: MethodDoc = MethodDoc {
    short: "Answers `$path` with `levels` components dropped from the end, as `dirname` does; \
            the answer is always a usable directory.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The path, with `/` and `\\` both read as separators.",
            shape: &[],
        },
        ParamDoc {
            name: "levels",
            desc: "How many components to drop; the default is one, `0` is the path itself with \
                   its separators normalized rather than PHP's `ValueError`, and more than \
                   there are stops at the root.",
            shape: &[],
        },
    ],
    ret: "The parent path, rendered with `Path::SEPARATOR`; the root for an absolute path and \
          `.` for a relative one — including `''`, which is `.` here and `''` in PHP — once \
          nothing is left.",
    errors: &[],
};

/// `Core\Path::extension`'s reference card — `rule:core-api/reference-card`.
const EXTENSION_DOC: MethodDoc = MethodDoc {
    short: "Answers the text after the last `.` of `$path`'s last component, without the dot, \
            as `pathinfo(…, PATHINFO_EXTENSION)` does.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path, with `/` and `\\` both read as separators.",
        shape: &[],
    }],
    ret: "The extension, or `null` where there is none — a dotfile such as `.gitignore` and a \
          trailing dot such as `report.` both have none, where PHP answers `gitignore` and \
          `''`.",
    errors: &[],
};

/// `Core\Path::withExtension`'s reference card — `rule:core-api/reference-card`.
const WITH_EXTENSION_DOC: MethodDoc = MethodDoc {
    short: "Answers `$path` with its last component's extension replaced by `$extension`, or \
            removed for `null` — the inverse of `Core\\Path::extension`, replacing the string \
            surgery PHP leaves this to.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The path, with `/` and `\\` both read as separators.",
            shape: &[],
        },
        ParamDoc {
            name: "extension",
            desc: "The new extension without its dot, exactly as `extension` answers it — an \
                   interior dot such as `tar.gz` is fine — or `null` to remove the existing \
                   one.",
            shape: &[],
        },
    ],
    ret: "The rewritten path, rendered with `Path::SEPARATOR`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$extension` is empty, starts with a `.`, or contains a path separator; or \
               `$path` names no file — `/` or `''` — so there is no extension to set.",
    }],
};

/// `Core\Path::join`'s reference card — `rule:core-api/reference-card`.
const JOIN_DOC: MethodDoc = MethodDoc {
    short: "Appends each of `$segments` to `$base` with a separator between — the \
            `$a . \"/\" . $b` every PHP program writes. Only the base decides the root: a \
            segment's own leading separator or drive is dropped rather than allowed to replace \
            what came before.",
    params: &[
        ParamDoc {
            name: "base",
            desc: "The path the segments are appended to; its root, if any, is the result's.",
            shape: &[],
        },
        ParamDoc {
            name: "segments",
            desc: "Any number of further path pieces, each split into components and appended \
                   in order.",
            shape: &[],
        },
    ],
    ret: "The joined path, rendered with `Path::SEPARATOR`; `$base` with its separators \
          normalized when there are no segments. Not a launder — a `..` segment still walks \
          up.",
    errors: &[],
};

/// `Core\Path::split`'s reference card — `rule:core-api/reference-card`.
const SPLIT_DOC: MethodDoc = MethodDoc {
    short: "Splits `$path` into its components — `explode(DIRECTORY_SEPARATOR, …)` for both \
            separators at once, and lossless: `Path::join(...Path::split($p))` is `$p` with \
            its separators normalized.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path, with `/` and `\\` both read as separators.",
        shape: &[],
    }],
    ret: "The components in order, an absolute path's root (`/` or `C:\\`, rendered with \
          `Path::SEPARATOR`) first; never an empty element, since a repeated or trailing \
          separator contributes nothing, and an empty array for `''`.",
    errors: &[],
};

/// `Core\Path::normalize`'s reference card — `rule:core-api/reference-card`.
const NORMALIZE_DOC: MethodDoc = MethodDoc {
    short: "Resolves `.` and `..` in `$path` lexically and re-renders it with \
            `Path::SEPARATOR` — the half of `realpath` that does not touch the disk. Not a \
            launder: removing `../` is not path-traversal safety, which is `Core\\IO::within`.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path, with `/` and `\\` both read as separators.",
        shape: &[],
    }],
    ret: "The normal form; a `..` that cannot be cancelled is kept on a relative path and \
          dropped on an absolute one, and a repeated or trailing separator goes. Every path \
          has one, so this never fails.",
    errors: &[],
};

/// `Core\Path::isAbsolute`'s reference card — `rule:core-api/reference-card`.
const IS_ABSOLUTE_DOC: MethodDoc = MethodDoc {
    short: "Answers whether `$path` begins at a root — a separator, or a drive letter followed \
            by a separator — replacing the manual checks PHP leaves this to.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path, with `/` and `\\` both read as separators.",
        shape: &[],
    }],
    ret: "`true` for `/tmp`, `\\tmp` and `C:/log` on every platform — the grammar is the same \
          everywhere, only the rendered separator differs — and `false` otherwise, `''` \
          included.",
    errors: &[],
};

/// `Core\Path::relativeTo`'s reference card — `rule:core-api/reference-card`.
const RELATIVE_TO_DOC: MethodDoc = MethodDoc {
    short: "Answers the relative path that leads from `$base` to `$path`, both resolved \
            lexically first — a member PHP has no equivalent of.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The destination.",
            shape: &[],
        },
        ParamDoc {
            name: "base",
            desc: "The directory the answer is relative to; one `..` is emitted per component \
                   of it that the two do not share.",
            shape: &[],
        },
    ],
    ret: "The relative path, rendered with `Path::SEPARATOR` and never carrying a root; `.` \
          when both name the same place; `null` when no relative path exists — one side is \
          absolute and the other is not, the two name different drives, or `$base` still \
          holds a `..` the answer would have to walk back into. Components compare byte for \
          byte, except a drive letter, which ignores ASCII case.",
    errors: &[],
};

/// `Core\Path::basename`'s `{withoutExtension?: bool}` — `pathinfo`'s
/// `PATHINFO_FILENAME` as an option rather than as a second member, since it
/// asks the same question of the same path.
const BASENAME_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "withoutExtension",
    ty: CoreTy::Bool,
    default: Const::Bool(false),
}];

/// `Core\Path::dirname`'s `{levels?: uint}` — PHP's second `dirname` argument,
/// with [`nvs_core_path_dirname`]'s own docs owning what `0` means here and
/// throws there.
const DIRNAME_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "levels",
    ty: CoreTy::Uint,
    default: Const::Uint(1),
}];

/// `Core\Path::SEPARATOR`, replacing PHP's `DIRECTORY_SEPARATOR`.
///
/// The one platform-dependent thing in this module: it is what every member
/// *emits*, never what one accepts. A constant rather than a member because
/// it has a value and no signature — `registry::CoreConst` owns that split.
const CONSTANTS: &[CoreConst] = &[CoreConst {
    name: "SEPARATOR",
    ty: CoreTy::Str,
    value: Const::Str(SEPARATOR),
    desc: "The separator this platform's paths are rendered with — `\\` on Windows and `/` \
           everywhere else, as `DIRECTORY_SEPARATOR` is; every member emits it and accepts \
           both.",
}];

/// The separator this platform's paths are written with — `\` on Windows and
/// `/` everywhere else, which is exactly `DIRECTORY_SEPARATOR`'s value.
const SEPARATOR: &str = std::path::MAIN_SEPARATOR_STR;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_path_basename" => (nvs_core_path_basename as *const ()).cast(),
        "nvs_core_path_dirname" => (nvs_core_path_dirname as *const ()).cast(),
        "nvs_core_path_extension" => (nvs_core_path_extension as *const ()).cast(),
        "nvs_core_path_with_extension" => (nvs_core_path_with_extension as *const ()).cast(),
        "nvs_core_path_join" => (nvs_core_path_join as *const ()).cast(),
        "nvs_core_path_split" => (nvs_core_path_split as *const ()).cast(),
        "nvs_core_path_normalize" => (nvs_core_path_normalize as *const ()).cast(),
        "nvs_core_path_is_absolute" => (nvs_core_path_is_absolute as *const ()).cast(),
        "nvs_core_path_relative_to" => (nvs_core_path_relative_to as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The grammar — one parse, shared by every member
// ============================================================================

/// One path, taken apart: at most a drive, whether a separator follows it, and
/// every non-empty component in order.
///
/// Borrowed from the subject rather than owned, since every member either
/// answers with one component or renders a fresh string; nothing here holds a
/// path past its own call.
///
/// **`drive.is_some()` implies `absolute`** — [`split_drive`] only recognizes
/// a drive that a separator follows, which is this module's gap 2.
#[derive(Debug)]
pub(crate) struct Parts<'a> {
    /// The `C:` of `C:\log`, without its separator, or `None` for a path that
    /// names no drive.
    pub(crate) drive: Option<&'a str>,
    /// Whether the path begins at a root — a separator, or a drive's.
    pub(crate) absolute: bool,
    /// Every component, in order, with empty ones dropped: a repeated
    /// separator names nothing, and neither does a trailing one.
    pub(crate) components: Vec<&'a str>,
}

/// Either separator, on every platform — spec § 8's acceptance rule, which is
/// deliberately not the emission rule ([`SEPARATOR`]).
fn is_separator(c: char) -> bool {
    c == '/' || c == '\\'
}

/// `path` split into its `C:`-style drive and the rest, or `None` where it
/// names no drive.
///
/// A drive is recognized **only** when a separator follows it, which is what
/// keeps an ordinary relative `a:b` from parsing as one — gap 2 above.
fn split_drive(path: &str) -> Option<(&str, &str)> {
    let bytes = path.as_bytes();
    let (Some(letter), Some(b':'), Some(third)) =
        (bytes.first(), bytes.get(1).copied(), bytes.get(2).copied())
    else {
        return None;
    };
    if letter.is_ascii_alphabetic() && (third == b'/' || third == b'\\') {
        return Some(path.split_at(2));
    }
    None
}

/// Takes one path apart. Total: every string is a path, including the empty
/// one, which is no drive, not absolute and no components.
pub(crate) fn parse(path: &str) -> Parts<'_> {
    let (drive, rest) = match split_drive(path) {
        Some((drive, rest)) => (Some(drive), rest),
        None => (None, path),
    };
    Parts {
        drive,
        absolute: rest.starts_with(is_separator),
        components: rest.split(is_separator).filter(|c| !c.is_empty()).collect(),
    }
}

/// The root `parts` begins at, rendered with [`SEPARATOR`] — empty for a
/// relative path.
fn root(parts: &Parts<'_>) -> String {
    let mut out = String::new();
    if let Some(drive) = parts.drive {
        out.push_str(drive);
    }
    if parts.absolute {
        out.push_str(SEPARATOR);
    }
    out
}

/// `parts`, with `components` in place of its own, rendered with
/// [`SEPARATOR`].
///
/// A relative path with no components renders as `.` rather than as the empty
/// string: every member here answers with a path a program can use, and the
/// empty string is not one.
fn render(parts: &Parts<'_>, components: &[&str]) -> String {
    let mut out = root(parts);
    if out.is_empty() && components.is_empty() {
        return ".".to_owned();
    }
    for (index, component) in components.iter().enumerate() {
        if index > 0 {
            out.push_str(SEPARATOR);
        }
        out.push_str(component);
    }
    out
}

/// One component's name split from its extension — the single definition both
/// [`nvs_core_path_extension`] and the two members that rewrite a name read,
/// so the leading-dot and trailing-dot rules cannot drift between them.
///
/// The extension carries no `.`, exactly as `Core\Path::extension` answers and
/// `Core\Path::withExtension` expects.
fn stem_and_extension(name: &str) -> (&str, Option<&str>) {
    match name.rfind('.') {
        // No dot at all, or a leading one, which names a dotfile rather than
        // an extension — this module's first divergence from PHP.
        None | Some(0) => (name, None),
        // A trailing dot: an empty extension is an absence, so `report.` keeps
        // its dot in the stem and answers `null`.
        Some(dot) if dot + 1 == name.len() => (name, None),
        Some(dot) => (&name[..dot], Some(&name[dot + 1..])),
    }
}

/// `parts`' components with `.` and `..` resolved lexically — the single
/// definition [`nvs_core_path_normalize`] and [`nvs_core_path_relative_to`]
/// share, since answering "where is this relative to that" means answering it
/// about two paths that have already been resolved.
///
/// **Lexical, and therefore not a launderer** — spec § 8 says so out loud: a
/// `..` is removed by counting components, and whether the component it
/// cancelled was a symlink is a question about the disk. `Core\IO::within` is
/// where a base directory is actually known.
///
/// A `..` that cannot be cancelled is *kept* on a relative path, because
/// `../a` names something real and nothing here knows what; on an absolute one
/// it is dropped, since a root has no parent.
fn resolved<'a>(parts: &Parts<'a>) -> Vec<&'a str> {
    let mut out: Vec<&'a str> = Vec::new();
    for component in &parts.components {
        match *component {
            "." => {}
            ".." => match out.last() {
                Some(&last) if last != ".." => {
                    out.pop();
                }
                None if parts.absolute => {}
                _ => out.push(".."),
            },
            name => out.push(name),
        }
    }
    out
}

/// The relative path, as components, from `origin` to `target` — both already
/// [`resolved`] — or `None` where no such path can be named.
///
/// The one refusal is an `origin` component this cannot invert: a `..` left
/// over on a relative base names a directory whose own name is unknown, so
/// there is no component that walks back into it.
fn walk<'a>(target: &[&'a str], origin: &[&'a str]) -> Option<Vec<&'a str>> {
    let shared = target
        .iter()
        .zip(origin)
        .take_while(|(here, there)| here == there)
        .count();
    let mut out = Vec::new();
    for component in &origin[shared..] {
        if *component == ".." {
            return None;
        }
        out.push("..");
    }
    out.extend_from_slice(&target[shared..]);
    Some(out)
}

/// A relative path built from components alone — what [`walk`]'s answer and
/// nothing else renders through.
fn relative(components: &[&str]) -> String {
    render(
        &Parts {
            drive: None,
            absolute: false,
            components: Vec::new(),
        },
        components,
    )
}

// ============================================================================
// Argument decoding — the same shape as `crate::str`'s, naming this class
// ============================================================================

/// One `string` argument as text, with `crate::str`'s `text` as the shape —
/// including its one failure, since the tag [`Value::as_text`] checks is
/// itself `rule:types/bytes`'s UTF-8 guarantee.
fn text<'a>(value: &'a Value, member: &str, position: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Path::{member} expected {:?} for {position}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// One `?string` argument, where `null` is the answer the member acts on
/// rather than a failure.
fn maybe_text<'a>(
    value: &'a Value,
    member: &str,
    position: &str,
) -> Result<Option<&'a str>, Fault> {
    match value.tag() {
        Some(Tag::Null) => Ok(None),
        _ => text(value, member, position).map(Some),
    }
}

/// One `bool` argument.
fn boolean(value: &Value, member: &str, position: &str) -> Result<bool, Fault> {
    value.as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Path::{member} expected {:?} for {position}, got tag {}",
            Tag::Bool,
            value.tag_byte()
        ))
    })
}

/// One `uint` argument, as a `usize` — saturating, since a count of components
/// larger than any path could hold behaves exactly as the maximum does.
fn count(value: &Value, member: &str, position: &str) -> Result<usize, Fault> {
    let raw = value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Path::{member} expected {:?} for {position}, got tag {}",
            Tag::Uint,
            value.tag_byte()
        ))
    })?;
    Ok(usize::try_from(raw).unwrap_or(usize::MAX))
}

/// A freshly built `string` result.
fn produced(text: &str) -> HelperResult {
    Ok(Value::str(NvsStr::new(text.as_bytes())))
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Path::basename(string $path, {withoutExtension?: bool}): string`
    /// — replacing PHP's `basename` and `pathinfo(…, PATHINFO_BASENAME)`.
    ///
    /// The **name** of the last component, so a trailing separator is ignored
    /// (`/a/b/` is `b`, as PHP's own `basename` answers) and a path that is
    /// nothing but a root has no name at all: `basename('/')` is `''`. That
    /// empty answer is PHP's too, and it is a `string` rather than R5's `null`
    /// because the spec's signature says so — a root is not an *absent* name.
    fn nvs_core_path_basename(_ctx, args: [2]) {
        let path = text(&args[0], "basename", "the path")?;
        let without_extension = boolean(&args[1], "basename", "the `withoutExtension` option")?;

        let parts = parse(path);
        let Some(name) = parts.components.last() else {
            return produced("");
        };
        match without_extension {
            true => produced(stem_and_extension(name).0),
            false => produced(name),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::dirname(string $path, {levels?: uint}): string` — replacing
    /// PHP's `dirname`, whose second argument becomes the one option here.
    ///
    /// Drops `levels` components from the end. Where that leaves nothing, the
    /// answer is the root for an absolute path and `.` for a relative one, so
    /// every answer is a directory a program can use — including
    /// `dirname('')`, which is `.` rather than PHP's `''` (this module's third
    /// divergence).
    ///
    /// **`levels: 0` is the path itself**, with its separators normalized,
    /// rather than PHP's `ValueError`. The option is a `uint`, so it is
    /// routinely a computed depth, and "go up zero levels" has an obvious
    /// total answer; a throw would only make an arithmetic result fatal.
    /// Asking to go up further than there are components stops at the root,
    /// exactly as walking one level at a time would.
    fn nvs_core_path_dirname(_ctx, args: [2]) {
        let path = text(&args[0], "dirname", "the path")?;
        let levels = count(&args[1], "dirname", "the `levels` option")?;

        let parts = parse(path);
        let keep = parts.components.len().saturating_sub(levels);
        produced(&render(&parts, &parts.components[..keep]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::extension(string $path): ?string` — replacing PHP's
    /// `pathinfo(…, PATHINFO_EXTENSION)`.
    ///
    /// The text after the last `.` of the last component, without the dot.
    /// `null` where there is none — including a dotfile and a trailing dot,
    /// which are this module's first two divergences from PHP and are stated
    /// once, on [`stem_and_extension`].
    fn nvs_core_path_extension(_ctx, args: [1]) {
        let path = text(&args[0], "extension", "the path")?;

        let parts = parse(path);
        match parts.components.last().and_then(|name| stem_and_extension(name).1) {
            None => Ok(Value::null()),
            Some(extension) => produced(extension),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::withExtension(string $path, ?string $extension): string` —
    /// replacing the manual string surgery PHP leaves this to.
    ///
    /// **It takes exactly what [`nvs_core_path_extension`] answers**: the
    /// extension undotted, or `null` to remove one. That inverse is the point
    /// of the member, so a leading `.` is refused rather than accepted as a
    /// second spelling (`rule:core-api/shape-rules` R20) — `withExtension($p, '.txt')` names
    /// `'txt'` in its diagnostic. An interior dot is fine, so `'tar.gz'` is a
    /// usable extension even though `extension` would then answer `'gz'`.
    ///
    /// A separator in the extension is refused too: it would silently turn one
    /// component into two, which is `join`'s job and not this member's.
    ///
    /// A path with no last component has no name to rewrite, so `'/'` and `''`
    /// throw rather than inventing one (`rule:core-api/shape-rules` R4).
    fn nvs_core_path_with_extension(_ctx, args: [2]) {
        let path = text(&args[0], "withExtension", "the path")?;
        let extension = maybe_text(&args[1], "withExtension", "the extension")?;

        if let Some(extension) = extension {
            if extension.is_empty() {
                return Err(Fault::thrown(
                    "Core\\Path::withExtension(): the extension must not be empty — pass `null` \
                     to remove one",
                ));
            }
            if let Some(undotted) = extension.strip_prefix('.') {
                return Err(Fault::thrown(format!(
                    "Core\\Path::withExtension(): the extension is written without its dot, so \
                     `{extension}` should be `{undotted}`"
                )));
            }
            if extension.contains(is_separator) {
                return Err(Fault::thrown(format!(
                    "Core\\Path::withExtension(): `{extension}` contains a path separator, which \
                     would make one component into two"
                )));
            }
        }

        let parts = parse(path);
        let Some(name) = parts.components.last() else {
            return Err(Fault::thrown(format!(
                "Core\\Path::withExtension(): `{path}` names no file, so there is no extension to \
                 set"
            )));
        };
        let mut renamed = stem_and_extension(name).0.to_owned();
        if let Some(extension) = extension {
            renamed.push('.');
            renamed.push_str(extension);
        }

        let mut components = parts.components.clone();
        let last = components.len() - 1;
        components[last] = &renamed;
        produced(&render(&parts, &components))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::join(string $base, string ...$segments): string` —
    /// replacing the `$a . "/" . $b` every PHP program writes.
    ///
    /// **Only the base decides the root, and a segment is only ever
    /// appended.** A segment's own leading separator and its own drive are
    /// dropped rather than made to replace what came before, which is where
    /// this parts company with `PathBuf::push` and Python's `os.path.join`.
    /// Their rule — an absolute segment wins — is exactly what turns
    /// `Path::join($root, $fromTheRequest)` into an escape from `$root`, and
    /// it buys nothing a call could not write by passing the absolute path as
    /// the base instead. AGENTS.md's priority 1 settles which way that trade
    /// goes.
    ///
    /// That is *not* a claim to be a launderer: a segment of `..` still walks
    /// up, and [`nvs_core_path_normalize`]'s own docs say why removing one is
    /// not path-traversal safety either.
    ///
    /// The tail arrives as one `array<string>` argument rather than as N of
    /// them — `registry::CoreTy::Variadic` owns why — so this is an ordinary
    /// two-slot helper.
    fn nvs_core_path_join(_ctx, args: [2]) {
        let base = text(&args[0], "join", "the base")?;
        // Unreachable from source for a stronger reason than a checker
        // refusal: parameter 1 is `CoreTy::Variadic`, and its doc comment
        // records that `nvs_ir::lower::lower_call_args` *builds* the
        // `array<string>` this slot holds out of every argument from that
        // position on. No source expression reaches the slot at all, so there
        // is no call — well-typed or not — that could put another tag here.
        let tail = args[1].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Path::join expected {:?} for the segments, got tag {}",
                Tag::Array,
                args[1].tag_byte()
            ))
        })?;

        // Read every segment out first: a `&str` borrows the `Value` it came
        // from, so the values have to outlive the components taken from them.
        let segments = crate::arr::borrowed(tail);
        let mut values = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = segments.next_slot(from) {
            // Unreachable from source, and not for the reason the tag guard
            // above is: this is `next_slot`/`value_at`'s shared post-condition.
            // Both read the same predicate in both shapes, and the loop body
            // between them only pushes, so a slot the first answered is one the
            // second has.
            values.push(segments.value_at(slot).ok_or_else(|| {
                Fault::fatal("Core\\Path::join read an empty slot the array reported as live")
            })?);
            from = slot + 1;
        }

        let parts = parse(base);
        let mut components = parts.components.clone();
        for value in &values {
            let segment = text(value, "join", "a segment")?;
            components.extend(parse(segment).components);
        }
        produced(&render(&parts, &components))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::split(string $path): array<string>` — replacing PHP's
    /// `explode(DIRECTORY_SEPARATOR, …)`, which only ever worked for one
    /// platform's paths at a time.
    ///
    /// **An absolute path's first element is its root** — `'/'`, or `'C:\'`,
    /// rendered with [`SEPARATOR`]. That is what makes the decomposition
    /// lossless: `Core\Path::join(...Core\Path::split($p))` is `$p` with its
    /// separators normalized, which is the property `join` is written against.
    /// A repeated separator and a trailing one contribute nothing, so the
    /// remaining elements are names and never the empty string — PHP's
    /// `explode` answers `['', 'a']` for `/a` and leaves the caller to know
    /// which empties meant something.
    fn nvs_core_path_split(_ctx, args: [1]) {
        let path = text(&args[0], "split", "the path")?;

        let parts = parse(path);
        let mut out = NvsArray::new();
        let root = root(&parts);
        if !root.is_empty() {
            out.append(Value::str(NvsStr::new(root.as_bytes())));
        }
        for component in &parts.components {
            out.append(Value::str(NvsStr::new(component.as_bytes())));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::normalize(string $path): string` — the lexical half of
    /// PHP's `realpath`, which is the half that does not touch the disk.
    ///
    /// Resolves `.` and `..` by counting components ([`resolved`] owns the
    /// rules) and re-renders with [`SEPARATOR`], so a repeated or trailing
    /// separator goes too. It never fails: every path has a normal form.
    ///
    /// **It is not a launderer**, and spec § 8 says so in the same words:
    /// removing `../` is not path-traversal safety, because the base directory
    /// this path is supposed to stay inside is not part of the input. The
    /// launderer is `Core\IO::within`, in § 14, which is where the base is
    /// known. `realpath`'s other half — following symlinks, and answering
    /// `false` for a path that does not exist — belongs there too.
    fn nvs_core_path_normalize(_ctx, args: [1]) {
        let path = text(&args[0], "normalize", "the path")?;

        let parts = parse(path);
        produced(&render(&parts, &resolved(&parts)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::isAbsolute(string $path): bool` — replacing the manual
    /// checks PHP leaves this to.
    ///
    /// True for a path beginning at a separator, and for one beginning at a
    /// drive followed by a separator. Both readings hold on **every**
    /// platform, which is this module's own docs' one-grammar rule; the spec's
    /// `Q` column marks this member neutral, and a lexical question about a
    /// string is exactly why.
    fn nvs_core_path_is_absolute(_ctx, args: [1]) {
        let path = text(&args[0], "isAbsolute", "the path")?;
        Ok(Value::bool(parse(path).absolute))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::relativeTo(string $path, string $base): ?string` — the
    /// member PHP has no equivalent of at all.
    ///
    /// Both sides are [`resolved`] first, then the shared leading components
    /// are dropped and one `..` is emitted per component of `$base` that is
    /// left. The answer is `.` where the two name the same place, and it never
    /// carries a root: a relative path is what was asked for.
    ///
    /// `null` — R5's one absence spelling — where **no** relative path exists,
    /// which is three shapes and no others:
    ///
    /// * one side is absolute and the other is not, so there is no common
    ///   starting point at all;
    /// * the two name different drives, which are different roots;
    /// * `$base` still holds a `..` this would have to walk back *into*, and
    ///   the name of the directory it left is not in the input ([`walk`]).
    ///
    /// A component is compared **exactly**, byte for byte. Case-insensitivity
    /// is a property of a filesystem rather than of a path, and this member
    /// touches no filesystem; the one exception is a drive letter, which is
    /// ASCII-case-insensitive everywhere it exists.
    fn nvs_core_path_relative_to(_ctx, args: [2]) {
        let path = text(&args[0], "relativeTo", "the path")?;
        let base = text(&args[1], "relativeTo", "the base")?;

        let (target, origin) = (parse(path), parse(base));
        let same_drive = match (target.drive, origin.drive) {
            (Some(here), Some(there)) => here.eq_ignore_ascii_case(there),
            (None, None) => true,
            _ => false,
        };
        if target.absolute != origin.absolute || !same_drive {
            return Ok(Value::null());
        }
        match walk(&resolved(&target), &resolved(&origin)) {
            None => Ok(Value::null()),
            Some(components) => produced(&relative(&components)),
        }
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsArray, OutputSink, Value, call};

    use super::SEPARATOR;

    /// Runs one member through the `rule:errors/propagation` boundary compiled code reaches it
    /// at, releasing every string this test built afterwards — the helper
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
        Value::str(nvs_runtime::NvsStr::new(text.as_bytes()))
    }

    /// The returned string's bytes, releasing the reference the helper handed
    /// back, with [`SEPARATOR`] rewritten to `/` so a row reads the same on
    /// both legs.
    fn taken(result: Value) -> String {
        let text = String::from_utf8(
            result
                .as_str_bytes()
                .expect("the member returned a string")
                .to_vec(),
        )
        .expect("the member returned UTF-8");
        #[expect(
            unsafe_code,
            reason = "a returned heap value carries one fresh reference, which \
                      the caller owns"
        )]
        unsafe {
            result.release();
        }
        text.replace(SEPARATOR, "/")
    }

    /// `basename`, with the option written out.
    fn basename(path: &str, without_extension: bool) -> String {
        taken(
            run(
                super::nvs_core_path_basename,
                &[s(path), Value::bool(without_extension)],
            )
            .expect("basename never fails"),
        )
    }

    /// `dirname`, with the option written out.
    fn dirname(path: &str, levels: u64) -> String {
        taken(
            run(
                super::nvs_core_path_dirname,
                &[s(path), Value::uint(levels)],
            )
            .expect("dirname never fails"),
        )
    }

    /// `extension`, as the `?string` it answers.
    fn extension(path: &str) -> Option<String> {
        let result =
            run(super::nvs_core_path_extension, &[s(path)]).expect("extension never fails");
        match result.tag() {
            Some(nvs_runtime::Tag::Null) => None,
            _ => Some(taken(result)),
        }
    }

    /// `split`, read back in order.
    fn split(path: &str) -> Vec<String> {
        let result = run(super::nvs_core_path_split, &[s(path)]).expect("split never fails");
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the \
                      handle takes over and releases on drop"
        )]
        let array =
            unsafe { NvsArray::from_raw(result.array_ptr().expect("split returns an array")) };
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let piece = array.value_at(slot).expect("a live slot holds a value");
            out.push(
                String::from_utf8(
                    piece
                        .as_str_bytes()
                        .expect("every element is a string")
                        .to_vec(),
                )
                .expect("every element is UTF-8")
                .replace(SEPARATOR, "/"),
            );
            from = slot + 1;
        }
        out
    }

    /// `isAbsolute`.
    fn is_absolute(path: &str) -> bool {
        run(super::nvs_core_path_is_absolute, &[s(path)])
            .expect("isAbsolute never fails")
            .as_bool()
            .expect("isAbsolute returns a bool")
    }

    /// Every row checked against PHP 8.5's `basename`, except the two this
    /// module diverges on deliberately.
    #[test]
    fn basename_answers_the_last_components_name() {
        assert_eq!(basename("/a/b/c.txt", false), "c.txt");
        assert_eq!(basename("/a/b/", false), "b");
        assert_eq!(basename("a", false), "a");
        assert_eq!(basename(r"C:\x\y", false), "y");
        // A root has no name, and neither has the empty path.
        assert_eq!(basename("/", false), "");
        assert_eq!(basename("", false), "");
        // Both separators are read on every platform.
        assert_eq!(basename("/a\\b", false), "b");
    }

    #[test]
    fn basename_without_extension_drops_only_a_real_extension() {
        assert_eq!(basename("/a/b/c.txt", true), "c");
        assert_eq!(basename("a.b.c", true), "a.b");
        assert_eq!(basename("plain", true), "plain");
        // A dotfile's whole name is its name, and a trailing dot is no
        // extension — the two divergences from PHP's `pathinfo`.
        assert_eq!(basename(".gitignore", true), ".gitignore");
        assert_eq!(basename("report.", true), "report.");
    }

    #[test]
    fn dirname_walks_up_and_stops_at_a_root() {
        assert_eq!(dirname("/a/b/c.txt", 1), "/a/b");
        assert_eq!(dirname("/a/b/", 1), "/a");
        assert_eq!(dirname("a/b", 1), "a");
        assert_eq!(dirname("a.txt", 1), ".");
        // A root is its own parent; a relative path with nothing left is `.`.
        assert_eq!(dirname("/", 1), "/");
        assert_eq!(dirname("/a", 9), "/");
        assert_eq!(dirname("a/b", 9), ".");
        // PHP answers `''` here, which is not a directory anything can use.
        assert_eq!(dirname("", 1), ".");
    }

    /// PHP throws a `ValueError` for `levels < 1`; this answers the path
    /// itself, with its separators normalized. See the member's own docs.
    #[test]
    fn dirname_at_zero_levels_is_the_path_normalized() {
        assert_eq!(dirname("/a\\b/c", 0), "/a/b/c");
        assert_eq!(dirname("a//b/", 0), "a/b");
    }

    #[test]
    fn extension_is_absent_rather_than_empty() {
        assert_eq!(extension("a.txt").as_deref(), Some("txt"));
        assert_eq!(extension("/a/b/c.tar.gz").as_deref(), Some("gz"));
        assert_eq!(extension("plain"), None);
        assert_eq!(extension("/a/b/"), None);
        assert_eq!(extension(""), None);
        // The two divergences from `pathinfo`, stated as rows.
        assert_eq!(extension(".gitignore"), None);
        assert_eq!(extension("report."), None);
    }

    /// The member's reason for existing: it takes what `extension` answers.
    #[test]
    fn with_extension_is_extensions_inverse() {
        let set = |path: &str, extension: Value| {
            taken(
                run(super::nvs_core_path_with_extension, &[s(path), extension])
                    .expect("a valid extension never fails"),
            )
        };
        assert_eq!(set("/a/b/c.txt", s("md")), "/a/b/c.md");
        assert_eq!(set("/a/b/c", s("md")), "/a/b/c.md");
        assert_eq!(set("c.txt", Value::null()), "c");
        assert_eq!(set(".gitignore", s("txt")), ".gitignore.txt");
        // An interior dot is a usable extension even though `extension` would
        // then answer only its tail.
        assert_eq!(set("a.txt", s("tar.gz")), "a.tar.gz");
        // The path is rendered with this platform's separator throughout.
        assert_eq!(set("a\\b.txt", s("md")), "a/b.md");
    }

    #[test]
    fn with_extension_refuses_what_extension_never_answers() {
        for bad in [".txt", "", "a/b", "a\\b"] {
            let status = run(super::nvs_core_path_with_extension, &[s("a.txt"), s(bad)])
                .expect_err("a dotted, empty or split extension is refused");
            assert_eq!(status, nvs_runtime::THROWN, "{bad}");
        }
        for nameless in ["/", ""] {
            let status = run(
                super::nvs_core_path_with_extension,
                &[s(nameless), s("txt")],
            )
            .expect_err("a path with no name is refused");
            assert_eq!(status, nvs_runtime::THROWN, "{nameless}");
        }
    }

    #[test]
    fn split_leads_with_the_root_and_never_with_an_empty() {
        assert_eq!(split("/a/b"), ["/", "a", "b"]);
        assert_eq!(split("a/b"), ["a", "b"]);
        assert_eq!(split("/a//b/"), ["/", "a", "b"]);
        assert_eq!(split("/"), ["/"]);
        assert_eq!(split(r"C:\x\y"), ["C:/", "x", "y"]);
        assert!(split("").is_empty());
        assert!(split("///").len() == 1);
    }

    /// `join`, with its variadic tail built as the one array the ABI passes.
    fn join(base: &str, segments: &[&str]) -> String {
        let mut tail = NvsArray::new();
        for segment in segments {
            tail.append(s(segment));
        }
        taken(
            run(super::nvs_core_path_join, &[s(base), Value::array(tail)])
                .expect("join never fails"),
        )
    }

    /// `normalize`.
    fn normalize(path: &str) -> String {
        taken(run(super::nvs_core_path_normalize, &[s(path)]).expect("normalize never fails"))
    }

    /// `relativeTo`, as the `?string` it answers.
    fn relative_to(path: &str, base: &str) -> Option<String> {
        let result = run(super::nvs_core_path_relative_to, &[s(path), s(base)])
            .expect("relativeTo never fails");
        match result.tag() {
            Some(nvs_runtime::Tag::Null) => None,
            _ => Some(taken(result)),
        }
    }

    #[test]
    fn join_only_ever_appends() {
        assert_eq!(
            join("/var/www", &["html", "index.php"]),
            "/var/www/html/index.php"
        );
        assert_eq!(join("a", &[]), "a");
        assert_eq!(join("a/", &["", "b"]), "a/b");
        // The base decides the root; a segment's own is dropped rather than
        // allowed to replace it, which is what keeps a segment from escaping.
        assert_eq!(join("/var/www", &["/etc/passwd"]), "/var/www/etc/passwd");
        assert_eq!(join("/var", &[r"C:\windows"]), "/var/windows");
        // Not a launderer, and it does not pretend to be one.
        assert_eq!(join("/var/www", &["..", "etc"]), "/var/www/../etc");
    }

    /// The property `split`'s root-first element exists for.
    #[test]
    fn join_reassembles_what_split_took_apart() {
        for path in ["/var/www/index.php", "var/www", "/", r"C:\x\y"] {
            let parts = split(path);
            let (base, segments) = parts.split_first().expect("a non-empty path splits");
            let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
            assert_eq!(join(base, &borrowed), path.replace('\\', "/"), "{path}");
        }
    }

    #[test]
    fn normalize_resolves_dots_by_counting_components() {
        assert_eq!(normalize("/var/www/../log/./app.log"), "/var/log/app.log");
        assert_eq!(normalize("a//b/./c/"), "a/b/c");
        assert_eq!(normalize("./a"), "a");
        // A root has no parent, so a `..` there is dropped.
        assert_eq!(normalize("/a/../.."), "/");
        // A relative `..` that cannot be cancelled names something real.
        assert_eq!(normalize("a/../.."), "..");
        assert_eq!(normalize("../../a"), "../../a");
        assert_eq!(normalize(""), ".");
        assert_eq!(normalize("."), ".");
    }

    #[test]
    fn relative_to_walks_up_and_then_down() {
        assert_eq!(
            relative_to("/var/www/index.php", "/var/www").as_deref(),
            Some("index.php")
        );
        assert_eq!(
            relative_to("/var/log/app.log", "/var/www/html").as_deref(),
            Some("../../log/app.log")
        );
        assert_eq!(relative_to("/var/www", "/var/www").as_deref(), Some("."));
        assert_eq!(relative_to("a/b", "a").as_deref(), Some("b"));
        // A drive letter is the one component compared case-insensitively.
        assert_eq!(relative_to(r"C:\x\y", r"c:\x").as_deref(), Some("y"));
    }

    #[test]
    fn relative_to_answers_null_where_no_relative_path_exists() {
        // Different starting points, different roots, and a base this cannot
        // walk back into — the three shapes the member's own docs name.
        assert_eq!(relative_to("/var/www", "var/www"), None);
        assert_eq!(relative_to(r"C:\x", r"D:\x"), None);
        assert_eq!(relative_to("a", ".."), None);
    }

    #[test]
    fn is_absolute_reads_one_grammar_on_every_platform() {
        for absolute in ["/a", r"\a", "C:/log", r"C:\log", "/"] {
            assert!(is_absolute(absolute), "{absolute}");
        }
        for relative in ["a", "a/b", "", "C:log", "a:b"] {
            assert!(!is_absolute(relative), "{relative}");
        }
    }
}
