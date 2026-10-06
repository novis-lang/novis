//! Novis's refcounted string: one heap allocation, a four-word header, and the
//! bytes inline behind it.
//!
//! This is the representation both `nvs_ir::ty::Ty::Str` and
//! `nvs_ir::ty::Ty::Bytes` lower to. They share it verbatim — the two differ
//! only in the UTF-8 guarantee
//! (`rule:types/bytes`), which is a
//! checker property, not a layout one — so nothing here validates encoding.
//! They are told apart at the *tag*, not here; the crate docs'
//! § *`bytes` is a tag, not a second heap shape* owns that split.
//!
//! # Layout
//!
//! ```text
//! offset 0                                          offset size_of::<StrHeader>()
//! +-------------+-----------+---------+-----------+ +------------------------------+
//! | refcount    | len       | cap     | graphemes | | cap bytes, the first len live |
//! +-------------+-----------+---------+-----------+ +------------------------------+
//! ```
//!
//! One allocation, not two. A `Box<StrData>` holding a `Box<[u8]>` would be
//! simpler to write, but it costs a second allocation and a second cache miss
//! on every string produced, which is a latency question rather than a
//! footprint one ([AGENTS.md](/AGENTS.md)). Codegen will
//! eventually inline the refcount increment/decrement using
//! [`REFCOUNT_OFFSET`]/[`LEN_OFFSET`]/[`CAP_OFFSET`]/[`PAYLOAD_OFFSET`] rather
//! than calling [`nvs_str_retain`]/[`nvs_str_release`]; those constants exist
//! so the layout is queried, never restated.
//!
//! # Capacity, and what it spends
//!
//! `cap` is how many payload bytes the allocation has room for; `len` is how
//! many are live. The two ways they come apart are [`nvs_str_append`] growing
//! one and [`NvsStr::build`] being asked for more capacity than its writer
//! filled; every other constructor sets them equal. `len` is written last in
//! both, so a buffer that is still being filled reads as empty rather than as
//! bytes nobody wrote.
//!
//! What the third word costs is **8 bytes per string allocation**. What it
//! buys is that `$out .= $piece` is not quadratic: without a capacity there is
//! nowhere to append *into*, so every iteration would allocate a fresh buffer
//! and copy the whole accumulation into it, at a cost growing super-linearly
//! with the number of appends. A string that is appended to holds up
//! to **twice its payload**, which is [`grown_capacity`]'s doubling; a string
//! that is never appended to holds exactly its payload. That is memory
//! footprint spent on latency, which is the direction [AGENTS.md](/AGENTS.md)
//! asks for, and it is the whole of what this word spends.
//!
//! # The cached grapheme count, and what it spends
//!
//! `rule:types/string-is-utf8` makes a
//! `string`'s length a count of extended grapheme clusters, which is O(n)
//! where PHP's `strlen` is O(1) — so a program asking twice would otherwise
//! pay twice. The fourth word is that answer, kept: [`NvsStr::grapheme_count`]
//! fills it on the first ask and reads it on every later one, and
//! [`COUNT_UNKNOWN`] is the "nobody has asked" state every fresh allocation
//! starts in. **Lazily**, because the overwhelming majority of strings a
//! request builds are never asked their length at all, and scanning each one
//! eagerly would be paying the cost this word exists to remove.
//!
//! What it costs is **8 more bytes per string allocation** — memory footprint
//! spent on latency, the direction [AGENTS.md](/AGENTS.md) asks for.
//!
//! A concatenation does **not** sum the two counts: a cluster can span the
//! join — a base letter in one buffer and a combining mark in the next — so
//! [`nvs_str_concat`], [`nvs_str_concat_n`] and [`nvs_str_append`] subtract
//! one for every seam [`crate::graphemes::joins_at`] answers for, which is
//! bounded by the clusters touching the seam rather than by either length.
//! They propagate a count **only when every operand already has one**: forcing
//! an uncached operand would make concatenation O(n) in segmentation, which is
//! exactly the cost being removed. They also leave it unknown where the seam
//! *re-groups* what follows it, which a run of regional indicators can do and
//! nothing else does — [`crate::graphemes::seam_joins`] is the one home for
//! that case. The result is otherwise left [`COUNT_UNKNOWN`], and the first
//! ask pays the ordinary scan.
//!
//! That rule is also what keeps the propagation sound over a `bytes`: the two
//! tags share this allocation and a `bytes` payload need not be UTF-8, but
//! nothing ever asks a `bytes` for a grapheme count, so no `bytes` allocation
//! has a cached one and no seam of one is ever read as text.
//!
//! # Why the refcount is a plain `Cell`
//!
//! A request shares nothing with any other request but compiled code
//! (`docs/adr/README.md`'s project-start decisions), and a value crossing a
//! `spawn`/`spawn worker`/`spawn script` boundary is deep-copied rather than
//! shared (`rule:classes/two-copy-depths`).
//! No `NvsStr` a request *allocates* is ever reachable from two threads, so
//! an atomic increment would buy nothing and cost a locked instruction on the
//! hottest operation in the runtime. [`NvsStr`] is correspondingly neither
//! `Send` nor `Sync`, which is what makes that reasoning checkable rather than
//! remembered.
//!
//! # An immortal string, and why the `Cell` survives it
//!
//! A string literal is not allocated at all. `nvs-codegen` writes a whole
//! [`StrHeader`] into the compiled unit's data section in front of the bytes
//! and hands out its address, so `$a["beta"]` in a loop is one constant rather
//! than an [`nvs_str_new`] per evaluation. That header's refcount word is
//! [`IMMORTAL_REFCOUNT`], and the three operations that touch a refcount —
//! [`NvsStr`]'s `Clone` and `Drop`, and [`nvs_str_retain`] — compare against
//! it first and return without writing.
//!
//! The pin is not an optimization the release path may skip; it is what keeps
//! the paragraph above sound. A compiled unit **is** shared between requests
//! (`docs/adr/README.md`'s project-start decisions say it is the one thing
//! that is), so an immortal header is reachable from two threads and the
//! sentence "no `NvsStr` is ever reachable from two threads" stops being true
//! as stated. What the `Cell` actually needs is narrower and does still hold:
//! **no refcount two threads can reach is ever written.** A word that is only
//! ever read races with nothing whatever its type, and every word that *is*
//! written belongs to an allocation [`NvsStr::try_alloc_uninit`] made on the
//! request's own thread and reachable from nowhere else.
//!
//! Two immortal headers exist, for two unrelated reasons. A literal's is in
//! the compiled unit, written by [`immortal_header_bytes`]; [`EMPTY_IMMORTAL`]
//! is this crate's own static, and it is the value every constructor here
//! answers with when the running request has been refused the memory it asked
//! for. Neither is ever written, so the paragraph above covers them alike.
//!
//! What it costs is one compare and a not-taken branch per release — the
//! hottest operation in the runtime — against a sentinel every allocated
//! string misses. Nothing else changes: an immortal string is never mutated
//! in place either, because [`nvs_str_append`] and [`concat_onto`] take their
//! in-place path only at a refcount of exactly one, so they copy out of an
//! immortal exactly as they copy out of a shared one.
//!
//! # Reading the payload as text
//!
//! [`NvsStr::text_of`] hands back a `&str` having validated nothing, and it is
//! the one unchecked read this crate owns — every caller wanting text goes
//! through it or through [`crate::Value::as_text`], rather than each running
//! its own `from_utf8`.
//!
//! What discharges it is a property of the **tag**, not of this module: a
//! `string` is well-formed UTF-8 by construction
//! (`rule:types/bytes`). Its § 3 makes
//! `bytes as string` — the one conversion that could introduce arbitrary
//! octets — checked and throwing, and every other producer either copies a
//! payload whole or joins payloads end to end, neither of which can split a
//! code point. So a `Tag::Str` value's buffer is text and a `Tag::Bytes`
//! value's is not, which is exactly why [`crate::Value::as_text`] is safe and
//! [`NvsStr::text_of`] is not: they share this allocation and the tag is the
//! whole of the difference.
//!
//! A debug build re-validates on every call. A producer that ever broke the
//! invariant therefore fails the test suite rather than reaching an optimized
//! build, which is what keeps the paragraph above a checked claim rather than
//! a remembered one. A payload longer than [`VALIDATED_WHOLE`] is validated at
//! its head and its tail only ([`reads_as_text`]), so a debug build's read of a
//! very large text costs the same as a short one's: every read re-validating
//! the whole of it made a loop of `Core` calls on it time per byte, and a test
//! suite's texts sit under the bound and are still validated whole.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error, realloc};
use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// The header sitting in front of every Novis string's bytes.
///
/// `#[repr(C)]` because compiled code reads these fields at fixed offsets.
/// Never construct one by value — it is only ever the first
/// `size_of::<StrHeader>()` bytes of a larger allocation made by
/// [`NvsStr::new`], and moving it would leave the payload behind.
#[repr(C)]
#[derive(Debug)]
pub struct StrHeader {
    /// How many owners hold this allocation. Reaching `0` frees it.
    refcount: Cell<usize>,
    /// Payload length in bytes. Written after construction only by
    /// [`nvs_str_append`], [`concat_onto`] and [`NvsStr::build`], and only
    /// while this allocation has exactly one owner.
    len: Cell<usize>,
    /// How many payload bytes the allocation has room for — what the layout
    /// this header was allocated with, and will be freed with, is computed
    /// from. Immutable for the allocation's lifetime: growing means a new
    /// allocation, never a bigger `cap` on this one.
    cap: usize,
    /// How many extended grapheme clusters the live payload holds, or
    /// [`COUNT_UNKNOWN`] while nobody has asked. Written by
    /// [`NvsStr::grapheme_count`] on the first ask and by the concatenation
    /// primitives when they can correct a seam — never for an immortal header,
    /// which two requests share and this module's docs say is never written.
    graphemes: Cell<usize>,
}

/// Byte offset of the reference count within [`StrHeader`].
pub const REFCOUNT_OFFSET: usize = std::mem::offset_of!(StrHeader, refcount);

/// Byte offset of the payload length within [`StrHeader`].
pub const LEN_OFFSET: usize = std::mem::offset_of!(StrHeader, len);

/// Byte offset of the payload capacity within [`StrHeader`].
pub const CAP_OFFSET: usize = std::mem::offset_of!(StrHeader, cap);

/// Byte offset of the cached grapheme count within [`StrHeader`].
pub const GRAPHEMES_OFFSET: usize = std::mem::offset_of!(StrHeader, graphemes);

/// Byte offset of the payload itself, relative to the [`StrHeader`] pointer.
pub const PAYLOAD_OFFSET: usize = std::mem::size_of::<StrHeader>();

/// The cached grapheme count of a string nobody has asked the length of.
///
/// `usize::MAX` is the sentinel for the reason [`IMMORTAL_REFCOUNT`] is: no
/// real string can reach it. Every cluster costs at least one payload byte and
/// [`try_str_layout`] refuses a payload past `isize::MAX`, so a count that
/// high would need more memory than the address space holding it — which is
/// why the state fits in the word rather than costing a second one for a flag.
pub const COUNT_UNKNOWN: usize = usize::MAX;

/// The alignment a [`StrHeader`] must be written at.
///
/// Published for `nvs-codegen`, which places one in a data section rather than
/// in an allocation and so has to state the alignment [`str_layout`] would
/// otherwise have handed to `alloc`.
pub const HEADER_ALIGN: usize = std::mem::align_of::<StrHeader>();

/// The reference count carried by a string that must never be freed.
///
/// A literal's header lives in the compiled unit's data section rather than in
/// an allocation, so there is nothing to hand back — and because the unit is
/// shared between requests, the word must never be *written* either. Every
/// operation that touches a refcount compares against this first; this
/// module's docs § *An immortal string* is why that compare is load-bearing
/// rather than an optimization.
///
/// `usize::MAX` is the sentinel because it is the one count a real string
/// cannot reach: every reference costs at least a machine word to hold, so a
/// count that high would need more memory than the address space holding it.
pub const IMMORTAL_REFCOUNT: usize = usize::MAX;

