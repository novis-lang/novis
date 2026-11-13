//! `Core\Json::encodeEach`: the text of a JSON array, or of a JSON Lines
//! stream, written in pieces while a `foreach` loop runs.
//!
//! `rule:core-classes/json-whole-and-stream` is the rule.
//!
//! # The walk
//!
//! Each `advance()` pulls items from the source one at a time, on the task's
//! own stack, and encodes each one with the encoder `Core\Json::encode` uses.
//! The encoded text goes into a buffer of pending bytes. Once the buffer has at
//! least [`CHUNK`] bytes, or the source has ended, `advance()` gives the first
//! [`CHUNK`] bytes as one piece. So no piece is larger than [`CHUNK`], and an
//! item larger than that is given in several pieces.
//!
//! **Each item is encoded and refused as `Core\Json::encode` encodes and
//! refuses it**, because it is the same walk with the same refusals. The
//! message names `Core\Json::encodeEach` in place of `Core\Json::encode`. The
//! pieces already given stay given, as with `decodeEach`.
//!
//! **What it spends:** per walk in flight, the pending buffer: up to one piece,
//! plus the text of the item being encoded, which an item larger than a piece
//! makes larger for as long as its pieces are being given. Nothing grows with
//! the number of items.

use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use super::{PIECES, PIECES_NAME, Profile, flag, rendered_chunks};

/// The member every message names.
const MEMBER: &str = "Core\\Json::encodeEach";

/// The largest piece: `Core\Request::bodyStream()`'s chunk size, and the
/// encoder's own.
const CHUNK: usize = 64 << 10;

/// [`PIECES`]' first slot: the key the writer is filed under with
/// `nvs_runtime::Ctx::hold_open_reader`, and `0` once the walk has ended.
const WRITER_AT: usize = 0;
/// [`PIECES`]' second slot: the piece the last `advance()` wrote, `null` before
/// the first one and after the last.
const PIECE_AT: usize = 1;
/// [`PIECES`]' third slot: the cursor `nvs_runtime::sequence::start` began over
/// the items. It is in a slot and not in the filed state so that the request's
/// own release frees it.
const ITEMS_AT: usize = 2;

/// One walk's state, filed on the request.
#[derive(Debug)]
struct Writer {
    /// Where an array's walk goes on, for `nvs_runtime::sequence::pull`.
    from: usize,
    /// Whether the opening `[` is written. JSON Lines has none.
    opened: bool,
    /// Whether the source has ended and the closing `]` is written.
    ended: bool,
    /// Whether the text is JSON Lines.
    lines: bool,
    /// Whether an item is written, so the next one needs a `,` before it.
    wrote_any: bool,
    /// The text written and not yet given.
    pending: Vec<u8>,
    /// How many bytes at the start of [`Self::pending`] are already given.
    given: usize,
}

impl nvs_runtime::HeldReader for Writer {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Writer {
    /// The bytes written and not yet given.
    fn waiting(&self) -> usize {
        self.pending.len() - self.given
    }

    /// Frees the bytes already given. Called before more text is written, when
    /// fewer than [`CHUNK`] bytes wait, so the copy is never larger than one
    /// piece.
    fn compact(&mut self) {
        if self.given > 0 {
            self.pending.drain(..self.given);
            self.given = 0;
        }
    }

    /// The next piece of at most [`CHUNK`] bytes, or `None` when nothing waits.
    fn take_piece(&mut self) -> Option<Vec<u8>> {
        let size = self.waiting().min(CHUNK);
        if size == 0 {
            return None;
        }
        let piece = self.pending[self.given..self.given + size].to_vec();
        self.given += size;
        if self.given == self.pending.len() {
            self.pending.clear();
            self.given = 0;
        }
        Some(piece)
    }
}

/// The writer filed under `key`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a walk read in another task than the one that
/// called [`MEMBER`].
fn writer_at(ctx: &mut nvs_runtime::Ctx, key: u64) -> Result<&mut Writer, Fault> {
    ctx.open_reader_mut(key)
        .and_then(|reader| reader.as_any_mut().downcast_mut::<Writer>())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "A `{PIECES_NAME}` walk was read in a task other than the one that called \
                 `{MEMBER}`. A walk works only in that task."
            ))
        })
}

