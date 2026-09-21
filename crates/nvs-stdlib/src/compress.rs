//! `Core\Compress` — spec § 17's codec class, and the decompression bound that
//! cannot be switched off (`rule:core-classes/decompression-bound`,
//! [ADR 0166](/docs/decisions/0166.md)).
//!
//! **Why this is Tier 0 rather than a sandboxed component.** What a compressed
//! input can do to a server is *policy*, and policy must be non-optional
//! (`rule:core-api/tier-roster`). A sandboxed decoder would get a memory cap for
//! free and the ratio rule not at all, and PHP's shape — `$max_length`,
//! optional, defaulted to unlimited — is the one this class exists to replace.
//!
//! **One API, five codec cases, no name-as-string.** `Core\Codec` is a closed
//! enum for the reason `Core\Crypto` has no cipher-name-as-string: a misspelled
//! algorithm is a compile error rather than a run-time answer of `false`. The
//! five cases are what [02-php-migration.md](/docs/spec/02-php-migration.md)
//! § *Compression* names — `gzencode`, `gzcompress` and `gzdeflate` differ only
//! in which header wraps one deflate stream, and a class that offered four
//! would leave `gzdeflate` with no spelling at all.
//!
//! **Whole buffer, and two streams beside it.** This is `Core\Xml`'s precedent
//! applied deliberately rather than by omission: a materialising member and a
//! streaming one answer different questions, so they are two surfaces stated as
//! two and never one member with a mode. [`CLASS`]'s first pair is the
//! whole-buffer group, which is every row of the migration table's *whole
//! buffer* group; [`COMPRESSOR`] and [`DECOMPRESSOR`] are the incremental one —
//! `deflate_init`/`deflate_add` and `inflate_init`/`inflate_add` — and the
//! section below is what they are and what they cost.
//!
//! **What this spends** (`rule:programs/memory-priority`): the output buffer,
//! and nothing else that outlives the call. Every decoder here is driven as a
//! `Read`, so the bound is applied *while* the output grows rather than to a
//! buffer already allocated — the difference between refusing a bomb and
//! surviving one. The compressors hold their own window, which is the codec's
//! own constant and never a function of the input's size.
//!
//! **Nothing in the response path compresses implicitly**, and that is a shape
//! rather than a gap. Compression in either direction is on the closed list of
//! what a proxy does earlier and better
//! (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`), so a program
//! that wants a compressed body compresses it with these members and sets its
//! own header. The server negotiates no `Content-Encoding` and offers no output
//! handler for one to be registered against, which
//! `the_server_still_sets_no_content_encoding_of_its_own` pins.
//!
//! # The incremental surface, and why it is two classes
//!
//! `Core\Compress\Compressor` and `Core\Compress\Decompressor` carry the same
//! two members — `add` and `finish` — and differ in the one thing a program can
//! observe: what `finish` answers. A decompressor's octets are whatever the
//! frame said they were, so its `finish` answers `tainted bytes` as
//! `rule:security/tainted-sources` has every other object-shaped reader of
//! untrusted octets do; a compressor's are the program's own in another shape,
//! so its `finish` answers `bytes`. One class would have to declare one return
//! type for both, and an instance carries no qualifier for the two members to
//! pass one between them. Both readings of that single type are worse than
//! spending a class name: plain `bytes` launders a hostile archive, and `tainted
//! bytes` would leave a program that compresses its own data with
//! `Core\Taint::assertTrusted` as its only spelling, since `bytes` has no
//! checked `as` conversion to launder through
//! (`rule:security/taint-propagation`).
//!
//! **A compressed frame is not an injection payload**, which is why the
//! compressing half is not tainted by its input the way
//! [`nvs_core_compress_compress`] is. The octets a sink could misread are
//! recovered only by a decompression, and that answers `tainted` again — so the
//! qualifier is carried where it can be acted on rather than on a frame no sink
//! reads as text.
//!
//! **A stream holds its chunks, not a codec state.** This is [`crate::hash`]'s
//! wall and it is answered the same way: a `Core` instance's slots hold values
//! Novis already holds ([`crate::instance`]), a `flate2` or brotli coder is a
//! native object with no Novis spelling, and an instance has no destructor to
//! free one with. So `add` retains its argument into an array slot and `finish`
//! runs the backend once over the concatenation. PHP's `$flush_mode` therefore
//! has no spelling here, and that is a property of this shape rather than an
//! omission: every one of its constants means *emit what you have now*, and
//! nothing is emitted before `finish`.
//!
//! **The bound is the stream's, applied once over the whole of it.** A
//! decompressor resolves [`Bound`] when it is opened — the ask against the
//! operator's ceiling, exactly as [`nvs_core_compress_decompress`] does — keeps
//! the two resolved numbers in its own slots, and `finish` measures the total
//! output against `min(total input × ratio, bytes)`. A bound charged per `add`
//! would not be a bound: ten chunks would each be given the whole ceiling, so a
//! stream would decompress ten times what one call may
//! (`rule:core-classes/decompression-bound`).
//!
//! **What a stream spends** (`rule:programs/memory-priority`): one reference per
//! chunk `add` was handed, held until `finish` — no copy, since a retained
//! `NvsStr` is the caller's own buffer — plus, for the length of `finish`, one
//! concatenation of all of them, which is the input a second time. So a program
//! that streams to keep its footprint flat does not get that here: the cost is
//! the total fed, charged to the request's `[limits] memory` and released at
//! `finish` or with the request. That is AGENTS.md's priority 5 spent to buy
//! priority 4, and it is observably `deflate_add`/`inflate_add` either way,
//! which is what keeps the choice cheap to reverse — when a runtime tag owns a
//! native object with a release hook, these slots become that object and no
//! written program changes.

use std::io::Read;

use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, ObjHeader, ThrownClass, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc,
    ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, its enum, and where its symbols live
// ============================================================================

/// `Core\Compress`'s fully-qualified name, written once so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Compress";

/// `Core\Codec`'s fully-qualified name, for [`CODEC`] and for the parameter
/// that takes one.
pub(crate) const CODEC_NAME: &str = r"Core\Codec";

/// The format a `Core\Compress` member reads or writes.
///
/// **Five cases and not four**, because the migration table's `gzencode`,
/// `gzcompress` and `gzdeflate` rows name three distinct byte formats over one
/// deflate stream: a gzip header, a zlib header, and no header. A class
/// carrying "deflate" once would have to pick which of the last two it meant
/// and leave the other unwritable, which is how PHP ended up with three
/// function names for one algorithm.
///
/// The integers are each case's own constant, per [`CoreEnum::cases`], and they
/// are ABI: [`codec_of`] reads them back out of an argument slot, so a case is
/// appended and never inserted.
pub(crate) const CODEC: CoreEnum = CoreEnum {
    name: CODEC_NAME,
    cases: &[
        ("Gzip", 0),
        ("Zlib", 1),
        ("Deflate", 2),
        ("Brotli", 3),
        ("Zstd", 4),
    ],
    doc: Some(&CODEC_DOC),
};

/// [`CODEC`]'s reference card — `rule:core-api/reference-card`.
const CODEC_DOC: EnumDoc = EnumDoc {
    short: "The format a `Core\\Compress` member reads or writes. The first three are one deflate \
            stream under three different headers, which is what PHP spelled as three function \
            names; the last two are the other two `Content-Encoding` formats in use.",
    cases: &[
        CaseDoc {
            name: "Gzip",
            desc: "RFC 1952 — a deflate stream under a gzip header, PHP's `gzencode` and \
                   `Content-Encoding: gzip`.",
        },
        CaseDoc {
            name: "Zlib",
            desc: "RFC 1950 — the same stream under a zlib header, PHP's `gzcompress` and what \
                   `Content-Encoding: deflate` names on the wire.",
        },
        CaseDoc {
            name: "Deflate",
            desc: "RFC 1951 — the stream with no header at all, PHP's `gzdeflate`.",
        },
        CaseDoc {
            name: "Brotli",
            desc: "RFC 7932, `Content-Encoding: br`.",
        },
        CaseDoc {
            name: "Zstd",
            desc: "RFC 8878, `Content-Encoding: zstd`.",
        },
    ],
};

/// `bytes|string` — what [`nvs_core_compress_compress`] takes, for
/// [`crate::hash`]'s reason: a `string` is valid UTF-8 and therefore already a
/// valid byte sequence, so one reader covers both tags without copying.
///
/// `decompress` deliberately does **not** take this union. Its subject is a
/// compressed frame, which is never text, and admitting `string` there would
/// invite a program to hand it a `string` it had already lost the octets of.
///
/// Both spellings are [`Qual::Contagious`], which is the honest classification
/// on both members: compressing changes the octets' shape and nothing about
/// where they came from, so a `tainted` input answers a `tainted` frame
/// (`rule:security/taint-propagation`).
const DATA: &[CoreTy] = &[
    CoreTy::Blob(Qual::Contagious),
    CoreTy::Text(Qual::Contagious),
];

