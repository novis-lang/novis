//! MWL's refcounted string: one heap allocation, a three-word header, and the
//! bytes inline behind it.
//!
//! This is the first non-scalar representation the runtime owns, and the one
//! both `mwl_ir::ty::Ty::Str` and `mwl_ir::ty::Ty::Bytes` lower to. They share
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
//! than calling [`mwl_str_retain`]/[`mwl_str_release`]; those constants exist
//! so the layout is queried, never restated.
//!
//! # Capacity, and what it spends
//!
//! `cap` is how many payload bytes the allocation has room for; `len` is how
//! many are live. **Every constructor here sets the two equal**, so the only
//! way a string ever holds an unused byte is [`mwl_str_append`] growing one.
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
//! No `MwlStr` is ever reachable from two threads, so an atomic increment
//! would buy nothing and cost a locked instruction on the hottest operation
//! in the runtime. [`MwlStr`] is correspondingly neither `Send` nor `Sync`,
//! which is what makes that reasoning checkable rather than remembered.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::Cell;
use std::fmt;
use std::ptr::NonNull;

/// The header sitting in front of every MWL string's bytes.
///
/// `#[repr(C)]` because compiled code reads these fields at fixed offsets.
/// Never construct one by value — it is only ever the first
/// `size_of::<StrHeader>()` bytes of a larger allocation made by
/// [`MwlStr::new`], and moving it would leave the payload behind.
#[repr(C)]
#[derive(Debug)]
pub struct StrHeader {
    /// How many owners hold this allocation. Reaching `0` frees it.
    refcount: Cell<usize>,
    /// Payload length in bytes. Written only by [`mwl_str_append`], and only
    /// while this allocation has exactly one owner.
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

/// The allocation shape for a string with room for `cap` payload bytes.
///
/// Takes the *capacity*, never the length: this is the layout an allocation is
/// both made and freed with, and those two must be the same one.
fn str_layout(cap: usize) -> Layout {
    let size = PAYLOAD_OFFSET
        .checked_add(cap)
        .expect("string capacity overflows the address space");
    Layout::from_size_align(size, std::mem::align_of::<StrHeader>())
        .expect("string layout is always valid: alignment is a power of two")
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

/// An owning handle to one reference of an MWL string.
///
/// Cloning retains, dropping releases — so Rust-side code (helpers, tests,
/// eventually the stdlib) manipulates strings without writing a refcount
/// operation by hand. Compiled code instead calls the
/// [`mwl_str_new`]/[`mwl_str_retain`]/[`mwl_str_release`] primitives, which
/// are the same operations with the ownership left implicit.
///
/// Neither `Send` nor `Sync`, by construction — see this module's docs.
#[repr(transparent)]
pub struct MwlStr {
    ptr: NonNull<StrHeader>,
}

impl MwlStr {
    /// Allocates a fresh string with a reference count of one.
    ///
    /// # Panics
    ///
    /// Aborts the process through [`handle_alloc_error`] if the allocator
    /// fails. A request-attributable out-of-memory is
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)'s
    /// resource-limit tier and belongs to the per-request arena that does not
    /// exist yet (known gap 4 in the crate docs); until it does, the global
    /// allocator's own behaviour is the honest one.
    #[must_use]
    pub fn new(bytes: &[u8]) -> Self {
        Self::from_pieces(&[bytes])
    }

    /// Allocates a fresh string holding `pieces` end to end, with a reference
    /// count of one — **one** allocation regardless of how many pieces there
    /// are, which is the whole reason concatenation does not simply build a
    /// `Vec` and hand it to [`MwlStr::new`]. See [`MwlStr::new`] for the
    /// allocation-failure behaviour, which is shared.
    #[must_use]
    pub fn from_pieces(pieces: &[&[u8]]) -> Self {
        let len = pieces
            .iter()
            .try_fold(0_usize, |total, piece| total.checked_add(piece.len()))
            .expect("an MWL string's length cannot overflow a usize");
        let ptr = Self::alloc_uninit(len, len);
        #[expect(
            unsafe_code,
            reason = "`alloc_uninit` returned an allocation with room for \
                      `len` payload bytes, so writing them stays inside it; \
                      the regions cannot overlap because each piece borrows a \
                      different allocation, and the running offset never \
                      exceeds `len` because that is their summed length"
        )]
        unsafe {
            let mut at = ptr.as_ptr().cast::<u8>().add(PAYLOAD_OFFSET);
            for piece in pieces {
                std::ptr::copy_nonoverlapping(piece.as_ptr(), at, piece.len());
                at = at.add(piece.len());
            }
        }
        Self { ptr }
    }

    /// A fresh allocation with room for `cap` payload bytes, a reference count
    /// of one, and a length of `len` whose bytes are **left uninitialized**.
    ///
    /// The one place an MWL string allocation is made, so [`str_layout`] is
    /// called with a capacity here and in [`Drop`] and nowhere else. The
    /// caller must write all `len` payload bytes before the handle escapes.
    ///
    /// # Panics
    ///
    /// Debug-asserts `len <= cap`; aborts through [`handle_alloc_error`] if
    /// the allocator fails, per [`MwlStr::new`].
    fn alloc_uninit(len: usize, cap: usize) -> NonNull<StrHeader> {
        debug_assert!(
            len <= cap,
            "an MWL string's length never exceeds its capacity"
        );
        let layout = str_layout(cap);
        #[expect(
            unsafe_code,
            reason = "a flexible-array-member allocation cannot be expressed \
                      in safe Rust; `layout` is non-zero-sized because \
                      PAYLOAD_OFFSET > 0, which is `alloc`'s one precondition"
        )]
        let raw = unsafe { alloc(layout) };
        let Some(ptr) = NonNull::new(raw.cast::<StrHeader>()) else {
            handle_alloc_error(layout)
        };
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
        ptr
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
    /// Equal to [`MwlStr::len`] for every string this module constructs; only
    /// [`mwl_str_append`] ever leaves the two apart. Exposed for the same
    /// reason [`MwlStr::refcount`] is: the growth policy is only checkable by
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
    /// Exposed because the refcount insertion policy in `mwl_ir::lower` is
    /// only checkable by observing it — see this module's own tests, and
    /// eventually `mwl-codegen`'s.
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
    /// pointer to [`mwl_str_release`] or [`MwlStr::from_raw`].
    #[must_use]
    pub fn into_raw(self) -> *mut StrHeader {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Reclaims a reference previously given up by [`MwlStr::into_raw`].
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer produced by [`MwlStr::into_raw`] (or by
    /// [`mwl_str_new`]) whose reference has not already been released, and it
    /// must not be reclaimed twice.
    ///
    /// # Panics
    ///
    /// If `ptr` is null, which no MWL string pointer ever is.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "reclaiming a reference is the caller's obligation to state"
    )]
    pub unsafe fn from_raw(ptr: *mut StrHeader) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("an MWL string pointer is never null"),
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
    /// `ptr` must refer to a live MWL string allocation that stays live for
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

    /// How many owners hold the string `ptr` refers to.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live MWL string allocation.
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

