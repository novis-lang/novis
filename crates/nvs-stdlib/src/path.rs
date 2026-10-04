//! `Core\Path` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 8, pure string algebra over paths.
//!
//! That section is authoritative for every signature. **No member touches the
//! disk**, so this module has no dependency at all — not `std::path`, whose
//! `Path`/`PathBuf` are a *host* abstraction that resolves differently on each
//! target and would drag `OsStr`'s non-UTF-8 question into a type `rule:types/bytes`
//! guarantees is text. Everything here is `&str` arithmetic. One member reads
//! the process rather than the disk: [`nvs_core_path_from_cwd`] asks
//! `nvs_runtime::capability::working_dir` for the working directory and then
//! joins with the same grammar as everything else.
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
//! # The three root shapes
//!
//! [`Root`] is the whole list of places a path can begin: nothing named — a
//! bare separator, or no root at all — a drive letter with a separator after
//! it (`C:\log`), or a UNC server and share (`\\server\share\f`). The doubled
//! separator belongs to the UNC root rather than being two empty components,
//! and the server and share belong to it rather than being names, so
//! `basename('\\server\share')` is the empty name every root has and every
//! member that re-renders a path renders the root back. What the third shape
//! spends is one more `&str` pair on the [`Parts`] a call already builds: a
//! stack value that dies with the call, per `rule:programs/memory-priority`.
//!
//! **`C:log` is not a root, and that is the bound this module states rather
//! than a shape it is missing.** Windows reads it as "the current directory
//! *on* drive C", but a drive is a root here only when a separator follows it
//! ([`split_drive`]), so `C:log` is one relative component whose name is
//! `C:log` and which round-trips as written. Reading `C:` as a root without
//! one would make `Path::split('a:b')` answer `['a:', 'b']` for an ordinary
//! relative path; losing the Windows-specific meaning is the cheaper of the
//! two.
//!
//! # What these members do with a qualifier
//!
//! `rule:security/unclassified-parameter-refuses-tainted`'s classification, and the one judgement in it worth writing
//! down: **`normalize` is not a launderer.** It is the member most likely to be
//! read as one — resolving `..` is exactly what stops a path climbing out of a
//! directory, so it *looks* like the thing that makes a `tainted` path safe —
//! and spec § 8 says outright that it is not. Its answer is still the caller's
//! bytes rearranged, and the sink it would have to launder for is a filesystem
//! nothing in this module opens. So it is `Qual::Contagious`, like every other
//! member that answers the caller's text, and `isAbsolute` is the class's only
//! `Qual::Neutral` row because a `bool` carries no byte of its subject.
//!
//! **`fromCwd` is the class's one launderer**, for the file-path sink, and it
//! checks nothing in the bytes. `rule:security/launderers-are-sink-named`
//! states it as that rule's one exception and why it holds: the member throws
//! while a request is answered, the user who typed the path can already open
//! what the process can, and the `fs` grants still bound every file the answer
//! names. `Qual::Launder` refuses a `secret` argument, so `tainted` is the only
//! qualifier it removes.