/// `Core\Compress`'s registry rows — § 17's whole-buffer pair.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "compress",
            names: &["data", "codec"],
            params: &[CoreTy::Union(DATA), CoreTy::Enum(CODEC_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_compress_compress",
            doc: Some(&COMPRESS_DOC),
        },
        CoreMethod {
            name: "decompress",
            names: &["data", "codec", "maxBytes", "maxRatio"],
            params: &[
                CoreTy::Blob(Qual::Contagious),
                CoreTy::Enum(CODEC_NAME),
                CoreTy::Uint,
                CoreTy::Uint,
            ],
            defaults: &[
                Const::Uint(DEFAULT_MAX_BYTES),
                Const::Uint(DEFAULT_MAX_RATIO),
            ],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_compress_decompress",
            doc: Some(&DECOMPRESS_DOC),
        },
        CoreMethod {
            name: "compressor",
            names: &["codec"],
            params: &[CoreTy::Enum(CODEC_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(COMPRESSOR_NAME),
            symbol: "nvs_core_compress_compressor",
            doc: Some(&COMPRESSOR_DOC),
        },
        CoreMethod {
            name: "decompressor",
            names: &["codec", "maxBytes", "maxRatio"],
            params: &[CoreTy::Enum(CODEC_NAME), CoreTy::Uint, CoreTy::Uint],
            defaults: &[
                Const::Uint(DEFAULT_MAX_BYTES),
                Const::Uint(DEFAULT_MAX_RATIO),
            ],
            return_ty: CoreTy::Instance(DECOMPRESSOR_NAME),
            symbol: "nvs_core_compress_decompressor",
            doc: Some(&DECOMPRESSOR_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Compress::compress`'s reference card — `rule:core-api/reference-card`.
const COMPRESS_DOC: MethodDoc = MethodDoc {
    short: "Compresses `$data` under `$codec`, replacing `gzencode`, `gzcompress`, `gzdeflate` and \
            `zlib_encode` at once — the format is a case of `Core\\Codec` rather than a third of a \
            function's name.",
    params: &[
        ParamDoc {
            name: "data",
            desc: "The octets to compress; a `string` is read as its UTF-8 bytes.",
            shape: &[],
        },
        ParamDoc {
            name: "codec",
            desc: "The format to write. `Core\\Codec::Gzip` is the one to reach for when the \
                   result crosses a wire.",
            shape: &[],
        },
    ],
    ret: "The compressed frame, as octets. Compressing is never refused for size: the output of a \
          compressor is bounded by its input.",
    errors: &[],
};

/// `Core\Compress::decompress`'s reference card — `rule:core-api/reference-card`.
const DECOMPRESS_DOC: MethodDoc = MethodDoc {
    short: "Decompresses `$data` under `$codec`, **under a bound that cannot be switched off** — \
            replacing `gzdecode`, `gzuncompress`, `gzinflate` and `zlib_decode`, whose `$max_length` \
            was optional and defaulted to unlimited.",
    params: &[
        ParamDoc {
            name: "data",
            desc: "The compressed frame. Tainted in, tainted out: decompressing tells you nothing \
                   about what the octets are safe for.",
            shape: &[],
        },
        ParamDoc {
            name: "codec",
            desc: "The format `$data` is in. Nothing is sniffed — a frame that is not this format \
                   is refused rather than guessed at.",
            shape: &[],
        },
        ParamDoc {
            name: "maxBytes",
            desc: "The most output this call will produce, in octets. A call may ask for less than \
                   `[limits] max_decompressed` and never for more; `0` is a bound of zero and not \
                   a spelling for unbounded.",
            shape: &[],
        },
        ParamDoc {
            name: "maxRatio",
            desc: "The most output per octet of input. The second half of the same bound: a bomb \
                   is small on the wire, so a byte ceiling alone is a ceiling a small request can \
                   still reach.",
            shape: &[],
        },
    ],
    ret: "The decompressed octets, never a truncation — a decompression that would pass either \
          bound throws instead of answering the prefix it had reached.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "`$data` is not a well-formed frame of `$codec`, or the output would pass either \
               half of the bound. Never an `IOError`: a hostile archive and a full disk are \
               different questions.",
    }],
};

/// `Core\Compress::compressor`'s reference card — `rule:core-api/reference-card`.
const COMPRESSOR_DOC: MethodDoc = MethodDoc {
    short: "Opens an incremental compression under `$codec`, as `deflate_init` does — a \
            `Core\\Compress\\Compressor` fed by `add` and closed by `finish`.",
    params: &[ParamDoc {
        name: "codec",
        desc: "The format to write, any `Core\\Codec` case. Fixed for the stream's whole life: a \
               frame is one format.",
        shape: &[],
    }],
    ret: "A fresh, open stream that has been fed nothing yet.",
    errors: &[],
};

/// `Core\Compress::decompressor`'s reference card — `rule:core-api/reference-card`.
const DECOMPRESSOR_DOC: MethodDoc = MethodDoc {
    short: "Opens an incremental decompression under `$codec`, as `inflate_init` does — a \
            `Core\\Compress\\Decompressor` fed by `add` and closed by `finish`, **under the same \
            bound that cannot be switched off**.",
    params: &[
        ParamDoc {
            name: "codec",
            desc: "The format the chunks are in. Nothing is sniffed — a frame that is not this \
                   format is refused rather than guessed at.",
            shape: &[],
        },
        ParamDoc {
            name: "maxBytes",
            desc: "The most output this whole stream will produce, in octets — resolved against \
                   `[limits] max_decompressed` here, at the opening, and charged once across every \
                   chunk rather than once per `add`.",
            shape: &[],
        },
        ParamDoc {
            name: "maxRatio",
            desc: "The most output per octet fed to the stream. The second half of the same bound, \
                   measured against everything `add` was handed.",
            shape: &[],
        },
    ],
    ret: "A fresh, open stream that has been fed nothing yet, carrying the bound it will be \
          measured against.",
    errors: &[],
};

/// [`COMPRESSOR`]'s name, written once — see [`NAME`].
pub(crate) const COMPRESSOR_NAME: &str = r"Core\Compress\Compressor";

/// The incremental compressing half — `deflate_init` and `deflate_add`.
///
/// Three slots and two members. `codec` is the [`CODEC`] case the stream was
/// opened with, as the integer the enum already is; `chunks` is every buffer
/// `add` has been handed, in order; `open` is `false` once `finish` has
/// answered. This module's own docs say why the state is the chunks rather than
/// a coder, why this is a class of its own beside [`DECOMPRESSOR`], and what
/// either one spends.
pub(crate) const COMPRESSOR: CoreClass = CoreClass {
    name: COMPRESSOR_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "add",
            names: &["data"],
            params: &[CoreTy::Union(DATA)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_compress_compressor_add",
            doc: Some(&COMPRESSOR_ADD_DOC),
        },
        CoreMethod {
            name: "finish",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_compress_compressor_finish",
            doc: Some(&COMPRESSOR_FINISH_DOC),
        },
    ],
    slots: &["codec", "chunks", "open"],
    constants: &[],
};

/// `$compressor->add`'s reference card — `rule:core-api/reference-card`.
const COMPRESSOR_ADD_DOC: MethodDoc = MethodDoc {
    short: "Feeds `$data` to the stream, as `deflate_add` does; the chunks are compressed in \
            order at `finish`.",
    params: &[ParamDoc {
        name: "data",
        desc: "The next octets; a `string` is read as its UTF-8 bytes.",
        shape: &[],
    }],
    ret: "Nothing. PHP's `deflate_add` answers whatever its flush mode let the coder emit, and \
          there is no such answer here: the frame is written whole at `finish`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The stream has already been finished — a frame is whole once it is written, so \
               open a new stream.",
    }],
};

/// `$compressor->finish`'s reference card — `rule:core-api/reference-card`.
const COMPRESSOR_FINISH_DOC: MethodDoc = MethodDoc {
    short: "Closes the stream and answers the frame for everything `add` fed it — the same \
            octets `Core\\Compress::compress` answers over the concatenation.",
    params: &[],
    ret: "The compressed frame, as `bytes`; the stream is finished afterwards and its chunks \
          released. Not tainted by its input, for the reason this module's docs give: a frame is \
          not a payload any sink reads as text.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The stream has already been finished — a second `finish` is refused rather than \
               answering a second frame.",
    }],
};

/// [`DECOMPRESSOR`]'s name, written once — see [`NAME`].
pub(crate) const DECOMPRESSOR_NAME: &str = r"Core\Compress\Decompressor";

/// The incremental decompressing half — `inflate_init` and `inflate_add`.
///
/// [`COMPRESSOR`]'s three slots and two more: `maxBytes` and `maxRatio` are the
/// [`Bound`] this stream was opened under, already resolved against the
/// operator's ceiling, so `finish` measures the whole stream against the numbers
/// the opening fixed and no later configuration read can raise them.
pub(crate) const DECOMPRESSOR: CoreClass = CoreClass {
    name: DECOMPRESSOR_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "add",
            names: &["data"],
            params: &[CoreTy::Blob(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_compress_decompressor_add",
            doc: Some(&DECOMPRESSOR_ADD_DOC),
        },
        CoreMethod {
            name: "finish",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedBytes,
            symbol: "nvs_core_compress_decompressor_finish",
            doc: Some(&DECOMPRESSOR_FINISH_DOC),
        },
    ],
    slots: &["codec", "chunks", "open", "maxBytes", "maxRatio"],
    constants: &[],
};

/// `$decompressor->add`'s reference card — `rule:core-api/reference-card`.
const DECOMPRESSOR_ADD_DOC: MethodDoc = MethodDoc {
    short: "Feeds `$data` to the stream, as `inflate_add` does; the chunks are decompressed in \
            order at `finish`, under the one bound the stream was opened with.",
    params: &[ParamDoc {
        name: "data",
        desc: "The next octets of the compressed frame. Nothing is decoded yet, so nothing here \
               is refused for size — the bound is the whole stream's and `finish` applies it.",
        shape: &[],
    }],
    ret: "Nothing. PHP's `inflate_add` answers whatever its flush mode let the decoder emit, and \
          a bound charged against such an answer would be a bound per call rather than per \
          stream.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The stream has already been finished — the octets are final, so open a new stream.",
    }],
};

/// `$decompressor->finish`'s reference card — `rule:core-api/reference-card`.
const DECOMPRESSOR_FINISH_DOC: MethodDoc = MethodDoc {
    short: "Closes the stream and answers everything `add` fed it, decompressed — the same octets \
            `Core\\Compress::decompress` answers over the concatenation, under the same bound.",
    params: &[],
    ret: "The decompressed octets as `tainted bytes`, never a truncation; the stream is finished \
          afterwards and its chunks released. Tainted because a frame's contents are whatever it \
          said they were, which is `rule:security/tainted-sources`' reading for every other reader \
          of untrusted octets.",
    errors: &[
        ErrorDoc {
            error: "ParseError",
            desc: "The chunks are not a well-formed frame of the stream's codec, or the output \
                   would pass either half of the bound the stream was opened under. The bound is \
                   the whole stream's: every chunk is charged against one ceiling.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The stream has already been finished — including by a `finish` that refused, \
                   since a refused frame is over.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_compress_compress" => (nvs_core_compress_compress as *const ()).cast(),
        "nvs_core_compress_decompress" => (nvs_core_compress_decompress as *const ()).cast(),
        "nvs_core_compress_compressor" => (nvs_core_compress_compressor as *const ()).cast(),
        "nvs_core_compress_decompressor" => (nvs_core_compress_decompressor as *const ()).cast(),
        "nvs_core_compress_compressor_add" => {
            (nvs_core_compress_compressor_add as *const ()).cast()
        }
        "nvs_core_compress_compressor_finish" => {
            (nvs_core_compress_compressor_finish as *const ()).cast()
        }
        "nvs_core_compress_decompressor_add" => {
            (nvs_core_compress_decompressor_add as *const ()).cast()
        }
        "nvs_core_compress_decompressor_finish" => {
            (nvs_core_compress_decompressor_finish as *const ()).cast()
        }
        _ => return None,
    })
}

// ============================================================================
// The bound
// ============================================================================

/// The output ceiling a call gets when it names none, and the value
/// `[limits] max_decompressed` itself defaults to — 64 MiB.
///
/// Large enough that no honest document reaches it and small enough that a
/// request holding one is still a request the machine can serve. The parameter
/// default and the configured ceiling are the same number on purpose: an
/// operator who has said nothing has said "the shipped bound", and one who has
/// written a smaller one has lowered every call that did not already ask for
/// less.
pub(crate) const DEFAULT_MAX_BYTES: u64 = 64 << 20;

/// The output-per-input ceiling a call gets when it names none, and the value
/// `[limits] max_decompression_ratio` itself defaults to.
///
/// 1000:1 passes every corpus a text document compresses to and stops the
/// class of input this bound exists for: the published zip bombs run from
/// 10^4:1 into 10^9:1, because they are built out of one repeated byte and a
/// real document is not.
pub(crate) const DEFAULT_MAX_RATIO: u64 = 1000;

/// `[limits] max_decompressed`, the byte half of the ceiling a call may lower
/// and may not raise.
const BYTES_KEY: &str = "max_decompressed";

/// `[limits] max_decompression_ratio`, the ratio half.
const RATIO_KEY: &str = "max_decompression_ratio";

/// What one `decompress` call may produce — the two numbers together, because
/// neither is a bound on its own.
///
/// A byte ceiling alone lets a 64 KiB request produce 64 MiB, which is the
/// amplification a bomb is for; a ratio alone lets a large upload produce a
/// proportionally large output with no absolute stop. `rule:core-classes/decompression-bound`
/// is the rule and [ADR 0166](/docs/decisions/0166.md) § 2 is why it is two
/// numbers rather than one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Bound {
    /// The most output, in octets.
    pub(crate) bytes: u64,
    /// The most output per octet of input.
    pub(crate) ratio: u64,
}

impl Bound {
    /// The operator's ceiling: `[limits] max_decompressed` and
    /// `[limits] max_decompression_ratio`, each falling back to its shipped
    /// default.
    ///
    /// **`false` does not turn either off.** `rule:config/three-changeability-classes`
    /// spells "no ceiling" that way for the limits a request may raise; this is
    /// not one of those, so an unbounded value reads as the shipped default and
    /// there is no configuration that removes the bound. A malformed value
    /// reads the same way, for the reason `Ctx`'s own readers answer their
    /// defaults: the file was parsed and refused once already, at the boundary
    /// that could name the line.
    pub(crate) fn ceiling(ctx: &Ctx) -> Self {
        Self {
            bytes: configured(ctx, BYTES_KEY, nvs_config::Unit::Bytes, DEFAULT_MAX_BYTES),
            ratio: configured(ctx, RATIO_KEY, nvs_config::Unit::Count, DEFAULT_MAX_RATIO),
        }
    }

    /// What a call asking for `asked` actually gets: the smaller of the two on
    /// each axis, independently.
    ///
    /// One direction only, which is the whole rule. Lowering is the caller's to
    /// do — a program that knows its input is a 4 KiB configuration file should
    /// say so — and raising is nobody's, because a ceiling a call can lift is a
    /// ceiling an attacker-shaped call can lift.
    pub(crate) fn within(self, asked: Self) -> Self {
        Self {
            bytes: asked.bytes.min(self.bytes),
            ratio: asked.ratio.min(self.ratio),
        }
    }

    /// The output ceiling for an input of `input` octets — the two numbers
    /// resolved into the one comparison a decode makes.
    pub(crate) fn output_ceiling(self, input: usize) -> u64 {
        let by_ratio = u64::try_from(input)
            .unwrap_or(u64::MAX)
            .saturating_mul(self.ratio);
        by_ratio.min(self.bytes)
    }
}

/// One `[limits]` directive as a quantity of `unit`, or `fallback` where the
/// configuration does not state a usable one.
///
/// The parse is `nvs_config::Quantity`'s and never this module's, so `"32M"`
/// means here what it means everywhere else
/// (`rule:config/ini-set-is-core-config-set`).
fn configured(ctx: &Ctx, key: &str, unit: nvs_config::Unit, fallback: u64) -> u64 {
    let Some(written) = ctx.config().and_then(|config| config.get(key)) else {
        return fallback;
    };
    let setting = nvs_config::Setting::Text(written);
    match nvs_config::Quantity::parse(key, unit, &setting) {
        Ok(nvs_config::Quantity::Bytes(value) | nvs_config::Quantity::Count(value)) => value,
        _ => fallback,
    }
}

// ============================================================================
// Reading arguments
// ============================================================================

/// One case of [`CODEC`], decoded from the integer an argument slot carries.
///
/// A Rust mirror of the source-visible enum rather than a reuse of it, for
/// [`crate::hash`]'s reason: [`CODEC`] is what the checker reads and this is
/// what dispatch matches on, and [`codec_of`] is the one place they are tied
/// together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Codec {
    /// RFC 1952 — deflate under a gzip header.
    Gzip,
    /// RFC 1950 — deflate under a zlib header.
    Zlib,
    /// RFC 1951 — deflate with no header.
    Deflate,
    /// RFC 7932 — brotli.
    Brotli,
    /// RFC 8878 — zstandard.
    Zstd,
}

impl Codec {
    /// The case's name as source spells it, for a diagnostic that has to say
    /// which format refused.
    fn spelled(self) -> &'static str {
        match self {
            Self::Gzip => "Gzip",
            Self::Zlib => "Zlib",
            Self::Deflate => "Deflate",
            Self::Brotli => "Brotli",
            Self::Zstd => "Zstd",
        }
    }
}

/// [`CODEC`]'s ordinal `raw` as a [`Codec`], or `None` for an integer no case
/// carries.
fn case_of(raw: Option<i64>) -> Option<Codec> {
    match raw? {
        0 => Some(Codec::Gzip),
        1 => Some(Codec::Zlib),
        2 => Some(Codec::Deflate),
        3 => Some(Codec::Brotli),
        4 => Some(Codec::Zstd),
        _ => None,
    }
}

/// The `Core\Codec` in slot `index`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot carrying something no case names — unreachable
/// from source, where the parameter's type is the enum.
fn codec_of(args: &[Value], index: usize, member: &str) -> Result<Codec, Fault> {
    // unreachable from source: the parameter is `CoreTy::Enum(CODEC_NAME)`, so
    // anything that is not a `Core\Codec` case is `E0401: expected
    // 'Core\Codec', found …` before the call runs.
    case_of(args[index].as_int()).ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Compress::{member} expected a `Core\\Codec` case, got tag {} value {:?}",
            args[index].tag_byte(),
            args[index].as_int()
        ))
    })
}

