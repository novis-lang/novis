//! `Core\Mail\Attachment`, and the two readings of an HTML body that `send` makes: its plain text
//! and the `cid:` URLs it uses.
//!
//! An attachment is a value with its bytes in it. `fromFile` and `inlineFile` read the file when
//! they are called, under `fs.read`, so a missing file or a path the grant does not cover throws
//! on the line that named it, and the type is detected from the bytes at once. What that spends
//! (`rule:programs/memory-priority`): the file's bytes, held from that call until the value is
//! freed, and charged to the request. The message itself is composed in memory for the length of
//! the send, about 1.37 times the attachments' size once base64 encoded. Streaming a file into
//! `DATA` is declined for now: one path is simpler, and memory is last in the ordering.
//!
//! An inline attachment has a random Content-ID, made when it is built, so `src()` can return
//! its `cid:` URL before any message exists. [`check_cids`] then holds a message to two rules:
//! every `cid:` the HTML uses names an inline attachment of this message, and every inline
//! attachment is used. Either miss is a mail that shows a broken image or a stray file, so both
//! throw where they can still be fixed.
//!
//! [`read_html`] walks the WHATWG parse [`crate::html::parse`] gives, so the plain text a
//! `Core\Html\Markup` body gets is built from the same tree a browser would build.

use nvs_runtime::{Ctx, Fault, NvsStr, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
    Qual, TextOrigin,
};
use crate::xml::{Kind, Parsed};

/// The name of the class.
pub(crate) const ATTACHMENT_NAME: &str = r"Core\Mail\Attachment";

/// Slot 0: the file name the message gives the part.
const NAME_SLOT: usize = 0;
/// Slot 1: the media type, such as `image/png`.
const TYPE_SLOT: usize = 1;
/// Slot 2: the bytes, always `Tag::Bytes`.
const DATA_SLOT: usize = 2;
/// Slot 3: the Content-ID without angle brackets for an inline attachment, `null` for any other.
const CONTENT_ID_SLOT: usize = 3;

/// The longest file name an attachment takes, in bytes. Percent-encoded, it still fits a header
/// line under RFC 5322's 998 octets.
const LONGEST_NAME: usize = 255;

/// The longest media type an attachment takes: RFC 6838's two names of 127 and the `/`.
const LONGEST_TYPE: usize = 255;

/// The octets an attachment is made from: text or `bytes`, `tainted` included, because mailing a
/// file a user uploaded is an ordinary thing to do.
const DATA: CoreTy = CoreTy::Union(&[CoreTy::Text(Qual::Neutral), CoreTy::Blob(Qual::Neutral)]);

/// `fromFile`'s bag.
const FILE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "name",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "type",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
];

/// The bag of `fromBytes`, `inlineFile` and `inlineBytes`.
const TYPE_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "type",
    ty: CoreTy::Text(Qual::Neutral),
    default: Const::Null,
}];