/// Writes one item, with the `,` or the line break around it.
///
/// # Errors
///
/// A `LogicError` for an item `Core\Json::encode` refuses, and whatever a
/// hand-written `toJson` threw.
fn write_item(ctx: &mut nvs_runtime::Ctx, key: u64, item: Value) -> Result<(), Fault> {
    let written = rendered_chunks(ctx, item, Profile::COMPACT, MEMBER, |why| {
        Fault::thrown_as(ThrownClass::Logic, format!("{MEMBER}(): {why}"))
    });
    #[expect(
        unsafe_code,
        reason = "the pull handed this frame its own reference, and the encoder \
                  only borrowed it"
    )]
    unsafe {
        item.release();
    }
    // A breach of the memory limit is already recorded, and stops the request
    // at its next check, so the refused text is no bytes.
    let chunks = written?.finish().unwrap_or_default();
    let writer = writer_at(ctx, key)?;
    writer.compact();
    if writer.wrote_any && !writer.lines {
        writer.pending.push(b',');
    }
    for chunk in chunks {
        writer.pending.extend_from_slice(&chunk);
    }
    if writer.lines {
        writer.pending.push(b'\n');
    }
    writer.wrote_any = true;
    Ok(())
}

/// The next piece, or `None` once every piece is given.
///
/// # Errors
///
/// [`write_item`]'s, and whatever a driven `Iterable` throws.
fn next_piece(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    items: Value,
) -> Result<Option<Vec<u8>>, Fault> {
    loop {
        let writer = writer_at(ctx, key)?;
        if writer.ended || writer.waiting() >= CHUNK {
            return Ok(writer.take_piece());
        }
        if !writer.opened {
            writer.opened = true;
            if !writer.lines {
                writer.pending.push(b'[');
            }
            continue;
        }
        let mut from = writer.from;
        let item = nvs_runtime::sequence::pull(ctx, items, &mut from, MEMBER)?;
        let writer = writer_at(ctx, key)?;
        writer.from = from;
        match item {
            Some(item) => write_item(ctx, key, item)?,
            None => {
                writer.compact();
                if !writer.lines {
                    writer.pending.push(b']');
                }
                writer.ended = true;
            }
        }
    }
}

