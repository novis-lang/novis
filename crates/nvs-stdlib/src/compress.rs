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
//! **Whole buffer here, and the incremental half is a different job.** This is
//! `Core\Xml`'s precedent applied deliberately rather than by omission: a
//! materialising member and a streaming one answer different questions, so they
//! are two surfaces stated as two and never one member with a mode. What is
//! written here is the whole-buffer pair, which is every row of the migration
//! table's *whole buffer* group. The incremental group — `deflate_init`,
//! `deflate_add`, `inflate_init` — is a `Core\Compress\Stream` object shaped
//! like [`crate::hash`]'s, and is this module's known gap 1 rather than a
//! second spelling of `decompress`.
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
//! # Known gaps
//!
//! 1. **The incremental surface is not written.** `deflate_init`/`deflate_add`
//!    and `inflate_init` map to a `Core\Compress\Stream` instance carrying the
//!    codec, the accumulated chunks and PHP's flush mode; the decompressing
//!    direction carries the same [`Bound`] as [`nvs_core_compress_decompress`]
//!    and charges every chunk against it, because a bound applied per call
//!    rather than per stream is not a bound.
//!    — owner: unowned

use std::io::Read;

use nvs_runtime::{Ctx, Fault, NvsStr, ThrownClass, Value};

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

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_compress_compress" => (nvs_core_compress_compress as *const ()).cast(),
        "nvs_core_compress_decompress" => (nvs_core_compress_decompress as *const ()).cast(),
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
fn data_of<'a>(args: &'a [Value], index: usize, member: &str) -> Result<&'a [u8], Fault> {
    // unreachable from source: both rows declare `bytes` or `bytes|string` in
    // this slot, so a third tag is `E0401` at the call site.
    args[index]
        .as_bytes()
        .or_else(|| args[index].as_str_bytes())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Compress::{member} expected a `bytes` or a `string`, got tag {}",
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
        let data = data_of(args, 0, "compress")?;
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
        let data = data_of(args, 0, "decompress")?;
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
    /// round-trip test.
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
        assert_eq!(enums, 2, "both members choose their format by enum case");
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
}
