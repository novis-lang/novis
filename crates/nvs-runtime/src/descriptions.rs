//! The storage for `Core\Reflect`'s descriptions: built once when a unit is
//! finished, owned by its [`ClassTable`](crate::ClassTable), and read by every
//! core.
//!
//! A description is a pure function of the compiled unit, as a string literal
//! is. So `nvs_stdlib::reflect::build_descriptions` writes every class's and
//! every enum's description once, before the table is shared, and
//! `forClass`, `forObject` and `EnumInfo::of` return the object built here.
//! This module owns the bytes and the layout of what is stored. What a row
//! *says* is `nvs_stdlib::reflect`'s, and its module doc is the home of the
//! design.
//!
//! # Decision: every object here is immortal, and holds no array
//!
//! [`ImmortalArena`] writes each description and each row as an immortal
//! instance ([`crate::immortal_object_bytes`]) and each string as an immortal
//! string ([`crate::immortal_header_bytes`]). Every core reads them at once, so
//! `crate::object`'s § *An immortal instance is on no list at all* is the
//! soundness argument: no word two cores can reach is ever written. An array
//! read writes its borrow flag, so no slot holds an array. A roster is a
//! `Vec<Value>` here instead, and a roster member builds the array it returns
//! from it.
//!
//! # Decision: a description names its rows by index and by stamp
//!
//! No slot holds an address. A description holds its class's index in the
//! table and the table's [`Descriptions::stamp`], and a member finds the rows
//! through the calling request's own table. A copy of a description, sent to a
//! task or kept past a reload, keeps both numbers. When the stamp is not the
//! calling unit's, the member throws, because the index would name another
//! class. Stamp `0` is never handed out: it marks a description built per call,
//! for a `Core` class that no unit table describes.
//!
//! **What it spends:** one allocation per description, row and string, held
//! once per process for each loaded unit and freed with its table.
//! [`ImmortalArena::bytes`] counts them.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::fmt;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::object::{ClassDesc, OBJ_ALIGN, OBJ_CLASS_OFFSET, field_offset, immortal_object_bytes};
use crate::string::{HEADER_ALIGN, PAYLOAD_OFFSET, immortal_header_bytes};
use crate::value::{Tag, Value};

/// The last stamp handed out. Starts at `0`, which no table is given.
static LAST_STAMP: AtomicU64 = AtomicU64::new(0);

/// A stamp no other set of descriptions in this process has, and never `0`.
///
/// `Relaxed` is enough: the only property needed is that two calls return two
/// different numbers, and every read-modify-write on one atomic is ordered.
#[must_use]
pub fn next_stamp() -> u64 {
    LAST_STAMP.fetch_add(1, Ordering::Relaxed) + 1
}

/// The memory every immortal description, row and string of one unit lives in.
///
/// One allocation per value, each freed when the arena is dropped. The arena is
/// owned by [`Descriptions`], which the unit's table owns, so the bytes live
/// exactly as long as every descriptor a request of that unit can reach.
#[derive(Default)]
pub struct ImmortalArena {
    blocks: Vec<(NonNull<u8>, Layout)>,
    bytes: usize,
}

impl fmt::Debug for ImmortalArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImmortalArena")
            .field("blocks", &self.blocks.len())
            .field("bytes", &self.bytes)
            .finish()
    }
}