/// The header bytes a compiled unit writes in front of a string literal's
/// payload, in the host's byte order — the payload itself, because the
/// grapheme count is one of the four words and a literal's is free here.
///
/// This is the whole of what `nvs-codegen` needs to know about [`StrHeader`]:
/// it emits these bytes, then the `len` payload bytes, and hands out the
/// address of the first — which is from then on an ordinary `*mut StrHeader`
/// that every primitive here reads exactly as it reads an allocated one. The
/// field order stays this module's secret, which is what the emitting side
/// asked for when it declined to write a header of its own.
///
/// Host order rather than the target's: this JIT compiles for the machine it
/// runs on, the same assumption `nvs_codegen::emit`'s 64-bit `POINTER_SIZE`
/// already makes. An ahead-of-time backend targeting another byte order would
/// take the target's endianness here.
#[must_use]
pub fn immortal_header_bytes(payload: &[u8]) -> [u8; PAYLOAD_OFFSET] {
    let mut header = [0_u8; PAYLOAD_OFFSET];
    let len = payload.len();
    // Counted here rather than left [`COUNT_UNKNOWN`] because this header is
    // the one nothing may ever write: it is in a compiled unit two requests
    // share, so the lazy fill every allocated string uses would be a race.
    // Compile time is where a literal's count is free, and a `bytes` literal —
    // whose payload need not be text — simply has none.
    let graphemes = std::str::from_utf8(payload).map_or(COUNT_UNKNOWN, crate::graphemes::count);
    // `cap` equals `len`, as it does for every string this module builds: an
    // immortal one has no spare room to append into and could not use it.
    for (offset, word) in [
        (REFCOUNT_OFFSET, IMMORTAL_REFCOUNT),
        (LEN_OFFSET, len),
        (CAP_OFFSET, len),
        (GRAPHEMES_OFFSET, graphemes),
    ] {
        header[offset..offset + std::mem::size_of::<usize>()].copy_from_slice(&word.to_ne_bytes());
    }
    header
}

/// A [`StrHeader`] in this crate's own static data, shareable because nothing
/// ever writes it.
///
/// A `StrHeader`'s counts are `Cell`s and a `static`'s type has to be `Sync`,
/// so the argument is made once here rather than at each use. The only header
/// this wraps is [`EMPTY_IMMORTAL`]'s: its reference count is
/// [`IMMORTAL_REFCOUNT`], which takes every retain and release down a path
/// that writes nothing, and its grapheme count is already known, so the lazy
/// fill an allocated string uses never reaches it either. That is this
/// module's docs § *An immortal string* verbatim, for a header in this crate
/// instead of in a compiled unit.
#[repr(transparent)]
struct SharedHeader(StrHeader);

#[expect(
    unsafe_code,
    reason = "every word of the wrapped header is read and none is written, \
              which is what the interior mutability would otherwise have to \
              be synchronized for"
)]
// SAFETY: as the type's own doc comment argues.
unsafe impl Sync for SharedHeader {}

/// The empty string every constructor here answers with when the request has
/// been refused the allocation it asked for.
///
/// It costs no allocation, which is the whole point of answering it on the
/// path where an allocation was refused, and it is immortal in
/// [`IMMORTAL_REFCOUNT`]'s sense — so compiled code owns it, transfers it into
/// an array or a `Value` and releases it exactly as it does an allocated
/// string, and nothing is ever freed. That is what lets a ctx-less `extern
/// "C"` primitive refuse by *returning a value* rather than by acquiring a
/// status its signature has nowhere to put.
static EMPTY_IMMORTAL: SharedHeader = SharedHeader(StrHeader {
    refcount: Cell::new(IMMORTAL_REFCOUNT),
    len: Cell::new(0),
    cap: 0,
    // Known rather than [`COUNT_UNKNOWN`], for [`immortal_header_bytes`]'s
    // reason: nothing may write this header, so the lazy fill is not available
    // to it. An empty payload has no clusters, so there is nothing to scan.
    graphemes: Cell::new(0),
});

/// The allocation shape for a string with room for `cap` payload bytes.
///
/// Takes the *capacity*, never the length: this is the layout an allocation is
/// both made and freed with, and those two must be the same one.
fn str_layout(cap: usize) -> Layout {
    try_str_layout(cap).expect("string capacity overflows the address space")
}

/// [`str_layout`], answering `None` for a capacity no allocation could have.
///
/// The two ways a layout does not exist are the two [`str_layout`] `expect`s
/// its way past: the header plus `cap` overflowing `usize`, and the
/// sum exceeding `isize::MAX`, which `Layout` refuses. Both are reachable from
/// a `Core` member taking a `uint` count — `nvs_runtime::affordable` accepts
/// anything up to `isize::MAX` and knows nothing of the header this allocation
/// then prepends, so a capacity of exactly `isize::MAX` clears that check and
/// has no layout. Answering `None` here is what makes
/// [`NvsStr::try_build`] total, and so what turns that case into the catchable
/// throw its callers already word — a FATAL for a resource refusal a program
/// asked for is the one outcome the fallible path exists to avoid.
fn try_str_layout(cap: usize) -> Option<Layout> {
    let size = PAYLOAD_OFFSET.checked_add(cap)?;
    Layout::from_size_align(size, std::mem::align_of::<StrHeader>()).ok()
}

/// The longest payload a debug build's [`NvsStr::text_of`] validates whole.
const VALIDATED_WHOLE: usize = 64 * 1024;

/// Whether `bytes` is UTF-8, as far as a debug build checks it on every read.
///
/// The whole payload up to [`VALIDATED_WHOLE`], and past it a window of half
/// that at each end. A window's cut can fall inside a code point: an
/// unfinished sequence at the end of the head is accepted, and the tail skips
/// the continuation bytes (at most three) a sequence that began before it left
/// behind. The module doc's § *Reading the payload as text* says why the
/// bound exists.
fn reads_as_text(bytes: &[u8]) -> bool {
    if bytes.len() <= VALIDATED_WHOLE {
        return std::str::from_utf8(bytes).is_ok();
    }
    let window = VALIDATED_WHOLE / 2;
    let head =
        std::str::from_utf8(&bytes[..window]).map_or_else(|e| e.error_len().is_none(), |_| true);
    let tail = &bytes[bytes.len() - window..];
    let cut = tail
        .iter()
        .take(3)
        .take_while(|&&byte| byte & 0xC0 == 0x80)
        .count();
    head && std::str::from_utf8(&tail[cut..]).is_ok()
}

/// How much room a string of `len` bytes takes when it has to grow to hold
/// `needed` — doubling, floored at what is actually asked for.
///
/// Doubling is what makes a loop of appends — or of concatenations onto a
/// solely-owned left operand — linear overall rather than quadratic: each
/// reallocation copies `len` bytes but at least doubles the room, so the copies
/// sum to under twice the final length however many of them there were. The
/// cost is that an accumulated-into string holds up to twice its payload, which
/// this module's docs state as what capacity spends. A concatenation whose left
/// operand is **shared** takes the exact length instead: it is building a value
/// for somebody else to hold, not an accumulation to append to again.
fn grown_capacity(len: usize, needed: usize) -> usize {
    needed.max(len.saturating_mul(2))
}

/// Records `count` as `header`'s cached grapheme count, unless this is an
/// immortal header.
///
/// An immortal one is in a compiled unit two requests share, so writing it
/// would be the race this module's docs § *An immortal string* rules out — and
/// it needs no write anyway, [`immortal_header_bytes`] having counted at
/// compile time. Every caller goes through here rather than remembering the
/// exception.
fn remember_count(header: &StrHeader, count: usize) {
    if header.refcount.get() != IMMORTAL_REFCOUNT {
        header.graphemes.set(count);
    }
}

/// The grapheme count `ptr` already holds, or `None` while nobody has asked.
///
/// The gate on every propagation: a piece with no count is one whose count
/// would have to be *scanned* for, and a concatenation that scans is the cost
/// this whole word exists to remove. It is also what keeps a `bytes` operand
/// out of the text paths — nothing ever asks a `bytes` its grapheme count, so
/// no `bytes` allocation has one.
///
/// # Safety
///
/// `ptr` must refer to a live Novis string allocation.
#[expect(
    unsafe_code,
    reason = "the pointee's liveness is the caller's obligation to state"
)]
unsafe fn cached_count(ptr: *const StrHeader) -> Option<usize> {
    #[expect(unsafe_code, reason = "the caller guarantees the pointee is live")]
    let count = unsafe { &*ptr }.graphemes.get();
    (count != COUNT_UNKNOWN).then_some(count)
}

/// The joined payload's grapheme count: the sum of the pieces', less one for
/// every seam a cluster spans — or `None` where a seam re-groups what follows
/// it and no correction to the sum is right.
///
/// `text` is the **joined** payload and `seams` its cumulative interior byte
/// offsets, in order — which is what makes the correction exact rather than a
/// window heuristic, per [`crate::graphemes::seam_joins`], which also owns the
/// one case answering `None`. A repeated offset is an empty piece and is
/// tested once, not twice: two seams at one byte are one join.
fn joined_count(text: &str, sum: usize, seams: impl Iterator<Item = usize>) -> Option<usize> {
    let mut total = sum;
    let mut previous = 0;
    for seam in seams {
        if seam != previous && crate::graphemes::seam_joins(text, seam)? {
            total -= 1;
        }
        previous = seam;
    }
    Some(total)
}

/// An owning handle to one reference of an Novis string.
///
/// Cloning retains, dropping releases — so Rust-side code (helpers, tests,
/// eventually the stdlib) manipulates strings without writing a refcount
/// operation by hand. Compiled code instead calls the
/// [`nvs_str_new`]/[`nvs_str_retain`]/[`nvs_str_release`] primitives, which
/// are the same operations with the ownership left implicit.
///
/// Neither `Send` nor `Sync`, by construction — see this module's docs.
#[repr(transparent)]
pub struct NvsStr {
    ptr: NonNull<StrHeader>,
}

impl NvsStr {
    /// Allocates a fresh string with a reference count of one.
    ///
    /// # Panics
    ///
    /// Aborts the process through [`handle_alloc_error`] if the allocator
    /// fails. A request-attributable out-of-memory is
    /// `rule:errors/escalation-ladder`'s
    /// resource-limit tier and belongs to the per-request arena that does not
    /// exist yet (known gap 3 in the crate docs); until it does, the global
    /// allocator's own behaviour is the honest one.
    #[must_use]
    pub fn new(bytes: &[u8]) -> Self {
        Self::from_pieces(&[bytes])
    }

    /// Allocates a fresh string holding `pieces` end to end, with a reference
    /// count of one — **one** allocation regardless of how many pieces there
    /// are, which is the whole reason concatenation does not simply build a
    /// `Vec` and hand it to [`NvsStr::new`]. See [`NvsStr::new`] for the
    /// allocation-failure behaviour, which is shared.
    #[must_use]
    pub fn from_pieces(pieces: &[&[u8]]) -> Self {
        let len = pieces
            .iter()
            .try_fold(0_usize, |total, piece| total.checked_add(piece.len()))
            .expect("an Novis string's length cannot overflow a usize");
        Self::build(len, |out| {
            for piece in pieces {
                out.push(piece);
            }
        })
    }

    /// A handle on [`EMPTY_IMMORTAL`] — the degenerate return every allocating
    /// path in this module takes when the request has been refused.
    ///
    /// Owned exactly as an allocated string is: dropping it frees nothing and
    /// retaining it writes nothing, so no caller learns the difference from
    /// the handle alone, and none has to.
    #[must_use]
    pub fn empty_immortal() -> Self {
        Self {
            ptr: NonNull::from(&EMPTY_IMMORTAL.0),
        }
    }

