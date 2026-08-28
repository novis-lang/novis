//! Novis's refcounted string: one heap allocation, a three-word header, and the
//! bytes inline behind it.
//!
//! This is the first non-scalar representation the runtime owns, and the one
//! both `nvs_ir::ty::Ty::Str` and `nvs_ir::ty::Ty::Bytes` lower to. They share
//! it verbatim — the two differ only in the UTF-8 guarantee
//! ([ADR 0009](../../../docs/adr/0009-string-and-bytes.md)), which is a
//! checker property, not a layout one — so nothing here validates encoding.
//! They are told apart at the *tag*, not here; the crate docs'
//! § *`bytes` is a tag, not a second heap shape* owns that split.
//!
//! # Layout
//!
//! ```text
//! offset 0                              offset size_of::<StrHeader>()
//! +-------------+-----------+---------+ +------------------------------+
//! | refcount    | len       | cap     | | cap bytes, the first len live |
//! +-------------+-----------+---------+ +------------------------------+
//! ```
//!
//! One allocation, not two. A `Box<StrData>` holding a `Box<[u8]>` would be
//! simpler to write, but it costs a second allocation and a second cache miss
//! on every string produced, which is a latency question (priority 3 in
//! [AGENTS.md](../../../AGENTS.md)) rather than a footprint one. Codegen will
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
//! What the third word costs is **8 bytes per string allocation**, a 16-byte
//! header becoming 24. What it buys is that `$out .= $piece` stops being
//! quadratic: without a capacity there is nowhere to append *into*, so every
//! iteration allocated a fresh buffer and copied the whole accumulation into
//! it — 50,000 appends took 238 ms and 100,000 took 1,386 ms, the
//! super-linear shape being the tell. A string that is appended to holds up
//! to **twice its payload**, which is [`grown_capacity`]'s doubling; a string
//! that is never appended to holds exactly its payload. That is priority 5
//! spent on priority 3, which is the direction [AGENTS.md](../../../AGENTS.md)
//! asks for, and it is the whole of what this word spends.
//!
//! # Why the refcount is a plain `Cell`
//!
//! A request shares nothing with any other request but compiled code
//! (`docs/adr/README.md`'s project-start decisions), and a value crossing a
//! `spawn`/`spawn worker`/`spawn script` boundary is deep-copied rather than
//! shared ([ADR 0023](../../../docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)).
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
//! written belongs to an allocation [`NvsStr::alloc_uninit`] made on the
//! request's own thread and reachable from nowhere else.
//!
//! What it costs is one compare and a not-taken branch per release — the
//! hottest operation in the runtime — against a sentinel every allocated
//! string misses. Nothing else changes: an immortal string is never mutated
//! in place either, because [`nvs_str_append`] takes its in-place path only at
//! a refcount of exactly one, so it copies out of an immortal exactly as it
//! copies out of a shared one.
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
//! ([ADR 0009](../../../docs/adr/0009-string-and-bytes.md)). Its § 3 makes
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
//! a remembered one.

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
    /// [`nvs_str_append`] and [`NvsStr::build`], and only while this
    /// allocation has exactly one owner.
    len: Cell<usize>,
    /// How many payload bytes the allocation has room for — what the layout
    /// this header was allocated with, and will be freed with, is computed
    /// from. Immutable for the allocation's lifetime: growing means a new
    /// allocation, never a bigger `cap` on this one.
    cap: usize,
}

/// Byte offset of the reference count within [`StrHeader`].
pub const REFCOUNT_OFFSET: usize = std::mem::offset_of!(StrHeader, refcount);

/// Byte offset of the payload length within [`StrHeader`].
pub const LEN_OFFSET: usize = std::mem::offset_of!(StrHeader, len);

/// Byte offset of the payload capacity within [`StrHeader`].
pub const CAP_OFFSET: usize = std::mem::offset_of!(StrHeader, cap);

/// Byte offset of the payload itself, relative to the [`StrHeader`] pointer.
pub const PAYLOAD_OFFSET: usize = std::mem::size_of::<StrHeader>();

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
/// payload, in the host's byte order.
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
pub fn immortal_header_bytes(len: usize) -> [u8; PAYLOAD_OFFSET] {
    let mut header = [0_u8; PAYLOAD_OFFSET];
    // `cap` equals `len`, as it does for every string this module builds: an
    // immortal one has no spare room to append into and could not use it.
    for (offset, word) in [
        (REFCOUNT_OFFSET, IMMORTAL_REFCOUNT),
        (LEN_OFFSET, len),
        (CAP_OFFSET, len),
    ] {
        header[offset..offset + std::mem::size_of::<usize>()].copy_from_slice(&word.to_ne_bytes());
    }
    header
}