impl Clone for MwlStr {
    fn clone(&self) -> Self {
        let header = self.header();
        header.refcount.set(
            header
                .refcount
                .get()
                .checked_add(1)
                .expect("an MWL string's reference count cannot overflow a usize"),
        );
        Self { ptr: self.ptr }
    }
}

impl Drop for MwlStr {
    fn drop(&mut self) {
        let remaining = self.header().refcount.get() - 1;
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
                      `len`, which `mwl_str_append` may have left smaller — \
                      before the header is freed"
        )]
        unsafe {
            dealloc(self.ptr.as_ptr().cast::<u8>(), layout);
        }
    }
}

impl fmt::Debug for MwlStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MwlStr")
            .field("refcount", &self.refcount())
            .field("bytes", &String::from_utf8_lossy(self.as_bytes()))
            .finish()
    }
}

impl PartialEq for MwlStr {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for MwlStr {}

/// Hashes the payload, and only the payload — never the pointer.
///
/// Together with [`Borrow<[u8]>`](std::borrow::Borrow) below this is what lets
/// [`crate::array`]'s index map be keyed by a handle and looked up by bytes:
/// a key is stored twice as a pointer, never twice as bytes. The two impls
/// must agree, which they do because both defer to `<[u8] as Hash>`.
impl std::hash::Hash for MwlStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl std::borrow::Borrow<[u8]> for MwlStr {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// These are `extern "C"` but deliberately *not* ADR 0002's checked-return
// helper shape, and deliberately not written through `mwl_helper!`. That shape
// exists to carry a failure back to the caller; none of these can fail — they
// take no MWL value, allocate at most once, and produce no status — so giving
// them an unused `*const Value`/`*mut Value` pair and a `catch_unwind` would
// cost the hottest operations in the runtime an ABI they never use. They are
// panic-free by construction instead: the only fallible step is allocation,
// which `handle_alloc_error` turns into an abort rather than an unwind. Every
// one is still `extern "C"` and never `extern "C-unwind"`, so nothing can
// unwind through a JIT frame either way.

/// Allocates a fresh string from `len` bytes at `ptr`, with a reference count
/// of one — `mwl_ir::InstKind::ConstStr`'s entry point.
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
pub unsafe extern "C" fn mwl_str_new(ptr: *const u8, len: usize) -> *mut StrHeader {
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
    MwlStr::new(bytes).into_raw()
}

/// Allocates a fresh string holding `lhs`'s bytes followed by `rhs`'s, with a
/// reference count of one — `mwl_ir::InstKind::Concat`'s entry point.
///
/// Neither operand is retained or released: that instruction only *reads* its
/// two operands to build the new buffer, and ownership of each stays wherever
/// it already was. `mwl_ir::ir::InstKind::Concat`'s own doc comment is the one
/// home for that rule.
///
/// # Safety
///
/// `lhs` and `rhs` must each refer to a live MWL string allocation. They may
/// be the same allocation: the pieces are read before the destination is
/// written, and the destination is a fresh allocation regardless.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw string pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_str_concat(
    lhs: *const StrHeader,
    rhs: *const StrHeader,
) -> *mut StrHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both pointees are live; the borrows \
                  end before `from_pieces` returns, and it writes only into \
                  the fresh allocation it made"
    )]
    let (left, right) = unsafe { (MwlStr::bytes_of(lhs), MwlStr::bytes_of(rhs)) };
    MwlStr::from_pieces(&[left, right]).into_raw()
}