/// One `advance()`: the next piece parked in [`PIECE_AT`], and whether there
/// was one. The end of the walk and a failure both give the walk up.
///
/// # Errors
///
/// [`next_piece`]'s.
fn step(ctx: &mut nvs_runtime::Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = crate::instance::receiver(value, &PIECES, member)?;
    let key = crate::instance::slot(receiver, WRITER_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{PIECES_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                PIECES.slots[WRITER_AT]
            ))
        })?;
    if key == 0 {
        crate::instance::set_slot(receiver, PIECE_AT, Value::null());
        return Ok(Value::bool(false));
    }
    let items = crate::instance::slot(receiver, ITEMS_AT);
    match next_piece(ctx, key, items) {
        Ok(Some(piece)) => {
            crate::instance::set_slot(receiver, PIECE_AT, Value::bytes(NvsStr::new(&piece)));
            Ok(Value::bool(true))
        }
        ended => {
            drop(ctx.take_open_reader(key));
            crate::instance::set_slot(receiver, WRITER_AT, Value::uint(0));
            crate::instance::set_slot(receiver, PIECE_AT, Value::null());
            crate::instance::set_slot(receiver, ITEMS_AT, Value::null());
            ended.map(|_| Value::bool(false))
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Json::encodeEach(Iterable<mixed> $items, { lines?: bool }): Iterable<bytes>`.
    ///
    /// **Nothing is written here.** The source's walk is begun, the writer is
    /// filed on the request, and the first item is pulled by the first
    /// `advance()`.
    fn nvs_core_json_encode_each(ctx, args: [2]) {
        let lines = flag(&args[1], "lines")?;
        let items = nvs_runtime::sequence::start(ctx, args[0], MEMBER)?;
        let key = ctx.hold_open_reader(Box::new(Writer {
            from: 0,
            opened: false,
            ended: false,
            lines,
            wrote_any: false,
            pending: Vec::new(),
            given: 0,
        }));
        Ok(crate::instance::build(&PIECES, [Value::uint(key), Value::null(), items]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<bytes>::iterate()` on a `Core\Json\Pieces`: the walk itself,
    /// for `Core\Csv\Rows`' reason.
    fn nvs_core_json_pieces_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &PIECES, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<bytes>::advance()` on a `Core\Json\Pieces`: writes the next
    /// piece, and gives the walk up once the last one is given.
    fn nvs_core_json_pieces_advance(ctx, args: [1]) {
        let stepped = step(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<bytes>::current()` on a `Core\Json\Pieces`: the piece the last
    /// `advance()` wrote, and the only one the walk holds.
    fn nvs_core_json_pieces_current(_ctx, args: [1]) {
        let read = crate::instance::receiver(args[0], &PIECES, nvs_runtime::sequence::CURRENT)
            .map(|receiver| {
                let held = crate::instance::slot(receiver, PIECE_AT);
                #[expect(
                    unsafe_code,
                    reason = "the slot keeps its reference until the next `advance`, so \
                              the value handed back needs one of its own"
                )]
                unsafe {
                    held.retain();
                }
                held
            });
        crate::cursor::consume(args[0]);
        read
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, Fault, NvsArray, OutputSink, ThrownClass, Value, call};

    /// Frees the one reference this frame owns.
    fn released(value: Value) {
        #[expect(unsafe_code, reason = "the case owns the one reference it releases")]
        unsafe {
            value.release();
        }
    }

    /// The walk `encodeEach` gives over `items`.
    fn walk(ctx: &mut Ctx, items: Value, lines: bool) -> Value {
        let args = [items, Value::bool(lines)];
        let answer = call(super::nvs_core_json_encode_each, ctx, &args);
        for arg in args {
            released(arg);
        }
        answer.expect("the walk starts")
    }

    /// One `advance()`: the piece's bytes, `None` at the end.
    fn next(ctx: &mut Ctx, pieces: Value) -> Result<Option<Vec<u8>>, Fault> {
        if super::step(ctx, pieces)?.as_bool() != Some(true) {
            return Ok(None);
        }
        let receiver = pieces.obj_ptr().expect("the walk is an object");
        let held = crate::instance::slot(receiver, super::PIECE_AT);
        Ok(Some(held.as_bytes().expect("a piece is bytes").to_vec()))
    }

    /// Every piece of a walk over `items`.
    fn all_pieces(items: Value, lines: bool) -> Vec<Vec<u8>> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let walk = walk(&mut ctx, items, lines);
        let mut out = Vec::new();
        while let Some(piece) = next(&mut ctx, walk).expect("every item encodes") {
            out.push(piece);
        }
        released(walk);
        out
    }

    /// `values` as a Novis list.
    fn list(values: impl IntoIterator<Item = Value>) -> Value {
        let mut array = NvsArray::new();
        for value in values {
            array.append(value);
        }
        Value::array(array)
    }

    /// The message and class of a thrown fault.
    fn thrown(fault: Fault) -> (ThrownClass, String) {
        match fault {
            Fault::Thrown(class, message) | Fault::ThrownWithSlots(class, message, _) => {
                (class, message.into_owned())
            }
            other => panic!("expected a thrown fault, got {other:?}"),
        }
    }

    /// No piece is larger than 64 KiB, an item larger than that is split, and
    /// the pieces joined are the array `Core\Json::encode` writes.
    // covers: Core\Json::encodeEach
    #[test]
    #[cfg_attr(
        miri,
        ignore = "a piece is a fixed 64 KiB, so the array is 186 KiB of text, which Miri takes minutes to write"
    )]
    fn encode_each_writes_an_array_in_pieces_of_at_most_64_kib() {
        let mut texts: Vec<String> = (0..3000).map(|at| format!("item-{at:04}")).collect();
        texts.insert(1500, "x".repeat(150 << 10));
        let items = list(texts.iter().map(|text| Value::text(text)));
        let pieces = all_pieces(items, false);
        // About 186 KiB of text, so three pieces.
        assert_eq!(
            pieces.len(),
            3,
            "the array is written in {} pieces",
            pieces.len()
        );
        for piece in &pieces {
            assert!(
                piece.len() <= super::CHUNK,
                "a piece of {} bytes",
                piece.len()
            );
        }
        let joined: Vec<u8> = pieces.concat();
        let expected = serde_json::to_string(&texts).expect("strings encode");
        assert_eq!(String::from_utf8(joined).expect("UTF-8"), expected);

        let empty = all_pieces(list([]), false);
        assert_eq!(
            empty,
            vec![b"[]".to_vec()],
            "an empty source is an empty array"
        );
    }

    /// JSON Lines is one value per line with no brackets, and an empty source
    /// gives no piece at all.
    // covers: Core\Json::encodeEach
    #[test]
    fn encode_each_writes_json_lines() {
        let items = list([
            Value::int(1),
            Value::text("a"),
            list([Value::bool(true), Value::null()]),
        ]);
        let joined = all_pieces(items, true).concat();
        assert_eq!(
            String::from_utf8(joined).expect("UTF-8"),
            "1\n\"a\"\n[true,null]\n"
        );
        assert!(all_pieces(list([]), true).is_empty(), "no item is no line");
    }

    /// Each piece escapes `<`, `>`, `&`, `'`, U+2028 and U+2029 as `encode`
    /// does, in an array and in JSON Lines.
    // covers: Core\Json::encodeEach
    #[test]
    fn encode_each_pieces_escape_what_encode_escapes() {
        let item = || Value::text("</script>&'\u{2028}\u{2029}");
        let escaped = "\"\\u003c/script\\u003e\\u0026\\u0027\\u2028\\u2029\"";
        let joined = all_pieces(list([item(), item()]), false).concat();
        assert_eq!(
            String::from_utf8(joined).expect("UTF-8"),
            format!("[{escaped},{escaped}]")
        );
        let joined = all_pieces(list([item()]), true).concat();
        assert_eq!(
            String::from_utf8(joined).expect("UTF-8"),
            format!("{escaped}\n")
        );
    }

    /// An instance with a `secret` property is refused with `encode`'s class
    /// and reason, and the walk ends there.
    // covers: Core\Json::encodeEach
    #[test]
    fn encode_each_refuses_a_secret_value_as_encode_does() {
        let (login, _) = super::super::tests::secret_classes();
        let mut ctx = Ctx::new(OutputSink::Sink);
        let secret = super::super::tests::instance(login);
        let encode = super::super::written(&mut ctx, secret, "Core\\Json::encode")
            .expect_err("encode refuses a secret property");
        let items = list([Value::int(1), secret]);
        let walk = walk(&mut ctx, items, false);
        let refused = next(&mut ctx, walk).expect_err("the secret item is refused");
        let (class, message) = thrown(refused);
        let (want_class, want) = thrown(encode);
        assert_eq!(class, want_class);
        assert_eq!(
            message,
            want.replace("Core\\Json::encode()", "Core\\Json::encodeEach()")
        );
        assert!(next(&mut ctx, walk).expect("an ended walk").is_none());
        released(walk);
    }
}