impl ImmortalArena {
    /// An empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many bytes the arena holds, summed over every allocation.
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.bytes
    }

    /// A fresh zeroed block of `size` bytes at `align`.
    fn block(&mut self, size: usize, align: usize) -> NonNull<u8> {
        let layout = Layout::from_size_align(size.max(1), align)
            .expect("a description's size is far below `isize::MAX`");
        #[expect(
            unsafe_code,
            reason = "the layout has a non-zero size, which is `alloc`'s one obligation"
        )]
        let ptr = unsafe { alloc(layout) };
        let Some(ptr) = NonNull::new(ptr) else {
            handle_alloc_error(layout);
        };
        self.blocks.push((ptr, layout));
        self.bytes += layout.size();
        ptr
    }

    /// An immortal `string` with `text` as its payload.
    pub fn string(&mut self, text: &str) -> Value {
        let header = immortal_header_bytes(text.as_bytes());
        let ptr = self.block(PAYLOAD_OFFSET + text.len(), HEADER_ALIGN);
        #[expect(
            unsafe_code,
            reason = "the block is `PAYLOAD_OFFSET + len` bytes, so the header and \
                      the payload both fit, and the two sources do not overlap it"
        )]
        unsafe {
            std::ptr::copy_nonoverlapping(header.as_ptr(), ptr.as_ptr(), PAYLOAD_OFFSET);
            std::ptr::copy_nonoverlapping(
                text.as_ptr(),
                ptr.as_ptr().add(PAYLOAD_OFFSET),
                text.len(),
            );
        }
        #[expect(
            unsafe_code,
            reason = "the address is a whole string header followed by UTF-8 text"
        )]
        unsafe {
            Value::from_parts(Tag::Str, ptr.as_ptr() as usize as u64)
        }
    }

    /// An immortal instance of `class` with `slots` in its slots, in order.
    ///
    /// # Safety
    ///
    /// `class` must outlive the arena, and must have exactly `slots.len()`
    /// slots. Every slot must be a scalar or an immortal string or object,
    /// because nothing ever releases what this object holds.
    #[expect(
        unsafe_code,
        reason = "the caller vouches for the class pointer and the slot count, \
                  which this function cannot check"
    )]
    pub unsafe fn object(&mut self, class: *const ClassDesc, slots: &[Value]) -> Value {
        debug_assert!(
            slots
                .iter()
                .all(|slot| slot.tag().is_none_or(|tag| !tag.is_refcounted())
                    || slot.str_ptr().is_some()
                    || slot.obj_ptr().is_some()),
            "a description's slot is a scalar, a string or an object"
        );
        let tags: Vec<Tag> = slots
            .iter()
            .map(|slot| slot.exact_tag().unwrap_or(Tag::Null))
            .collect();
        let bytes = immortal_object_bytes(&tags);
        let ptr = self.block(bytes.len(), OBJ_ALIGN);
        #[expect(
            unsafe_code,
            reason = "the block is exactly the instance's size, the class word and \
                      every slot are inside it, and nothing else holds the block yet"
        )]
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.as_ptr(), bytes.len());
            ptr.as_ptr()
                .add(OBJ_CLASS_OFFSET)
                .cast::<*const ClassDesc>()
                .write_unaligned(class);
            for (index, slot) in slots.iter().enumerate() {
                ptr.as_ptr()
                    .add(field_offset(index))
                    .cast::<Value>()
                    .write_unaligned(*slot);
            }
            Value::from_parts(Tag::Object, ptr.as_ptr() as usize as u64)
        }
    }
}

impl Drop for ImmortalArena {
    fn drop(&mut self) {
        for (ptr, layout) in self.blocks.drain(..) {
            #[expect(
                unsafe_code,
                reason = "each block was allocated by `block` with this layout and is \
                          freed once, here; nothing reads it after its table is gone"
            )]
            unsafe {
                dealloc(ptr.as_ptr(), layout);
            }
        }
    }
}

/// One method's row and its parameters' rows.
#[derive(Debug)]
pub struct MethodRows {
    /// The method's row.
    pub info: Value,
    /// One row per parameter, in declaration order.
    pub parameters: Vec<Value>,
}

/// One attach site's row and its payload, field by field.
#[derive(Debug)]
pub struct AttributeRows {
    /// The attach site's row.
    pub info: Value,
    /// Each payload field as its name, a string, and its value.
    pub fields: Vec<(Value, Value)>,
}

/// One class's description and every roster's rows.
///
/// When it is built per call for a `Core` class, every value is an owned
/// reference instead, and its builder releases each one.
#[derive(Debug)]
pub struct ClassRows {
    /// The description itself.
    pub info: Value,
    /// One row per property.
    pub properties: Vec<Value>,
    /// One entry per method.
    pub methods: Vec<MethodRows>,
    /// One row per constant.
    pub constants: Vec<Value>,
    /// One entry per attach site.
    pub attributes: Vec<AttributeRows>,
}

/// One enum's description and its cases.
#[derive(Debug)]
pub struct EnumRows {
    /// The description itself.
    pub info: Value,
    /// Each case as its name, a string, and its value.
    pub cases: Vec<(Value, Value)>,
}

/// Every description of one unit, and the arena they live in.
#[derive(Debug)]
pub struct Descriptions {
    stamp: u64,
    classes: Vec<ClassRows>,
    enums: Vec<EnumRows>,
    // Declared last, so it is dropped after every row that points into it.
    arena: ImmortalArena,
}

impl Descriptions {
    /// The descriptions of one unit: `classes` by class index and `enums` by
    /// enum index, every value in `arena`, all built under `stamp`.
    #[must_use]
    pub const fn new(
        stamp: u64,
        arena: ImmortalArena,
        classes: Vec<ClassRows>,
        enums: Vec<EnumRows>,
    ) -> Self {
        Self {
            stamp,
            classes,
            enums,
            arena,
        }
    }

    /// The stamp every description here carries.
    #[must_use]
    pub const fn stamp(&self) -> u64 {
        self.stamp
    }

    /// The class at `index`, or `None` past the last one.
    #[must_use]
    pub fn class(&self, index: usize) -> Option<&ClassRows> {
        self.classes.get(index)
    }

    /// The enum at `index`, or `None` past the last one.
    #[must_use]
    pub fn enum_rows(&self, index: usize) -> Option<&EnumRows> {
        self.enums.get(index)
    }

    /// How many bytes every description, row and string here takes.
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.arena.bytes()
    }
}
