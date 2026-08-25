//! MWL's refcounted string: one heap allocation, a two-word header, and the
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
//! offset 0                    offset size_of::<StrHeader>()
//! +-------------+-----------+ +----------------------------+
//! | refcount    | len       | | len bytes of payload       |
//! +-------------+-----------+ +----------------------------+
//! ```
//!
//! One allocation, not two. A `Box<StrData>` holding a `Box<[u8]>` would be
//! simpler to write, but it costs a second allocation and a second cache miss
//! on every string produced, which is a latency question (priority 3 in
//! [AGENTS.md](../../../AGENTS.md)) rather than a footprint one. Codegen will
//! eventually inline the refcount increment/decrement using
//! [`REFCOUNT_OFFSET`]/[`LEN_OFFSET`]/[`PAYLOAD_OFFSET`] rather than calling
//! [`mwl_str_retain`]/[`mwl_str_release`]; those constants exist so the layout
//! is queried, never restated.
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
    /// Payload length in bytes. Immutable for the allocation's lifetime.
    len: usize,
}

/// Byte offset of the reference count within [`StrHeader`].
pub const REFCOUNT_OFFSET: usize = std::mem::offset_of!(StrHeader, refcount);

/// Byte offset of the payload length within [`StrHeader`].
pub const LEN_OFFSET: usize = std::mem::offset_of!(StrHeader, len);

/// Byte offset of the payload itself, relative to the [`StrHeader`] pointer.
pub const PAYLOAD_OFFSET: usize = std::mem::size_of::<StrHeader>();

/// The allocation shape for a string of `len` payload bytes.
fn str_layout(len: usize) -> Layout {
    let size = PAYLOAD_OFFSET
        .checked_add(len)
        .expect("string length overflows the address space");
    Layout::from_size_align(size, std::mem::align_of::<StrHeader>())
        .expect("string layout is always valid: alignment is a power of two")
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
        let layout = str_layout(len);
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
            reason = "`ptr` is a fresh, uninitialized, correctly aligned \
                      allocation of exactly `layout`, so writing the header \
                      and then `len` payload bytes behind it stays inside it; \
                      the regions cannot overlap because each piece borrows a \
                      different allocation, and the running offset never \
                      exceeds `len` because that is their summed length"
        )]
        unsafe {
            ptr.as_ptr().write(StrHeader {
                refcount: Cell::new(1),
                len,
            });
            let mut at = raw.add(PAYLOAD_OFFSET);
            for piece in pieces {
                std::ptr::copy_nonoverlapping(piece.as_ptr(), at, piece.len());
                at = at.add(piece.len());
            }
        }
        Self { ptr }
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
        self.header().len
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
            let len = (*ptr).len;
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
        let layout = str_layout(self.len());
        #[expect(
            unsafe_code,
            reason = "this handle held the last reference, so nothing else can \
                      observe the allocation; `layout` is recomputed from the \
                      same `len` `new` allocated with, before the header is freed"
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
        assert_eq!(REFCOUNT_OFFSET, 0);
        assert_eq!(LEN_OFFSET, std::mem::size_of::<usize>());
        assert_eq!(PAYLOAD_OFFSET, 2 * std::mem::size_of::<usize>());
    }
}