/// `Core\Mail\Attachment` — a file sent with a message, or an image shown inside its HTML.
///
/// One class for both kinds, because both are passed in the one `attachments` option, and the
/// Content-ID slot is which kind it is. `src()` on an attachment that is not inline throws.
// rank: 882
pub(crate) const ATTACHMENT: CoreClass = CoreClass {
    name: ATTACHMENT_NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "fromFile",
            names: &["path"],
            // A sink, as every path in `Core\IO` is: `..` and the separators direct the
            // resolver, so a `tainted` path is refused where it is written.
            params: &[CoreTy::Path(Qual::Sink), CoreTy::Options(FILE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(ATTACHMENT_NAME),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_mail_attachment_from_file",
            doc: Some(&FROM_FILE_DOC),
        },
        CoreMethod {
            name: "fromBytes",
            names: &["name", "data"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                DATA,
                CoreTy::Options(TYPE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(ATTACHMENT_NAME),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_mail_attachment_from_bytes",
            doc: Some(&FROM_BYTES_DOC),
        },
        CoreMethod {
            name: "inlineFile",
            names: &["path"],
            params: &[CoreTy::Path(Qual::Sink), CoreTy::Options(TYPE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(ATTACHMENT_NAME),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_mail_attachment_inline_file",
            doc: Some(&INLINE_FILE_DOC),
        },
        CoreMethod {
            name: "inlineBytes",
            names: &["name", "data"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                DATA,
                CoreTy::Options(TYPE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(ATTACHMENT_NAME),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_mail_attachment_inline_bytes",
            doc: Some(&INLINE_BYTES_DOC),
        },
    ],
    instance: &[
        CoreMethod {
            name: "src",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            text: TextOrigin::Clean("A `cid:` URL with random hex digits."),
            symbol: "nvs_core_mail_attachment_src",
            doc: Some(&SRC_DOC),
        },
        // The name, the type and the bytes can come from a request, so each is read back as
        // `tainted`: a reader never removes a qualifier.
        reader(
            "name",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_mail_attachment_name",
            &NAME_DOC,
        ),
        reader(
            "type",
            CoreTy::Str,
            "nvs_core_mail_attachment_type",
            &TYPE_DOC,
        ),
        reader(
            "bytes",
            CoreTy::Bytes,
            "nvs_core_mail_attachment_bytes",
            &BYTES_DOC,
        ),
        CoreMethod {
            name: "isInline",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_mail_attachment_is_inline",
            doc: Some(&IS_INLINE_DOC),
        },
    ],
    slots: &["name", "type", "data", "contentId"],
    constants: &[],
};

/// The class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "A file that is sent with a message, or an image that is shown inside its HTML. Pass \
            it in the `attachments` option of `Core\\Mail::send`.",
};

/// The `RuntimeError` every constructor documents for its name and type.
const NAME_OR_TYPE_ERROR: ErrorDoc = ErrorDoc {
    error: "RuntimeError",
    desc: "The file name is empty, is longer than 255 bytes, or contains a control character. \
           This error is also thrown when `type` is not a media type such as `image/png`.",
};

/// The `type` option's card on `fromFile` and `fromBytes`.
const TYPE_PARAM: ParamDoc = ParamDoc {
    name: "type",
    desc: "The media type, such as `application/pdf`. A media type says what kind of file it is. \
           By default, the type is detected from the first bytes. If it cannot be detected, it is \
           `application/octet-stream`.",
    shape: &[],
};

/// The `type` option's card on `inlineFile` and `inlineBytes`.
const INLINE_TYPE_PARAM: ParamDoc = ParamDoc {
    name: "type",
    desc: "The media type, such as `image/svg+xml`. A media type says what kind of file it is. \
           By default, the type is detected from the first bytes. Give it for an SVG image, \
           because the type of an SVG cannot be detected.",
    shape: &[],
};

/// `fromFile`'s card.
const FROM_FILE_DOC: MethodDoc = MethodDoc {
    short: "Reads the file at `$path` and returns it as an attachment. The file is read at this \
            call, so it needs the `fs.read` capability here.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The file to attach. A relative path must be a string written directly in the \
                   code. It is joined to the folder of the file that contains it.",
            shape: &[],
        },
        ParamDoc {
            name: "name",
            desc: "The file name that the recipient sees. By default, it is the last part of \
                   the path.",
            shape: &[],
        },
        TYPE_PARAM,
    ],
    ret: "An attachment. Pass it in the `attachments` option of `Core\\Mail::send`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path. This error is also \
                   thrown when the name or the type is not allowed, as for `fromBytes`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The file does not exist or cannot be read.",
        },
    ],
};

/// `fromBytes`' card.
const FROM_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns an attachment with the file name `$name` and the content `$data`. Use it \
            for a file that your program made, such as a CSV export.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The file name that the recipient sees, such as `invoice.pdf`. A name with \
                   letters that are not ASCII is encoded with RFC 2231.",
            shape: &[],
        },
        ParamDoc {
            name: "data",
            desc: "The content, as a `string` or `bytes`.",
            shape: &[],
        },
        TYPE_PARAM,
    ],
    ret: "An attachment. Pass it in the `attachments` option of `Core\\Mail::send`.",
    errors: &[NAME_OR_TYPE_ERROR],
};

/// `inlineFile`'s card.
const INLINE_FILE_DOC: MethodDoc = MethodDoc {
    short: "Reads the image at `$path` and returns it as an inline attachment. An inline \
            attachment is an image shown inside the HTML of the mail, not as a download. Write \
            its `src()` in an `<img src>` of the HTML body to show the image in the message.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The image file. A relative path must be a string written directly in the \
                   code. It is joined to the folder of the file that contains it.",
            shape: &[],
        },
        INLINE_TYPE_PARAM,
    ],
    ret: "An inline attachment. Pass it in the `attachments` option of `Core\\Mail::send`, \
          together with an HTML body that uses its `src()`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path. This error is also \
                   thrown when `type` is not a media type.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The file does not exist or cannot be read.",
        },
    ],
};

/// `inlineBytes`' card.
const INLINE_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns an inline attachment with the file name `$name` and the content `$data`. \
            An inline attachment is an image shown inside the HTML of the mail, not as a \
            download. Write its `src()` in an `<img src>` of the HTML body to show the image.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The file name of the image, such as `logo.png`.",
            shape: &[],
        },
        ParamDoc {
            name: "data",
            desc: "The image, as a `string` or `bytes`.",
            shape: &[],
        },
        INLINE_TYPE_PARAM,
    ],
    ret: "An inline attachment. Pass it in the `attachments` option of `Core\\Mail::send`, \
          together with an HTML body that uses its `src()`.",
    errors: &[NAME_OR_TYPE_ERROR],
};

/// `src`'s card.
const SRC_DOC: MethodDoc = MethodDoc {
    short: "Returns the URL to write in `<img src=\"…\">`, such as `cid:3f2a…@inline`. A `cid:` \
            URL points at an image inside the same mail. A mail program shows the image there.",
    params: &[],
    ret: "The `cid:` URL of this inline attachment.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The attachment is not inline. Only `inlineFile` and `inlineBytes` make an \
               attachment with a `src()`.",
    }],
};

/// One reader of an attachment: no parameter, and text read back as `tainted`.
const fn reader(
    name: &'static str,
    return_ty: CoreTy,
    symbol: &'static str,
    doc: &'static MethodDoc,
) -> CoreMethod {
    CoreMethod {
        name,
        names: &[],
        params: &[],
        defaults: &[],
        return_ty,
        text: TextOrigin::Source,
        symbol,
        doc: Some(doc),
    }
}