/// The allocation shape for a string with room for `cap` payload bytes.
///
/// Takes the *capacity*, never the length: this is the layout an allocation is
/// both made and freed with, and those two must be the same one.
fn str_layout(cap: usize) -> Layout {
    try_str_layout(cap).expect("string capacity overflows the address space")
}

/// [`str_layout`], answering `None` for a capacity no allocation could have.
///
/// The two ways a layout does not exist are the same two `str_layout` used to
/// `expect` its way past: the header plus `cap` overflowing `usize`, and the
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

/// How much room a string of `len` bytes takes when it has to grow to hold
/// `needed` — doubling, floored at what is actually asked for.
///
/// Doubling is what makes a loop of appends linear overall rather than
/// quadratic: each reallocation copies `len` bytes but at least doubles the
/// room, so the copies sum to under twice the final length however many
/// appends there were. The cost is that an appended-to string holds up to
/// twice its payload, which this module's docs state as what capacity spends.
fn grown_capacity(len: usize, needed: usize) -> usize {
    needed.max(len.saturating_mul(2))
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
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)'s
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
    /// # Panics
    ///
    /// Aborts through [`handle_alloc_error`] if the allocator fails, per
    /// [`NvsStr::new`]. [`NvsStr::try_build`] is the half that answers instead.
    #[must_use]
    pub fn build(capacity: usize, write: impl FnOnce(&mut StrWriter<'_>)) -> Self {
        Self::written_into(Self::alloc_uninit(0, capacity), capacity, write)
    }

    /// [`NvsStr::build`], answering `None` where that one aborts.
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
    /// `capacity` still grows through [`StrWriter::grow`], which aborts, so
    /// this is for a producer whose capacity is *exact* — which is the same
    /// set of producers as the ones whose capacity is a count.
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

    /// A fresh allocation with room for `cap` payload bytes, a reference count
    /// of one, and a length of `len` whose bytes are **left uninitialized**.
    ///
    /// The one place an Novis string allocation is made, so [`str_layout`] is
    /// called with a capacity here and in [`Drop`] and nowhere else. The
    /// caller must write all `len` payload bytes before the handle escapes.
    ///
    /// # Panics
    ///
    /// Debug-asserts `len <= cap`; aborts through [`handle_alloc_error`] if
    /// the allocator fails, per [`NvsStr::new`].
    fn alloc_uninit(len: usize, cap: usize) -> NonNull<StrHeader> {
        Self::try_alloc_uninit(len, cap).unwrap_or_else(|| handle_alloc_error(str_layout(cap)))
    }

    /// [`NvsStr::alloc_uninit`], answering `None` where that one aborts.
    ///
    /// The `alloc` call itself is here rather than in both, so the layout an
    /// allocation is made with stays the single expression [`Drop`] frees it
    /// with.
    fn try_alloc_uninit(len: usize, cap: usize) -> Option<NonNull<StrHeader>> {
        debug_assert!(
            len <= cap,
            "an Novis string's length never exceeds its capacity"
        );
        let layout = try_str_layout(cap)?;
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
    /// ADR 0009's UTF-8 invariant.
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
            std::str::from_utf8(bytes).is_ok(),
            "a `string` payload is well-formed UTF-8 by ADR 0009's construction"
        );
        #[expect(
            unsafe_code,
            reason = "the caller guarantees this payload is a `string`'s, which \
                      ADR 0009 makes well-formed UTF-8; the assertion above is \
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
        if needed > self.capacity {
            self.grow(needed);
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

    /// Grows the allocation to hold `needed` bytes.
    ///
    /// The same doubling `nvs_str_append` grows by, so a producer whose
    /// capacity was a guess pays exactly what the `String` it replaced paid,
    /// and one whose capacity was exact never reaches here at all.
    #[cold]
    fn grow(&mut self, needed: usize) {
        let capacity = grown_capacity(self.capacity, needed);
        let bigger = str_layout(capacity);
        // `realloc` and not an allocate-copy-free of our own, for the reason
        // `Vec` uses it: an allocator that can extend the block in place does,
        // and the bytes already written are then not moved at all. Measured —
        // `Core\Str::join` copying instead cost the whole saving this seam
        // exists for, and then some.
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
                      header `alloc_uninit` wrote; the capacity it will be \
                      freed with has to be the one it now has"
        )]
        unsafe {
            (&raw mut (*ptr.as_ptr()).cap).write(capacity);
        }
        self.ptr = ptr;
        self.capacity = capacity;
    }
}