use nvs_runtime::{Fault, HelperResult, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreConst, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc,
    ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Path`'s registry rows, in the spec's own order — every member of
/// § 8, plus the `SEPARATOR` constant on [`CONSTANTS`].
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Path",
    doc: Some(&CARD),
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
        CoreMethod {
            name: "fromCwd",
            names: &["path"],
            // Plain text and not a path parameter: a path parameter's literal
            // is joined to the file that wrote it, and the whole point of this
            // member is the other base. A launderer for the file-path sink, so
            // a path from `Core\Cli::arguments` reaches a path door — the
            // module doc's § *What these members do with a qualifier*.
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_path_from_cwd",
            doc: Some(&FROM_CWD_DOC),
        },
        // The next two are folded while checking (`nvs_types::paths`'s
        // § *The file that wrote the call*): the call is replaced by the path
        // of the file that wrote it, and these symbols are reached only by a
        // source the compiler was handed as text, with no file behind it.
        CoreMethod {
            name: "thisFile",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_path_this_file",
            doc: Some(&THIS_FILE_DOC),
        },
        CoreMethod {
            name: "thisDir",
            names: &["join"],
            params: &[CoreTy::Nullable(&CoreTy::Text(Qual::Neutral))],
            defaults: &[Const::Null],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_path_this_dir",
            doc: Some(&THIS_DIR_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: CONSTANTS,
};

/// `Core\Path`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads and builds file paths as text. It never touches the disk. Every method accepts \
            `/` and `\\` as separators on every platform, and writes `Path::SEPARATOR`. This \
            replaces PHP's `basename`, `dirname`, `pathinfo` and the string work around \
            `DIRECTORY_SEPARATOR`.",
};

/// `Core\Path::basename`'s reference card — `rule:core-api/reference-card`.
const BASENAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the last part of `$path`: the file name, or the name of the last folder. A \
            separator at the end is ignored, so `/srv/shop/` gives `shop`. This replaces PHP's \
            `basename`.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The path. `/` and `\\` are both separators on every platform.",
            shape: &[],
        },
        ParamDoc {
            name: "withoutExtension",
            desc: "When `true`, the extension is removed, so `report.pdf` gives `report`. The \
                   default is `false`.",
            shape: &[],
        },
    ],
    ret: "The name. For a path that is only a root, such as `/` or `C:\\`, the result is the \
          empty string.",
    errors: &[],
};

/// `Core\Path::dirname`'s reference card — `rule:core-api/reference-card`.
const DIRNAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the folder that contains `$path`. The option `levels` goes up more than one \
            folder. This replaces PHP's `dirname`.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The path. `/` and `\\` are both separators on every platform.",
            shape: &[],
        },
        ParamDoc {
            name: "levels",
            desc: "How many parts to remove from the end. The default is `1`. With `0`, the result \
                   is the path with its separators cleaned up. A number larger than the path \
                   stops at the root.",
            shape: &[],
        },
    ],
    ret: "The folder, written with `Path::SEPARATOR`. When nothing is left, the result is the \
          root for an absolute path and `.` for a relative one. `dirname('')` is `.`, and PHP \
          returns `''`.",
    errors: &[],
};

/// `Core\Path::extension`'s reference card — `rule:core-api/reference-card`.
const EXTENSION_DOC: MethodDoc = MethodDoc {
    short: "Returns the extension of the last part of `$path`: the text after the last `.`, \
            without the dot. This replaces PHP's `pathinfo($path, PATHINFO_EXTENSION)`.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path. `/` and `\\` are both separators on every platform.",
        shape: &[],
    }],
    ret: "The extension, or `null` when there is none. A name that starts with a dot, such as \
          `.gitignore`, has none. A name that ends with a dot, such as `report.`, has none too. \
          PHP returns `gitignore` and `''` for these two.",
    errors: &[],
};

/// `Core\Path::withExtension`'s reference card — `rule:core-api/reference-card`.
const WITH_EXTENSION_DOC: MethodDoc = MethodDoc {
    short: "Returns `$path` with the extension of its last part changed to `$extension`. When \
            `$extension` is `null`, the extension is removed. PHP has no function for this.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The path. `/` and `\\` are both separators on every platform.",
            shape: &[],
        },
        ParamDoc {
            name: "extension",
            desc: "The new extension, without its dot, the same way `Core\\Path::extension` \
                   returns it. It may have a dot inside, such as `tar.gz`. `null` removes the \
                   extension.",
            shape: &[],
        },
    ],
    ret: "The new path, written with `Core\\Path::SEPARATOR`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$extension` is empty, starts with a `.`, or contains `/` or `\\`. Or `$path` has \
               no file name, such as `/` or `''`.",
    }],
};

/// `Core\Path::join`'s reference card — `rule:core-api/reference-card`.
const JOIN_DOC: MethodDoc = MethodDoc {
    short: "Adds each of `$segments` to the end of `$base`, with one separator between each \
            part. This replaces the `$a . \"/\" . $b` that PHP programs write. Only `$base` \
            decides where the result starts. A segment that starts with `/` or a drive is \
            added after `$base` and does not replace it.",
    params: &[
        ParamDoc {
            name: "base",
            desc: "The first path. The result starts where this path starts.",
            shape: &[],
        },
        ParamDoc {
            name: "segments",
            desc: "Any number of paths to add, in order. A leading `/`, `\\` or drive in one of \
                   them is removed.",
            shape: &[],
        },
    ],
    ret: "The joined path, written with `Core\\Path::SEPARATOR`. With no segments, the result \
          is `$base` with its extra separators removed. A `..` segment stays in the result and \
          still means the parent folder.",
    errors: &[],
};