/// `name`'s card.
const NAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the file name of the attachment, such as `invoice.pdf`.",
    params: &[],
    ret: "The file name the recipient sees. `name()` returns `null` for a received file with \
          no name.",
    errors: &[],
};

/// `type`'s card.
const TYPE_DOC: MethodDoc = MethodDoc {
    short: "Returns the media type of the attachment, such as `application/pdf`.",
    params: &[],
    ret: "The `type` given when the attachment was made, or the detected type.",
    errors: &[],
};

/// `bytes`' card.
const BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns the content of the attachment.",
    params: &[],
    ret: "The bytes that are sent.",
    errors: &[],
};

/// `isInline`'s card.
const IS_INLINE_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the attachment is shown inside the HTML body.",
    params: &[],
    ret: "`true` for an attachment made with `inlineFile` or `inlineBytes`. `false` for one made \
          with `fromFile` or `fromBytes`.",
    errors: &[],
};

nvs_runtime::nvs_helper! {
    /// `$attachment->name(): string`.
    fn nvs_core_mail_attachment_name(_ctx, args: [1]) {
        crate::instance::read_slot(args, &ATTACHMENT, NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `$attachment->type(): string`.
    fn nvs_core_mail_attachment_type(_ctx, args: [1]) {
        crate::instance::read_slot(args, &ATTACHMENT, TYPE_SLOT, "type")
    }
}

nvs_runtime::nvs_helper! {
    /// `$attachment->bytes(): bytes`.
    fn nvs_core_mail_attachment_bytes(_ctx, args: [1]) {
        crate::instance::read_slot(args, &ATTACHMENT, DATA_SLOT, "bytes")
    }
}

nvs_runtime::nvs_helper! {
    /// `$attachment->isInline(): bool` — whether the attachment has a Content-ID.
    fn nvs_core_mail_attachment_is_inline(_ctx, args: [1]) {
        let member = format!("{ATTACHMENT_NAME}::isInline");
        let object = crate::instance::receiver(args[0], &ATTACHMENT, &member)?;
        Ok(Value::bool(crate::instance::slot(object, CONTENT_ID_SLOT).as_text().is_some()))
    }
}

/// This module's arm of the `Core` symbol table — see [`crate::address`].
pub(super) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        "nvs_core_mail_attachment_name" => (nvs_core_mail_attachment_name as *const ()).cast(),
        "nvs_core_mail_attachment_type" => (nvs_core_mail_attachment_type as *const ()).cast(),
        "nvs_core_mail_attachment_bytes" => (nvs_core_mail_attachment_bytes as *const ()).cast(),
        "nvs_core_mail_attachment_is_inline" => {
            (nvs_core_mail_attachment_is_inline as *const ()).cast()
        }
        "nvs_core_mail_attachment_from_file" => {
            (nvs_core_mail_attachment_from_file as *const ()).cast()
        }
        "nvs_core_mail_attachment_from_bytes" => {
            (nvs_core_mail_attachment_from_bytes as *const ()).cast()
        }
        "nvs_core_mail_attachment_inline_file" => {
            (nvs_core_mail_attachment_inline_file as *const ()).cast()
        }
        "nvs_core_mail_attachment_inline_bytes" => {
            (nvs_core_mail_attachment_inline_bytes as *const ()).cast()
        }
        "nvs_core_mail_attachment_src" => (nvs_core_mail_attachment_src as *const ()).cast(),
        _ => return None,
    }))
}

/// The constructors and `src`, for the tests of [`super`]: a helper is a private `fn`.
#[cfg(test)]
pub(super) const HELPERS: [nvs_runtime::NvsFn; 5] = [
    nvs_core_mail_attachment_from_file,
    nvs_core_mail_attachment_from_bytes,
    nvs_core_mail_attachment_inline_file,
    nvs_core_mail_attachment_inline_bytes,
    nvs_core_mail_attachment_src,
];

/// One attachment of a message, read back out of its value and checked again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Part {
    /// The file name, through [`name_of`].
    pub name: String,
    /// The media type, through [`type_of`] or detected.
    pub media_type: String,
    /// The bytes.
    pub data: Vec<u8>,
    /// The Content-ID, without angle brackets, for an inline attachment.
    pub content_id: Option<String>,
}

/// `name` as a file name a header can carry, or a `RuntimeError` naming what is wrong with it.
///
/// # Errors
///
/// A catchable `RuntimeError` for an empty name, one over [`LONGEST_NAME`] bytes, and one with a
/// control character, which a header line cannot hold as it is.
pub(super) fn name_of(name: &str, member: &str) -> Result<String, Fault> {
    if name.is_empty() {
        return Err(Fault::thrown(format!(
            "{member}: the file name is empty. Give the attachment a name such as `report.pdf`."
        )));
    }
    if name.len() > LONGEST_NAME {
        return Err(Fault::thrown(format!(
            "{member}: the file name has {} bytes, and an attachment name has at most \
             {LONGEST_NAME}",
            name.len()
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(Fault::thrown(format!(
            "{member}: the file name {name:?} contains a control character"
        )));
    }
    Ok(name.to_owned())
}

/// `written` as a media type, `type/subtype` with RFC 6838's characters, lowercased.
///
/// # Errors
///
/// A catchable `RuntimeError` for anything else, such as a parameter after a `;`.
pub(super) fn type_of(written: &str, member: &str) -> Result<String, Fault> {
    let name = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&byte))
    };
    match written.split_once('/') {
        Some((kind, sub)) if written.len() <= LONGEST_TYPE && name(kind) && name(sub) => {
            Ok(written.to_ascii_lowercase())
        }
        _ => Err(Fault::thrown(format!(
            "{member}: {written:?} is not a media type. Write one such as `image/png`."
        ))),
    }
}