    /// Allocates a fresh string with room for `capacity` bytes and lets
    /// `write` fill it in place, with a reference count of one. The result's
    /// length is what was written.
    ///
    /// The seam for a producer that would otherwise **build a `String` and
    /// have it copied**: the buffer it writes into *is* the allocation the
    /// value is answered from, so the bytes are written once instead of once
    /// into a `String` and again into here. It is `String`'s own shape
    /// otherwise, growth included — a writer that exceeds `capacity` grows by
    /// the same doubling [`grown_capacity`] gives `nvs_str_append`, so a
    /// producer whose length is only a good guess is never *worse* off than
    /// the `String` it replaces, and one whose length is exact
    /// (`Core\Str::repeat`, `padStart`) allocates exactly once.
    ///
    /// [`NvsStr::from_pieces`] is the case where the pieces are already in
    /// hand; this is the case where a loop produces them, and it costs no
    /// scratch buffer to hold them in.
    ///
    /// # A refused allocation
    ///
    /// Answers [`NvsStr::empty_immortal`] and **does not run `write`**, which
    /// is what leaves this signature — and so the whole of `nvs-stdlib` that
    /// reaches it — unchanged by the refusal. It is sound because the request
    /// is already over by then: until its next poll it can build wrong values
    /// and compare them, and it can write no output and reach no `Core`
    /// member, every one of those passing [`crate::run_helper`]'s question
    /// first.
    ///
    /// [`NvsStr::try_build`] is the half that says which of the two happened.
    ///
    /// # Panics
    ///
    /// Aborts through [`handle_alloc_error`] if the allocator fails for a
    /// reason no ceiling explains — see [`NvsStr::alloc_or_refusal`].
    #[must_use]
    pub fn build(capacity: usize, write: impl FnOnce(&mut StrWriter<'_>)) -> Self {
        let Some(ptr) = Self::alloc_or_refusal(0, capacity) else {
            return Self::empty_immortal();
        };
        Self::written_into(ptr, capacity, write)
    }

    /// [`NvsStr::build`], answering `None` where that one answers an empty
    /// string or aborts.
    ///
    /// The seam for a producer whose capacity is a **count off a call site**
    /// rather than a bound on something already in memory. `crate::affordable`
    /// refuses only a size past `isize::MAX`, so every count between that and
    /// what the machine can actually serve reaches the allocator — and an
    /// allocator that refuses is an abort, which takes the process and every
    /// in-flight request with it for what a caller may well want to handle.
    /// Asking is both exact and the whole difference between a throw and that.
    ///
    /// **Only the first allocation is fallible.** A writer that exceeds
    /// `capacity` grows through [`StrWriter::grow`], which stops writing
    /// rather than answering here, so this is for a producer whose capacity is
    /// *exact* — which is the same set of producers as the ones whose capacity
    /// is a count.
    ///
    /// A `None` the running request was **refused** is not the throw the
    /// paragraph above is about: the breach is recorded by then, so
    /// [`crate::run_helper`] reports the ceiling rather than the member's own
    /// error, and a resource limit stays `rule:errors/escalation-ladder`'s
    /// uncatchable tier instead of becoming a `catch` a program can carry on
    /// from.
    #[must_use]
    pub fn try_build(capacity: usize, write: impl FnOnce(&mut StrWriter<'_>)) -> Option<Self> {
        let ptr = Self::try_alloc_uninit(0, capacity)?;
        Some(Self::written_into(ptr, capacity, write))
    }

    /// The half [`NvsStr::build`] and [`NvsStr::try_build`] share: run `write`
    /// against an allocation already in hand, then publish what it wrote.
    fn written_into(
        ptr: NonNull<StrHeader>,
        capacity: usize,
        write: impl FnOnce(&mut StrWriter<'_>),
    ) -> Self {
        let mut out = StrWriter {
            ptr,
            written: 0,
            capacity,
            owns: PhantomData,
        };
        write(&mut out);
        let (ptr, written) = (out.ptr, out.written);
        std::mem::forget(out);
        #[expect(
            unsafe_code,
            reason = "`ptr` is the allocation the writer just finished with, \
                      and no other handle to it exists"
        )]
        unsafe { ptr.as_ref() }.len.set(written);
        Self { ptr }
    }

    /// [`NvsStr::try_alloc_uninit`], answering `None` only where the running
    /// request has been **refused** the memory and owes a degenerate return.
    ///
    /// The seam every primitive in this module allocates through, and what
    /// replaced an aborting wrapper: a refusal this runtime makes on purpose
    /// must not reach [`handle_alloc_error`], which takes the process and
    /// every other in-flight request with it.
    ///
    /// So the three answers below it collapse to the two a caller can act on.
    /// An allocation is `Some`. A failure the request was refused —
    /// [`crate::budget::affords`] having already recorded the breach and asked
    /// for the poll that reports it — is `None`, and what the caller owes is a
    /// value that cost nothing. A failure **no ceiling explains** is neither:
    /// the allocator itself said no, which is [`NvsStr::new`]'s known gap and
    /// not a thing this request can be brought back from, so it aborts here
    /// exactly where it aborted before.
    ///
    /// The caller must write all `len` payload bytes before the handle
    /// escapes.
    ///
    /// # Panics
    ///
    /// Aborts through [`handle_alloc_error`] for that third case.
    fn alloc_or_refusal(len: usize, cap: usize) -> Option<NonNull<StrHeader>> {
        match Self::try_alloc_uninit(len, cap) {
            Some(ptr) => Some(ptr),
            None if crate::budget::refused() => None,
            None => handle_alloc_error(str_layout(cap)),
        }
    }

    /// A fresh allocation with room for `cap` payload bytes, a reference count
    /// of one, and a length of `len` whose bytes are **left uninitialized** —
    /// `None` for a capacity this request cannot be given.
    ///
    /// The one place an Novis string allocation is made, so [`str_layout`] is
    /// called with a capacity here and in [`Drop`] and nowhere else, and so
    /// this is where the ceiling is asked about a *fresh* string.
    /// [`StrWriter::grow`] is the other side of that, for one already in hand.
    ///
    /// Fallible rather than aborting because each of its two callers has
    /// somewhere to put the answer: [`NvsStr::try_build`] hands it to a `Core`
    /// member that throws, and [`NvsStr::alloc_or_refusal`] sorts it into a
    /// refusal and an abort.
    ///
    /// # Panics
    ///
    /// Debug-asserts `len <= cap`.
    fn try_alloc_uninit(len: usize, cap: usize) -> Option<NonNull<StrHeader>> {
        debug_assert!(
            len <= cap,
            "an Novis string's length never exceeds its capacity"
        );
        let layout = try_str_layout(cap)?;
        // `rule:errors/on-limit`'s memory ceiling, asked in front of the
        // allocation rather than behind it: `crate::budget::add` compares once
        // the block is already held, which bounds a *loop* of allocations and
        // cannot bound a single one. A `false` has already recorded the breach
        // and asked for the poll that reports it, so every degenerate return
        // below this is answered by a request that is already over.
        if !crate::budget::affords(layout.size()) {
            return None;
        }
        #[expect(
            unsafe_code,
            reason = "a flexible-array-member allocation cannot be expressed \
                      in safe Rust; `layout` is non-zero-sized because \
                      PAYLOAD_OFFSET > 0, which is `alloc`'s one precondition"
        )]
        let raw = unsafe { alloc(layout) };
        let ptr = NonNull::new(raw.cast::<StrHeader>())?;
        #[expect(
            unsafe_code,
            reason = "`ptr` is a fresh, correctly aligned allocation of exactly \
                      `layout`, which begins with room for the header"
        )]
        unsafe {
            ptr.as_ptr().write(StrHeader {
                refcount: Cell::new(1),
                len: Cell::new(len),
                cap,
                graphemes: Cell::new(COUNT_UNKNOWN),
            });
        }
        Some(ptr)
    }

    /// The payload.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        #[expect(
            unsafe_code,
            reason = "`self.ptr` is live for `&self`'s borrow: this handle owns \
                      one of the references keeping it alive"
        )]
        unsafe {
            Self::bytes_of(self.ptr.as_ptr())
        }
    }

    /// The payload length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.header().len.get()
    }

    /// How many payload bytes the allocation has room for.
    ///
    /// Equal to [`NvsStr::len`] for every string this module constructs; only
    /// [`nvs_str_append`] ever leaves the two apart. Exposed for the same
    /// reason [`NvsStr::refcount`] is: the growth policy is only checkable by
    /// observing it.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.header().cap
    }

    /// Whether the payload is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// How many owners currently hold this allocation.
    ///
    /// Exposed because the refcount insertion policy in `nvs_ir::lower` is
    /// only checkable by observing it — see this module's own tests, and
    /// eventually `nvs-codegen`'s.
    #[must_use]
    pub fn refcount(&self) -> usize {
        self.header().refcount.get()
    }

    /// How many extended grapheme clusters the payload holds — `rule:types/string-is-utf8`'s
    /// unit, and this module's docs § *The cached grapheme count* for why the
    /// answer is kept.
    ///
    /// The first ask scans; every later one reads the word. **Only call this
    /// on a `string`**: the payload is read as text, which is
    /// [`NvsStr::text_of`]'s obligation and is discharged by the *tag*, so a
    /// `bytes` goes through [`crate::Value::grapheme_count`] and is answered
    /// `None` instead of arriving here.
    #[must_use]
    pub fn grapheme_count(&self) -> usize {
        let ptr = self.ptr.as_ptr();
        #[expect(unsafe_code, reason = "the pointee is live for `&self`'s borrow")]
        unsafe {
            Self::grapheme_count_of(ptr)
        }
    }

    /// [`NvsStr::grapheme_count`] against a raw pointer, for a caller holding
    /// one rather than a handle — `nvs_stdlib::granularity`'s seam, which
    /// reaches a string through a [`crate::Value`] and owns no reference of
    /// its own to hand over.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live Novis string allocation whose payload is a
    /// `string`'s — well-formed UTF-8 — which is [`NvsStr::text_of`]'s
    /// obligation and is what the `Tag::Str` a caller matched on states.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "both the pointee's liveness and its payload's encoding are \
                  the caller's obligation to state"
    )]
    pub unsafe fn grapheme_count_of(ptr: *const StrHeader) -> usize {
        #[expect(unsafe_code, reason = "the caller guarantees the pointee is live")]
        let header = unsafe { &*ptr };
        let cached = header.graphemes.get();
        if cached != COUNT_UNKNOWN {
            return cached;
        }
        #[expect(
            unsafe_code,
            reason = "the caller guarantees a live `string` payload, which is \
                      exactly `text_of`'s obligation"
        )]
        let count = crate::graphemes::count(unsafe { Self::text_of(ptr) });
        remember_count(header, count);
        count
    }

    fn header(&self) -> &StrHeader {
        #[expect(
            unsafe_code,
            reason = "`self.ptr` is live for `&self`'s borrow: this handle \
                      owns one of the references keeping it alive"
        )]
        unsafe {
            self.ptr.as_ref()
        }
    }

    /// Gives up ownership of this handle's reference, yielding the raw
    /// pointer compiled code holds.
    ///
    /// The caller now owns exactly one reference and must eventually pass the
    /// pointer to [`nvs_str_release`] or [`NvsStr::from_raw`].
    #[must_use]
    pub fn into_raw(self) -> *mut StrHeader {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Reclaims a reference previously given up by [`NvsStr::into_raw`].
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer produced by [`NvsStr::into_raw`] (or by
    /// [`nvs_str_new`]) whose reference has not already been released, and it
    /// must not be reclaimed twice.
    ///
    /// # Panics
    ///
    /// If `ptr` is null, which no Novis string pointer ever is.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "reclaiming a reference is the caller's obligation to state"
    )]
    pub unsafe fn from_raw(ptr: *mut StrHeader) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("an Novis string pointer is never null"),
        }
    }

    /// The payload of the string `ptr` refers to, borrowed without taking a
    /// reference of its own.
    ///
    /// This is the one place the payload's position is computed, so
    /// [`crate::Value`] can expose a string's bytes without restating the
    /// layout.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live Novis string allocation that stays live for
    /// the whole of `'a`.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the pointee's liveness is the caller's obligation to state"
    )]
    pub unsafe fn bytes_of<'a>(ptr: *const StrHeader) -> &'a [u8] {
        #[expect(
            unsafe_code,
            reason = "the caller guarantees the allocation is live for 'a; \
                      `new` initialized `len` payload bytes at PAYLOAD_OFFSET \
                      and nothing ever writes them again"
        )]
        unsafe {
            let len = (*ptr).len.get();
            std::slice::from_raw_parts(ptr.cast::<u8>().add(PAYLOAD_OFFSET), len)
        }
    }

    /// The payload as text, validating nothing.
    ///
    /// The one unchecked read in this crate — this module's § *Reading the
    /// payload as text* states what discharges it, and
    /// [`crate::Value::as_text`] is the safe caller that does.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live Novis string allocation that stays live for
    /// the whole of `'a`, **and** its payload must be a `string`'s rather than
    /// a `bytes`'s: the two share this allocation and only the former carries
    /// `rule:types/bytes`'s UTF-8 invariant.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "both the pointee's liveness and its payload's encoding are \
                  the caller's obligation to state"
    )]
    pub unsafe fn text_of<'a>(ptr: *const StrHeader) -> &'a str {
        #[expect(
            unsafe_code,
            reason = "the caller guarantees the allocation is live for 'a"
        )]
        let bytes = unsafe { Self::bytes_of(ptr) };
        debug_assert!(
            reads_as_text(bytes),
            "a `string` payload is well-formed UTF-8 by `rule:types/bytes`'s construction"
        );
        #[expect(
            unsafe_code,
            reason = "the caller guarantees this payload is a `string`'s, which \
                      `rule:types/bytes` makes well-formed UTF-8; the assertion above is \
                      the debug build's check of that"
        )]
        unsafe {
            std::str::from_utf8_unchecked(bytes)
        }
    }

    /// How many owners hold the string `ptr` refers to.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live Novis string allocation.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the pointee's liveness is the caller's obligation to state"
    )]
    pub unsafe fn refcount_of(ptr: *const StrHeader) -> usize {
        #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
        unsafe {
            (*ptr).refcount.get()
        }
    }
}