/// `Core\Path::split`'s reference card — `rule:core-api/reference-card`.
const SPLIT_DOC: MethodDoc = MethodDoc {
    short: "Splits `$path` into its parts: the folders and the file name, in order. This \
            replaces PHP's `explode(\"/\", $path)`. `Core\\Path::join` puts the parts back \
            together into the same path.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path. `/` and `\\` are both separators on every platform.",
        shape: &[],
    }],
    ret: "The parts in order. When the path starts at a root, the root is the first part: `/`, \
          a drive such as `C:\\`, or a share such as `\\\\server\\share\\`. The root is written \
          with `Core\\Path::SEPARATOR`. No part is empty, because a repeated separator or one \
          at the end adds nothing. The empty path gives an empty array.",
    errors: &[],
};

/// `Core\Path::normalize`'s reference card — `rule:core-api/reference-card`.
const NORMALIZE_DOC: MethodDoc = MethodDoc {
    short: "Removes the `.` and `..` parts from `$path` and writes it with \
            `Core\\Path::SEPARATOR`. The method works on the text only and never looks at the \
            disk. The result can still point outside a folder, so it does not make a path \
            safe. `Core\\IO::within` checks that.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path. `/` and `\\` are both separators on every platform.",
        shape: &[],
    }],
    ret: "The same path without `.` parts, and with each `..` removing the folder before it. \
          A `..` at the start of a relative path stays. A `..` that would go above a root is \
          removed, because a root has no parent. A repeated separator or one at the end is \
          removed. The empty path gives `.`.",
    errors: &[],
};