impl Drop for StrWriter<'_> {
    /// Frees the allocation, which only happens when `write` panicked partway
    /// through: [`NvsStr::build`] takes the allocation out of the writer on
    /// the path that finishes. A helper's panic is a contained `FATAL`
    /// (ADR 0020), so it must not also be a leak.
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
                      same `cap` `alloc_uninit` allocated with — never from \
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
// These are `extern "C"` but deliberately *not* ADR 0002's checked-return
// helper shape, and deliberately not written through `nvs_helper!`. That shape
// exists to carry a failure back to the caller; none of these can fail — they
// take no Novis value, allocate at most once, and produce no status — so giving
// them an unused `*const Value`/`*mut Value` pair and a `catch_unwind` would
// cost the hottest operations in the runtime an ABI they never use. They are
// panic-free by construction instead: the only fallible step is allocation,
// which `handle_alloc_error` turns into an abort rather than an unwind. Every
// one is still `extern "C"` and never `extern "C-unwind"`, so nothing can
// unwind through a JIT frame either way.

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

/// Allocates a fresh string holding `lhs`'s bytes followed by `rhs`'s, with a
/// reference count of one — `nvs_ir::InstKind::Concat`'s entry point for the
/// two-operand case, which is the common one and is kept because it needs
/// neither the stack array nor the count [`nvs_str_concat_n`] takes.
///
/// Neither operand is retained or released: that instruction only *reads* its
/// two operands to build the new buffer, and ownership of each stays wherever
/// it already was. `nvs_ir::ir::InstKind::Concat`'s own doc comment is the one
/// home for that rule.
///
/// # Safety
///
/// `lhs` and `rhs` must each refer to a live Novis string allocation. They may
/// be the same allocation: the pieces are read before the destination is
/// written, and the destination is a fresh allocation regardless.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw string pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_concat(
    lhs: *const StrHeader,
    rhs: *const StrHeader,
) -> *mut StrHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both pointees are live; the borrows \
                  end before `from_pieces` returns, and it writes only into \
                  the fresh allocation it made"
    )]
    let (left, right) = unsafe { (NvsStr::bytes_of(lhs), NvsStr::bytes_of(rhs)) };
    NvsStr::from_pieces(&[left, right]).into_raw()
}