/// The one handle an [`NvsStr::build`] writer has on the allocation it fills.
///
/// A cursor over payload bytes that are not yet initialized, so it hands out
/// no reference to them and cannot outlive the call that made it. Every write
/// goes through [`StrWriter::push`], which is where the room is checked — once
/// per piece, not once per byte — and where the allocation grows if a producer
/// wrote past the capacity it asked for.
#[derive(Debug)]
pub struct StrWriter<'a> {
    /// The allocation being filled. Replaced wholesale by [`StrWriter::grow`],
    /// which is the only thing that may write this field.
    ptr: NonNull<StrHeader>,
    /// How many bytes the pieces so far occupy. The header's own `len` stays
    /// `0` until [`NvsStr::build`] finishes, so a panic mid-write cannot leave
    /// uninitialized bytes readable.
    written: usize,
    /// How many the allocation has room for.
    capacity: usize,
    /// The writer owns the allocation for the call's duration, and hands it to
    /// [`NvsStr::build`] at the end.
    owns: PhantomData<&'a mut [u8]>,
}

impl StrWriter<'_> {
    /// Appends `piece` to what has been written so far, growing the allocation
    /// if it does not fit.
    pub fn push(&mut self, piece: &[u8]) {
        let needed = self
            .written
            .checked_add(piece.len())
            .expect("an Novis string's length cannot overflow a usize");
        if needed > self.capacity && !self.grow(needed) {
            // The writer's half of the degenerate return: refused the room, it
            // writes no more, and [`NvsStr::build`] publishes the prefix that
            // did fit. A wrong value the request is already too dead to act on
            // — and the only answer that keeps the write below inside the
            // payload its `#[expect]` argues it stays inside.
            return;
        }
        #[expect(
            unsafe_code,
            reason = "`needed` is at most `capacity` by the branch above, so \
                      the write stays inside the payload; the regions cannot \
                      overlap because this writer is the only handle to a \
                      fresh allocation"
        )]
        unsafe {
            let at = self.ptr.as_ptr().cast::<u8>().add(PAYLOAD_OFFSET);
            std::ptr::copy_nonoverlapping(piece.as_ptr(), at.add(self.written), piece.len());
        }
        self.written = needed;
    }

    /// Appends `piece`'s bytes — [`StrWriter::push`] spelled for text, which
    /// is what every `Core\Str` producer has in hand.
    pub fn push_str(&mut self, piece: &str) {
        self.push(piece.as_bytes());
    }

    /// Grows the allocation to hold `needed` bytes, answering `false` where the
    /// request has been refused the extra room.
    ///
    /// The same doubling `nvs_str_append` grows by, so a producer whose
    /// capacity is a guess pays exactly what the `String` it replaces would,
    /// and one whose capacity is exact never reaches here at all.
    ///
    /// The second place a string allocation is asked for, so the pre-check
    /// [`NvsStr::try_alloc_uninit`] makes is made here too — against the
    /// *difference* between the two layouts, which is what the balance moves by
    /// when `realloc` answers. Without it a producer whose capacity is a guess
    /// could double its way past the ceiling inside one member call, reaching
    /// no poll and no [`crate::run_helper`] question on the way.
    #[cold]
    fn grow(&mut self, needed: usize) -> bool {
        let capacity = grown_capacity(self.capacity, needed);
        let bigger = str_layout(capacity);
        if !crate::budget::affords(bigger.size() - str_layout(self.capacity).size()) {
            return false;
        }
        // `realloc` and not an allocate-copy-free of our own, for the reason
        // `Vec` uses it: an allocator that can extend the block in place does,
        // and the bytes already written are then not moved at all. Copying
        // instead costs `Core\Str::join` the whole saving this seam exists
        // for, and then some.
        #[expect(
            unsafe_code,
            reason = "`self.ptr` was allocated with `str_layout(self.capacity)` \
                      and this writer is its only handle; `bigger.size()` is \
                      larger, so `realloc`'s contract is met"
        )]
        let raw = unsafe {
            realloc(
                self.ptr.as_ptr().cast::<u8>(),
                str_layout(self.capacity),
                bigger.size(),
            )
        };
        let Some(ptr) = NonNull::new(raw.cast::<StrHeader>()) else {
            handle_alloc_error(bigger)
        };
        #[expect(
            unsafe_code,
            reason = "`ptr` is the reallocated block, which begins with the \
                      header `try_alloc_uninit` wrote; the capacity it will be \
                      freed with has to be the one it now has"
        )]
        unsafe {
            (&raw mut (*ptr.as_ptr()).cap).write(capacity);
        }
        self.ptr = ptr;
        self.capacity = capacity;
        true
    }
}

impl Drop for StrWriter<'_> {
    /// Frees the allocation, which only happens when `write` panicked partway
    /// through: [`NvsStr::build`] takes the allocation out of the writer on
    /// the path that finishes. A helper's panic is a contained `FATAL`
    /// (`rule:errors/escalation-ladder`), so it must not also be a leak.
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "this writer is the only handle to the allocation, and \
                      the layout is the one it was made with"
        )]
        unsafe {
            dealloc(self.ptr.as_ptr().cast::<u8>(), str_layout(self.capacity));
        }
    }
}

impl Clone for NvsStr {
    fn clone(&self) -> Self {
        let header = self.header();
        let count = header.refcount.get();
        // An immortal header is shared between requests and read-only — see
        // this module's docs § *An immortal string*.
        if count != IMMORTAL_REFCOUNT {
            header.refcount.set(
                count
                    .checked_add(1)
                    .expect("an Novis string's reference count cannot overflow a usize"),
            );
        }
        Self { ptr: self.ptr }
    }
}

impl Drop for NvsStr {
    fn drop(&mut self) {
        let count = self.header().refcount.get();
        // The one compare an immortal literal costs the release path, and the
        // reason it is not optional: this header may be in another request's
        // hands too, and freeing or writing it would be wrong twice over.
        if count == IMMORTAL_REFCOUNT {
            return;
        }
        let remaining = count - 1;
        if remaining > 0 {
            self.header().refcount.set(remaining);
            return;
        }
        let layout = str_layout(self.capacity());
        #[expect(
            unsafe_code,
            reason = "this handle held the last reference, so nothing else can \
                      observe the allocation; `layout` is recomputed from the \
                      same `cap` `try_alloc_uninit` allocated with — never from \
                      `len`, which `nvs_str_append` may have left smaller — \
                      before the header is freed"
        )]
        unsafe {
            dealloc(self.ptr.as_ptr().cast::<u8>(), layout);
        }
    }
}

impl fmt::Debug for NvsStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NvsStr")
            .field("refcount", &self.refcount())
            .field("bytes", &String::from_utf8_lossy(self.as_bytes()))
            .finish()
    }
}

impl PartialEq for NvsStr {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for NvsStr {}

/// Hashes the payload, and only the payload — never the pointer.
///
/// Together with [`Borrow<[u8]>`](std::borrow::Borrow) below this is what lets
/// [`crate::array`]'s index map be keyed by a handle and looked up by bytes:
/// a key is stored twice as a pointer, never twice as bytes. The two impls
/// must agree, which they do because both defer to `<[u8] as Hash>`.
impl std::hash::Hash for NvsStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl std::borrow::Borrow<[u8]> for NvsStr {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// These are `extern "C"` but deliberately *not* `rule:errors/propagation`'s checked-return
// helper shape, and deliberately not written through `nvs_helper!`. That shape
// exists to carry a failure back to a caller that can act on one, and these
// have no such caller: they take no `Ctx`, so giving the hottest operations in
// the runtime an unused `*const Value`/`*mut Value` pair and a `catch_unwind`
// would cost every call an ABI it never uses.
//
// An allocation here can be **refused**. `rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`
// asks the memory ceiling in front of the allocation rather than behind it, and
// `NvsStr::alloc_or_refusal` is the seam every primitive below allocates
// through. A refusal is answered with a degenerate return chosen so that the
// reference it yields balances the references it consumed — each one's own doc
// comment names which value that is — and never with a status. It is sound
// because the refusal has already published the breach into the word compiled
// code polls: the program reaches its next poll and, in between, no `Core`
// member, no output and nothing durable.
//
// They are panic-free by construction. The abort that remains is the failure no
// ceiling explains — the platform heap saying no — which `handle_alloc_error`
// turns into an abort rather than an unwind; and every one of these is
// `extern "C"` and never `extern "C-unwind"`, so nothing can unwind through a
// JIT frame either way.

/// Allocates a fresh string from `len` bytes at `ptr`, with a reference count
/// of one — `nvs_ir::InstKind::ConstStr`'s entry point.
///
/// # Safety
///
/// `ptr` must be valid for reads of `len` bytes, or `len` must be zero.
#[expect(
    unsafe_code,
    reason = "compiled code passes a pointer and a length; the contract cannot \
              be expressed in the signature, so the function is honestly unsafe"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_new(ptr: *const u8, len: usize) -> *mut StrHeader {
    let bytes = if len == 0 {
        &[][..]
    } else {
        #[expect(
            unsafe_code,
            reason = "the caller guarantees `ptr` is valid for `len` bytes; the \
                      zero-length case is split out because `from_raw_parts` \
                      rejects a null pointer even for an empty slice"
        )]
        unsafe {
            std::slice::from_raw_parts(ptr, len)
        }
    };
    NvsStr::new(bytes).into_raw()
}