/// Appends `suffix`'s bytes to `target`'s — `mwl_ir::InstKind::StrAppend`'s
/// entry point, and the reason [`StrHeader`] carries a capacity at all.
///
/// **Consumes one reference to `target` and yields one to the result**, which
/// is `mwl_array_set`'s protocol verbatim: the reference consumed is the
/// holder's, the one yielded replaces it in that same slot, and the caller
/// therefore retains nothing and releases nothing.
/// `mwl_ir::ir::InstKind::ArraySet`'s own doc comment is the worked statement
/// of that protocol and the one home for it. `suffix` is only *read*, exactly
/// as [`mwl_str_concat`] reads both of its operands — neither retained nor
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
///   let that owner observe it. This is the same separation `mwl_array_set`
///   performs for the same reason, and it is what keeps the in-place path
///   sound rather than merely fast.
/// - **Too little room**, in which case [`grown_capacity`] decides how much to
///   ask for and the payload moves once.
///
/// # Safety
///
/// `target` must refer to a live MWL string allocation whose reference this
/// caller owns and does not release again; `suffix` must refer to a live one.
/// They may be the same allocation — `$s .= $s` — which is why the in-place
/// path's two byte ranges are argued disjoint rather than assumed so.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw string pointers whose liveness and \
              ownership the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_str_append(
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
            .expect("an MWL string's length cannot overflow a usize");
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
        let grown = MwlStr::alloc_uninit(needed, grown_capacity(len, needed));
        let dst = grown.as_ptr().cast::<u8>().add(PAYLOAD_OFFSET);
        std::ptr::copy_nonoverlapping(target.cast::<u8>().add(PAYLOAD_OFFSET), dst, len);
        std::ptr::copy_nonoverlapping(src, dst.add(len), added);
        // Last, so that `suffix`'s bytes are read before a `suffix == target`
        // release can free them.
        mwl_str_release(target);
        grown.as_ptr()
    }
}