/// The `bytes|string` in slot `index` as octets — see [`DATA`].
///
/// `class` is spelled out rather than assumed, because the streams' `add` reads
/// its chunk through here and a message naming the wrong class is a message a
/// reader cannot grep for.
fn data_of<'a>(
    args: &'a [Value],
    index: usize,
    class: &str,
    member: &str,
) -> Result<&'a [u8], Fault> {
    // unreachable from source: every row declares `bytes` or `bytes|string` in
    // this slot, so a third tag is `E0401` at the call site.
    args[index]
        .as_bytes()
        .or_else(|| args[index].as_str_bytes())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{class}::{member} expected a `bytes` or a `string`, got tag {}",
                args[index].tag_byte()
            ))
        })
}

/// The `uint` in slot `index`, for [`codec_of`]'s reason.
fn uint_of(args: &[Value], index: usize, member: &str, position: &str) -> Result<u64, Fault> {
    // unreachable from source: the two bound parameters are `CoreTy::Uint`, so
    // a negative or non-integer argument is `E0401` and an omitted one is the
    // row's own `Const::Uint` default rather than an absent slot.
    args[index].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Compress::{member} expected a `uint` for {position}, got tag {}",
            args[index].tag_byte()
        ))
    })
}

// ============================================================================
// The codecs
// ============================================================================

/// `data` compressed under `codec`.
///
/// Every backend here is driven whole rather than incrementally, which is what
/// this module's *whole buffer* half means: the input is already a value the
/// request holds, so there is no window to stream over that would hold less.
pub(crate) fn compress_to(codec: Codec, data: &[u8]) -> Result<Vec<u8>, Fault> {
    let mut out = Vec::new();
    let wrote = match codec {
        Codec::Gzip => {
            flate2::read::GzEncoder::new(data, flate2::Compression::default()).read_to_end(&mut out)
        }
        Codec::Zlib => flate2::read::ZlibEncoder::new(data, flate2::Compression::default())
            .read_to_end(&mut out),
        Codec::Deflate => flate2::read::DeflateEncoder::new(data, flate2::Compression::default())
            .read_to_end(&mut out),
        Codec::Brotli => {
            // Quality 5 and a 22-bit window: the default quality 11 is roughly
            // an order of magnitude slower to encode for a few percent of size,
            // which is AGENTS.md's priority 3 against its priority 5 and not a
            // close call on a request path.
            brotli::CompressorReader::new(data, 4096, 5, 22).read_to_end(&mut out)
        }
        Codec::Zstd => {
            out = ruzstd::encoding::compress_to_vec(
                data,
                ruzstd::encoding::CompressionLevel::Fastest,
            );
            Ok(out.len())
        }
    };
    match wrote {
        Ok(_) => Ok(out),
        // unreachable from source, and not because a diagnostic gets there
        // first: these encoders read from a slice and write to a `Vec`, so
        // neither side has a failure mode and the only `io::Error` either could
        // produce is one the allocator would have aborted on. A `fatal` rather
        // than a throw for that reason — a compressor failing is a defect here,
        // not a property of the caller's data.
        Err(why) => Err(Fault::fatal(format!(
            "Core\\Compress::compress could not write a `Core\\Codec::{}` frame: {why}",
            codec.spelled()
        ))),
    }
}

