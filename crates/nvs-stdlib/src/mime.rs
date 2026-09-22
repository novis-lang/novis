//! `Core\Mime` — spec § 17's detection class: what a run of octets *is*, read
//! from the octets and from nothing else.
//!
//! **Why this is Tier 0 rather than an extension.** A type read off a file
//! extension is a type an attacker chose, and a program that acts on one — an
//! upload store deciding what to write, a response deciding what to send — is
//! deciding with the attacker's answer. Which question a program asks about
//! untrusted bytes is policy (`rule:core-api/tier-roster`), so the member that
//! answers it is always present and takes no name to be misled by.
//!
//! **A fixed table, and not libmagic's rule language.** libmagic's power is a
//! little language whose programs are data: offsets, indirections, arithmetic
//! on a value read out of the input, and a search that walks the file. That is
//! a parser for attacker-supplied bytes *and* an interpreter for a ruleset,
//! which is two attack surfaces to buy one answer. [`SIGNATURES`] is the whole
//! of what this class knows — a compiled-in list of literal octet runs at fixed
//! offsets, matched by comparison and nothing else — so detection is bounded,
//! total, and reads no further into the input than the widest signature.
//!
//! **The answer is a case, never a string a caller has to spell.** [`TYPE`] is
//! closed and its zero case is `Unknown`, so a program compares against
//! `Core\Mime\Type::Png` rather than against `"image/png"`, and a misspelling is
//! a compile error rather than a comparison that is quietly never true. The
//! IANA name exists as [`nvs_core_mime_media_type`]'s answer, one direction
//! only: a case can be spelled, and a spelling can never become a case.
//!
//! **Detection launders nothing** (`rule:security/launderers-are-sink-named`).
//! Bytes that detect as `image/png` are still `tainted`, because a signature
//! says what the first octets look like and says nothing about what the rest is
//! safe for — a PNG header sits in front of anything at all. So no parameter
//! here is [`Qual::Launder`], and there is no member on this class that takes
//! octets and answers text: the argument's octets have no route back out.
//!
//! **What this spends** (`rule:programs/memory-priority`): nothing that
//! outlives the call. `detect` copies no part of its argument and allocates
//! nothing; the answer is an integer.
//!
//! **What a prefix cannot say, this class does not say**, and where that shows
//! is the table's shape rather than a hole in it. A format serialized as text —
//! SVG, JSON, CSV, HTML, plain text — opens with whatever its author wrote, so
//! naming one from a prefix is guessing, and the members that decide such a
//! format decide it by parsing: `Core\Json::isValid` and `Core\Xml`'s reader.
//! A container whose contents name the format answers the *container*: `Zip`,
//! because what distinguishes `.docx`, `.xlsx`, `.odt` and `.jar` is a named
//! entry inside, which is `Core\Zip`'s reading; and `Ebml`, because what
//! distinguishes a WebM from a Matroska file is the `DocType` element, which is
//! a parse of the same kind. Spec § 17's row defers *what deliberately has no
//! case* to this doc ([01-core-library.md](/docs/spec/01-core-library.md) § 17),
//! and this is it: `Unknown` for a format with no signature, and a container
//! case for a format the signature reaches but cannot narrow.