/// Whether two strings hold the same bytes — `mwl_ir::ir::BinOp::Eq` over a
/// `Ty::Str` operand pair.
///
/// A byte comparison, not a collation: `string` is guaranteed-valid UTF-8
/// ([ADR 0009](../../../docs/adr/0009-string-and-bytes.md)), and PHP's `===`
/// on two strings is byte equality, which is what MWL keeps. Neither operand
/// is retained or released — the same read-only treatment
/// [`mwl_str_concat`] gives its two.
///
/// # Safety
///
/// `lhs` and `rhs` must each refer to a live MWL string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw string pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_str_eq(lhs: *const StrHeader, rhs: *const StrHeader) -> bool {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both pointees are live; both borrows \
                  end with this comparison"
    )]
    unsafe {
        MwlStr::bytes_of(lhs) == MwlStr::bytes_of(rhs)
    }
}

/// Adds a reference — `mwl_ir::InstKind::Retain` for a `Ty::Str` operand.
///
/// A null `ptr` is a no-op: see [`crate::object`]'s *A null payload is `null`*
/// for why that is the rule rather than a defensive check.
///
/// # Safety
///
/// `ptr` must be null or refer to a live MWL string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw string pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_str_retain(ptr: *mut StrHeader) {
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
    header.refcount.set(
        header
            .refcount
            .get()
            .checked_add(1)
            .expect("an MWL string's reference count cannot overflow a usize"),
    );
}