/// The largest decoding window a `Core\Codec::Zstd` frame may ask for.
///
/// A bound the output ceiling cannot stand in for. A zstd frame names its
/// window in its header and the decoder holds that much memory before it reads
/// a block, so six octets of header are enough to ask this process for a
/// hundred megabytes — which is what the backend allows by default — and a
/// frame that then decompresses to nothing never reaches [`Bound`] at all. 8
/// MiB is the window every real encoder stays under at every level, and the
/// number ADR 0180 fixes for what one decode may hold.
const ZSTD_WINDOW: u64 = 8 << 20;

/// `data` decompressed under `codec`, refusing rather than truncating at
/// `bound` — `rule:core-classes/decompression-bound`.
///
/// The ceiling is applied *during* the decode and not after it: each decoder is
/// a `Read`, and the read stops one octet past what the bound allows, so a bomb
/// costs the ceiling and never the frame's declared size. Reading that one
/// extra octet is what tells a document that exactly fills the bound from one
/// that passes it.
///
/// `member` is what a refusal names. This is not `Core\Compress`'s decode
/// alone — a compressed HTTP reply is decoded here too, under the same bound —
/// and a program told to lower `$maxBytes` on a member it never called cannot
/// act on the message.
pub(crate) fn decompress_within(
    codec: Codec,
    data: &[u8],
    bound: Bound,
    member: &str,
) -> Result<Vec<u8>, Fault> {
    let ceiling = bound.output_ceiling(data.len());
    let stop = usize::try_from(ceiling.saturating_add(1)).unwrap_or(usize::MAX);
    let mut out = Vec::new();
    let read = match codec {
        Codec::Gzip => flate2::read::GzDecoder::new(data)
            .take(stop as u64)
            .read_to_end(&mut out),
        Codec::Zlib => flate2::read::ZlibDecoder::new(data)
            .take(stop as u64)
            .read_to_end(&mut out),
        Codec::Deflate => flate2::read::DeflateDecoder::new(data)
            .take(stop as u64)
            .read_to_end(&mut out),
        Codec::Brotli => brotli::Decompressor::new(data, 4096)
            .take(stop as u64)
            .read_to_end(&mut out),
        Codec::Zstd => {
            match ruzstd::decoding::StreamingDecoder::new_with_max_window_size(data, ZSTD_WINDOW) {
                Ok(decoder) => decoder.take(stop as u64).read_to_end(&mut out),
                Err(ruzstd::decoding::errors::FrameDecoderError::WindowSizeTooBig {
                    requested,
                    ..
                }) => return Err(over_window(requested, member)),
                Err(why) => return Err(malformed(codec, &why.to_string(), member)),
            }
        }
    };
    if let Err(why) = read {
        return Err(malformed(codec, &why.to_string(), member));
    }
    if out.len() as u64 > ceiling {
        return Err(over_bound(codec, data.len(), bound, ceiling, member));
    }
    Ok(out)
}

/// The refusal for a frame that is not what its codec case says it is.
///
/// A `ParseError` for `Core\Json::decode`'s reason — input did not match a
/// format this code declared — and specifically **not** an `IOError`, so a
/// caller cannot confuse a hostile input with a failing disk.
fn malformed(codec: Codec, why: &str, member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Parse,
        format!(
            "{member}: not a well-formed `Core\\Codec::{}` frame: {why}",
            codec.spelled()
        ),
    )
}

/// The refusal for a frame whose output passes the bound.
///
/// It names which half stopped it and what the other half was, because the
/// first question a caller asks is whether to raise their own argument or ask
/// the operator about `[limits]` — and only one of those two is ever the
/// answer.
fn over_bound(codec: Codec, input: usize, bound: Bound, ceiling: u64, member: &str) -> Fault {
    let half = if ceiling == bound.bytes {
        format!("the {} octet ceiling", bound.bytes)
    } else {
        format!("the {}:1 ratio over {input} octets of input", bound.ratio)
    };
    Fault::thrown_as(
        ThrownClass::Parse,
        format!(
            "{member}: a `Core\\Codec::{}` frame decompressing past {half} is refused rather than \
             truncated (rule:core-classes/decompression-bound). `[limits] max_decompressed` and \
             `[limits] max_decompression_ratio` are the operator's ceiling, which a call naming \
             `$maxBytes` or `$maxRatio` lowers and nothing raises.",
            codec.spelled()
        ),
    )
}

/// The refusal for a zstd frame asking for a window past [`ZSTD_WINDOW`].
///
/// A `ParseError` for [`malformed`]'s reason — it is a statement about the
/// input and not about this machine — and it names both numbers, because the
/// only thing a caller can do about it is know that the frame was built for a
/// decoder with more room than this one offers.
fn over_window(requested: u64, member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Parse,
        format!(
            "{member}: the `Core\\Codec::Zstd` frame asks for a {requested} octet decoding \
             window, past the {ZSTD_WINDOW} this process holds for one. Refused before the \
             window is allocated, because a frame's header alone asks for it \
             (rule:core-classes/decompression-bound)."
        ),
    )
}

/// One coding undone as its octets arrive, for a body that is not all here yet.
///
/// [`decompress_within`] is a slice in and a `Vec` out, which is the shape a
/// buffered body has and a streamed one never will: the reader feeding this is
/// still waiting on a socket. Each backend's decoder is already a `Read`, so
/// what this adds is a source that blocks, a decode step bounded by the
/// caller's buffer rather than by the frame, and the window cap that bounds a
/// streamed decode in place of an output ceiling.
///
/// **What it spends:** one decoding window, released with the reader — 32 KiB
/// for the three DEFLATE framings, which is that format's window; at most
/// [`ZSTD_WINDOW`] for zstd; and 16 MiB for brotli, which is that format's
/// largest window with the large-window extension off, as it is here. There is
/// no total-output ceiling and there cannot be one: a stream has no total, and
/// what bounds it instead is the `idle` and `maxDuration` of the socket it
/// arrives on (`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`).
pub(crate) struct Decoder {
    /// Which coding, for what a refusal names.
    codec: Codec,
    /// The member the call was made from, as [`decompress_within`]'s `member`.
    member: String,
    /// The decode, and the source until there is one.
    state: State,
}

/// A decoder is built at its first read rather than when it is asked for,
/// because a zstd one reads the frame header as it is constructed and that
/// header is on the socket with the rest of the body. Building all of them
/// that way is one shape rather than one per backend, and it keeps the head of
/// a streamed reply free of a wait for the body's first octets.
enum State {
    /// The source, with nothing read off it yet.
    Waiting(Option<Box<dyn Read>>),
    /// The decoder, reading.
    Running(Box<dyn Read>),
}

impl Decoder {
    /// A decode of `codec` over `source`, which `member` names in a refusal.
    pub(crate) fn over(codec: Codec, source: Box<dyn Read>, member: &str) -> Self {
        Self {
            codec,
            member: member.to_owned(),
            state: State::Waiting(Some(source)),
        }
    }

    /// What a refusal from this decode names.
    pub(crate) fn member(&self) -> &str {
        &self.member
    }

    /// The next decoded octets, at most `out.len()` of them, with `0` for the
    /// end of the frame.
    ///
    /// # Errors
    ///
    /// A `ParseError` for a frame that is not what its coding says it is, and
    /// for a zstd frame whose window passes [`ZSTD_WINDOW`]. A refusal the
    /// source itself raised arrives as one of those too, flattened by whatever
    /// the backend wrapped it in, so a caller that has a truer one kept
    /// elsewhere should prefer it.
    pub(crate) fn pull_into(&mut self, out: &mut [u8]) -> Result<usize, Fault> {
        let waiting = match &mut self.state {
            State::Waiting(source) => source.take(),
            State::Running(_) => None,
        };
        if let Some(source) = waiting {
            self.state = State::Running(built(self.codec, source, &self.member)?);
        }
        let State::Running(decoder) = &mut self.state else {
            // The one path that leaves a taken source behind is a frame
            // refused at its header, and a reader that has been refused is
            // over — so this is a caller reading past its own refusal.
            return Err(malformed(
                self.codec,
                "a frame already refused is not read again",
                &self.member,
            ));
        };
        decoder
            .read(out)
            .map_err(|why| malformed(self.codec, &why.to_string(), &self.member))
    }
}