use nvs_runtime::{Fault, NvsStr, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, its enum, and where its symbols live
// ============================================================================

/// `Core\Mime`'s fully-qualified name, written once so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Mime";

/// `Core\Mime\Type`'s fully-qualified name, for [`TYPE`] and for the parameter
/// and return position that carry one.
pub(crate) const TYPE_NAME: &str = r"Core\Mime\Type";

/// What a run of octets is, as far as its first octets can say.
///
/// **Closed, with `Unknown` as its zero case.** A detection that cannot say is
/// the ordinary answer rather than an error — most of what a program is handed
/// is text, and text has no signature — so the case that means "no signature in
/// [`SIGNATURES`] matched" is the one a caller gets by default and the one it
/// has to handle.
///
/// The integers are each case's own constant, per [`CoreEnum::cases`], and they
/// are ABI: [`Media::of_ordinal`] reads them back out of an argument slot, so a
/// case is appended and never inserted.
pub(crate) const TYPE: CoreEnum = CoreEnum {
    name: TYPE_NAME,
    cases: &[
        ("Unknown", 0),
        ("Png", 1),
        ("Jpeg", 2),
        ("Gif", 3),
        ("Webp", 4),
        ("Tiff", 5),
        ("Avif", 6),
        ("Heic", 7),
        ("Pdf", 8),
        ("Mp4", 9),
        ("Ogg", 10),
        ("Wav", 11),
        ("Zip", 12),
        ("Gzip", 13),
        ("Zstd", 14),
        ("Xz", 15),
        ("Wasm", 16),
        ("Ebml", 17),
    ],
    doc: Some(&TYPE_DOC),
};

/// [`TYPE`]'s reference card — `rule:core-api/reference-card`.
const TYPE_DOC: EnumDoc = EnumDoc {
    short: "What `Core\\Mime::detect` read out of a run of octets. Closed, so a program compares \
            against a case rather than against a media-type spelling that may never occur, and \
            `Unknown` is the answer for every format whose serialization is text.",
    cases: &[
        CaseDoc {
            name: "Unknown",
            desc: "No signature matched. The ordinary answer for text — JSON, CSV, SVG, HTML — \
                   and for octets that are nothing in particular.",
        },
        CaseDoc {
            name: "Png",
            desc: "`image/png`.",
        },
        CaseDoc {
            name: "Jpeg",
            desc: "`image/jpeg`.",
        },
        CaseDoc {
            name: "Gif",
            desc: "`image/gif`, both the 87a and 89a versions.",
        },
        CaseDoc {
            name: "Webp",
            desc: "`image/webp` — a RIFF container whose form is `WEBP`.",
        },
        CaseDoc {
            name: "Tiff",
            desc: "`image/tiff`, in either byte order.",
        },
        CaseDoc {
            name: "Avif",
            desc: "`image/avif`.",
        },
        CaseDoc {
            name: "Heic",
            desc: "`image/heic` — what a phone camera uploads.",
        },
        CaseDoc {
            name: "Pdf",
            desc: "`application/pdf`.",
        },
        CaseDoc {
            name: "Mp4",
            desc: "`video/mp4`, by its ISO base media brand.",
        },
        CaseDoc {
            name: "Ogg",
            desc: "`application/ogg` — the container, which carries audio and video alike and \
                   says which only inside.",
        },
        CaseDoc {
            name: "Wav",
            desc: "`audio/wav` — a RIFF container whose form is `WAVE`.",
        },
        CaseDoc {
            name: "Zip",
            desc: "`application/zip`, including every format that *is* a zip archive: `.docx`, \
                   `.xlsx`, `.odt`, `.jar`.",
        },
        CaseDoc {
            name: "Gzip",
            desc: "`application/gzip`, PHP's `gzencode` output and `Core\\Codec::Gzip`'s frame.",
        },
        CaseDoc {
            name: "Zstd",
            desc: "`application/zstd`, `Core\\Codec::Zstd`'s frame.",
        },
        CaseDoc {
            name: "Xz",
            desc: "`application/x-xz`.",
        },
        CaseDoc {
            name: "Wasm",
            desc: "`application/wasm`, a binary WebAssembly module.",
        },
        CaseDoc {
            name: "Ebml",
            desc: "`video/matroska`, the EBML container every `.mkv` and every `.webm` is — WebM \
                   being a Matroska profile. Which of the two a file is lives in its `DocType` \
                   element, which a signature cannot reach, so this case never narrows to \
                   `video/webm`.",
        },
    ],
};

/// `Core\Mime`'s registry rows — the detection, and the one direction in which
/// a case becomes a name.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[
        CoreMethod {
            name: "detect",
            names: &["data"],
            params: &[CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Enum(TYPE_NAME),
            symbol: "nvs_core_mime_detect",
            doc: Some(&DETECT_DOC),
        },
        CoreMethod {
            name: "mediaType",
            names: &["type"],
            params: &[CoreTy::Enum(TYPE_NAME)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_mime_media_type",
            doc: Some(&MEDIA_TYPE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Mime::detect`'s reference card — `rule:core-api/reference-card`.
const DETECT_DOC: MethodDoc = MethodDoc {
    short: "Reports what `$data` is, read from the octets themselves — replacing \
            `mime_content_type`, `finfo_buffer` and `exif_imagetype`. There is no parameter for a \
            file name, because a type read off an extension is a type an attacker chose.",
    params: &[ParamDoc {
        name: "data",
        desc: "The octets to inspect. Only the first few are read — the widest signature in the \
               table is sixteen — so a prefix of an upload answers what the whole of it would.",
        shape: &[],
    }],
    ret: "The case naming what the octets are, or `Core\\Mime\\Type::Unknown` where no signature \
          matched — which includes every format serialized as text. **Detection is not a \
          laundering**: bytes that detect as `image/png` are still `tainted`, because a signature \
          describes the first octets and says nothing about what the rest is safe for.",
    errors: &[],
};

/// `Core\Mime::mediaType`'s reference card — `rule:core-api/reference-card`.
const MEDIA_TYPE_DOC: MethodDoc = MethodDoc {
    short: "The IANA media type `$type` is spelled with, for a `Content-Type` header or a stored \
            record. One direction only: a case has a name, and a name never becomes a case.",
    params: &[ParamDoc {
        name: "type",
        desc: "The case to spell, usually one `detect` just answered.",
        shape: &[],
    }],
    ret: "The media type, as `image/png` — and `application/octet-stream` for \
          `Core\\Mime\\Type::Unknown`, which is what a program that must send something should \
          send for octets it could not name.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_mime_detect" => (nvs_core_mime_detect as *const ()).cast(),
        "nvs_core_mime_media_type" => (nvs_core_mime_media_type as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The table
// ============================================================================

/// One case of [`TYPE`], decoded from the integer an argument slot carries.
///
/// A Rust mirror of the source-visible enum rather than a reuse of it, for
/// [`crate::compress`]'s reason: [`TYPE`] is what the checker reads and this is
/// what the table is written in terms of, and [`Media::ordinal`] is the one
/// place the two are tied together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Media {
    /// No signature matched.
    Unknown,
    /// `image/png`.
    Png,
    /// `image/jpeg`.
    Jpeg,
    /// `image/gif`.
    Gif,
    /// `image/webp`.
    Webp,
    /// `image/tiff`.
    Tiff,
    /// `image/avif`.
    Avif,
    /// `image/heic`.
    Heic,
    /// `application/pdf`.
    Pdf,
    /// `video/mp4`.
    Mp4,
    /// `application/ogg`.
    Ogg,
    /// `audio/wav`.
    Wav,
    /// `application/zip`.
    Zip,
    /// `application/gzip`.
    Gzip,
    /// `application/zstd`.
    Zstd,
    /// `application/x-xz`.
    Xz,
    /// `application/wasm`.
    Wasm,
    /// `video/matroska` — an EBML container, Matroska or WebM alike.
    Ebml,
}

impl Media {
    /// The case's own constant, which is [`TYPE`]'s ordinal for it.
    ///
    /// An exhaustive `match` on purpose: a case added to [`Media`] and not
    /// numbered here fails to compile, which is the one direction of drift a
    /// test cannot catch earlier than the build does.
    fn ordinal(self) -> i64 {
        match self {
            Self::Unknown => 0,
            Self::Png => 1,
            Self::Jpeg => 2,
            Self::Gif => 3,
            Self::Webp => 4,
            Self::Tiff => 5,
            Self::Avif => 6,
            Self::Heic => 7,
            Self::Pdf => 8,
            Self::Mp4 => 9,
            Self::Ogg => 10,
            Self::Wav => 11,
            Self::Zip => 12,
            Self::Gzip => 13,
            Self::Zstd => 14,
            Self::Xz => 15,
            Self::Wasm => 16,
            Self::Ebml => 17,
        }
    }

    /// [`TYPE`]'s ordinal `raw` as a [`Media`], or `None` for an integer no case
    /// carries.
    fn of_ordinal(raw: i64) -> Option<Self> {
        match raw {
            0 => Some(Self::Unknown),
            1 => Some(Self::Png),
            2 => Some(Self::Jpeg),
            3 => Some(Self::Gif),
            4 => Some(Self::Webp),
            5 => Some(Self::Tiff),
            6 => Some(Self::Avif),
            7 => Some(Self::Heic),
            8 => Some(Self::Pdf),
            9 => Some(Self::Mp4),
            10 => Some(Self::Ogg),
            11 => Some(Self::Wav),
            12 => Some(Self::Zip),
            13 => Some(Self::Gzip),
            14 => Some(Self::Zstd),
            15 => Some(Self::Xz),
            16 => Some(Self::Wasm),
            17 => Some(Self::Ebml),
            _ => None,
        }
    }

    /// The IANA media type this case is spelled with.
    ///
    /// Every answer is a literal in this file, which is what makes
    /// [`nvs_core_mime_media_type`]'s `string` unable to carry anything from a
    /// caller's octets: there is no input it could be built out of.
    fn media_type(self) -> &'static str {
        match self {
            Self::Unknown => "application/octet-stream",
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Tiff => "image/tiff",
            Self::Avif => "image/avif",
            Self::Heic => "image/heic",
            Self::Pdf => "application/pdf",
            Self::Mp4 => "video/mp4",
            Self::Ogg => "application/ogg",
            Self::Wav => "audio/wav",
            Self::Zip => "application/zip",
            Self::Gzip => "application/gzip",
            Self::Zstd => "application/zstd",
            Self::Xz => "application/x-xz",
            Self::Wasm => "application/wasm",
            Self::Ebml => "video/matroska",
        }
    }
}

/// One entry of the table: the octet runs that must all be present, and what
/// their presence means.
///
/// A clause is `(offset, octets)` and is checked by comparison at that exact
/// offset. There is deliberately no *search*, no offset read out of the input
/// and no arithmetic — that is the half of libmagic's rule language this class
/// declines, and it is why matching is bounded by the table rather than by the
/// input's length.
struct Signature {
    /// Every clause must match for the signature to.
    clauses: &'static [(usize, &'static [u8])],
    /// What a match means.
    media: Media,
}

/// Every signature this class knows, in presentation order.
///
/// **First match wins, and no two entries can both match**: two signatures
/// either differ in their octets at a shared offset or ask about different
/// offsets of the same container, so the order here is for a reader and not a
/// precedence rule.
///
/// **No signature is shorter than three octets, and none is a run a text
/// document could plausibly open with.** That is why there is no `image/bmp`:
/// its whole signature is `BM`, and a two-octet signature over arbitrary input
/// is a coin flip. A detection that answers wrongly is worse than one that
/// answers `Unknown`, so the table would rather be short.
const SIGNATURES: &[Signature] = &[
    // Images. PNG's eight octets are the strongest signature here by some way:
    // the two line endings are there so a transfer that rewrites them corrupts
    // the header rather than the pixels.
    Signature {
        clauses: &[(0, b"\x89PNG\r\n\x1a\n")],
        media: Media::Png,
    },
    Signature {
        clauses: &[(0, b"\xff\xd8\xff")],
        media: Media::Jpeg,
    },
    Signature {
        clauses: &[(0, b"GIF87a")],
        media: Media::Gif,
    },
    Signature {
        clauses: &[(0, b"GIF89a")],
        media: Media::Gif,
    },
    // The two RIFF forms: the container is the same four octets and the form
    // is four more at offset 8, which is the only place either is decided.
    Signature {
        clauses: &[(0, b"RIFF"), (8, b"WEBP")],
        media: Media::Webp,
    },
    Signature {
        clauses: &[(0, b"RIFF"), (8, b"WAVE")],
        media: Media::Wav,
    },
    Signature {
        clauses: &[(0, b"II*\x00")],
        media: Media::Tiff,
    },
    Signature {
        clauses: &[(0, b"MM\x00*")],
        media: Media::Tiff,
    },
    // The ISO base media family, whose first four octets are a box length and
    // therefore say nothing: the type is `ftyp` at 4 and the brand at 8. The
    // brand list is what the table carries, and it grows by appending.
    Signature {
        clauses: &[(4, b"ftyp"), (8, b"avif")],
        media: Media::Avif,
    },
    Signature {
        clauses: &[(4, b"ftyp"), (8, b"heic")],
        media: Media::Heic,
    },
    Signature {
        clauses: &[(4, b"ftyp"), (8, b"mif1")],
        media: Media::Heic,
    },
    Signature {
        clauses: &[(4, b"ftyp"), (8, b"isom")],
        media: Media::Mp4,
    },
    Signature {
        clauses: &[(4, b"ftyp"), (8, b"mp42")],
        media: Media::Mp4,
    },
    Signature {
        clauses: &[(4, b"ftyp"), (8, b"avc1")],
        media: Media::Mp4,
    },
    Signature {
        clauses: &[(0, b"OggS")],
        media: Media::Ogg,
    },
    // EBML's header, which is where the answer stops: Matroska and WebM share
    // these four octets and are told apart by the `DocType` element further in,
    // so the case is the container the prefix identifies exactly.
    Signature {
        clauses: &[(0, b"\x1a\x45\xdf\xa3")],
        media: Media::Ebml,
    },
    Signature {
        clauses: &[(0, b"%PDF-")],
        media: Media::Pdf,
    },
    // Archives. The three zip records are an ordinary archive, an empty one and
    // a spanned one; all three are the same format and answer the same case.
    Signature {
        clauses: &[(0, b"PK\x03\x04")],
        media: Media::Zip,
    },
    Signature {
        clauses: &[(0, b"PK\x05\x06")],
        media: Media::Zip,
    },
    Signature {
        clauses: &[(0, b"PK\x07\x08")],
        media: Media::Zip,
    },
    // The third octet is the compression method, and deflate is the only one
    // gzip has ever defined — asking for it keeps this signature at three
    // octets rather than two.
    Signature {
        clauses: &[(0, b"\x1f\x8b\x08")],
        media: Media::Gzip,
    },
    Signature {
        clauses: &[(0, b"\x28\xb5\x2f\xfd")],
        media: Media::Zstd,
    },
    Signature {
        clauses: &[(0, b"\xfd7zXZ\x00")],
        media: Media::Xz,
    },
    Signature {
        clauses: &[(0, b"\x00asm")],
        media: Media::Wasm,
    },
];

/// What `data` is, as far as [`SIGNATURES`] can say.
///
/// Total: octets that match nothing are [`Media::Unknown`], which is an answer
/// rather than a failure. The walk reads at most the widest signature's span
/// and never the whole of `data`, so calling this on a large upload costs the
/// same as calling it on its first line.
pub(crate) fn detect_media(data: &[u8]) -> Media {
    for signature in SIGNATURES {
        let matched = signature.clauses.iter().all(|(at, octets)| {
            data.len() >= at + octets.len() && &data[*at..at + octets.len()] == *octets
        });
        if matched {
            return signature.media;
        }
    }
    Media::Unknown
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Mime::detect(bytes $data): Mime\Type` — replacing
    /// `mime_content_type`, `finfo_buffer` and `exif_imagetype`.
    ///
    /// An enum answers as its ordinal, exactly as a user-declared enum does
    /// (`rule:enums/closed-integer-type`).
    fn nvs_core_mime_detect(_ctx, args: [1]) {
        // unreachable from source: the parameter is `CoreTy::Blob`, so anything
        // that is not `bytes` is `E0401` at the call site.
        let Some(data) = args[0].as_bytes() else {
            return Err(Fault::fatal(format!(
                "Core\\Mime::detect expected a `bytes`, got tag {}",
                args[0].tag_byte()
            )));
        };
        Ok(Value::int(detect_media(data).ordinal()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Mime::mediaType(Mime\Type $type): string` — the case's IANA name,
    /// replacing `image_type_to_mime_type` and the arrays every framework keeps
    /// for the same job.
    ///
    /// The answer is a literal from [`Media::media_type`] and never anything
    /// built out of an argument, which is why a member handing back a `string`
    /// launders nothing: there is no caller input in it.
    fn nvs_core_mime_media_type(_ctx, args: [1]) {
        // unreachable from source: the parameter is `CoreTy::Enum(TYPE_NAME)`,
        // so an integer no case carries is `E0401: expected 'Core\Mime\Type'`
        // before the call runs.
        let Some(media) = args[0].as_int().and_then(Media::of_ordinal) else {
            return Err(Fault::fatal(format!(
                "Core\\Mime::mediaType expected a `Core\\Mime\\Type` case, got tag {} value {:?}",
                args[0].tag_byte(),
                args[0].as_int()
            )));
        };
        Ok(Value::str(NvsStr::new(media.media_type().as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Every case of [`TYPE`], as the Rust mirror and in ordinal order — the
    /// roster these tests sweep, so a case added to one and not the other fails
    /// here.
    const EVERY: [Media; 18] = [
        Media::Unknown,
        Media::Png,
        Media::Jpeg,
        Media::Gif,
        Media::Webp,
        Media::Tiff,
        Media::Avif,
        Media::Heic,
        Media::Pdf,
        Media::Mp4,
        Media::Ogg,
        Media::Wav,
        Media::Zip,
        Media::Gzip,
        Media::Zstd,
        Media::Xz,
        Media::Wasm,
        Media::Ebml,
    ];

    /// The shortest run of octets that satisfies every clause of `signature`,
    /// with zeros wherever the table asks about nothing.
    ///
    /// This is what makes the sweep below an assertion about the *table* rather
    /// than about a corpus somebody assembled: the input is derived from the
    /// entry, so an entry nobody wrote a fixture for is still swept.
    fn from_signature(signature: &Signature) -> Vec<u8> {
        let width = signature
            .clauses
            .iter()
            .map(|(at, octets)| at + octets.len())
            .max()
            .unwrap_or(0);
        let mut frame = vec![0_u8; width];
        for (at, octets) in signature.clauses {
            frame[*at..at + octets.len()].copy_from_slice(octets);
        }
        frame
    }

    /// The detection is from the octets, and there is no name for it to be
    /// misled by — Stage 3's first rule and spec § 17's row.
    ///
    /// A sweep over the whole table rather than a handful of fixtures, counted
    /// so an entry that stops matching cannot pass by being skipped: every
    /// signature detects as its own case, which also holds the table's
    /// first-match-wins order, since an entry shadowed by an earlier one would
    /// answer that one's case here.
    #[test]
    fn a_type_is_detected_from_magic_bytes_and_never_from_a_file_extension() {
        let mut detected = 0;
        for signature in SIGNATURES {
            let frame = from_signature(signature);
            assert_eq!(
                detect_media(&frame),
                signature.media,
                "{:?} did not detect its own signature",
                signature.media
            );
            detected += 1;
        }
        assert_eq!(detected, SIGNATURES.len());

        // The extension is never consulted because there is nowhere to put one:
        // every parameter on this class is octets or a case, so a program has
        // no way to hand a name in even by accident.
        for member in CLASS.members() {
            for param in member.params {
                assert!(
                    matches!(param, CoreTy::Blob(_) | CoreTy::Enum(_)),
                    "`{}::{}` takes something a file name could arrive as",
                    CLASS.name,
                    member.name
                );
            }
        }

        // And the two halves of that, asserted on octets: a name that says
        // `.png` is not a PNG, and a PNG is one whatever it was called.
        assert_eq!(detect_media(b"holiday.png"), Media::Unknown);
        let png = from_signature(&SIGNATURES[0]);
        assert_eq!(detect_media(&png), Media::Png);

        // A prefix too short for the signature it is a prefix of is `Unknown`
        // rather than a guess at the rest.
        assert_eq!(detect_media(&png[..4]), Media::Unknown);
        assert_eq!(detect_media(b""), Media::Unknown);
    }

    /// The answer is a case of a closed enum, `Unknown` is the case for "no
    /// signature matched", and nothing here answers a media type a caller would
    /// have to compare a string against.
    ///
    /// The mirror's variants are spelled exactly as the source cases, which is
    /// what lets `Debug` hold the two rosters together here rather than a third
    /// list of names that could drift from both.
    #[test]
    fn the_answer_is_a_closed_enum_plus_unknown_and_never_a_free_string() {
        let detect = CLASS
            .members()
            .find(|member| member.name == "detect")
            .expect("the class detects");
        let CoreTy::Enum(answers) = detect.return_ty else {
            panic!("`detect` answers {:?} rather than a case", detect.return_ty)
        };
        assert_eq!(answers, TYPE_NAME);

        assert_eq!(TYPE.cases.len(), EVERY.len());
        assert_eq!(
            TYPE.cases[0],
            ("Unknown", 0),
            "the zero case is the unknown"
        );
        for (ordinal, (name, value)) in TYPE.cases.iter().enumerate() {
            let mirror = EVERY[ordinal];
            assert_eq!(format!("{mirror:?}"), *name);
            assert_eq!(mirror.ordinal(), *value);
            assert_eq!(Media::of_ordinal(*value), Some(mirror));
        }

        // Closed in the direction that matters: an ordinal past the roster is
        // not a case, so nothing outside this list can arrive as one.
        let past = i64::try_from(EVERY.len()).expect("the roster fits an `i64`");
        assert_eq!(Media::of_ordinal(past), None);
        assert_eq!(Media::of_ordinal(-1), None);

        // The media type is reachable only from a case, never the other way:
        // `mediaType` takes the enum, and no member takes a string at all.
        let spelling = CLASS
            .members()
            .find(|member| member.name == "mediaType")
            .expect("a case can be spelled");
        assert!(matches!(spelling.params, [CoreTy::Enum(_)]));
        let names: BTreeSet<&str> = EVERY.iter().map(|case| case.media_type()).collect();
        assert_eq!(names.len(), EVERY.len(), "no two cases share a spelling");
        assert_eq!(Media::Unknown.media_type(), "application/octet-stream");
    }

    /// `rule:security/launderers-are-sink-named`: detection is not one of the
    /// members that can remove `tainted`, and this class has no route by which
    /// it could be.
    ///
    /// Asserted over the rows rather than over a call, because the property is
    /// a compile-time one: what launders is written in the parameter's
    /// classification, so a member that grew a [`Qual::Launder`] would still
    /// pass every test about what it answers.
    #[test]
    fn detected_bytes_are_still_tainted_because_detection_is_not_a_launderer() {
        let mut octets = 0;
        for member in CLASS.members() {
            for param in member.params {
                match param {
                    CoreTy::Blob(qual) => {
                        assert_eq!(
                            *qual,
                            Qual::Neutral,
                            "`{}::{}` takes octets as {qual:?}, which is not what detection does \
                             to them",
                            CLASS.name,
                            member.name
                        );
                        octets += 1;
                    }
                    CoreTy::Enum(_) => {}
                    other => panic!(
                        "`{}::{}` takes {other:?}, which this class has no member for",
                        CLASS.name, member.name
                    ),
                }
            }
        }
        assert_eq!(octets, 1, "`detect` is the one member that takes octets");

        // The structural half: the member that takes octets answers a case, and
        // the member that answers text takes no octets — so there is no member
        // on this class through which an argument's bytes could come back out
        // unqualified, whatever a future row's classification said.
        for member in CLASS.members() {
            let takes_octets = member
                .params
                .iter()
                .any(|param| matches!(param, CoreTy::Blob(_) | CoreTy::Bytes));
            let answers_data = matches!(
                member.return_ty,
                CoreTy::Str | CoreTy::Bytes | CoreTy::Text(_) | CoreTy::Blob(_)
            );
            assert!(
                !(takes_octets && answers_data),
                "`{}::{}` takes octets and answers data, which is a laundering shape",
                CLASS.name,
                member.name
            );
        }
    }

    /// An EBML container is `Ebml` whatever it holds, which is the whole of what
    /// the four-octet header says.
    ///
    /// The two frames differ only in the `DocType` element — the one place a
    /// Matroska file and a WebM file disagree — and the assertion is that the
    /// answer does *not* differ with it: a table that grew a rule for the octets
    /// past its widest signature would answer two cases here. The spelling is
    /// checked across the whole roster rather than on this case alone, so a
    /// `video/webm` arriving anywhere fails.
    #[test]
    fn an_ebml_container_is_reported_as_ebml() {
        // The header, its unknown-size length octet and the fields before the
        // `DocType` element (`\x42\x82`), which carries its own length and name.
        let container = |doc_type: &[u8]| {
            let mut frame = b"\x1a\x45\xdf\xa3\x01\x00\x00\x00\x00\x00\x00\x23\x42\x82".to_vec();
            frame.push(0x80 | u8::try_from(doc_type.len()).expect("a short `DocType`"));
            frame.extend_from_slice(doc_type);
            frame
        };
        let matroska = container(b"matroska");
        let webm = container(b"webm");
        assert_eq!(detect_media(&matroska), Media::Ebml);
        assert_eq!(detect_media(&webm), Media::Ebml);
        assert_eq!(detect_media(&matroska), detect_media(&webm));

        // Both sides of the bound the header is: the four octets answer, and
        // three of them are a prefix of nothing this table will guess at.
        assert_eq!(detect_media(&webm[..4]), Media::Ebml);
        assert_eq!(detect_media(&webm[..3]), Media::Unknown);

        // A case a caller cannot narrow is spelled as the container, and the
        // narrower spelling exists nowhere on the roster.
        assert_eq!(Media::Ebml.media_type(), "video/matroska");
        for case in EVERY {
            assert_ne!(case.media_type(), "video/webm");
        }
    }
}