/// Concatenation's whole body: appends every piece in `rest` to `lhs`,
/// consuming one reference to `lhs` and yielding one to the result.
///
/// **A solely-owned left operand is written into and handed straight back.**
/// No allocation, and not one byte of the accumulation moves — which is what
/// makes `$s = $s . $x` linear rather than quadratic, exactly as
/// [`nvs_str_append`] makes `$s .= $x` linear. Sole ownership is the whole of
/// what makes that write unobservable: no second owner can see the operand
/// change under it, and the lowering hands this reference over only where the
/// holder is re-pointed at the result anyway
/// (`nvs_ir::ir::InstKind::Concat`'s own doc comment is that protocol's one
/// home).
///
/// Everything else allocates once and copies every piece into it: a **shared**
/// left operand takes exactly the bytes the result needs, since nothing will
/// be appended to the copy that the copy was not already made for, and a
/// solely-owned one with too little room takes [`grown_capacity`]'s doubling,
/// so a run of concatenations reallocates a logarithmic number of times rather
/// than once per piece. Either way `lhs` is released **last**, after every
/// piece has been read, because a piece may be `lhs` itself.
///
/// **A refused allocation answers a static empty string** rather than
/// acquiring a status the `extern "C"` signatures above this have nowhere to
/// put — see [`NvsStr::empty_immortal`]. The consumed reference is released on
/// that path too, so a refusal balances exactly as a served call does.
///
/// # Safety
///
/// `lhs` must refer to a live Novis string allocation whose reference this
/// caller owns and does not release again, and every piece in `rest` to a live
/// one. A piece may be `lhs`: every piece is read before `lhs`'s length moves
/// and before the release that can free it, and the bytes written always begin
/// past `lhs`'s own payload.
#[expect(
    unsafe_code,
    reason = "the pointees' liveness and the left operand's ownership are the \
              caller's obligation to state"
)]
unsafe fn concat_onto(lhs: *mut StrHeader, rest: &[*const StrHeader]) -> *mut StrHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees every pointee is live and that `lhs`'s \
                  reference is this function's; each read and write below stays \
                  within one of those allocations' own payload regions"
    )]
    unsafe {
        if rest.is_empty() {
            return lhs;
        }
        let header = &*lhs;
        let len = header.len.get();
        let added = rest
            .iter()
            .try_fold(0_usize, |total, piece| {
                total.checked_add(NvsStr::bytes_of(*piece).len())
            })
            .expect("an Novis string's length cannot overflow a usize");
        let needed = len
            .checked_add(added)
            .expect("an Novis string's length cannot overflow a usize");
        // Every piece or none: one uncached operand and the sum is unknown,
        // which is `cached_count`'s whole rule. Read before anything moves,
        // since the write below invalidates the left operand's own word.
        let counts = cached_count(lhs).and_then(|first| {
            rest.iter().try_fold(first, |total, piece| {
                cached_count(*piece).map(|c| total + c)
            })
        });
        let solely_owned = header.refcount.get() == 1;
        let reused = solely_owned && header.cap >= needed;
        let out = if reused {
            lhs
        } else {
            let cap = if solely_owned {
                grown_capacity(len, needed)
            } else {
                needed
            };
            let Some(fresh) = NvsStr::alloc_or_refusal(needed, cap) else {
                nvs_str_release(lhs);
                return NvsStr::empty_immortal().into_raw();
            };
            std::ptr::copy_nonoverlapping(
                lhs.cast::<u8>().add(PAYLOAD_OFFSET),
                fresh.as_ptr().cast::<u8>().add(PAYLOAD_OFFSET),
                len,
            );
            fresh.as_ptr()
        };
        let dst = out.cast::<u8>().add(PAYLOAD_OFFSET);
        let mut written = len;
        for piece in rest {
            // A piece that *is* the reused destination reads its own payload,
            // which ends where this write begins — the two ranges are disjoint
            // for the same reason `nvs_str_append`'s are.
            let bytes = NvsStr::bytes_of(*piece);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst.add(written), bytes.len());
            written += bytes.len();
        }
        debug_assert_eq!(written, needed, "a piece was copied at the wrong offset");
        // The joined text is read off the buffer rather than through the
        // header, and the seams off the pieces, while `len` is still the left
        // operand's own: a reused destination is also a possible piece, so a
        // header already carrying the joined length would hand the seam walk
        // bytes that are not the piece's. The seams are the cumulative lengths,
        // recomputed rather than kept, because keeping them would be the
        // scratch allocation this path exists to do without.
        let joined = counts.and_then(|sum| {
            let mut at = len;
            let seams = std::iter::once(len).chain(rest[..rest.len() - 1].iter().map(|piece| {
                at += NvsStr::bytes_of(*piece).len();
                at
            }));
            let payload = std::slice::from_raw_parts(dst, needed);
            debug_assert!(
                std::str::from_utf8(payload).is_ok(),
                "a cached grapheme count belongs to a `string`, whose payload is \
                 well-formed UTF-8 by `rule:types/bytes`'s construction"
            );
            joined_count(std::str::from_utf8_unchecked(payload), sum, seams)
        });
        if reused {
            header.len.set(needed);
            header.graphemes.set(COUNT_UNKNOWN);
        } else {
            nvs_str_release(lhs);
        }
        if let Some(joined) = joined {
            remember_count(&*out, joined);
        }
        out
    }
}

/// Concatenates `rhs` onto `lhs` — `nvs_ir::InstKind::Concat`'s entry point for
/// the two-operand case, which is the common one and is kept because it needs
/// neither the stack array nor the count [`nvs_str_concat_n`] takes.
///
/// **Consumes one reference to `lhs` and yields one to the result**, reusing
/// `lhs`'s own buffer whenever nothing else holds it; `rhs` is only *read*, so
/// it is neither retained nor released. [`concat_onto`] is the whole body and
/// `nvs_ir::ir::InstKind::Concat`'s own doc comment is the one home for that
/// protocol.
///
/// # Safety
///
/// `lhs` must refer to a live Novis string allocation whose reference this
/// caller owns and does not release again; `rhs` must refer to a live one.
/// They may be the same allocation — `$s = $s . $s` — which [`concat_onto`]'s
/// contract covers.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw string pointers whose liveness and \
              ownership the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_concat(
    lhs: *mut StrHeader,
    rhs: *const StrHeader,
) -> *mut StrHeader {
    #[expect(
        unsafe_code,
        reason = "this function's own contract is `concat_onto`'s, one piece \
                  wide; the slice is a borrow of the caller's argument"
    )]
    unsafe {
        concat_onto(lhs, std::slice::from_ref(&rhs))
    }
}

/// Concatenates every piece after the first onto that first one —
/// [`nvs_str_concat`] for three or more operands, and the entry point
/// `nvs_ir::InstKind::Concat` takes once it carries that many.
///
/// **One allocation for the whole expression, and none at all where the left
/// operand can be written into.** Folding `"<tr><td>" . $i . "</td>"` left into
/// a chain of [`nvs_str_concat`] calls allocates n-1 buffers for an n-operand
/// concatenation and copies a growing prefix into each one; here the total
/// length is summed first and every piece is copied once.
///
/// Ownership is [`nvs_str_concat`]'s exactly: one reference to `pieces[0]` is
/// consumed and one to the result is yielded, while every later piece is only
/// read. A `count` of one hands that same reference straight back, and a
/// `count` of zero — which `nvs_ir::ir::InstKind::Concat` never emits, its
/// `pieces` always holding two or more — has no operand to consume and answers
/// the static empty string.
///
/// # Safety
///
/// `pieces` must point at `count` consecutive `*const StrHeader`, each
/// referring to a live Novis string allocation, and this caller must own the
/// first one's reference and not release it again. Two of them may be the same
/// allocation, which [`concat_onto`]'s contract covers.
#[expect(
    unsafe_code,
    reason = "compiled code passes a stack array of raw string pointers whose \
              liveness, count and leading operand's ownership the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_concat_n(
    pieces: *const *const StrHeader,
    count: usize,
) -> *mut StrHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `count` live pointers at `pieces` and a \
                  live pointee behind each; the rest of the contract is \
                  `concat_onto`'s"
    )]
    unsafe {
        let pieces = std::slice::from_raw_parts(pieces, count);
        let Some((&lhs, rest)) = pieces.split_first() else {
            return NvsStr::empty_immortal().into_raw();
        };
        concat_onto(lhs.cast_mut(), rest)
    }
}

/// Appends `suffix`'s bytes to `target`'s — `nvs_ir::InstKind::StrAppend`'s
/// entry point, and the reason [`StrHeader`] carries a capacity at all.
///
/// **Consumes one reference to `target` and yields one to the result**, which
/// is `nvs_array_set`'s protocol verbatim: the reference consumed is the
/// holder's, the one yielded replaces it in that same slot, and the caller
/// therefore retains nothing and releases nothing.
/// `nvs_ir::ir::InstKind::ArraySet`'s own doc comment is the worked statement
/// of that protocol and the one home for it. `suffix` is only *read*, exactly
/// as [`nvs_str_concat`] reads both of its operands — neither retained nor
/// released here.
///
/// Whenever `target` was solely owned and already had the room, the pointer
/// returned **is** the pointer given and not one byte of the accumulation
/// moves. That is the whole point: without it, `$out .= $piece` copies the
/// entire accumulation every iteration, which is quadratic in the number of
/// appends.
///
/// Exactly two things force a fresh allocation instead:
///
/// - **A reference count above one.** A second owner can see these bytes, and
///   an append is a write; `rule:types/arrays`'s copy-on-write value semantics do not
///   let that owner observe it. This is the same separation `nvs_array_set`
///   performs for the same reason, and it is what keeps the in-place path
///   sound rather than merely fast.
/// - **Too little room**, in which case [`grown_capacity`] decides how much to
///   ask for and the payload moves once.
///
/// **A refused allocation answers `target` unchanged**, which is the one
/// degenerate return that balances here: the reference consumed and the
/// reference yielded are then the same reference, so a refusal neither leaks
/// the accumulation nor over-releases it, and the slot the caller writes the
/// result into ends up holding exactly what it already held.
///
/// # Safety
///
/// `target` must refer to a live Novis string allocation whose reference this
/// caller owns and does not release again; `suffix` must refer to a live one.
/// They may be the same allocation — `$s .= $s` — which is why the in-place
/// path's two byte ranges are argued disjoint rather than assumed so.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw string pointers whose liveness and \
              ownership the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_append(
    target: *mut StrHeader,
    suffix: *const StrHeader,
) -> *mut StrHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both pointees are live; every read and \
                  write below stays within one of the two allocations' own \
                  payload regions, bounded by the `cap` each was made with"
    )]
    unsafe {
        let header = &*target;
        let len = header.len.get();
        let added = (*suffix).len.get();
        let needed = len
            .checked_add(added)
            .expect("an Novis string's length cannot overflow a usize");
        let src = suffix.cast::<u8>().add(PAYLOAD_OFFSET);
        // Read before either payload moves, and used after: an append changes
        // the count of the string it answers, so a stale word would outlive
        // the bytes it counted. `None` on either side leaves the result
        // `COUNT_UNKNOWN`, which the in-place path below writes back over the
        // target's own — the one place a cached count is *invalidated* rather
        // than corrected.
        let counts = cached_count(target).zip(cached_count(suffix));
        if header.refcount.get() == 1 && header.cap >= needed {
            // The two ranges cannot overlap even when `suffix == target`: the
            // source is the first `added` bytes of the payload and the
            // destination begins at `len`, which is `added` when they are the
            // same allocation and irrelevant when they are not.
            let dst = target.cast::<u8>().add(PAYLOAD_OFFSET + len);
            std::ptr::copy_nonoverlapping(src, dst, added);
            header.len.set(needed);
            header.graphemes.set(COUNT_UNKNOWN);
            if let Some((held, gained)) = counts
                && let Some(joined) =
                    joined_count(NvsStr::text_of(target), held + gained, std::iter::once(len))
            {
                remember_count(header, joined);
            }
            return target;
        }
        let Some(grown) = NvsStr::alloc_or_refusal(needed, grown_capacity(len, needed)) else {
            return target;
        };
        let dst = grown.as_ptr().cast::<u8>().add(PAYLOAD_OFFSET);
        std::ptr::copy_nonoverlapping(target.cast::<u8>().add(PAYLOAD_OFFSET), dst, len);
        std::ptr::copy_nonoverlapping(src, dst.add(len), added);
        if let Some((held, gained)) = counts
            && let Some(joined) = joined_count(
                NvsStr::text_of(grown.as_ptr()),
                held + gained,
                std::iter::once(len),
            )
        {
            remember_count(grown.as_ref(), joined);
        }
        // Last, so that `suffix`'s bytes are read before a `suffix == target`
        // release can free them.
        nvs_str_release(target);
        grown.as_ptr()
    }
}