/// The media type of `data`: `written`, checked, or else what `Core\Mime` detects, or else
/// `application/octet-stream`.
fn media_type(written: Value, data: &[u8], member: &str) -> Result<String, Fault> {
    match written.as_text() {
        Some(written) => type_of(written, member),
        None => Ok(crate::mime::detect_media(data).media_type().to_owned()),
    }
}

/// The octets in `value`, which [`DATA`] makes a `string` or `bytes`.
fn octets_of<'a>(value: &'a Value, member: &str) -> Result<&'a [u8], Fault> {
    value
        .as_bytes()
        .or_else(|| value.as_str_bytes())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a `string` or `bytes` for `$data`, got tag {}",
                value.tag_byte()
            ))
        })
}

/// The `string` in `value`, which the row declares.
fn text_of<'a>(value: &'a Value, what: &str, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `string` for `{what}`, got tag {}",
            value.tag_byte()
        ))
    })
}

/// A new random Content-ID: 128 bits from the runtime's CSPRNG as hex, at `inline`.
fn content_id(ctx: &mut Ctx) -> String {
    let mut random = [0_u8; 16];
    crate::random::draw(ctx, |rng| rand::Rng::fill_bytes(rng, &mut random));
    let hex: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("{hex}@inline")
}

/// The attachment value: four slots, the bytes under `Tag::Bytes`. `Core\Test\SentMail` builds
/// its attachments with it too.
pub(crate) fn built(name: &str, media_type: &str, data: &[u8], content_id: Option<&str>) -> Value {
    crate::instance::build(
        &ATTACHMENT,
        [
            Value::text(name),
            Value::text(media_type),
            Value::bytes(NvsStr::new(data)),
            content_id.map_or_else(Value::null, Value::text),
        ],
    )
}

/// An attachment of a received message, which `Core\Mail\Message::parse` builds: its name is
/// `null` where nothing was left of the name the sender wrote.
pub(super) fn received(
    name: Option<&str>,
    media_type: &str,
    data: &[u8],
    content_id: Option<&str>,
) -> Value {
    crate::instance::build(
        &ATTACHMENT,
        [
            name.map_or_else(Value::null, Value::text),
            Value::text(media_type),
            Value::bytes(NvsStr::new(data)),
            content_id.map_or_else(Value::null, Value::text),
        ],
    )
}

/// The attachment a `fromFile` or `inlineFile` call builds: the file read under `fs.read`, its
/// name the call's or the path's last part.
fn from_file(
    ctx: &mut Ctx,
    path: &str,
    name: Option<&str>,
    written_type: Value,
    inline: bool,
    member: &str,
) -> Result<Value, Fault> {
    let data = crate::io::slurp(ctx, std::path::Path::new(path), member)?;
    let name = match name {
        Some(name) => name.to_owned(),
        None => std::path::Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
    };
    let name = name_of(&name, member)?;
    let media_type = media_type(written_type, &data, member)?;
    let id = inline.then(|| content_id(ctx));
    Ok(built(&name, &media_type, &data, id.as_deref()))
}

/// The attachment a `fromBytes` or `inlineBytes` call builds.
fn from_bytes(ctx: &mut Ctx, args: &[Value], inline: bool, member: &str) -> Result<Value, Fault> {
    let name = name_of(text_of(&args[0], "name", member)?, member)?;
    let data = octets_of(&args[1], member)?;
    let media_type = media_type(args[2], data, member)?;
    let id = inline.then(|| content_id(ctx));
    Ok(built(&name, &media_type, data, id.as_deref()))
}