/// The backend decoder for `codec`, reading `source`.
///
/// Every window this process will hold for one decode is decided here:
/// DEFLATE's is 32 KiB by the format, brotli's is 16 MiB by the same, and
/// zstd's is the only one a frame gets to ask for, which is why it is the only
/// one with a number beside it ([`ZSTD_WINDOW`]).
fn built(codec: Codec, source: Box<dyn Read>, member: &str) -> Result<Box<dyn Read>, Fault> {
    Ok(match codec {
        Codec::Gzip => Box::new(flate2::read::GzDecoder::new(source)),
        Codec::Zlib => Box::new(flate2::read::ZlibDecoder::new(source)),
        Codec::Deflate => Box::new(flate2::read::DeflateDecoder::new(source)),
        Codec::Brotli => Box::new(brotli::Decompressor::new(source, 4096)),
        Codec::Zstd => {
            match ruzstd::decoding::StreamingDecoder::new_with_max_window_size(source, ZSTD_WINDOW)
            {
                Ok(decoder) => Box::new(decoder),
                Err(ruzstd::decoding::errors::FrameDecoderError::WindowSizeTooBig {
                    requested,
                    ..
                }) => return Err(over_window(requested, member)),
                Err(why) => return Err(malformed(codec, &why.to_string(), member)),
            }
        }
    })
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Compress::compress(bytes|string $data, Codec $codec): bytes` —
    /// replacing `gzencode`, `gzcompress`, `gzdeflate` and `zlib_encode`.
    ///
    /// Total for every input and every case, which is why the card carries no
    /// errors: a compressor's output is bounded by its input, so there is
    /// nothing here for a bound to be about.
    fn nvs_core_compress_compress(_ctx, args: [2]) {
        let data = data_of(args, 0, NAME, "compress")?;
        let codec = codec_of(args, 1, "compress")?;
        Ok(Value::bytes(NvsStr::new(&compress_to(codec, data)?)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Compress::decompress(bytes $data, Codec $codec, uint $maxBytes = 67108864,
    /// uint $maxRatio = 1000): bytes` — replacing `gzdecode`, `gzuncompress`,
    /// `gzinflate` and `zlib_decode`, and their optional-and-unlimited
    /// `$max_length` with a bound that is neither.
    ///
    /// The two bound arguments are **asks**, not settings: what the decode runs
    /// under is [`Bound::within`], the smaller of the ask and what the operator
    /// configured, on each axis independently. A call therefore never has to be
    /// trusted, and a context that was never configured is bounded by the
    /// shipped defaults rather than by nothing.
    fn nvs_core_compress_decompress(ctx, args: [4]) {
        let data = data_of(args, 0, NAME, "decompress")?;
        let codec = codec_of(args, 1, "decompress")?;
        let asked = Bound {
            bytes: uint_of(args, 2, "decompress", "`$maxBytes`")?,
            ratio: uint_of(args, 3, "decompress", "`$maxRatio`")?,
        };
        let bound = Bound::ceiling(ctx).within(asked);
        Ok(Value::bytes(NvsStr::new(&decompress_within(
            codec,
            data,
            bound,
            "Core\\Compress::decompress()",
        )?)))
    }
}

// ============================================================================
// The streams — the incremental form
// ============================================================================

/// Either stream class's `codec` slot, by index.
const CODEC_SLOT: usize = 0;

/// Either stream class's `chunks` slot, by index.
const CHUNKS_SLOT: usize = 1;

/// Either stream class's `open` slot, by index.
const OPEN_SLOT: usize = 2;

/// [`DECOMPRESSOR`]'s `maxBytes` slot, by index.
const MAX_BYTES_SLOT: usize = 3;

/// [`DECOMPRESSOR`]'s `maxRatio` slot, by index.
const MAX_RATIO_SLOT: usize = 4;

/// The receiver of one of a stream's two members, with the codec it was opened
/// under read back out of it.
///
/// The two are decoded together because neither member has anything to do
/// without both, and because reading the `codec` slot is where a receiver that
/// is not one of that class's shows up.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is no object or whose `codec` slot
/// holds no [`CODEC`] ordinal — compiled code can produce neither — and a
/// [`Fault::thrown`] for a stream a `finish` has already closed, which is the
/// one of the three a program causes.
fn open_stream(
    value: Value,
    class: &CoreClass,
    member: &str,
) -> Result<(*mut ObjHeader, Codec), Fault> {
    let receiver = crate::instance::receiver(value, class, member)?;
    // unreachable from source: `compressor` and `decompressor` are the only
    // writers of this slot, and each decodes the ordinal before it writes it,
    // so an integer no `Core\Codec` case carries cannot be in one.
    let codec = case_of(crate::instance::slot(receiver, CODEC_SLOT).as_int()).ok_or_else(|| {
        Fault::fatal(format!(
            "{}::{member} received a stream whose `codec` slot is not one `Core\\Compress` wrote",
            class.name
        ))
    })?;
    if crate::instance::slot(receiver, OPEN_SLOT).as_bool() != Some(true) {
        return Err(Fault::thrown(format!(
            "{}::{member}(): this stream is finished — a frame is whole once it has been \
             answered, so open a new stream with Core\\Compress rather than reusing this one",
            class.name
        )));
    }
    Ok((receiver, codec))
}

/// One chunk retained into a stream's `chunks` slot.
///
/// **Retains its argument rather than copying it**, which is [`crate::hash`]'s
/// reading and its reason: the buffer is immutable-until-copied, so holding a
/// reference to the caller's own is both the cheap answer and the correct one.
fn add_chunk(args: &[Value], class: &CoreClass, member: &str) -> Result<Value, Fault> {
    let (receiver, _) = open_stream(args[0], class, member)?;
    // Read before the retain, so a tag that reached here leaves the stream
    // exactly as it found it.
    let _ = data_of(args, 1, class.name, member)?;
    #[expect(
        unsafe_code,
        reason = "the chunk array takes over a reference of its own, and the \
                  argument's belongs to the caller"
    )]
    unsafe {
        args[1].retain();
    }
    crate::identity_store::edit(receiver, CHUNKS_SLOT, class, member, |chunks| {
        chunks.append(args[1]);
    })?;
    Ok(Value::null())
}

/// Every chunk `add` retained, concatenated in the order it was handed them.
///
/// **What it spends:** the concatenation, so a `finish` holds the total fed
/// twice — which is why [`close`] runs on the result of this rather than after
/// the backend, releasing the chunks before a codec allocates anything.
///
/// # Errors
///
/// A [`Fault::fatal`] for a `chunks` slot holding anything but the array `add`
/// writes, which is this crate disagreeing with itself.
fn concatenated(
    receiver: *mut ObjHeader,
    class: &CoreClass,
    member: &str,
) -> Result<Vec<u8>, Fault> {
    let mut data: Vec<u8> = Vec::new();
    let chunks = crate::identity_store::borrow(receiver, CHUNKS_SLOT, class, member)?;
    let mut from = 0_usize;
    while let Some(slot) = chunks.next_slot(from) {
        let held = chunks
            .value_at(slot)
            .expect("next_slot only names live entries");
        // unreachable from source: `add` is the only writer of this slot and
        // its one parameter is `bytes` or `bytes|string`, so a chunk carries
        // `Tag::Bytes` or `Tag::Str` and the pair below answers for both.
        let octets = held
            .as_bytes()
            .or_else(|| held.as_str_bytes())
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{}::{member} found tag {} among its chunks, which only `add` writes",
                    class.name,
                    held.tag_byte()
                ))
            })?;
        data.extend_from_slice(octets);
        from = slot + 1;
    }
    Ok(data)
}

/// Closes `receiver` and releases the chunks it was holding.
///
/// Called before the backend runs rather than after it, so a `finish` that
/// refuses leaves a closed stream holding nothing: a refused frame is over, and
/// the octets that produced it are not worth holding for a second attempt that
/// would be refused the same way.
fn close(receiver: *mut ObjHeader) {
    crate::instance::set_slot(receiver, OPEN_SLOT, Value::bool(false));
    crate::identity_store::replace(receiver, CHUNKS_SLOT);
}

/// One of [`DECOMPRESSOR`]'s two bound slots, as the number the opening
/// resolved and wrote there.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot holding anything but a `uint`, which is this
/// crate disagreeing with itself.
fn bound_slot(receiver: *mut ObjHeader, slot: usize, member: &str) -> Result<u64, Fault> {
    // unreachable from source: `decompressor` is the only writer of either
    // slot, and writes a `uint` it has already read out of an argument.
    crate::instance::slot(receiver, slot)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{DECOMPRESSOR_NAME}::{member} expected a `uint` in its `{}` slot",
                DECOMPRESSOR.slots[slot]
            ))
        })
}