/// Whether two strings hold the same bytes — `nvs_ir::ir::BinOp::Eq` over a
/// `Ty::Str` operand pair.
///
/// A byte comparison, not a collation: `string` is guaranteed-valid UTF-8
/// (`rule:types/bytes`), and PHP's `===`
/// on two strings is byte equality, which is what Novis keeps. Two operands at
/// one address are equal without a byte being read, which is what two reads of
/// one stored text give — `$m->text() == $m->group(0)` over a match of
/// megabytes. Neither operand is retained or released — the same read-only
/// treatment [`nvs_str_concat`] gives its two.
///
/// # Safety
///
/// `lhs` and `rhs` must each refer to a live Novis string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw string pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_eq(lhs: *const StrHeader, rhs: *const StrHeader) -> bool {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both pointees are live; both borrows \
                  end with this comparison"
    )]
    unsafe {
        std::ptr::eq(lhs, rhs) || NvsStr::bytes_of(lhs) == NvsStr::bytes_of(rhs)
    }
}

/// Adds a reference — `nvs_ir::InstKind::Retain` for a `Ty::Str` operand.
///
/// A null `ptr` is a no-op: see [`crate::object`]'s *A null payload is `null`*
/// for why that is the rule rather than a defensive check. A string literal's
/// header is a no-op too, for the reason [`IMMORTAL_REFCOUNT`] states.
///
/// # Safety
///
/// `ptr` must be null or refer to a live Novis string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw string pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_retain(ptr: *mut StrHeader) {
    if ptr.is_null() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the pointee is live; incrementing in \
                  place is the whole operation, so no handle is created that \
                  a later drop could double-release"
    )]
    let header = unsafe { &*ptr };
    let count = header.refcount.get();
    // See [`IMMORTAL_REFCOUNT`]: a literal's header is not ours to write.
    if count != IMMORTAL_REFCOUNT {
        header.refcount.set(
            count
                .checked_add(1)
                .expect("an Novis string's reference count cannot overflow a usize"),
        );
    }
}