/// Allocates a fresh string holding every piece's bytes in order, with a
/// reference count of one — [`nvs_str_concat`] for three or more operands, and
/// the entry point `nvs_ir::InstKind::Concat` takes once it carries that many.
///
/// **One allocation for the whole expression.** `"<tr><td>" . $i . "</td>"`
/// used to fold left into a chain of [`nvs_str_concat`] calls, so an n-operand
/// concatenation allocated n-1 buffers and copied a growing prefix into each
/// one; here the total length is summed first and every piece is copied once.
///
/// Ownership is [`nvs_str_concat`]'s exactly: each piece is only *read*, so
/// none is retained and none is released — `nvs_ir::ir::InstKind::Concat`'s own
/// doc comment is the one home for that rule.
///
/// It does not route through [`NvsStr::from_pieces`], which wants a
/// `&[&[u8]]`: materializing one from the pointer array would be a second
/// allocation on the path whose whole point is to have exactly one. The two
/// loops here are that function's two, over `bytes_of` instead of over
/// borrowed slices.
///
/// # Safety
///
/// `pieces` must point at `count` consecutive `*const StrHeader`, each
/// referring to a live Novis string allocation. Two of them may be the same
/// allocation: every piece is read before the destination — a fresh
/// allocation regardless — is written.
#[expect(
    unsafe_code,
    reason = "compiled code passes a stack array of raw string pointers whose \
              liveness and count the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_str_concat_n(
    pieces: *const *const StrHeader,
    count: usize,
) -> *mut StrHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `count` live pointers at `pieces` and a \
                  live pointee behind each; every borrow ends before the fresh \
                  allocation this returns is written"
    )]
    unsafe {
        let pieces = std::slice::from_raw_parts(pieces, count);
        let len = pieces
            .iter()
            .try_fold(0_usize, |total, piece| {
                total.checked_add(NvsStr::bytes_of(*piece).len())
            })
            .expect("an Novis string's length cannot overflow a usize");
        let out = NvsStr::alloc_uninit(len, len);
        let dst = out.as_ptr().cast::<u8>().add(PAYLOAD_OFFSET);
        let mut written = 0_usize;
        for piece in pieces {
            let bytes = NvsStr::bytes_of(*piece);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst.add(written), bytes.len());
            written += bytes.len();
        }
        out.as_ptr()
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
/// moves. That is the whole point: `$out .= $piece` copied the entire
/// accumulation every iteration before this existed, which is quadratic in
/// the number of appends.
///
/// Exactly two things force a fresh allocation instead:
///
/// - **A reference count above one.** A second owner can see these bytes, and
///   an append is a write; ADR 0007 § 5's copy-on-write value semantics do not
///   let that owner observe it. This is the same separation `nvs_array_set`
///   performs for the same reason, and it is what keeps the in-place path
///   sound rather than merely fast.
/// - **Too little room**, in which case [`grown_capacity`] decides how much to
///   ask for and the payload moves once.
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
        if header.refcount.get() == 1 && header.cap >= needed {
            // The two ranges cannot overlap even when `suffix == target`: the
            // source is the first `added` bytes of the payload and the
            // destination begins at `len`, which is `added` when they are the
            // same allocation and irrelevant when they are not.
            let dst = target.cast::<u8>().add(PAYLOAD_OFFSET + len);
            std::ptr::copy_nonoverlapping(src, dst, added);
            header.len.set(needed);
            return target;
        }
        let grown = NvsStr::alloc_uninit(needed, grown_capacity(len, needed));
        let dst = grown.as_ptr().cast::<u8>().add(PAYLOAD_OFFSET);
        std::ptr::copy_nonoverlapping(target.cast::<u8>().add(PAYLOAD_OFFSET), dst, len);
        std::ptr::copy_nonoverlapping(src, dst.add(len), added);
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
/// ([ADR 0009](../../../docs/adr/0009-string-and-bytes.md)), and PHP's `===`
/// on two strings is byte equality, which is what Novis keeps. Neither operand
/// is retained or released — the same read-only treatment
/// [`nvs_str_concat`] gives its two.
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
        NvsStr::bytes_of(lhs) == NvsStr::bytes_of(rhs)
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
            let joined = nvs_str_concat(lhs, rhs);
            assert_eq!(NvsStr::bytes_of(joined), b"quadruple(5) = 20");
            assert_eq!(NvsStr::refcount_of(joined), 1);
            // Neither operand is retained or released by the concatenation —
            // see `nvs_str_concat`'s own doc comment.
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
                let joined = nvs_str_concat(lhs, rhs);
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
            let pieces = [open.cast_const(), mid.cast_const(), close.cast_const()];
            let joined = nvs_str_concat_n(pieces.as_ptr(), pieces.len());
            assert_eq!(NvsStr::bytes_of(joined), b"<tr><td>7</td></tr>");
            assert_eq!(NvsStr::refcount_of(joined), 1);
            // No piece is retained or released — `nvs_str_concat`'s rule,
            // unchanged by the arity.
            for piece in pieces {
                assert_eq!(NvsStr::refcount_of(piece), 1);
            }
            nvs_str_release(joined);

            // An empty piece, and the same allocation appearing twice: every
            // piece is read before the fresh destination is written.
            let repeated = [
                mid.cast_const(),
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
    /// than asserted about: an n-piece concatenation allocates one buffer,
    /// where the fold of two-operand `nvs_str_concat` calls it replaced
    /// allocated n-1 of them and copied its leading pieces n-1 times.
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

            let before = allocated_bytes();
            let joined = nvs_str_concat_n(pieces.as_ptr(), pieces.len());
            let n_ary = allocated_bytes() - before;
            assert_eq!(NvsStr::bytes_of(joined).len(), total);
            assert_eq!(n_ary, PAYLOAD_OFFSET + total);
            nvs_str_release(joined);

            // The shape it replaced, for the same eight pieces: seven
            // allocations, each holding the accumulation so far.
            let before = allocated_bytes();
            let mut folded = nvs_str_concat(pieces[0], pieces[1]);
            for piece in &pieces[2..] {
                let next = nvs_str_concat(folded, *piece);
                nvs_str_release(folded);
                folded = next;
            }
            let fold = allocated_bytes() - before;
            assert_eq!(NvsStr::bytes_of(folded).len(), total);
            nvs_str_release(folded);

            assert!(
                n_ary * 4 < fold,
                "eight pieces cost {n_ary} bytes n-ary against the fold's {fold}: \
                 the pieces are being copied more than once"
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
        assert_eq!(PAYLOAD_OFFSET, 3 * word);
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
            // Copying the accumulation every iteration would be quadratic —
            // 5 MB for this run. Doubling makes the reallocations sum to under
            // four times the final length, headers included.
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
        view[..PAYLOAD_OFFSET].copy_from_slice(&immortal_header_bytes(bytes.len()));
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
    /// what a literal used to cost was one `nvs_str_new` per evaluation, freed
    /// again straight away, which a `live_bytes` delta cannot see at all.
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
}