/// Drops a reference, freeing the allocation if it was the last —
/// `mwl_ir::InstKind::Release` for a `Ty::Str` operand.
///
/// A null `ptr` is a no-op — see [`mwl_str_retain`].
///
/// # Safety
///
/// `ptr` must be null, or refer to a live MWL string allocation whose
/// reference this caller owns; a non-null one must not be released twice.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw string pointer whose ownership the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_str_release(ptr: *mut StrHeader) {
    if ptr.is_null() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns the reference `from_raw` \
                  reclaims; dropping the handle is what releases it"
    )]
    unsafe {
        drop(MwlStr::from_raw(ptr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_string_has_one_reference_and_its_bytes() {
        let s = MwlStr::new(b"Hello, World!");
        assert_eq!(s.as_bytes(), b"Hello, World!");
        assert_eq!(s.len(), 13);
        assert!(!s.is_empty());
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn an_empty_string_is_a_real_allocation() {
        let s = MwlStr::new(b"");
        assert!(s.is_empty());
        assert_eq!(s.as_bytes(), b"");
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn cloning_retains_and_dropping_releases() {
        let s = MwlStr::new(b"abc");
        let t = s.clone();
        assert_eq!(s.refcount(), 2);
        assert_eq!(t.refcount(), 2);
        assert_eq!(t.as_bytes(), b"abc");
        drop(t);
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn the_raw_primitives_move_the_same_count() {
        let raw = MwlStr::new(b"xy").into_raw();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            mwl_str_retain(raw);
            assert_eq!(MwlStr::refcount_of(raw), 2);
            assert_eq!(MwlStr::bytes_of(raw), b"xy");
            mwl_str_release(raw);
            assert_eq!(MwlStr::refcount_of(raw), 1);
            mwl_str_release(raw);
        }
    }

    #[test]
    fn str_new_copies_the_bytes_it_is_given() {
        let source = b"Hello, World!".to_vec();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let owned = unsafe {
            let raw = mwl_str_new(source.as_ptr(), source.len());
            MwlStr::from_raw(raw)
        };
        drop(source);
        assert_eq!(owned.as_bytes(), b"Hello, World!");
    }

    #[test]
    fn str_new_accepts_a_null_pointer_for_an_empty_string() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let owned = unsafe { MwlStr::from_raw(mwl_str_new(std::ptr::null(), 0)) };
        assert!(owned.is_empty());
    }

    #[test]
    fn concat_joins_two_strings_into_one_fresh_allocation() {
        let lhs = MwlStr::new(b"quadruple(5) = ").into_raw();
        let rhs = MwlStr::new(b"20").into_raw();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let joined = mwl_str_concat(lhs, rhs);
            assert_eq!(MwlStr::bytes_of(joined), b"quadruple(5) = 20");
            assert_eq!(MwlStr::refcount_of(joined), 1);
            // Neither operand is retained or released by the concatenation —
            // see `mwl_str_concat`'s own doc comment.
            assert_eq!(MwlStr::refcount_of(lhs), 1);
            assert_eq!(MwlStr::refcount_of(rhs), 1);
            mwl_str_release(joined);
            mwl_str_release(lhs);
            mwl_str_release(rhs);
        }
    }

    #[test]
    fn concat_handles_empty_operands_and_an_aliased_one() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let empty = MwlStr::new(b"").into_raw();
            let abc = MwlStr::new(b"abc").into_raw();

            for (lhs, rhs, expected) in [
                (empty, abc, &b"abc"[..]),
                (abc, empty, &b"abc"[..]),
                (empty, empty, &b""[..]),
                // The same allocation on both sides: read twice, written
                // nowhere but the fresh result.
                (abc, abc, &b"abcabc"[..]),
            ] {
                let joined = mwl_str_concat(lhs, rhs);
                assert_eq!(MwlStr::bytes_of(joined), expected);
                mwl_str_release(joined);
            }

            mwl_str_release(empty);
            mwl_str_release(abc);
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
        assert_eq!(MwlStr::new(b"abc").capacity(), 3);
        assert_eq!(MwlStr::new(b"").capacity(), 0);
        assert_eq!(MwlStr::from_pieces(&[b"ab", b"cd"]).capacity(), 4);
    }

    #[test]
    fn appending_into_spare_capacity_keeps_the_same_allocation() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            // The first append has nothing to double, so it grows exactly;
            // the second doubles and leaves room the third appends into.
            let (b, c, d) = (
                MwlStr::new(b"b").into_raw(),
                MwlStr::new(b"c").into_raw(),
                MwlStr::new(b"d").into_raw(),
            );
            let acc = mwl_str_append(MwlStr::new(b"a").into_raw(), b);
            let grown = mwl_str_append(acc, c);
            assert_eq!(MwlStr::bytes_of(grown), b"abc");
            let same = mwl_str_append(grown, d);
            assert_eq!(
                same, grown,
                "the fourth byte fit in the room the third made"
            );
            assert_eq!(MwlStr::bytes_of(same), b"abcd");
            assert_eq!(MwlStr::refcount_of(same), 1);
            for ptr in [same, b, c, d] {
                mwl_str_release(ptr);
            }
        }
    }

    #[test]
    fn appending_to_a_shared_string_separates_rather_than_writing_it() {
        let held = MwlStr::new(b"abc");
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            // One reference for `held`, one for the append to consume.
            let target = held.clone().into_raw();
            let suffix = MwlStr::new(b"def").into_raw();
            let appended = mwl_str_append(target, suffix);
            assert_ne!(
                appended, target,
                "a second owner forbids the in-place write"
            );
            assert_eq!(MwlStr::bytes_of(appended), b"abcdef");
            assert_eq!(MwlStr::refcount_of(appended), 1);
            mwl_str_release(appended);
            mwl_str_release(suffix);
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
            let s = MwlStr::new(b"xy").into_raw();
            let doubled = mwl_str_append(s, s);
            assert_eq!(MwlStr::bytes_of(doubled), b"xyxy");
            let quadrupled = mwl_str_append(doubled, doubled);
            assert_eq!(MwlStr::bytes_of(quadrupled), b"xyxyxyxy");
            assert_eq!(MwlStr::refcount_of(quadrupled), 1);
            mwl_str_release(quadrupled);
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
            let piece = MwlStr::new(b"0123456789").into_raw();
            let mut acc = MwlStr::new(b"").into_raw();

            let before = allocated_bytes();
            for _ in 0..RUN {
                acc = mwl_str_append(acc, piece);
            }
            let spent = allocated_bytes() - before;

            assert_eq!(MwlStr::bytes_of(acc).len(), RUN * PIECE);
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
            let same = mwl_str_append(acc, piece);
            assert_eq!(same, acc, "the append had the room to write into");
            assert_eq!(
                allocated_bytes() - quiet,
                0,
                "an in-place append allocated something"
            );
            mwl_str_release(same);
            mwl_str_release(piece);
        }
    }

    #[test]
    fn appending_stays_linear_because_capacity_doubles() {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            let piece = MwlStr::new(b"0123456789").into_raw();
            let mut acc = MwlStr::new(b"").into_raw();
            for _ in 0..1_000 {
                acc = mwl_str_append(acc, piece);
            }
            assert_eq!(MwlStr::bytes_of(acc).len(), 10_000);
            // Doubling caps the room at under twice the payload, which is what
            // this module's docs state capacity spends.
            let owned = MwlStr::from_raw(acc);
            assert!(owned.capacity() < 2 * owned.len());
            mwl_str_release(piece);
        }
    }
}