/// Drops a reference, freeing the allocation if it was the last —
/// `nvs_ir::InstKind::Release` for a `Ty::Str` operand.
///
/// A null `ptr` is a no-op, and so is a string literal's immortal header —
/// see [`nvs_str_retain`]. Releasing one is therefore always sound however
/// many times it happens, which is what lets compiled code transfer a literal
/// into an array or a `Value` without a special case.
///
/// # Safety
///
/// `ptr` must be null, or refer to a live Novis string allocation whose
/// reference this caller owns; a non-null one must not be released twice.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw string pointer whose ownership the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_release(ptr: *mut StrHeader) {
    if ptr.is_null() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns the reference `from_raw` \
                  reclaims; dropping the handle is what releases it"
    )]
    unsafe {
        drop(NvsStr::from_raw(ptr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ptr` with one more reference on it — what a case hands a
    /// concatenation whose leading operand it means to go on using, since
    /// [`concat_onto`] consumes one. The operand is left *shared* by it, which
    /// is also what keeps the result a fresh allocation.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live Novis string allocation.
    #[expect(
        unsafe_code,
        reason = "the pointee's liveness is the caller's obligation to state"
    )]
    unsafe fn lent(ptr: *mut StrHeader) -> *mut StrHeader {
        #[expect(unsafe_code, reason = "the caller guarantees the pointee is live")]
        unsafe {
            nvs_str_retain(ptr);
        }
        ptr
    }

    #[test]
    fn the_debug_read_check_validates_a_long_text_at_both_ends_and_across_a_cut() {
        // `é` is two bytes, so a run of them past the bound puts each window's
        // cut inside a code point.
        let long = "é".repeat(VALIDATED_WHOLE);
        assert!(reads_as_text(long.as_bytes()));
        assert!(reads_as_text(format!("x{long}").as_bytes()));

        let mut broken_head = long.clone().into_bytes();
        broken_head[1] = 0xFF;
        assert!(!reads_as_text(&broken_head));
        let mut broken_tail = long.into_bytes();
        let last = broken_tail.len() - 1;
        broken_tail[last] = 0xFF;
        assert!(!reads_as_text(&broken_tail));

        assert!(!reads_as_text(b"short \xFF text"));
    }

    #[test]
    fn a_fresh_string_has_one_reference_and_its_bytes() {
        let s = NvsStr::new(b"Hello, World!");
        assert_eq!(s.as_bytes(), b"Hello, World!");
        assert_eq!(s.len(), 13);
        assert!(!s.is_empty());
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn an_empty_string_is_a_real_allocation() {
        let s = NvsStr::new(b"");
        assert!(s.is_empty());
        assert_eq!(s.as_bytes(), b"");
        assert_eq!(s.refcount(), 1);
    }

    /// The degenerate return the ctx-less primitives refuse with, from the
    /// side this module owns: it is empty, it costs no allocation, and it
    /// survives being released more times than there were references — which
    /// is what lets it stand in for an allocation compiled code already owns a
    /// reference to.
    #[test]
    fn the_refused_empty_string_is_immortal_and_costs_no_allocation() {
        use crate::counting_alloc::allocated_bytes;

        let before = allocated_bytes();
        let s = NvsStr::empty_immortal();
        assert!(s.is_empty());
        assert_eq!(s.as_bytes(), b"");
        assert_eq!(s.refcount(), IMMORTAL_REFCOUNT);
        assert_eq!(s.grapheme_count(), 0, "answered without a scan or a write");

        let t = s.clone();
        assert_eq!(t.refcount(), IMMORTAL_REFCOUNT, "a retain writes nothing");
        drop(t);

        let raw = s.into_raw();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_str_release(raw);
            // One release past the references there were: a primitive that
            // refuses hands this to a caller that will release it, and that
            // caller must not be the one to free a static.
            nvs_str_release(raw);
            assert_eq!((*raw).refcount.get(), IMMORTAL_REFCOUNT);
            assert_eq!(NvsStr::bytes_of(raw), b"");
        }

        assert_eq!(
            allocated_bytes(),
            before,
            "the value answered on a refusal must not itself allocate"
        );
    }

    #[test]
    fn cloning_retains_and_dropping_releases() {
        let s = NvsStr::new(b"abc");
        let t = s.clone();
        assert_eq!(s.refcount(), 2);
        assert_eq!(t.refcount(), 2);
        assert_eq!(t.as_bytes(), b"abc");
        drop(t);
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn the_raw_primitives_move_the_same_count() {
        let raw = NvsStr::new(b"xy").into_raw();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_str_retain(raw);
            assert_eq!(NvsStr::refcount_of(raw), 2);
            assert_eq!(NvsStr::bytes_of(raw), b"xy");
            nvs_str_release(raw);
            assert_eq!(NvsStr::refcount_of(raw), 1);
            nvs_str_release(raw);
        }
    }

    #[test]
    fn str_new_copies_the_bytes_it_is_given() {
        let source = b"Hello, World!".to_vec();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let owned = unsafe {
            let raw = nvs_str_new(source.as_ptr(), source.len());
            NvsStr::from_raw(raw)
        };
        drop(source);
        assert_eq!(owned.as_bytes(), b"Hello, World!");
    }

    #[test]
    fn str_new_accepts_a_null_pointer_for_an_empty_string() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let owned = unsafe { NvsStr::from_raw(nvs_str_new(std::ptr::null(), 0)) };
        assert!(owned.is_empty());
    }

    #[test]
    fn concat_joins_two_strings_into_one_fresh_allocation() {
        let lhs = NvsStr::new(b"quadruple(5) = ").into_raw();
        let rhs = NvsStr::new(b"20").into_raw();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let joined = nvs_str_concat(lent(lhs), rhs);
            assert_eq!(NvsStr::bytes_of(joined), b"quadruple(5) = 20");
            assert_eq!(NvsStr::refcount_of(joined), 1);
            // The concatenation consumed the reference it was lent and left
            // the one this case holds; `rhs` it only read. See
            // `nvs_str_concat`'s own doc comment.
            assert_eq!(NvsStr::refcount_of(lhs), 1);
            assert_eq!(NvsStr::refcount_of(rhs), 1);
            nvs_str_release(joined);
            nvs_str_release(lhs);
            nvs_str_release(rhs);
        }
    }

    #[test]
    fn concat_handles_empty_operands_and_an_aliased_one() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let empty = NvsStr::new(b"").into_raw();
            let abc = NvsStr::new(b"abc").into_raw();

            for (lhs, rhs, expected) in [
                (empty, abc, &b"abc"[..]),
                (abc, empty, &b"abc"[..]),
                (empty, empty, &b""[..]),
                // The same allocation on both sides: read twice, written
                // nowhere but the fresh result.
                (abc, abc, &b"abcabc"[..]),
            ] {
                let joined = nvs_str_concat(lent(lhs), rhs);
                assert_eq!(NvsStr::bytes_of(joined), expected);
                nvs_str_release(joined);
            }

            nvs_str_release(empty);
            nvs_str_release(abc);
        }
    }

    #[test]
    fn concat_n_joins_every_piece_into_one_fresh_allocation() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let open = NvsStr::new(b"<tr><td>").into_raw();
            let mid = NvsStr::new(b"7").into_raw();
            let close = NvsStr::new(b"</td></tr>").into_raw();
            let empty = NvsStr::new(b"").into_raw();

            // The shape this exists for: three pieces, one allocation.
            let pieces = [
                lent(open).cast_const(),
                mid.cast_const(),
                close.cast_const(),
            ];
            let joined = nvs_str_concat_n(pieces.as_ptr(), pieces.len());
            assert_eq!(NvsStr::bytes_of(joined), b"<tr><td>7</td></tr>");
            assert_eq!(NvsStr::refcount_of(joined), 1);
            // The leading piece's lent reference is consumed and no other is
            // retained or released — `nvs_str_concat`'s rule, unchanged by the
            // arity.
            for piece in pieces {
                assert_eq!(NvsStr::refcount_of(piece), 1);
            }
            nvs_str_release(joined);

            // An empty piece, and the same allocation appearing twice: every
            // piece is read before the fresh destination is written.
            let repeated = [
                lent(mid).cast_const(),
                empty.cast_const(),
                mid.cast_const(),
                mid.cast_const(),
            ];
            let joined = nvs_str_concat_n(repeated.as_ptr(), repeated.len());
            assert_eq!(NvsStr::bytes_of(joined), b"777");
            nvs_str_release(joined);

            for piece in [open, mid, close, empty] {
                nvs_str_release(piece);
            }
        }
    }

    /// The claim § B of `docs/perf/userland-gap.md` asks for, measured rather
    /// than asserted about: an n-piece concatenation allocates one buffer, of
    /// exactly the length it answers, where a fold of two-operand
    /// `nvs_str_concat` calls reallocates every time the accumulation outgrows
    /// the room it has and copies what is already there into the new one.
    ///
    /// Each call is handed a reference of its own to its leading operand, that
    /// being what [`concat_onto`] consumes — which is also what keeps the n-ary
    /// reading exact rather than zero: `pieces` still holds that operand, so
    /// nothing may be written into it and the buffer is a fresh one.
    ///
    /// Bytes-ever-allocated, for the reason
    /// [`appending_into_spare_capacity_allocates_nothing`] reads the same
    /// counter: a `live_bytes` delta cannot tell a copy that was freed again
    /// from no copy at all, and every intermediate here is freed.
    #[test]
    fn an_n_ary_concatenation_allocates_one_buffer() {
        use crate::counting_alloc::allocated_bytes;

        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let pieces: Vec<*const StrHeader> = (0..8)
                .map(|_| NvsStr::new(b"0123456789").into_raw().cast_const())
                .collect();
            let total = 8 * 10;

            nvs_str_retain(pieces[0].cast_mut());
            let before = allocated_bytes();
            let joined = nvs_str_concat_n(pieces.as_ptr(), pieces.len());
            let n_ary = allocated_bytes() - before;
            assert_eq!(NvsStr::bytes_of(joined).len(), total);
            assert_eq!(n_ary, PAYLOAD_OFFSET + total);
            nvs_str_release(joined);

            // The fold, over the same pieces. Each join consumes the
            // accumulation's reference and yields the result's, so there is no
            // release between them.
            nvs_str_retain(pieces[0].cast_mut());
            let before = allocated_bytes();
            let mut folded = nvs_str_concat(pieces[0].cast_mut(), pieces[1]);
            for piece in &pieces[2..] {
                folded = nvs_str_concat(folded, *piece);
            }
            let fold = allocated_bytes() - before;
            assert_eq!(NvsStr::bytes_of(folded).len(), total);
            nvs_str_release(folded);

            assert!(
                n_ary < fold,
                "eight pieces cost {n_ary} bytes n-ary against the fold's {fold}: \
                 the n-ary path is folding rather than sizing the result once"
            );

            for piece in pieces {
                nvs_str_release(piece.cast_mut());
            }
        }
    }

    #[test]
    fn the_layout_constants_describe_the_real_header() {
        let word = std::mem::size_of::<usize>();
        assert_eq!(REFCOUNT_OFFSET, 0);
        assert_eq!(LEN_OFFSET, word);
        assert_eq!(CAP_OFFSET, 2 * word);
        assert_eq!(GRAPHEMES_OFFSET, 3 * word);
        assert_eq!(PAYLOAD_OFFSET, 4 * word);
    }

    #[test]
    fn a_constructed_string_has_no_spare_capacity() {
        assert_eq!(NvsStr::new(b"abc").capacity(), 3);
        assert_eq!(NvsStr::new(b"").capacity(), 0);
        assert_eq!(NvsStr::from_pieces(&[b"ab", b"cd"]).capacity(), 4);
    }

    #[test]
    fn appending_into_spare_capacity_keeps_the_same_allocation() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            // The first append has nothing to double, so it grows exactly;
            // the second doubles and leaves room the third appends into.
            let (b, c, d) = (
                NvsStr::new(b"b").into_raw(),
                NvsStr::new(b"c").into_raw(),
                NvsStr::new(b"d").into_raw(),
            );
            let acc = nvs_str_append(NvsStr::new(b"a").into_raw(), b);
            let grown = nvs_str_append(acc, c);
            assert_eq!(NvsStr::bytes_of(grown), b"abc");
            let same = nvs_str_append(grown, d);
            assert_eq!(
                same, grown,
                "the fourth byte fit in the room the third made"
            );
            assert_eq!(NvsStr::bytes_of(same), b"abcd");
            assert_eq!(NvsStr::refcount_of(same), 1);
            for ptr in [same, b, c, d] {
                nvs_str_release(ptr);
            }
        }
    }

    #[test]
    fn appending_to_a_shared_string_separates_rather_than_writing_it() {
        let held = NvsStr::new(b"abc");
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            // One reference for `held`, one for the append to consume.
            let target = held.clone().into_raw();
            let suffix = NvsStr::new(b"def").into_raw();
            let appended = nvs_str_append(target, suffix);
            assert_ne!(
                appended, target,
                "a second owner forbids the in-place write"
            );
            assert_eq!(NvsStr::bytes_of(appended), b"abcdef");
            assert_eq!(NvsStr::refcount_of(appended), 1);
            nvs_str_release(appended);
            nvs_str_release(suffix);
        }
        assert_eq!(held.as_bytes(), b"abc");
        assert_eq!(held.refcount(), 1);
    }

    #[test]
    fn appending_a_string_to_itself_reads_before_it_writes() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            // Always the grow path: a self-append needs twice the payload and
            // doubling never leaves that much room, so both operands are the
            // one allocation the append then releases — after reading it.
            let s = NvsStr::new(b"xy").into_raw();
            let doubled = nvs_str_append(s, s);
            assert_eq!(NvsStr::bytes_of(doubled), b"xyxy");
            let quadrupled = nvs_str_append(doubled, doubled);
            assert_eq!(NvsStr::bytes_of(quadrupled), b"xyxyxyxy");
            assert_eq!(NvsStr::refcount_of(quadrupled), 1);
            nvs_str_release(quadrupled);
        }
    }

    /// The claim § B of `docs/perf/userland-gap.md` asks for, measured rather
    /// than asserted about: an append a solely-owned string has the room for
    /// allocates nothing at all, and a run of them allocates a multiple of the
    /// *final* length rather than of the accumulation copied each time.
    ///
    /// Bytes-ever-allocated is the only reading that shows it — a `live_bytes`
    /// delta cannot tell a copy that was freed again from no copy at all,
    /// which is the same reason `array::tests::an_integer_subscript_allocates_no_key`
    /// reads this counter.
    #[test]
    fn appending_into_spare_capacity_allocates_nothing() {
        use crate::counting_alloc::allocated_bytes;

        const RUN: usize = 1_000;
        const PIECE: usize = 10;

        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let piece = NvsStr::new(b"0123456789").into_raw();
            let mut acc = NvsStr::new(b"").into_raw();

            let before = allocated_bytes();
            for _ in 0..RUN {
                acc = nvs_str_append(acc, piece);
            }
            let spent = allocated_bytes() - before;

            assert_eq!(NvsStr::bytes_of(acc).len(), RUN * PIECE);
            // Copying the accumulation every iteration would be quadratic.
            // Doubling makes the reallocations sum to under four times the
            // final length, headers included.
            assert!(
                spent < 4 * RUN * PIECE,
                "{RUN} appends allocated {spent} bytes for a {}-byte result: the \
                 accumulation is being copied rather than appended into",
                RUN * PIECE
            );

            // And the last doubling left room, so one more append allocates
            // nothing at all and answers with the pointer it was given.
            let quiet = allocated_bytes();
            let same = nvs_str_append(acc, piece);
            assert_eq!(same, acc, "the append had the room to write into");
            assert_eq!(
                allocated_bytes() - quiet,
                0,
                "an in-place append allocated something"
            );
            nvs_str_release(same);
            nvs_str_release(piece);
        }
    }

    /// `$s = $s . $x` is linear, which is the whole of what the ownership
    /// hand-off buys: the accumulation is written into rather than copied out
    /// of, so a run of concatenations allocates a multiple of the *final*
    /// length rather than of what each iteration would have copied.
    /// `nvs_ir::ir::InstKind::Concat` is the home for the protocol this case
    /// stands in for — `acc` is the binding whose one reference the lowering
    /// hands over and re-points at the result.
    ///
    /// Bytes-ever-allocated is the only reading that shows it, for
    /// `appending_into_spare_capacity_allocates_nothing`'s reason: a
    /// `live_bytes` delta cannot tell a copy that was freed again from no copy
    /// at all.
    #[test]
    fn concat_reuses_a_solely_owned_left_operand() {
        use crate::counting_alloc::allocated_bytes;

        const RUN: usize = 1_000;
        const PIECE: usize = 10;

        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let piece = NvsStr::new(b"0123456789").into_raw();
            let mut acc = NvsStr::new(b"").into_raw();

            let before = allocated_bytes();
            for _ in 0..RUN {
                acc = nvs_str_concat(acc, piece);
            }
            let spent = allocated_bytes() - before;

            assert_eq!(NvsStr::bytes_of(acc).len(), RUN * PIECE);
            // Copying the accumulation into a fresh buffer every iteration
            // would be quadratic; doubling makes the reallocations sum to
            // under four times the final length, headers included.
            assert!(
                spent < 4 * RUN * PIECE,
                "{RUN} concatenations allocated {spent} bytes for a {}-byte result: the \
                 accumulation is being copied rather than written into",
                RUN * PIECE
            );

            // And the last doubling left room, so one more concatenation
            // allocates nothing at all and answers the pointer it was handed.
            let quiet = allocated_bytes();
            let same = nvs_str_concat(acc, piece);
            assert_eq!(same, acc, "the concatenation had the room to write into");
            assert_eq!(
                allocated_bytes() - quiet,
                0,
                "an in-place concatenation allocated something"
            );
            nvs_str_release(same);

            // A second owner forbids that write, exactly as it forbids an
            // append's: the result is a separate buffer and the other owner's
            // bytes are untouched.
            let held = NvsStr::new(b"abc");
            let shared = held.clone().into_raw();
            let joined = nvs_str_concat(shared, piece);
            assert_ne!(joined, shared, "a second owner forbids the in-place write");
            assert_eq!(NvsStr::bytes_of(joined), b"abc0123456789");
            assert_eq!(held.as_bytes(), b"abc");
            assert_eq!(held.refcount(), 1);
            nvs_str_release(joined);
            nvs_str_release(piece);
        }
    }

    /// Three or more pieces reuse the leading operand on the same terms, and
    /// a piece that *is* that operand is read before the write that would
    /// change it — `$s = $s . $s . "!"`.
    #[test]
    fn concat_n_reuses_the_leading_operand_and_reads_a_self_piece_first() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            // Room to spare, so the leading operand is written into: a build
            // of capacity 16 holding 2 bytes has the 5 the result needs.
            let acc = NvsStr::build(16, |w| w.push_str("xy")).into_raw();
            let bang = NvsStr::new(b"!").into_raw();
            let pieces = [acc.cast_const(), acc.cast_const(), bang.cast_const()];
            let joined = nvs_str_concat_n(pieces.as_ptr(), pieces.len());
            assert_eq!(
                joined, acc,
                "the leading operand had the room to write into"
            );
            assert_eq!(NvsStr::bytes_of(joined), b"xyxy!");
            nvs_str_release(joined);
            nvs_str_release(bang);
        }
    }

    #[test]
    fn appending_stays_linear_because_capacity_doubles() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let piece = NvsStr::new(b"0123456789").into_raw();
            let mut acc = NvsStr::new(b"").into_raw();
            for _ in 0..1_000 {
                acc = nvs_str_append(acc, piece);
            }
            assert_eq!(NvsStr::bytes_of(acc).len(), 10_000);
            // Doubling caps the room at under twice the payload, which is what
            // this module's docs state capacity spends.
            let owned = NvsStr::from_raw(acc);
            assert!(owned.capacity() < 2 * owned.len());
            nvs_str_release(piece);
        }
    }

    /// The bytes `nvs-codegen` puts in a unit's data section for one string
    /// literal, in a `Vec<u64>` so the header lands at the alignment it has
    /// there — this is the only way this crate's own tests can hold an
    /// immortal string, since nothing here constructs one.
    fn immortal_unit(bytes: &[u8]) -> Vec<u64> {
        assert!(HEADER_ALIGN <= std::mem::size_of::<u64>());
        let total = PAYLOAD_OFFSET + bytes.len();
        let mut words = vec![0_u64; total.div_ceil(std::mem::size_of::<u64>())];
        #[expect(
            unsafe_code,
            reason = "`words` owns `total` bytes at a u64's alignment, which is \
                      the whole point of allocating it as one; the view ends \
                      with this function"
        )]
        let view =
            unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr().cast::<u8>(), total) };
        view[..PAYLOAD_OFFSET].copy_from_slice(&immortal_header_bytes(bytes));
        view[PAYLOAD_OFFSET..].copy_from_slice(bytes);
        words
    }

    /// The claim § B of `docs/perf/userland-gap.md` makes for a string
    /// literal, from the side this crate owns: an immortal header costs
    /// nothing to retain or release, is never freed however many times it is
    /// released, and — the half that keeps the plain `Cell` sound — is never
    /// *written*.
    ///
    /// The zero here is `allocated_bytes`, for the reason
    /// [`an_n_ary_concatenation_allocates_one_buffer`] reads the same counter:
    /// an `nvs_str_new` per evaluation, freed again straight away, is exactly
    /// what a `live_bytes` delta cannot see at all.
    #[test]
    fn an_immortal_string_is_never_written_freed_or_allocated_for() {
        use crate::counting_alloc::allocated_bytes;

        let mut unit = immortal_unit(b"beta");
        let ptr = unit.as_mut_ptr().cast::<StrHeader>();

        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            assert_eq!(NvsStr::bytes_of(ptr), b"beta");
            assert_eq!(NvsStr::refcount_of(ptr), IMMORTAL_REFCOUNT);

            let before = allocated_bytes();
            nvs_str_retain(ptr);
            nvs_str_release(ptr);
            nvs_str_release(ptr);
            // One more release than there were references: an immortal cannot
            // be over-released, which is what lets compiled code transfer one
            // into an array or a `Value` with no special case.
            nvs_str_release(ptr);
            assert_eq!(allocated_bytes() - before, 0);
            assert_eq!(NvsStr::refcount_of(ptr), IMMORTAL_REFCOUNT);

            // `Clone` and `Drop` take the same two paths as the primitives.
            let handle = NvsStr::from_raw(ptr);
            let second = handle.clone();
            assert_eq!(second.refcount(), IMMORTAL_REFCOUNT);
            drop(second);
            drop(handle);
            assert_eq!(NvsStr::refcount_of(ptr), IMMORTAL_REFCOUNT);

            // `$s .= "!"` where `$s` holds a literal: the in-place path wants a
            // refcount of exactly one, so an immortal target copies out instead
            // of writing into a word two requests share.
            let suffix = NvsStr::new(b"!").into_raw();
            let grown = nvs_str_append(ptr, suffix.cast_const());
            assert_ne!(grown.cast_const(), ptr.cast_const());
            assert_eq!(NvsStr::bytes_of(grown), b"beta!");
            assert_eq!(NvsStr::bytes_of(ptr), b"beta");
            assert_eq!(NvsStr::refcount_of(ptr), IMMORTAL_REFCOUNT);
            nvs_str_release(grown);
            nvs_str_release(suffix);
        }
    }

    /// The strings a grapheme count is interesting over: an empty one, plain
    /// ASCII, a `CRLF` (GB3, the one ASCII rule that joins two bytes), a
    /// combining mark with and without its base, a ZWJ sequence, and a
    /// regional-indicator run whose parity a seam can split.
    const CORPUS: &[&str] = &[
        "",
        "a",
        "cafe",
        "e\u{301}",
        "\u{301}",
        "\r",
        "\n",
        "\r\n",
        "\u{1F1E9}",
        "\u{1F1E9}\u{1F1EA}",
        "\u{1F468}\u{200D}\u{1F469}",
        "\u{200D}",
        "héllo",
    ];

    /// `rule:types/bytes`'s *Consequences* asks that an immutable string's count be
    /// computed once: the first ask scans, every later one reads the fourth
    /// word, and a literal — whose header the compiled unit already carries —
    /// never scans at all.
    #[test]
    fn a_repeated_grapheme_count_is_answered_from_the_header() {
        use crate::graphemes::{forget_scans, scans};
        use unicode_segmentation::UnicodeSegmentation;

        let text = "e\u{301}\u{1F1E9}\u{1F1EA}ok";
        let expected = text.graphemes(true).count();
        let subject = NvsStr::new(text.as_bytes());

        forget_scans();
        assert_eq!(subject.grapheme_count(), expected);
        assert_eq!(scans(), 1, "the first ask is the one that scans");
        for _ in 0..16 {
            assert_eq!(subject.grapheme_count(), expected);
        }
        assert_eq!(scans(), 1, "every later ask reads the header");

        let mut unit = immortal_unit(text.as_bytes());
        let ptr = unit.as_mut_ptr().cast::<StrHeader>();
        forget_scans();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            assert_eq!(NvsStr::grapheme_count_of(ptr), expected);
            assert_eq!(NvsStr::grapheme_count_of(ptr), expected);
            // Asking did not write the word two requests share.
            assert_eq!(NvsStr::refcount_of(ptr), IMMORTAL_REFCOUNT);
        }
        assert_eq!(scans(), 0, "a literal's count is in the compiled unit");
    }

    /// The half a sum cannot do: a cluster spanning the join is one cluster,
    /// so `4 + 1` is `4` — and the correction costs no rescan of either side.
    #[test]
    fn a_concatenation_corrects_the_boundary_without_rescanning() {
        use crate::graphemes::{forget_scans, scans};

        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            let base = NvsStr::new(b"cafe").into_raw();
            let mark = NvsStr::new("\u{301}".as_bytes()).into_raw();
            let empty = NvsStr::new(b"").into_raw();
            // Room to spare, so the append below takes its in-place path.
            let target = NvsStr::build(16, |out| out.push(b"cafe")).into_raw();
            assert_eq!(NvsStr::grapheme_count_of(base), 4);
            assert_eq!(NvsStr::grapheme_count_of(mark), 1);
            assert_eq!(NvsStr::grapheme_count_of(empty), 0);
            assert_eq!(NvsStr::grapheme_count_of(target), 4);

            forget_scans();
            let joined = nvs_str_concat(lent(base), mark);
            assert_eq!(NvsStr::grapheme_count_of(joined), 4, "the seam joined");
            let apart = nvs_str_concat(lent(base), base);
            assert_eq!(NvsStr::grapheme_count_of(apart), 8, "and this one did not");
            assert_eq!(scans(), 0, "neither side was rescanned");

            // Three pieces with an empty one between them: two seams at one
            // byte are one join, not two.
            let pieces = [
                lent(base).cast_const(),
                empty.cast_const(),
                mark.cast_const(),
            ];
            let spanning = nvs_str_concat_n(pieces.as_ptr(), pieces.len());
            assert_eq!(NvsStr::grapheme_count_of(spanning), 4);
            assert_eq!(scans(), 0);

            // An append corrects its own seam without moving the payload.
            let appended = nvs_str_append(target, mark);
            assert_eq!(appended.cast_const(), target.cast_const(), "in place");
            assert_eq!(NvsStr::grapheme_count_of(appended), 4);
            assert_eq!(scans(), 0);

            // A flag pasted onto an odd run of them re-groups every flag after
            // the seam, so no per-seam correction is right and the result is
            // left to the ordinary scan — `graphemes::seam_joins` owns why.
            let one_flag = NvsStr::new("\u{1F1E9}".as_bytes()).into_raw();
            let pair = NvsStr::new("\u{1F1E9}\u{1F1EA}".as_bytes()).into_raw();
            assert_eq!(NvsStr::grapheme_count_of(one_flag), 1);
            assert_eq!(NvsStr::grapheme_count_of(pair), 1);
            forget_scans();
            let regrouped = nvs_str_concat(lent(one_flag), pair);
            assert_eq!(scans(), 0, "the concatenation itself still scans nothing");
            assert_eq!(NvsStr::grapheme_count_of(regrouped), 2);
            assert_eq!(scans(), 1);

            forget_scans();
            // An operand nobody has counted leaves the result uncounted rather
            // than scanning it: a concatenation that segments is the cost the
            // whole word exists to remove.
            let fresh = NvsStr::new("\u{301}".as_bytes()).into_raw();
            let unknown = nvs_str_concat(lent(base), fresh);
            assert_eq!(scans(), 0, "building it scanned nothing");
            assert_eq!(NvsStr::grapheme_count_of(unknown), 4);
            assert_eq!(scans(), 1, "the first ask paid the ordinary scan");

            for ptr in [
                base, mark, empty, joined, apart, spanning, appended, one_flag, pair, regrouped,
                fresh, unknown,
            ] {
                nvs_str_release(ptr);
            }
        }
    }

    /// The failure mode is a wrong index, not a slow one: however a string was
    /// built, its cached count is what a fresh segmentation of the same bytes
    /// answers.
    #[test]
    fn a_cached_count_equals_a_fresh_scan_over_the_corpus() {
        use unicode_segmentation::UnicodeSegmentation;

        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            for left in CORPUS {
                for right in CORPUS {
                    let expected = format!("{left}{right}").graphemes(true).count();

                    let lhs = NvsStr::new(left.as_bytes());
                    let rhs = NvsStr::new(right.as_bytes()).into_raw();
                    // Counted first, so the concatenation propagates rather
                    // than leaving the result unknown.
                    let _ = lhs.grapheme_count();
                    let _ = NvsStr::grapheme_count_of(rhs);

                    // A concatenation consumes one reference to its leading
                    // operand, so each call below is handed a clone's and
                    // `lhs` keeps the one it holds — which also makes these
                    // two the *shared* leading operand, whose result is a
                    // fresh buffer.
                    let joined = nvs_str_concat(lhs.clone().into_raw(), rhs);
                    assert_eq!(
                        NvsStr::grapheme_count_of(joined),
                        expected,
                        "{left:?} . {right:?}"
                    );

                    let leading = lhs.clone().into_raw();
                    let pieces = [leading.cast_const(), rhs.cast_const()];
                    let n_ary = nvs_str_concat_n(pieces.as_ptr(), pieces.len());
                    assert_eq!(
                        NvsStr::grapheme_count_of(n_ary),
                        expected,
                        "concat_n {left:?} . {right:?}"
                    );

                    // Both append paths, and both concatenation paths over a
                    // solely-owned leading operand: one with room to spare,
                    // which writes into the operand's own buffer and corrects
                    // the seam there, one without.
                    for capacity in [left.len(), left.len() + right.len()] {
                        let target =
                            NvsStr::build(capacity, |out| out.push(left.as_bytes())).into_raw();
                        let _ = NvsStr::grapheme_count_of(target);
                        let appended = nvs_str_append(target, rhs);
                        assert_eq!(
                            NvsStr::grapheme_count_of(appended),
                            expected,
                            "append {left:?} .= {right:?} at capacity {capacity}"
                        );
                        nvs_str_release(appended);

                        let owned =
                            NvsStr::build(capacity, |out| out.push(left.as_bytes())).into_raw();
                        let _ = NvsStr::grapheme_count_of(owned);
                        let reused = nvs_str_concat(owned, rhs);
                        assert_eq!(
                            NvsStr::grapheme_count_of(reused),
                            expected,
                            "concat into {left:?} at capacity {capacity}"
                        );
                        nvs_str_release(reused);
                    }

                    for ptr in [rhs, joined, n_ary] {
                        nvs_str_release(ptr);
                    }
                }
            }
        }
    }
}