nvs_runtime::nvs_helper! {
    /// `Core\Compress::compressor(Codec $codec): Compress\Compressor` —
    /// replacing `deflate_init`, and the whole of PHP's deflate context.
    ///
    /// Takes the same `Core\Codec` [`nvs_core_compress_compress`] does, and
    /// fixes it for the stream's life: a frame is one format, so there is
    /// nothing a later `add` could choose.
    fn nvs_core_compress_compressor(_ctx, args: [1]) {
        // The ordinal rather than the decoded case, since a slot holds values
        // Novis holds — but decoded first, so a bad one is refused here rather
        // than at whichever `add` happens to read it back.
        let _ = codec_of(args, 0, "compressor")?;
        Ok(crate::instance::build(
            &COMPRESSOR,
            [args[0], Value::array(NvsArray::new()), Value::bool(true)],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Compress::decompressor(Codec $codec, uint $maxBytes = 67108864,
    /// uint $maxRatio = 1000): Compress\Decompressor` — replacing
    /// `inflate_init`, under the bound that cannot be switched off.
    ///
    /// **The bound is resolved here and stored resolved.** The two arguments are
    /// asks, as [`nvs_core_compress_decompress`]'s are, so what lands in the
    /// slots is [`Bound::within`]'s answer — the smaller of the ask and the
    /// operator's ceiling on each axis. Resolving at the opening rather than at
    /// `finish` is what makes the number the stream was opened under the number
    /// it is measured against, whatever the request does to its configuration in
    /// between.
    fn nvs_core_compress_decompressor(ctx, args: [3]) {
        let _ = codec_of(args, 0, "decompressor")?;
        let asked = Bound {
            bytes: uint_of(args, 1, "decompressor", "`$maxBytes`")?,
            ratio: uint_of(args, 2, "decompressor", "`$maxRatio`")?,
        };
        let bound = Bound::ceiling(ctx).within(asked);
        Ok(crate::instance::build(
            &DECOMPRESSOR,
            [
                args[0],
                Value::array(NvsArray::new()),
                Value::bool(true),
                Value::uint(bound.bytes),
                Value::uint(bound.ratio),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$compressor->add(bytes|string $data): void` — replacing `deflate_add`.
    fn nvs_core_compress_compressor_add(_ctx, args: [2]) {
        add_chunk(args, &COMPRESSOR, "add")
    }
}

nvs_runtime::nvs_helper! {
    /// `$compressor->finish(): bytes` — replacing the `ZLIB_FINISH` call every
    /// `deflate_add` sequence ends with, and answering the same octets
    /// [`nvs_core_compress_compress`] would over the concatenation.
    fn nvs_core_compress_compressor_finish(_ctx, args: [1]) {
        let (receiver, codec) = open_stream(args[0], &COMPRESSOR, "finish")?;
        let data = concatenated(receiver, &COMPRESSOR, "finish")?;
        close(receiver);
        Ok(Value::bytes(NvsStr::new(&compress_to(codec, &data)?)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$decompressor->add(bytes $data): void` — replacing `inflate_add`.
    fn nvs_core_compress_decompressor_add(_ctx, args: [2]) {
        add_chunk(args, &DECOMPRESSOR, "add")
    }
}

nvs_runtime::nvs_helper! {
    /// `$decompressor->finish(): tainted bytes` — the whole stream decoded
    /// under the whole stream's bound.
    ///
    /// One [`decompress_within`] over the concatenation, which is what makes the
    /// bound the stream's: the ratio half is measured against everything `add`
    /// was handed and the byte half against one ceiling, so feeding a frame in
    /// ten chunks buys exactly what feeding it in one does.
    fn nvs_core_compress_decompressor_finish(_ctx, args: [1]) {
        let (receiver, codec) = open_stream(args[0], &DECOMPRESSOR, "finish")?;
        let bound = Bound {
            bytes: bound_slot(receiver, MAX_BYTES_SLOT, "finish")?,
            ratio: bound_slot(receiver, MAX_RATIO_SLOT, "finish")?,
        };
        let data = concatenated(receiver, &DECOMPRESSOR, "finish")?;
        close(receiver);
        Ok(Value::bytes(NvsStr::new(&decompress_within(
            codec,
            &data,
            bound,
            "Core\\Compress\\Decompressor::finish()",
        )?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every case of [`CODEC`], as the Rust mirror — the roster these tests
    /// sweep, so a case added to one and not the other fails to compile here.
    const EVERY: [Codec; 5] = [
        Codec::Gzip,
        Codec::Zlib,
        Codec::Deflate,
        Codec::Brotli,
        Codec::Zstd,
    ];

    /// A bound roomy enough not to be the subject of a test that is about
    /// something else.
    const ROOMY: Bound = Bound {
        bytes: 1 << 20,
        ratio: 1 << 20,
    };

    /// A refusal's message, asserting on the way past that it is a
    /// `ParseError` — the goal's "a refusal is a diagnostic and never a failed
    /// I/O error", which every case below gets for free by reading its message
    /// through here.
    fn refusal(fault: &Fault) -> String {
        match fault {
            Fault::Thrown(ThrownClass::Parse, message) => message.to_string(),
            Fault::Thrown(class, message) => {
                panic!("a bound refusal is a `ParseError`, not {class:?}: {message}")
            }
            _ => panic!("a bound refusal is a throw a program can catch"),
        }
    }

    /// What these cases decode as, since every refusal names the member it is
    /// answering.
    const MEMBER: &str = "Core\\Compress::decompress()";

    /// A zstd frame's window is memory its header asks for and the decoder
    /// holds before it reads a block, so [`Bound`] never sees it — which is
    /// why [`ZSTD_WINDOW`] is a second number and why this case is written on
    /// a frame that has no blocks at all.
    ///
    /// Six octets are a whole header: the magic, a descriptor naming neither a
    /// content size nor a dictionary, and a window descriptor whose exponent
    /// of 14 asks for 16 MiB.
    #[test]
    fn a_zstd_frame_whose_window_passes_8_mib_is_refused_before_it_is_allocated() {
        let header = [0x28_u8, 0xB5, 0x2F, 0xFD, 0x00, 0x70];
        let why = refusal(
            &decompress_within(Codec::Zstd, &header, ROOMY, MEMBER)
                .expect_err("a window past the ceiling"),
        );
        assert!(
            why.contains("8388608"),
            "the refusal names the ceiling: {why}"
        );
        assert!(
            why.contains("16777216"),
            "and the window that was asked for: {why}"
        );

        // The ceiling is on the window a frame declares and not on what it
        // decompresses to, so the frames this class writes still round trip.
        let payload = vec![b'Z'; 64 << 10];
        let frame = compress_to(Codec::Zstd, &payload).expect("compresses");
        assert_eq!(
            decompress_within(Codec::Zstd, &frame, ROOMY, MEMBER)
                .expect("a frame under the window"),
            payload
        );
    }

    /// `rule:core-classes/decompression-bound`'s first half: the class is one
    /// API over every case, and each direction is the other's inverse.
    ///
    /// A sweep rather than five assertions, so a codec that answers plausibly
    /// on its own line still fails: the count is asserted, and the payload is
    /// one that every one of the five compresses differently.
    #[test]
    fn gzip_deflate_brotli_and_zstd_each_round_trip() {
        let payload: Vec<u8> = (0..4096_u32).map(|n| (n % 251) as u8).collect();
        let mut round_tripped = 0;
        for codec in EVERY {
            let frame = compress_to(codec, &payload).expect("compresses");
            assert!(
                !frame.is_empty(),
                "{:?} wrote an empty frame",
                codec.spelled()
            );
            let back = decompress_within(codec, &frame, ROOMY, MEMBER).expect("decompresses");
            assert_eq!(back, payload, "{} did not round trip", codec.spelled());
            round_tripped += 1;
        }
        assert_eq!(round_tripped, EVERY.len());
    }

    /// The codec is a closed enum in the *registry*, not a string the caller
    /// might misspell — `Core\Crypto`'s no-cipher-name-as-string on a second
    /// surface.
    ///
    /// Asserted over the rows rather than over a call, because the property is
    /// about what the checker will accept: a `string` parameter anywhere on
    /// this class is the defect, and a member that took one would still pass a
    /// round-trip test. The count it compares against is derived from the rows
    /// themselves, so a member added to the class either names its format
    /// `codec` and takes the enum, or fails here.
    // covers: Core\Codec
    #[test]
    fn the_codec_is_an_enum_and_there_is_no_name_as_string_spelling() {
        let mut enums = 0;
        for member in CLASS.members() {
            for (index, param) in member.params.iter().enumerate() {
                match param {
                    CoreTy::Enum(name) => {
                        assert_eq!(*name, CODEC_NAME);
                        assert_eq!(
                            member.names[index], "codec",
                            "the enum parameter is the one named `codec`"
                        );
                        enums += 1;
                    }
                    CoreTy::Text(_) | CoreTy::Str => {
                        panic!("`{}::{}` takes a string", CLASS.name, member.name)
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(
            enums,
            CLASS
                .members()
                .filter(|member| member.names.contains(&"codec"))
                .count(),
            "every member naming a format chooses it by enum case"
        );
        assert!(
            CODEC.cases.iter().any(|(name, _)| *name == "Deflate"),
            "raw deflate is a case, so `gzdeflate` has a spelling"
        );
    }

    /// The ratio half refuses, and what it refuses with is the whole output
    /// rather than the prefix it had reached.
    ///
    /// The payload is 64 KiB of one byte, which every codec here takes to well
    /// under a hundred octets, so a ratio of 2 is passed by all five and the
    /// sweep is the assertion.
    #[test]
    fn a_decompression_exceeding_the_ratio_throws_rather_than_truncating() {
        let bomb = vec![b'A'; 64 << 10];
        let mut refused = 0;
        for codec in EVERY {
            let frame = compress_to(codec, &bomb).expect("compresses");
            let bound = Bound {
                bytes: u64::MAX,
                ratio: 2,
            };
            let why = refusal(
                &decompress_within(codec, &frame, bound, MEMBER)
                    .expect_err("a 64 KiB expansion passes a 2:1 ratio"),
            );
            assert!(
                why.contains("rule:core-classes/decompression-bound"),
                "the refusal names the rule: {why}"
            );
            assert!(
                why.contains("ratio"),
                "the refusal names the half that stopped it: {why}"
            );
            refused += 1;
        }
        assert_eq!(refused, EVERY.len());
    }

    /// The absolute half refuses on its own, with the ratio wide open — so a
    /// large input cannot buy a proportionally large output.
    #[test]
    fn a_decompression_exceeding_the_absolute_ceiling_throws_rather_than_truncating() {
        let payload = vec![b'B'; 32 << 10];
        for codec in EVERY {
            let frame = compress_to(codec, &payload).expect("compresses");
            let bound = Bound {
                bytes: 1024,
                ratio: u64::MAX,
            };
            let why = refusal(
                &decompress_within(codec, &frame, bound, MEMBER).expect_err("32 KiB passes 1 KiB"),
            );
            assert!(
                why.contains("1024 octet ceiling"),
                "the refusal names the ceiling it hit: {why}"
            );

            // The last accepted value beside the first refused one: at exactly
            // the payload's size the same frame decompresses whole, so the
            // refusal above is the bound and not an off-by-one in the decoder.
            let exact = Bound {
                bytes: payload.len() as u64,
                ratio: u64::MAX,
            };
            assert_eq!(
                decompress_within(codec, &frame, exact, MEMBER)
                    .expect("the exact size is accepted"),
                payload
            );
        }
    }

    /// `rule:core-classes/decompression-bound`'s one-way rule, over both axes
    /// at once.
    ///
    /// [`Bound::within`] is the whole enforcement, so it is what the case asks:
    /// an ask below the ceiling is taken, an ask above it is not, and the two
    /// axes are resolved independently rather than by picking the "smaller"
    /// bound as a pair.
    #[test]
    fn a_call_may_lower_the_bound_and_may_not_raise_it_past_the_configured_ceiling() {
        let ceiling = Bound {
            bytes: 1 << 20,
            ratio: 100,
        };

        let lower = ceiling.within(Bound {
            bytes: 4096,
            ratio: 10,
        });
        assert_eq!(
            lower,
            Bound {
                bytes: 4096,
                ratio: 10
            }
        );

        let raised = ceiling.within(Bound {
            bytes: u64::MAX,
            ratio: u64::MAX,
        });
        assert_eq!(raised, ceiling, "an ask above the ceiling is the ceiling");

        let mixed = ceiling.within(Bound {
            bytes: 512,
            ratio: u64::MAX,
        });
        assert_eq!(
            mixed,
            Bound {
                bytes: 512,
                ratio: 100
            },
            "each axis is resolved on its own"
        );

        // And the clamp is what the decode runs under, not advice beside it.
        let frame = compress_to(Codec::Gzip, &vec![b'C'; 8192]).expect("compresses");
        let bound = Bound {
            bytes: 1024,
            ratio: u64::MAX,
        }
        .within(Bound {
            bytes: u64::MAX,
            ratio: u64::MAX,
        });
        decompress_within(Codec::Gzip, &frame, bound, MEMBER).expect_err("the ceiling still holds");
    }

    /// There is no value, in an argument or in `nvs.toml`, that means "no
    /// bound".
    ///
    /// Three spellings a caller might reach for, each asserted to be an
    /// ordinary bound rather than an escape: the largest `uint` is the
    /// configured ceiling, `0` is a bound of zero octets and not a sentinel,
    /// and `[limits] max_decompressed = false` — `rule:config/three-changeability-classes`'s
    /// spelling of "no ceiling" for the limits a request may raise — reads as
    /// the shipped default here.
    #[test]
    fn there_is_no_spelling_for_an_unbounded_decompression() {
        let frame = compress_to(Codec::Zlib, &vec![b'D'; 16 << 10]).expect("compresses");

        let ceiling = Bound {
            bytes: 4096,
            ratio: 4,
        };
        let widest = ceiling.within(Bound {
            bytes: u64::MAX,
            ratio: u64::MAX,
        });
        decompress_within(Codec::Zlib, &frame, widest, MEMBER)
            .expect_err("`uint`'s maximum is not an escape");

        let zero = Bound { bytes: 0, ratio: 0 };
        decompress_within(Codec::Zlib, &frame, zero, MEMBER).expect_err("zero is a bound of zero");

        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(
            "[limits]\nmax_decompressed = false\nmax_decompression_ratio = false\n",
        ));
        assert_eq!(
            Bound::ceiling(&ctx),
            Bound {
                bytes: DEFAULT_MAX_BYTES,
                ratio: DEFAULT_MAX_RATIO
            },
            "`false` does not remove this ceiling"
        );

        // And a context nobody configured is bounded by the same defaults
        // rather than by nothing.
        assert_eq!(
            Bound::ceiling(&Ctx::buffered()),
            Bound {
                bytes: DEFAULT_MAX_BYTES,
                ratio: DEFAULT_MAX_RATIO
            }
        );
    }

    /// Releases the references a case made for a call, which the callee
    /// borrows rather than takes.
    fn release(values: Vec<Value>) {
        #[expect(
            unsafe_code,
            reason = "every one of these is a reference this frame made, and \
                      no callee took one"
        )]
        unsafe {
            for value in values {
                value.release();
            }
        }
    }

    /// The member itself, driven the way compiled code drives it: the octets
    /// and the [`CODEC`] ordinal arrive in argument slots, and the frame that
    /// comes back is one `decompress` reads.
    ///
    /// Each row asks twice, once with `bytes` and once with the same octets as
    /// a `string`, because [`DATA`]'s union is only honest if both answer the
    /// same frame. The ordinal is the loop's index rather than a decoded case,
    /// so a row that decoded to a different codec would fail the round trip
    /// that follows it — which is the ABI [`case_of`] is written around.
    // covers: Core\Compress::compress
    #[test]
    fn the_member_writes_one_frame_for_bytes_and_for_the_same_text() {
        let mut ctx = Ctx::buffered();
        let text = "the quick brown fox jumps over the lazy dog\n".repeat(64);
        let payload = text.as_bytes();

        let mut agreed = 0_usize;
        for (ordinal, codec) in EVERY.into_iter().enumerate() {
            let case = Value::int(i64::try_from(ordinal).expect("five cases fit an `i64`"));
            let octets = Value::bytes(NvsStr::new(payload));
            let as_text = Value::str(NvsStr::new(payload));

            let from_octets =
                nvs_runtime::call(nvs_core_compress_compress, &mut ctx, &[octets, case])
                    .expect("a frame answers");
            let from_text =
                nvs_runtime::call(nvs_core_compress_compress, &mut ctx, &[as_text, case])
                    .expect("a frame answers");

            let frame = from_octets
                .as_bytes()
                .expect("the member answers `bytes`")
                .to_vec();
            let both_spellings = from_text.as_bytes() == Some(&frame[..]);
            let smaller = frame.len() < payload.len();
            let round_tripped =
                decompress_within(codec, &frame, ROOMY, MEMBER).is_ok_and(|back| back == payload);
            if both_spellings && smaller && round_tripped {
                agreed += 1;
            }

            release(vec![octets, as_text, from_octets, from_text]);
        }
        assert_eq!(
            agreed,
            EVERY.len(),
            "every case writes one frame for both spellings of `$data`"
        );
    }

    /// The member itself, on both sides of the bound it cannot be asked to
    /// drop.
    ///
    /// Every case is swept twice: an honest frame under the defaults the
    /// registry row carries, which answers the octets that went in, and the
    /// same frame with the two asks at `1`, which refuses. The second half is
    /// what makes the first one mean anything — a member that ignored its two
    /// arguments would pass the round trip alone.
    // covers: Core\Compress::decompress
    #[test]
    fn the_member_answers_an_honest_frame_and_refuses_one_past_the_ask() {
        let mut ctx = Ctx::buffered();
        // English text rather than one repeated byte: a buffer of one byte
        // compresses past `DEFAULT_MAX_RATIO` under every case here, so the
        // roomy half would be refused for the reason the tight half is.
        let payload = "the quick brown fox jumps over the lazy dog\n"
            .repeat(64)
            .into_bytes();

        let mut agreed = 0_usize;
        for (ordinal, codec) in EVERY.into_iter().enumerate() {
            let case = Value::int(i64::try_from(ordinal).expect("five cases fit an `i64`"));
            let frame = Value::bytes(NvsStr::new(
                &compress_to(codec, &payload).expect("compresses"),
            ));
            let roomy = [
                frame,
                case,
                Value::uint(DEFAULT_MAX_BYTES),
                Value::uint(DEFAULT_MAX_RATIO),
            ];
            let tight = [frame, case, Value::uint(1), Value::uint(1)];

            let whole = nvs_runtime::call(nvs_core_compress_decompress, &mut ctx, &roomy)
                .expect("an honest frame answers");
            let answered = whole.as_bytes() == Some(&payload[..]);

            let refused =
                nvs_runtime::call(nvs_core_compress_decompress, &mut ctx, &tight).is_err();
            let caught = ctx.take_pending().is_some_and(|why| {
                why.contains("rule:core-classes/decompression-bound") && why.contains(NAME)
            });
            if answered && refused && caught {
                agreed += 1;
            }

            release(vec![frame, whole]);
        }
        assert_eq!(
            agreed,
            EVERY.len(),
            "every case reads its own frame and refuses one past the ask"
        );
    }

    /// `Core\Compress::compressor` driven the way compiled code drives it: the
    /// ordinal arrives in an argument slot, and what comes back is a stream
    /// `add` and `finish` accept.
    ///
    /// Each case opens **two** streams and feeds them different chunks,
    /// because the one thing an opening can get wrong that a single stream
    /// would not show is handing out shared state. Each frame is compared
    /// against [`nvs_core_compress_compress`]'s over that stream's own chunks,
    /// so a stream carrying the other's octets fails here.
    // covers: Core\Compress::compressor
    #[test]
    fn the_member_opens_a_stream_of_its_own_for_every_case() {
        let mut ctx = Ctx::buffered();
        let left = b"the quick brown fox ".as_slice();
        let right = b"jumps over the lazy dog\n".as_slice();
        let joined = [left, right].concat();

        let mut agreed = 0_usize;
        for (ordinal, codec) in EVERY.into_iter().enumerate() {
            let case = Value::int(i64::try_from(ordinal).expect("five cases fit an `i64`"));
            let first = nvs_runtime::call(nvs_core_compress_compressor, &mut ctx, &[case])
                .expect("a stream opens");
            let second = nvs_runtime::call(nvs_core_compress_compressor, &mut ctx, &[case])
                .expect("a second stream opens");

            let head = Value::bytes(NvsStr::new(left));
            let tail = Value::bytes(NvsStr::new(right));
            let mut fed = |stream: Value, chunk: Value| {
                nvs_runtime::call(nvs_core_compress_compressor_add, &mut ctx, &[stream, chunk])
                    .expect("an open stream takes a chunk");
            };
            fed(first, head);
            fed(first, tail);
            fed(second, tail);

            let whole = nvs_runtime::call(nvs_core_compress_compressor_finish, &mut ctx, &[first])
                .expect("finishes");
            let alone = nvs_runtime::call(nvs_core_compress_compressor_finish, &mut ctx, &[second])
                .expect("finishes");

            let over_both =
                whole.as_bytes() == Some(&compress_to(codec, &joined).expect("compresses")[..]);
            let over_its_own =
                alone.as_bytes() == Some(&compress_to(codec, right).expect("compresses")[..]);
            if over_both && over_its_own {
                agreed += 1;
            }

            release(vec![first, second, head, tail, whole, alone]);
        }
        assert_eq!(
            agreed,
            EVERY.len(),
            "every case opens a stream holding its own chunks"
        );
    }

    /// `Core\Compress::decompressor` driven the way compiled code drives it,
    /// with the half of its contract no single `add` can show: the bound is
    /// the *stream's*, measured once over everything it was fed.
    ///
    /// Each case feeds one frame of 64 KiB of one byte in 256-octet pieces
    /// into a stream opened at a ratio of `1`, so a stream charging the bound
    /// per `add` would give the same frame sixty-odd separate allowances and
    /// answer. The honest half beside it is a second stream under the
    /// registry row's own defaults, which answers the octets that went in.
    // covers: Core\Compress::decompressor
    #[test]
    fn the_stream_measures_one_bound_over_every_piece_it_was_fed() {
        let mut ctx = Ctx::buffered();
        let payload = "the quick brown fox jumps over the lazy dog\n"
            .repeat(64)
            .into_bytes();

        let mut agreed = 0_usize;
        for (ordinal, codec) in EVERY.into_iter().enumerate() {
            let case = Value::int(i64::try_from(ordinal).expect("five cases fit an `i64`"));
            let frame = compress_to(codec, &payload).expect("compresses");

            let roomy = [
                case,
                Value::uint(DEFAULT_MAX_BYTES),
                Value::uint(DEFAULT_MAX_RATIO),
            ];
            let whole = nvs_runtime::call(nvs_core_compress_decompressor, &mut ctx, &roomy)
                .expect("a stream opens");
            let tight = [case, Value::uint(u64::MAX), Value::uint(1)];
            let drip = nvs_runtime::call(nvs_core_compress_decompressor, &mut ctx, &tight)
                .expect("a second stream opens");

            let mut chunks = Vec::new();
            for piece in frame.chunks(256) {
                let chunk = Value::bytes(NvsStr::new(piece));
                for stream in [whole, drip] {
                    nvs_runtime::call(
                        nvs_core_compress_decompressor_add,
                        &mut ctx,
                        &[stream, chunk],
                    )
                    .expect("an open stream takes a piece");
                }
                chunks.push(chunk);
            }

            let read = nvs_runtime::call(nvs_core_compress_decompressor_finish, &mut ctx, &[whole])
                .expect("an honest stream answers");
            let answered = read.as_bytes() == Some(&payload[..]);

            let refused =
                nvs_runtime::call(nvs_core_compress_decompressor_finish, &mut ctx, &[drip])
                    .is_err();
            let caught = ctx
                .take_pending()
                .is_some_and(|why| why.contains("rule:core-classes/decompression-bound"));
            if answered && refused && caught {
                agreed += 1;
            }

            chunks.push(whole);
            chunks.push(drip);
            chunks.push(read);
            release(chunks);
        }
        assert_eq!(
            agreed,
            EVERY.len(),
            "every case reads its own stream and refuses one past the bound"
        );
    }

    /// `$compressor->add` on the half a frame alone cannot show: a chunk is
    /// **retained** rather than copied, and every one of them is released when
    /// the stream closes.
    ///
    /// One buffer is fed to one stream a thousand times. A member copying its
    /// argument leaves that buffer's count where it found it; one that leaked
    /// a reference leaves it raised after `finish` answered. The frame beside
    /// those two counts is [`nvs_core_compress_compress`]'s over the thousand
    /// copies joined, so a stream that dropped or reordered a chunk fails here
    /// as well.
    // covers: Core\Compress\Compressor::add
    #[test]
    fn a_chunk_is_retained_once_per_add_and_released_when_the_stream_closes() {
        const FED: usize = 1_000;
        let mut ctx = Ctx::buffered();
        // [`CODEC`]'s first case, read from the roster rather than named twice,
        // as the ordinal a compiled call site passes.
        let case = Value::int(0);
        let codec = EVERY[0];

        let piece = b"order 1042 shipped to Berlin\n".as_slice();
        let chunk = Value::bytes(NvsStr::new(piece));
        let buffer = chunk
            .buffer_ptr()
            .expect("a `bytes` value carries a buffer");
        let stream = nvs_runtime::call(nvs_core_compress_compressor, &mut ctx, &[case])
            .expect("a stream opens");
        for _ in 0..FED {
            nvs_runtime::call(nvs_core_compress_compressor_add, &mut ctx, &[stream, chunk])
                .expect("an open stream takes a chunk");
        }

        #[expect(
            unsafe_code,
            reason = "this frame owns the reference `chunk` was built with, so \
                      the buffer is live across every read below"
        )]
        let while_open = unsafe { NvsStr::refcount_of(buffer) };
        let frame = nvs_runtime::call(nvs_core_compress_compressor_finish, &mut ctx, &[stream])
            .expect("finishes");
        #[expect(
            unsafe_code,
            reason = "the frame's own reference outlives `finish`, which released \
                      only what the stream was holding"
        )]
        let after_close = unsafe { NvsStr::refcount_of(buffer) };

        assert_eq!(
            frame.as_bytes(),
            Some(&compress_to(codec, &piece.repeat(FED)).expect("compresses")[..]),
            "the frame is the one `compress` writes over every chunk, in order"
        );
        assert_eq!(
            while_open,
            FED + 1,
            "every `add` retains the caller's buffer rather than copying it"
        );
        assert_eq!(after_close, 1, "`finish` releases every chunk it joined");

        release(vec![stream, chunk, frame]);
    }

    /// `$compressor->finish` on the two halves the frame comparison beside it
    /// cannot show: what a stream nobody fed answers, and the closing that
    /// makes a stream a one-shot.
    ///
    /// Every case finishes a stream given nothing, which must be the frame
    /// [`nvs_core_compress_compress`] writes for no octets rather than an
    /// empty `bytes`, and is then asked for a second frame and handed one more
    /// chunk. Both are refused, and the refusal is read back through
    /// [`Ctx::take_pending`] so a `Fault::fatal` in its place fails here — a
    /// program catches the one and cannot catch the other.
    // covers: Core\Compress\Compressor::finish
    #[test]
    fn an_empty_stream_answers_a_whole_frame_and_then_closes_for_good() {
        let mut ctx = Ctx::buffered();

        let mut agreed = 0_usize;
        for (ordinal, codec) in EVERY.into_iter().enumerate() {
            let case = Value::int(i64::try_from(ordinal).expect("five cases fit an `i64`"));
            let stream = nvs_runtime::call(nvs_core_compress_compressor, &mut ctx, &[case])
                .expect("a stream opens");

            let frame = nvs_runtime::call(nvs_core_compress_compressor_finish, &mut ctx, &[stream])
                .expect("an open stream finishes");
            let whole = frame.as_bytes() == Some(&compress_to(codec, b"").expect("compresses")[..]);

            let twice = nvs_runtime::call(nvs_core_compress_compressor_finish, &mut ctx, &[stream])
                .is_err();
            let said = ctx.take_pending().is_some_and(|why| {
                why.contains(r"Core\Compress\Compressor::finish(): this stream is finished")
            });

            let chunk = Value::bytes(NvsStr::new(b"after the end".as_slice()));
            let later =
                nvs_runtime::call(nvs_core_compress_compressor_add, &mut ctx, &[stream, chunk])
                    .is_err();
            let _ = ctx.take_pending();

            if whole && twice && said && later {
                agreed += 1;
            }

            release(vec![stream, chunk, frame]);
        }
        assert_eq!(
            agreed,
            EVERY.len(),
            "every case answers a whole frame for no octets and then closes for good"
        );
    }

    /// `$decompressor->add` on the half its opening's test does not ask: the
    /// member never looks inside the piece it is handed.
    ///
    /// Every case opens a stream with **no room at all** and feeds it
    /// sixty-four pieces of octets no codec can read. All sixty-four must
    /// answer, because nothing is decoded and nothing is measured until
    /// `finish`; a member reading its piece, or charging the bound as it went,
    /// would refuse somewhere in that loop instead.
    // covers: Core\Compress\Decompressor::add
    #[test]
    fn a_piece_is_taken_unread_and_the_refusal_waits_for_finish() {
        let mut ctx = Ctx::buffered();
        let junk = Value::bytes(NvsStr::new(b"not a frame in any format".as_slice()));

        let mut agreed = 0_usize;
        for ordinal in 0..EVERY.len() {
            let case = Value::int(i64::try_from(ordinal).expect("five cases fit an `i64`"));
            let stream = nvs_runtime::call(
                nvs_core_compress_decompressor,
                &mut ctx,
                &[case, Value::uint(0), Value::uint(0)],
            )
            .expect("a stream opens");

            let took = (0..64).all(|_| {
                nvs_runtime::call(
                    nvs_core_compress_decompressor_add,
                    &mut ctx,
                    &[stream, junk],
                )
                .is_ok()
            });
            let refused =
                nvs_runtime::call(nvs_core_compress_decompressor_finish, &mut ctx, &[stream])
                    .is_err();
            let _ = ctx.take_pending();

            if took && refused {
                agreed += 1;
            }

            release(vec![stream]);
        }
        release(vec![junk]);
        assert_eq!(
            agreed,
            EVERY.len(),
            "every case takes a piece unread and refuses only at `finish`"
        );
    }

    /// `$decompressor->finish` on the half a successful read cannot show: a
    /// refusal closes the stream exactly as an answer does.
    ///
    /// Every case is given a frame its stream has no room for, so the first
    /// `finish` throws the bound's refusal. The second `finish` and the `add`
    /// after it then meet the *finished stream's* own message rather than the
    /// bound's again, which is what says the refusing path ran [`close`] —
    /// a stream left open by a refusal would answer the bound here twice.
    // covers: Core\Compress\Decompressor::finish
    #[test]
    fn a_refused_finish_closes_the_stream_exactly_as_an_answer_does() {
        let mut ctx = Ctx::buffered();
        let payload = "the quick brown fox jumps over the lazy dog\n"
            .repeat(16)
            .into_bytes();

        let mut agreed = 0_usize;
        for (ordinal, codec) in EVERY.into_iter().enumerate() {
            let case = Value::int(i64::try_from(ordinal).expect("five cases fit an `i64`"));
            let frame = Value::bytes(NvsStr::new(
                &compress_to(codec, &payload).expect("compresses"),
            ));
            let stream = nvs_runtime::call(
                nvs_core_compress_decompressor,
                &mut ctx,
                &[case, Value::uint(8), Value::uint(DEFAULT_MAX_RATIO)],
            )
            .expect("a stream opens");
            nvs_runtime::call(
                nvs_core_compress_decompressor_add,
                &mut ctx,
                &[stream, frame],
            )
            .expect("an open stream takes a piece");

            let refused =
                nvs_runtime::call(nvs_core_compress_decompressor_finish, &mut ctx, &[stream])
                    .is_err();
            let bound = ctx
                .take_pending()
                .is_some_and(|why| why.contains("rule:core-classes/decompression-bound"));

            let twice =
                nvs_runtime::call(nvs_core_compress_decompressor_finish, &mut ctx, &[stream])
                    .is_err();
            let closed = ctx.take_pending().is_some_and(|why| {
                why.contains(r"Core\Compress\Decompressor::finish(): this stream is finished")
            });

            let later = nvs_runtime::call(
                nvs_core_compress_decompressor_add,
                &mut ctx,
                &[stream, frame],
            )
            .is_err();
            let _ = ctx.take_pending();

            if refused && bound && twice && closed && later {
                agreed += 1;
            }

            release(vec![stream, frame]);
        }
        assert_eq!(
            agreed,
            EVERY.len(),
            "a refusal closes the stream, so the call after it meets a finished one"
        );
    }
}