/// `Core\Path::isAbsolute`'s reference card — `rule:core-api/reference-card`.
const IS_ABSOLUTE_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$path` starts at a root. A root is a separator, a drive followed \
            by a separator, or a network share. The check is the same on every platform.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path. `/` and `\\` are both separators on every platform.",
        shape: &[],
    }],
    ret: "`true` for `/tmp`, `\\tmp`, `C:/log` and `\\\\server\\share`. `false` for a \
          relative path such as `logs/app.log`, for `C:log`, and for the empty string.",
    errors: &[],
};

/// `Core\Path::relativeTo`'s reference card — `rule:core-api/reference-card`.
const RELATIVE_TO_DOC: MethodDoc = MethodDoc {
    short: "Returns the path that leads from the folder `$base` to `$path`. The `.` and `..` \
            parts of both paths are removed first, as `Core\\Path::normalize` does. PHP has \
            no function for this.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The place the result leads to.",
            shape: &[],
        },
        ParamDoc {
            name: "base",
            desc: "The folder the result starts from. The result has one `..` for each folder \
                   of `$base` that `$path` does not share.",
            shape: &[],
        },
    ],
    ret: "A relative path, written with `Core\\Path::SEPARATOR`. It is `.` when both paths \
          name the same place. It is `null` when no relative path exists: one path is \
          absolute and the other is not, the paths start at different roots, or `$base` \
          starts with more `..` parts than `$path` does. Folder names are compared byte for \
          byte. A drive letter and a server name ignore upper and lower case.",
    errors: &[],
};

/// `Core\Path::fromCwd`'s reference card — `rule:core-api/reference-card`.
const FROM_CWD_DOC: MethodDoc = MethodDoc {
    short: "Joins `$path` to the folder the program was started from, and returns the full \
            path. Use it for a path that a user typed on the command line.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path. A relative path is joined to the working folder. A full path is \
               returned as it is, with its `.` and `..` parts removed.",
        shape: &[],
    }],
    ret: "A full path, written with `Core\\Path::SEPARATOR`, with its `.` and `..` parts \
          removed. The method does not check that the file exists. The result is not \
          `tainted`, so you can pass a path from `Core\\Cli::arguments` to `Core\\IO::read`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The program is answering a web request. A server has no working folder that \
               belongs to the app.",
    }],
};

/// `Core\Path::thisFile`'s reference card — `rule:core-api/reference-card`.
const THIS_FILE_DOC: MethodDoc = MethodDoc {
    short: "Returns the full path of the source file that contains this call. The compiler \
            writes the path in place of the call, so the call costs nothing when the program \
            runs.",
    params: &[],
    ret: "The full path of the file, written with `Core\\Path::SEPARATOR`. In a bundled \
          program, the folder is the one beside the executable.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The source was given to the compiler as text, so there is no file to name.",
    }],
};

/// `Core\Path::thisDir`'s reference card — `rule:core-api/reference-card`.
const THIS_DIR_DOC: MethodDoc = MethodDoc {
    short: "Returns the full path of the folder that contains this source file. The compiler \
            writes the path in place of the call, so the call costs nothing when the program \
            runs.",
    params: &[ParamDoc {
        name: "join",
        desc: "A relative path written as a string literal, such as `'data'`. It is added to \
               the folder. A variable or a full path does not compile. Use \
               `Core\\Path::join(Core\\Path::thisDir(), $part)` for a part the program builds.",
        shape: &[],
    }],
    ret: "The full path of the folder, with `$join` added when you give one, and with its \
          `.` and `..` parts removed.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The source was given to the compiler as text, so there is no folder to name.",
    }],
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
        "nvs_core_path_from_cwd" => (nvs_core_path_from_cwd as *const ()).cast(),
        "nvs_core_path_this_file" => (nvs_core_path_this_file as *const ()).cast(),
        "nvs_core_path_this_dir" => (nvs_core_path_this_dir as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The grammar — one parse, shared by every member
// ============================================================================

/// The root a path begins at, where it names one — the module docs' three
/// shapes, of which this is the whole list.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Root<'a> {
    /// No named root. The path is relative (`a/b`) or begins at the current
    /// drive's root (`/a/b`), and `Parts::absolute` is what tells those apart.
    Unnamed,
    /// The `C:` of `C:\log`, without its separator.
    Drive(&'a str),
    /// The `\\server\share` of `\\server\share\f`, as the two names it is made
    /// of. The share is absent for `\\server` alone, which names a host and no
    /// export on it.
    Unc {
        server: &'a str,
        share: Option<&'a str>,
    },
}

/// One path, taken apart: its root, whether it begins at one, and every
/// non-empty component in order.
///
/// Borrowed from the subject rather than owned, since every member either
/// answers with one component or renders a fresh string; nothing here holds a
/// path past its own call.
///
/// **A named root implies `absolute`** — a drive is only [`split_drive`]'s
/// when a separator follows it, and a UNC root begins at two.
#[derive(Debug)]
pub(crate) struct Parts<'a> {
    /// Where the path begins, and under what name.
    pub(crate) root: Root<'a>,
    /// Whether the path begins at a root — a separator, or a drive's, or a
    /// UNC share's.
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
/// keeps an ordinary relative `a:b` from parsing as one and is the bound the
/// module docs state over `C:log`.
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

/// Whether `path` begins at a root, read from its first three bytes — the
/// answer [`parse`]'s `absolute` gives, without walking the components.
///
/// Every root begins with a separator, except a drive, and [`split_drive`]
/// recognizes a drive only when a separator follows it.
fn begins_at_root(path: &str) -> bool {
    path.starts_with(is_separator) || split_drive(path).is_some()
}

/// `path` split into the server and share of its UNC root and the rest that
/// follows them, or `None` where it does not begin at one.
///
/// A UNC root is **exactly two** leading separators followed by a non-empty
/// server name. Three or more is an ordinary absolute path — `\\\a` names `a`
/// under the root, not a host called nothing — and so is a bare `\\`.
fn split_unc(path: &str) -> Option<(&str, Option<&str>, &str)> {
    let doubled = path
        .strip_prefix(is_separator)
        .and_then(|rest| rest.strip_prefix(is_separator))?;
    let (server, after) = match doubled.find(is_separator) {
        Some(cut) => doubled.split_at(cut),
        None => (doubled, ""),
    };
    if server.is_empty() {
        return None;
    }
    let tail = after.trim_start_matches(is_separator);
    let (share, rest) = match tail.find(is_separator) {
        Some(cut) => tail.split_at(cut),
        None => (tail, ""),
    };
    Some((server, (!share.is_empty()).then_some(share), rest))
}

/// Takes one path apart. Total: every string is a path, including the empty
/// one, which is no root, not absolute and no components.
pub(crate) fn parse(path: &str) -> Parts<'_> {
    if let Some((server, share, rest)) = split_unc(path) {
        return Parts {
            root: Root::Unc { server, share },
            absolute: true,
            components: rest.split(is_separator).filter(|c| !c.is_empty()).collect(),
        };
    }
    let (root, rest) = match split_drive(path) {
        Some((drive, rest)) => (Root::Drive(drive), rest),
        None => (Root::Unnamed, path),
    };
    Parts {
        root,
        absolute: rest.starts_with(is_separator),
        components: rest.split(is_separator).filter(|c| !c.is_empty()).collect(),
    }
}

/// The root `parts` begins at, rendered with [`SEPARATOR`] — empty for a
/// relative path.
fn root(parts: &Parts<'_>) -> String {
    let mut out = String::new();
    match parts.root {
        Root::Unnamed => {}
        Root::Drive(drive) => out.push_str(drive),
        Root::Unc { server, share } => {
            out.push_str(SEPARATOR);
            out.push_str(SEPARATOR);
            out.push_str(server);
            if let Some(share) = share {
                out.push_str(SEPARATOR);
                out.push_str(share);
            }
        }
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
            root: Root::Unnamed,
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
    /// appended.** A segment's own leading separator and its own root are
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
    /// **An absolute path's first element is its root** — `'/'`, `'C:\'` or
    /// `'\\server\share\'`, rendered with [`SEPARATOR`]. That is what makes
    /// the decomposition lossless:
    /// `Core\Path::join(...Core\Path::split($p))` is `$p` with its
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
    /// True for a path beginning at a separator, at a drive followed by a
    /// separator, or at a UNC server. All three readings hold on **every**
    /// platform, which is this module's own docs' one-grammar rule; the spec's
    /// `Q` column marks this member neutral, and a lexical question about a
    /// string is exactly why.
    fn nvs_core_path_is_absolute(_ctx, args: [1]) {
        let path = text(&args[0], "isAbsolute", "the path")?;
        Ok(Value::bool(begins_at_root(path)))
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
    /// * the two begin at different roots — different drives, different UNC
    ///   shares, or a drive against a share;
    /// * `$base` still holds a `..` this would have to walk back *into*, and
    ///   the name of the directory it left is not in the input ([`walk`]).
    ///
    /// A component is compared **exactly**, byte for byte. Case-insensitivity
    /// is a property of a filesystem rather than of a path, and this member
    /// touches no filesystem. The exceptions are the two names resolved
    /// outside any filesystem: a drive letter, and a UNC server, which is a
    /// host name. A share name is the remote host's own, so it is compared
    /// like any other component.
    fn nvs_core_path_relative_to(_ctx, args: [2]) {
        let path = text(&args[0], "relativeTo", "the path")?;
        let base = text(&args[1], "relativeTo", "the base")?;

        let (target, origin) = (parse(path), parse(base));
        let same_root = match (target.root, origin.root) {
            (Root::Unnamed, Root::Unnamed) => true,
            (Root::Drive(here), Root::Drive(there)) => here.eq_ignore_ascii_case(there),
            (
                Root::Unc { server: here, share: ours },
                Root::Unc { server: there, share: theirs },
            ) => here.eq_ignore_ascii_case(there) && ours == theirs,
            _ => false,
        };
        if target.absolute != origin.absolute || !same_root {
            return Ok(Value::null());
        }
        match walk(&resolved(&target), &resolved(&origin)) {
            None => Ok(Value::null()),
            Some(components) => produced(&relative(&components)),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::fromCwd(string $path): string` — `realpath`'s use in a
    /// command-line tool, without the disk: a path the user typed, made
    /// absolute against the directory the program was started from.
    ///
    /// **The one member of this class that reads the process**, and it reads
    /// it through `nvs_runtime::capability::working_dir`, which throws while
    /// a request is being answered. The working directory is read first, so a
    /// request throws whatever the argument is, and that throw is what makes
    /// the row's `Qual::Launder` safe: request data never reaches the answer.
    /// `rule:programs/relative-paths-resolve-from-their-file` owns why nothing
    /// else resolves against that directory.
    ///
    /// The join is this module's grammar: the directory's text and the
    /// argument are one path to [`parse`], and [`resolved`] removes `.` and
    /// `..`, exactly as `normalize` does. An argument that begins at a root
    /// replaces the directory, as a second `join` segment would not.
    fn nvs_core_path_from_cwd(ctx, args: [1]) {
        const MEMBER: &str = r"Core\Path::fromCwd";

        let path = text(&args[0], "fromCwd", "the path")?;
        let cwd = nvs_runtime::capability::working_dir(ctx, MEMBER)?;
        if begins_at_root(path) {
            let parts = parse(path);
            return produced(&render(&parts, &resolved(&parts)));
        }
        let Some(cwd) = cwd.to_str() else {
            return Err(Fault::thrown(format!(
                "{MEMBER} cannot read the working directory as text, because its name is not \
                 valid UTF-8"
            )));
        };
        let joined = format!("{cwd}{SEPARATOR}{path}");
        let parts = parse(&joined);
        produced(&render(&parts, &resolved(&parts)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::thisFile(): string` — `__FILE__`, as a member the checker
    /// folds. `nvs_types::paths` replaces every call written in a file with
    /// that file's path, so this body runs only for a source the compiler was
    /// handed as text, which has no file to name.
    fn nvs_core_path_this_file(_ctx, _args: [0]) {
        // no case can reach this: every case is a file, so the checker folds
        // the call. `this_file_and_this_dir_throw_without_a_file` is what
        // asserts it instead.
        Err(Fault::thrown(
            r"Core\Path::thisFile has no file to name: this source was compiled from text",
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Path::thisDir(?string $join = null): string` — `__DIR__`, as a
    /// member the checker folds. [`nvs_core_path_this_file`]'s reason for a
    /// body that only throws.
    fn nvs_core_path_this_dir(_ctx, _args: [1]) {
        // no case can reach this: every case is a file, so the checker folds
        // the call. `this_file_and_this_dir_throw_without_a_file` is what
        // asserts it instead.
        Err(Fault::thrown(
            r"Core\Path::thisDir has no folder to name: this source was compiled from text",
        ))
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
    // covers: Core\Path::basename
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

    // covers: Core\Path::basename
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

    // covers: Core\Path::dirname
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
    // covers: Core\Path::dirname
    #[test]
    fn dirname_at_zero_levels_is_the_path_normalized() {
        assert_eq!(dirname("/a\\b/c", 0), "/a/b/c");
        assert_eq!(dirname("a//b/", 0), "a/b");
    }

    // covers: Core\Path::extension
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
    // covers: Core\Path::withExtension
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

    // covers: Core\Path::withExtension
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

    // covers: Core\Path::split
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

    // covers: Core\Path::join
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
    // covers: Core\Path::join
    #[test]
    fn join_reassembles_what_split_took_apart() {
        for path in [
            "/var/www/index.php",
            "var/www",
            "/",
            r"C:\x\y",
            r"\\server\share\f",
        ] {
            let parts = split(path);
            let (base, segments) = parts.split_first().expect("a non-empty path splits");
            let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
            assert_eq!(join(base, &borrowed), path.replace('\\', "/"), "{path}");
        }
    }

    // covers: Core\Path::normalize
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

    // covers: Core\Path::relativeTo
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
        // A drive letter is compared case-insensitively — a name no
        // filesystem owns, as a UNC server is.
        assert_eq!(relative_to(r"C:\x\y", r"c:\x").as_deref(), Some("y"));
    }

    // covers: Core\Path::relativeTo
    #[test]
    fn relative_to_answers_null_where_no_relative_path_exists() {
        // Different starting points, different roots, and a base this cannot
        // walk back into — the three shapes the member's own docs name.
        assert_eq!(relative_to("/var/www", "var/www"), None);
        assert_eq!(relative_to(r"C:\x", r"D:\x"), None);
        assert_eq!(relative_to("a", ".."), None);
    }

    // covers: Core\Path::isAbsolute
    #[test]
    fn is_absolute_reads_one_grammar_on_every_platform() {
        for absolute in ["/a", r"\a", "C:/log", r"C:\log", "/"] {
            assert!(is_absolute(absolute), "{absolute}");
        }
        for relative in ["a", "a/b", "", "C:log", "a:b"] {
            assert!(!is_absolute(relative), "{relative}");
        }
    }

    /// `isAbsolute` reads three bytes rather than parsing the whole path, so
    /// a long path costs the same as a short one; this pins that the short
    /// read agrees with the full parse every other member reads.
    // covers: Core\Path::isAbsolute
    #[test]
    fn is_absolute_agrees_with_the_full_parse() {
        for path in [
            "",
            "/",
            "\\",
            "a",
            "/a",
            r"\a",
            "C:",
            "C:/",
            r"C:\a",
            "C:a",
            "1:/a",
            "a:b",
            r"\\",
            r"\\\a",
            r"\\server",
            r"\\server\share\f",
            "//server/",
            ":",
            "é:/a",
        ] {
            assert_eq!(
                super::begins_at_root(path),
                super::parse(path).absolute,
                "{path}"
            );
            assert_eq!(is_absolute(path), super::parse(path).absolute, "{path}");
        }
    }

    /// The third root shape. A UNC server and share are the *root*, so the
    /// doubled separator survives every member that re-renders a path and the
    /// share is not a component a `..` can climb out of — which is the whole
    /// difference from parsing `\\server\share\f` as an ordinary absolute
    /// path.
    #[test]
    fn a_unc_path_round_trips_as_a_third_root_shape() {
        assert!(is_absolute(r"\\server\share\f"));
        assert_eq!(normalize(r"\\server\share\f"), "//server/share/f");
        assert_eq!(normalize("//server/share//a/./../f"), "//server/share/f");
        assert_eq!(split(r"\\server\share\f"), ["//server/share/", "f"]);
        // The root is where walking up stops, and it has no name of its own.
        assert_eq!(dirname(r"\\server\share\f", 1), "//server/share/");
        assert_eq!(normalize(r"\\server\share\..\.."), "//server/share/");
        assert_eq!(basename(r"\\server\share", false), "");
        // A server with no share is a root as well; three separators are an
        // ordinary absolute path rather than a host named nothing.
        assert_eq!(normalize(r"\\server"), "//server/");
        assert_eq!(split(r"\\\a"), ["/", "a"]);
        assert_eq!(normalize(r"\\"), "/");
        // The server is a host name and ignores ASCII case; the share is a
        // name the remote host owns, so it compares like any component, and
        // no UNC path is relative to a path under another root.
        assert_eq!(
            relative_to(r"\\server\share\a\b", r"\\SERVER\share\a").as_deref(),
            Some("b")
        );
        assert_eq!(relative_to(r"\\server\one\a", r"\\server\two\a"), None);
        assert_eq!(relative_to(r"\\server\share\a", "/a"), None);
    }

    /// `rule:security/launderers-are-sink-named`'s one exception, its first
    /// half. The row launders its one parameter and answers a plain `string`,
    /// so the checker gives a path typed on the command line to a path door.
    // covers: Core\Path::fromCwd
    #[test]
    fn from_cwd_returns_a_string_without_tainted() {
        use crate::registry::{CoreTy, Qual};

        let row = super::CLASS
            .methods
            .iter()
            .find(|method| method.name == "fromCwd")
            .expect("`Core\\Path::fromCwd` is registered");
        assert!(
            matches!(row.params, [CoreTy::Text(Qual::Launder)]),
            "the one parameter is a launderer, so its `tainted` does not reach the answer"
        );
        assert!(
            matches!(row.return_ty, CoreTy::Str),
            "the answer is a plain `string`: no path sink transforms a value on its own"
        );
    }

    /// The exception's second half, and what makes the first one safe: a
    /// context answering a request throws whatever the argument is, so
    /// request data never reaches the laundered answer.
    // covers: Core\Path::fromCwd
    #[test]
    fn from_cwd_throws_while_a_request_is_answered() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
        let path = s("report.txt");
        let result = call(super::nvs_core_path_from_cwd, &mut ctx, &[path]);
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built for the \
                      argument, and the helper borrowed rather than consumed it"
        )]
        unsafe {
            path.release();
        }
        assert!(result.is_err(), "a request throws before the join");
        let message = ctx.pending().expect("the throw is pending").into_owned();
        assert!(
            message.contains("cannot be used while answering a request"),
            "{message}"
        );
    }

    /// The two symbols behind the members the checker folds. They run only
    /// for a source with no file, and then each throws and names what is
    /// missing.
    // covers: Core\Path::thisFile
    // covers: Core\Path::thisDir
    #[test]
    fn this_file_and_this_dir_throw_without_a_file() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(super::nvs_core_path_this_file, &mut ctx, &[]);
        assert!(result.is_err(), "no file to name");
        let message = ctx.pending().expect("the throw is pending").into_owned();
        assert!(message.contains("has no file to name"), "{message}");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(super::nvs_core_path_this_dir, &mut ctx, &[Value::null()]);
        assert!(result.is_err(), "no folder to name");
        let message = ctx.pending().expect("the throw is pending").into_owned();
        assert!(message.contains("has no folder to name"), "{message}");
    }
}