nvs_runtime::nvs_helper! {
    /// `Core\Mail\Attachment::fromFile(path $path, {name?, type?}): Core\Mail\Attachment`.
    fn nvs_core_mail_attachment_from_file(ctx, args: [3]) {
        let member = format!("{ATTACHMENT_NAME}::fromFile");
        let path = text_of(&args[0], "path", &member)?.to_owned();
        let name = args[1].as_text().map(str::to_owned);
        from_file(ctx, &path, name.as_deref(), args[2], false, &member)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Mail\Attachment::fromBytes(string $name, string|bytes $data, {type?}):
    /// Core\Mail\Attachment`.
    fn nvs_core_mail_attachment_from_bytes(ctx, args: [3]) {
        from_bytes(ctx, args, false, &format!("{ATTACHMENT_NAME}::fromBytes"))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Mail\Attachment::inlineFile(path $path, {type?}): Core\Mail\Attachment`.
    fn nvs_core_mail_attachment_inline_file(ctx, args: [2]) {
        let member = format!("{ATTACHMENT_NAME}::inlineFile");
        let path = text_of(&args[0], "path", &member)?.to_owned();
        from_file(ctx, &path, None, args[1], true, &member)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Mail\Attachment::inlineBytes(string $name, string|bytes $data, {type?}):
    /// Core\Mail\Attachment`.
    fn nvs_core_mail_attachment_inline_bytes(ctx, args: [3]) {
        from_bytes(ctx, args, true, &format!("{ATTACHMENT_NAME}::inlineBytes"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$attachment->src(): string` — the `cid:` URL of an inline attachment.
    fn nvs_core_mail_attachment_src(_ctx, args: [1]) {
        let member = format!("{ATTACHMENT_NAME}::src");
        let object = crate::instance::receiver(args[0], &ATTACHMENT, &member)?;
        match crate::instance::slot(object, CONTENT_ID_SLOT).as_text() {
            Some(id) => Ok(Value::text(&format!("cid:{id}"))),
            None => Err(Fault::thrown(format!(
                "{member}: this attachment is not inline, so it has no `src()`. Build it with \
                 `inlineFile` or `inlineBytes` to show it inside the HTML."
            ))),
        }
    }
}

/// The attachment in `value`, read back and checked again, so a value rebuilt from bytes is held
/// to what a constructor holds it to.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not an attachment, which the option's type rules out,
/// and [`name_of`]'s and [`type_of`]'s refusals.
pub(super) fn part_of(value: Value, member: &str) -> Result<Part, Fault> {
    let object = crate::instance::receiver(value, &ATTACHMENT, "attachments")?;
    let name = crate::instance::slot(object, NAME_SLOT);
    let media_type = crate::instance::slot(object, TYPE_SLOT);
    let data = crate::instance::slot(object, DATA_SLOT);
    // The slot's type is `?string`, so a name that is not text is `null`.
    if name.as_text().is_none() {
        return Err(Fault::thrown(format!(
            "{member}: an attachment of a received message has no file name. Send it with \
             `{ATTACHMENT_NAME}::fromBytes`, which gives it one."
        )));
    }
    let (Some(name), Some(media_type), Some(data)) =
        (name.as_text(), media_type.as_text(), data.as_bytes())
    else {
        return Err(Fault::fatal(format!(
            "{member} expected a name, a type and bytes in a `{ATTACHMENT_NAME}`"
        )));
    };
    Ok(Part {
        name: name_of(name, member)?,
        media_type: type_of(media_type, member)?,
        data: data.to_vec(),
        content_id: crate::instance::slot(object, CONTENT_ID_SLOT)
            .as_text()
            .map(str::to_owned),
    })
}

/// `; key="value"`, or `; key*=UTF-8''…` by RFC 2231 when the value is not ASCII.
///
/// [`name_of`] refused every control character, so the quoted form needs only `\` and `"`
/// escaped, and the encoded form has no byte a header line cannot hold.
pub(super) fn parameter(key: &str, value: &str) -> String {
    if value.is_ascii() {
        let quoted = value.replace('\\', "\\\\").replace('"', "\\\"");
        return format!("; {key}=\"{quoted}\"");
    }
    let mut encoded = String::with_capacity(value.len() * 3);
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    format!("; {key}*=UTF-8''{encoded}")
}

/// Every inline attachment used by a `cid:` of the HTML, and every `cid:` naming one.
///
/// # Errors
///
/// A catchable `RuntimeError` naming the first `cid:` with no inline attachment, or the first
/// inline attachment no `cid:` uses.
pub(super) fn check_cids(used: &[String], parts: &[Part], member: &str) -> Result<(), Fault> {
    for cid in used {
        if !parts
            .iter()
            .any(|part| part.content_id.as_deref() == Some(cid.as_str()))
        {
            return Err(Fault::thrown(format!(
                "{member}: the HTML uses `cid:{cid}`, and no inline attachment of this message \
                 has that `src()`. Pass the attachment in `attachments`."
            )));
        }
    }
    for part in parts {
        if let Some(id) = &part.content_id
            && !used.iter().any(|cid| cid == id)
        {
            return Err(Fault::thrown(format!(
                "{member}: the inline attachment {:?} is not used. Write its `src()` in the \
                 HTML, or attach it with `fromFile` or `fromBytes`.",
                part.name
            )));
        }
    }
    Ok(())
}

/// What `send` reads out of an HTML body.
pub(super) struct Html {
    /// The plain-text version.
    pub text: String,
    /// Every `cid:` URL the HTML uses, without `cid:`, in document order.
    pub cids: Vec<String>,
    /// The parse itself, which [`super::css::inline`] writes out again when [`Self::styled`].
    pub tree: Parsed,
    /// Whether the HTML has a `<style>` element, so there is CSS to inline.
    pub styled: bool,
    /// Whether the HTML has a `<link>` whose `rel` names `stylesheet`.
    pub linked: bool,
    /// How many elements deep the tree is at its deepest.
    pub depth: usize,
}

/// The widest line of the plain text, in characters. A word longer than this, such as a URL, is
/// kept whole on a line of its own.
const WIDTH: usize = 78;

/// The elements whose content a reader does not see, and so the plain text leaves out.
const HIDDEN: &[&str] = &["head", "style", "script", "template", "title"];

/// The elements that start and end a block: a blank line before and after.
const BLOCKS: &[&str] = &[
    "p",
    "div",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "section",
    "article",
    "header",
    "footer",
    "main",
    "nav",
    "aside",
    "address",
    "figure",
    "figcaption",
    "table",
    "hr",
    "pre",
    "dl",
    "form",
    "fieldset",
    "center",
];

/// One line of the plain text before it is wrapped.
struct Line {
    /// The text.
    text: String,
    /// How far a continuation line is indented: two for a list item, so it lines up after `- `.
    hang: usize,
    /// A line of `<pre>`, which is never wrapped.
    pre: bool,
}

/// The plain text being written while [`read_html`] walks the tree.
#[derive(Default)]
struct Plain {
    /// The lines finished so far.
    lines: Vec<Line>,
    /// The line being written.
    line: String,
    /// The current line's hang.
    hang: usize,
    /// A blank line goes before the next line written.
    gap: bool,
    /// How many `<pre>` elements the walk is inside.
    pre: usize,
    /// How many lists the walk is inside.
    lists: usize,
    /// One count per open table row: the cells written in it.
    cells: Vec<usize>,
    /// One entry per open `<a>`: where its text starts in `line`, and how many lines there were.
    links: Vec<(usize, usize)>,
    /// How many [`HIDDEN`] elements the walk is inside.
    hidden: usize,
}

impl Plain {
    /// Character data, its white space collapsed to one space outside `<pre>`.
    fn text(&mut self, text: &str) {
        if self.hidden > 0 {
            return;
        }
        if self.pre > 0 {
            for ch in text.chars() {
                match ch {
                    '\n' => self.finish(true),
                    '\r' => {}
                    other => self.line.push(other),
                }
            }
            return;
        }
        for ch in text.chars() {
            if ch.is_whitespace() {
                if !self.line.is_empty() && !self.line.ends_with([' ', '\t']) {
                    self.line.push(' ');
                }
            } else {
                self.line.push(ch);
            }
        }
    }

    /// Ends the current line. A line with no text is dropped, unless `keep` (inside `<pre>`).
    fn finish(&mut self, keep: bool) {
        let text = self.line.trim_end().to_owned();
        self.line.clear();
        let hang = std::mem::take(&mut self.hang);
        if text.trim().is_empty() && !keep {
            return;
        }
        if self.gap && !self.lines.is_empty() {
            self.lines.push(Line {
                text: String::new(),
                hang: 0,
                pre: false,
            });
        }
        self.gap = false;
        self.lines.push(Line {
            text,
            hang,
            pre: self.pre > 0,
        });
    }

    /// A block starts or ends: the line ends, and a blank line goes before the next one.
    fn block(&mut self) {
        self.finish(false);
        self.gap = true;
    }

    /// The walk enters `element`. The content of a [`HIDDEN`] element writes nothing.
    fn enter(&mut self, element: &Parsed) {
        let name = element.name.as_str();
        if HIDDEN.contains(&name) {
            self.hidden += 1;
        }
        if self.hidden > 0 {
            return;
        }
        if BLOCKS.contains(&name) {
            self.block();
        }
        match name {
            "pre" => self.pre += 1,
            "br" => {
                if self.line.trim().is_empty() {
                    self.gap = true;
                }
                self.finish(self.pre > 0);
            }
            "ul" | "ol" => {
                if self.lists == 0 {
                    self.block();
                } else {
                    self.finish(false);
                }
                self.lists += 1;
            }
            "li" => {
                self.finish(false);
                self.line = format!("{}- ", "  ".repeat(self.lists.saturating_sub(1)));
                self.hang = self.line.len();
            }
            "tr" => {
                self.finish(false);
                self.cells.push(0);
            }
            "td" | "th" => {
                if let Some(count) = self.cells.last_mut() {
                    if *count > 0 {
                        let kept = self.line.trim_end().len();
                        self.line.truncate(kept);
                        self.line.push('\t');
                    }
                    *count += 1;
                }
            }
            "img" => {
                if let Some(alt) = attribute(element, "alt") {
                    self.text(alt);
                }
            }
            "a" => self.links.push((self.line.len(), self.lines.len())),
            _ => {}
        }
    }

    /// The walk leaves `element`.
    fn leave(&mut self, element: &Parsed) {
        let name = element.name.as_str();
        if HIDDEN.contains(&name) {
            self.hidden = self.hidden.saturating_sub(1);
            return;
        }
        if self.hidden > 0 {
            return;
        }
        match name {
            "pre" => self.pre = self.pre.saturating_sub(1),
            "ul" | "ol" => {
                self.lists = self.lists.saturating_sub(1);
                if self.lists == 0 {
                    self.block();
                } else {
                    self.finish(false);
                }
            }
            "li" => self.finish(false),
            "tr" => {
                self.cells.pop();
                self.finish(false);
            }
            "a" => self.link(element),
            _ => {}
        }
        if BLOCKS.contains(&name) {
            self.block();
        }
    }

    /// The link `element` ends: its URL goes after its text in parentheses, unless the text is
    /// already the URL, or the URL leads nowhere a reader of plain text can go.
    fn link(&mut self, element: &Parsed) {
        let Some((start, count)) = self.links.pop() else {
            return;
        };
        let Some(href) = attribute(element, "href").map(str::trim) else {
            return;
        };
        let lower = href.to_ascii_lowercase();
        if href.is_empty()
            || href.starts_with('#')
            || lower.starts_with("cid:")
            || lower.starts_with("javascript:")
        {
            return;
        }
        let label = if self.lines.len() == count {
            self.line.get(start..).unwrap_or_default().trim()
        } else {
            ""
        };
        let shown = href
            .strip_prefix("mailto:")
            .or_else(|| href.strip_prefix("tel:"))
            .unwrap_or(href);
        if label == href || label == shown {
            return;
        }
        if !self.line.is_empty() && !self.line.ends_with([' ', '\t']) {
            self.line.push(' ');
        }
        self.line.push('(');
        self.line.push_str(href);
        self.line.push(')');
    }

    /// The lines, each wrapped at [`WIDTH`], joined with `\n`, and ending with one.
    fn written(mut self) -> String {
        self.finish(false);
        let mut out = String::new();
        for line in &self.lines {
            for piece in wrapped(line) {
                out.push_str(&piece);
                out.push('\n');
            }
        }
        out
    }
}

/// `line` broken at spaces into lines of at most [`WIDTH`] characters. A continuation line starts
/// with the line's hang in spaces. A word wider than that stays whole.
fn wrapped(line: &Line) -> Vec<String> {
    if line.pre || line.text.chars().count() <= WIDTH {
        return vec![line.text.clone()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut width = 0;
    let indent = " ".repeat(line.hang);
    for word in line.text.split(' ').filter(|word| !word.is_empty()) {
        let size = word.chars().count();
        if width > 0 && width + 1 + size > WIDTH && current.trim().len() > line.hang {
            out.push(std::mem::take(&mut current));
            current.push_str(&indent);
            width = line.hang;
        }
        if width > 0 && !current.ends_with(' ') {
            current.push(' ');
            width += 1;
        }
        current.push_str(word);
        width += size;
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out
}

/// The value of `element`'s attribute `name`.
fn attribute<'a>(element: &'a Parsed, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// Every `cid:` URL inside CSS text, such as `url(cid:…)`, added to `found` without `cid:`.
fn css_cids(css: &str, found: &mut Vec<String>) {
    let lower = css.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find("cid:") {
        let start = from + at + 4;
        let end = css[start..]
            .find(|ch: char| ch == ')' || ch == '"' || ch == '\'' || ch.is_whitespace())
            .map_or(css.len(), |len| start + len);
        if end > start {
            found.push(css[start..end].to_owned());
        }
        from = end.max(start);
    }
}

/// The plain text of `html` and every `cid:` URL it uses, from one WHATWG parse.
///
/// The plain text: a paragraph, a heading and every other block is a block of lines with a blank
/// line between two blocks. `<br>` ends a line, a list item is a line starting `- `, a link is
/// its text and then its URL in parentheses, an image is its `alt` text, and the cells of a table
/// row are separated by tabs. `<head>`, `<style>` and `<script>` are left out. The parser has
/// already decoded every entity. Every line but one in `<pre>` is wrapped at [`WIDTH`].
///
/// The walk keeps its own stack, so a deeply nested document cannot overflow the thread's.
///
/// The same walk notes what [`super::css`] needs: a `<style>` element, a stylesheet `<link>` and
/// the deepest nesting. Inlining moves CSS into `style` attributes and nothing else, so the plain
/// text and the `cid:` URLs read here are the same as those of the inlined HTML.
pub(super) fn read_html(html: &str) -> Html {
    enum Step<'a> {
        Enter(&'a Parsed, usize),
        Leave(&'a Parsed),
    }
    let tree = crate::html::parse(html);
    let mut plain = Plain::default();
    let mut cids = Vec::new();
    let (mut styled, mut linked, mut depth) = (false, false, 0);
    let mut stack = vec![Step::Enter(&tree, 0)];
    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(node, level) => match node.kind {
                Kind::Text => plain.text(&node.text),
                Kind::Element => {
                    let level = level + 1;
                    depth = depth.max(level);
                    for (key, value) in &node.attributes {
                        let value = value.trim();
                        if value.len() > 4 && value[..4].eq_ignore_ascii_case("cid:") {
                            cids.push(value[4..].to_owned());
                        } else if key.eq_ignore_ascii_case("style") {
                            css_cids(value, &mut cids);
                        }
                    }
                    if node.name == "style" {
                        styled = true;
                        for child in &node.children {
                            css_cids(&child.text, &mut cids);
                        }
                    }
                    if node.name == "link" && stylesheet_link(node) {
                        linked = true;
                    }
                    plain.enter(node);
                    stack.push(Step::Leave(node));
                    stack.extend(
                        node.children
                            .iter()
                            .rev()
                            .map(|child| Step::Enter(child, level)),
                    );
                }
                Kind::Document => stack.extend(
                    node.children
                        .iter()
                        .rev()
                        .map(|child| Step::Enter(child, level)),
                ),
                Kind::Comment | Kind::ProcessingInstruction => {}
            },
            Step::Leave(node) => plain.leave(node),
        }
    }
    Html {
        text: plain.written(),
        cids,
        tree,
        styled,
        linked,
        depth,
    }
}

/// Whether a `<link>` loads a stylesheet: its `rel` has the token `stylesheet`, in any case.
fn stylesheet_link(link: &Parsed) -> bool {
    link.attributes.iter().any(|(key, value)| {
        key.eq_ignore_ascii_case("rel")
            && value
                .split_ascii_whitespace()
                .any(|token| token.eq_ignore_ascii_case("stylesheet"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A part with the given name and Content-ID, and no bytes.
    fn part(name: &str, content_id: Option<&str>) -> Part {
        Part {
            name: name.to_owned(),
            media_type: "image/png".to_owned(),
            data: Vec::new(),
            content_id: content_id.map(str::to_owned),
        }
    }

    /// The plain text of a mail body keeps what a reader needs: paragraphs as blocks, list items
    /// as `- ` lines, a link's URL after its text, an image's `alt`, table cells split by tabs,
    /// decoded entities, and nothing from `<head>`, `<style>` or `<script>`.
    // covers: Core\Mail::send
    #[test]
    fn the_plain_text_keeps_links_lists_and_paragraphs() {
        let html = "<html><head><title>T</title><style>p { color: red }</style></head><body>\
                    <h1>Your order</h1><p>Thanks,   Ada &amp; Bo.<br>Line two.</p>\
                    <ul><li>Tea</li><li>Cake</li></ul>\
                    <p>See <a href=\"https://example.com/o/42\">your order</a> or \
                    <a href=\"https://example.com\">https://example.com</a>.</p>\
                    <p><img src=\"cid:x@inline\" alt=\"Logo\"></p>\
                    <table><tr><td>Tea</td><td>3.00</td></tr></table>\
                    <script>alert(1)</script></body></html>";
        let read = read_html(html);
        assert_eq!(
            read.text,
            "Your order\n\nThanks, Ada & Bo.\nLine two.\n\n- Tea\n- Cake\n\n\
             See your order (https://example.com/o/42) or https://example.com.\n\nLogo\n\n\
             Tea\t3.00\n"
        );
        assert_eq!(read.cids, ["x@inline"]);
    }

    /// A long paragraph is wrapped at 78 columns, and a list item's continuation lines up after
    /// its `- `.
    // covers: Core\Mail::send
    #[test]
    fn a_long_line_is_wrapped_at_78_columns() {
        let words = "word ".repeat(40);
        let read = read_html(&format!("<p>{words}</p><ul><li>{words}</li></ul>"));
        for line in read.text.lines() {
            assert!(line.chars().count() <= WIDTH, "{line:?}");
        }
        assert!(read.text.lines().any(|line| line.starts_with("  word")));
    }

    /// Every `cid:` needs an inline attachment of the message.
    // covers: Core\Mail::send, Core\Mail\Attachment::inlineBytes
    #[test]
    fn a_cid_with_no_inline_image_throws() {
        let parts = [part("logo.png", Some("a@inline"))];
        let used = ["a@inline".to_owned(), "b@inline".to_owned()];
        let refused = check_cids(&used, &parts, "Core\\Mail::send");
        assert!(
            refused
                .as_ref()
                .is_err_and(|fault| format!("{fault:?}").contains("cid:b@inline")),
            "{refused:?}"
        );
        assert!(check_cids(&used[..1], &parts, "Core\\Mail::send").is_ok());
    }

    /// Every inline attachment needs a `cid:` in the HTML. An attachment that is not inline needs
    /// none.
    // covers: Core\Mail::send, Core\Mail\Attachment::inlineBytes
    #[test]
    fn an_inline_image_that_is_not_used_throws() {
        let parts = [part("report.pdf", None), part("logo.png", Some("a@inline"))];
        let refused = check_cids(&[], &parts, "Core\\Mail::send");
        assert!(
            refused
                .as_ref()
                .is_err_and(|fault| format!("{fault:?}").contains("logo.png")),
            "{refused:?}"
        );
        assert!(check_cids(&[], &parts[..1], "Core\\Mail::send").is_ok());
    }

    /// An ASCII file name is quoted, and one that is not ASCII is RFC 2231 encoded, so neither
    /// can end the header. A control character is refused.
    // covers: Core\Mail\Attachment::fromBytes
    #[test]
    fn a_non_ascii_file_name_is_rfc_2231_encoded() {
        assert_eq!(
            parameter("filename", "a \"b\".pdf"),
            "; filename=\"a \\\"b\\\".pdf\""
        );
        assert_eq!(
            parameter("filename", "Rechnung März.pdf"),
            "; filename*=UTF-8''Rechnung%20M%C3%A4rz.pdf"
        );
        assert!(name_of("a\r\nBcc: x@example.com", "m").is_err());
        assert!(name_of("", "m").is_err());
        assert!(name_of(&"a".repeat(256), "m").is_err());
    }

    /// Without a `type`, the type is detected from the bytes, and bytes that match no signature
    /// are `application/octet-stream`. A written type is checked.
    // covers: Core\Mail\Attachment::fromBytes
    #[test]
    fn an_attachment_type_is_detected_from_its_bytes() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
        assert_eq!(
            media_type(Value::null(), png, "m").ok().as_deref(),
            Some("image/png")
        );
        assert_eq!(
            media_type(Value::null(), b"a,b\n", "m").ok().as_deref(),
            Some("application/octet-stream")
        );
        let written = Value::text("Text/CSV");
        assert_eq!(
            media_type(written, b"a,b\n", "m").ok().as_deref(),
            Some("text/csv")
        );
        assert!(type_of("text/csv; charset=utf-8", "m").is_err());
        assert!(type_of("csv", "m").is_err());
        #[expect(unsafe_code, reason = "this test owns the value it made")]
        // SAFETY: the value carries the one reference this test made, and is not read after.
        unsafe {
            written.release();
        }
    }
}
