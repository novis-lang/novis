//! The pending-exception value, and the primitives compiled code calls to
//! raise, inspect and re-take one.
//!
//! # An exception is an ordinary object
//!
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) § 10
//! makes `Throwable` the root of a small class tree whose members are
//! *readonly properties* — `message`, `previous`, `backtrace`, `location` —
//! not `getX()` accessors, and makes user classes extend it directly. So an
//! exception is a [`crate::NvsObj`] like any other: allocated by
//! `nvs_object_new`, refcounted by `nvs_object_retain`/`nvs_object_release`,
//! read by an ordinary `FieldGet`. There is no second representation here, and
//! no `Ty::Throwable` in the IR.
//!
//! What survives is the part the *runtime* owns: which object is pending, and
//! growing its backtrace as a throw travels. Both need to reach two slots of
//! an object they otherwise know nothing about, which is what [`MESSAGE_SLOT`]
//! and friends are for.
//!
//! # The slot indices are load-bearing
//!
//! `crate::object` lays a subclass's slots out *after* its parent's, and
//! `Throwable` is the root of every exception class in existence — so
//! `message` is slot 0 and `backtrace` slot 2 for `LogicError`, for a user's
//! `ConfigError extends Throwable`, and for anything else that can be thrown.
//! `nvs_hir::errors::PROPERTIES` is the one home for that order; the constants
//! below restate the indices this crate needs because `nvs-runtime`
//! depends on nothing (see [`crate`]'s own docs), and
//! `nvs-codegen`'s `the_runtime_and_the_compiler_agree_on_every_throwable_slot`
//! is the test that holds the two together.
//!
//! # The backtrace is built as the throw propagates
//!
//! A frame label is pushed by [`nvs_trace_push`] from the *error* path of each
//! compiled frame the throw travels out of — never from a push/pop record kept
//! on the way in. That is the whole reason
//! `rule:errors/propagation` can claim a call
//! costs a compare-and-branch: a frame-record scheme would move the cost onto
//! the success path, which is the path that runs. The consequence is visible
//! and deliberate: the trace holds exactly the frames the exception *unwound
//! out of*, so a `catch` in the frame that called the thrower sees the
//! thrower's frame and nothing below it. PHP instead snapshots the whole stack
//! at construction; matching that needs a walk of Novis's own frame chain.

use crate::ctx::Ctx;
use crate::object::{ClassDesc, NvsObj, ObjHeader};
use crate::{NvsArray, NvsStr, Value};

/// The slot `Throwable::$message` occupies — see this module's docs.
pub const MESSAGE_SLOT: usize = 0;
/// The slot `Throwable::$previous` occupies.
pub const PREVIOUS_SLOT: usize = 1;
/// The slot `Throwable::$backtrace` occupies.
pub const BACKTRACE_SLOT: usize = 2;
/// The slot `Throwable::$location` occupies.
pub const LOCATION_SLOT: usize = 3;

/// How many slots any exception class has at minimum — every one of the four
/// above. A descriptor with fewer is not an exception class, and every
/// operation here refuses it rather than reading past the allocation.
pub const SLOT_COUNT: usize = 4;

/// The slot `ParseError::$issues` occupies — the one property that class
/// declares beyond the root's own
/// (`rule:core-classes/derive-reports-every-field`).
///
/// `ParseError` inherits exactly [`SLOT_COUNT`] slots and adds this one, so a
/// descriptor with more than [`SLOT_COUNT`] fields is the only shape it can
/// take. `nvs_hir::errors::ISSUES_SLOT` is the compiler's copy, and
/// `nvs-codegen`'s `the_runtime_and_the_compiler_agree_on_every_throwable_slot`
/// is what holds the two together.
pub const ISSUES_SLOT: usize = SLOT_COUNT;

/// The slot `Core\Db\DbError::$kind` occupies —
/// `rule:core-classes/db-error`'s normalised condition,
/// which `nvs_stdlib::db`'s `statement_failure` fills through
/// [`Ctx::raise_with_slots`].
///
/// Equal to [`ISSUES_SLOT`] and derived the same way rather than from it: these
/// are unrelated siblings that each declare one property beyond the root's own,
/// so each first own slot lands immediately after [`SLOT_COUNT`], and writing
/// either in terms of the other would make an accident look like a rule. `nvs_hir::errors::KIND_SLOT` is the compiler's copy, held to this one
/// by `nvs-codegen`'s `the_runtime_and_the_compiler_agree_on_every_throwable_slot`.
pub const KIND_SLOT: usize = SLOT_COUNT;

/// The slot `Core\Db\DbError::$sqlState` occupies — the five-character code the
/// server sent, beside the [`KIND_SLOT`] normalised from it
/// (`rule:core-classes/db-error`).
///
/// Derived from [`KIND_SLOT`] where that constant is deliberately *not* derived
/// from [`ISSUES_SLOT`]: these two are properties of one class in declaration
/// order, which is a rule, where those are unrelated siblings that merely
/// coincide. `nvs_hir::errors::SQL_STATE_SLOT` is the compiler's copy, held to
/// this one by `nvs-codegen`'s
/// `the_runtime_and_the_compiler_agree_on_every_throwable_slot`.
pub const SQL_STATE_SLOT: usize = KIND_SLOT + 1;

/// The slot `Core\Db\DbError::$driverCode` occupies — the vendor integer, where
/// the driver has one.
///
/// It is `null` on PostgreSQL and always will be: the `SQLSTATE` *is* that
/// server's code, and `nvs_db::ServerError` owns why a second integer invented
/// to fill the shape would be a value with no meaning. The slot exists ahead of
/// the driver that fills it because § 8 fixes the property order, and one added
/// later would move every slot after it.
/// `nvs_hir::errors::DRIVER_CODE_SLOT` is the compiler's copy.
pub const DRIVER_CODE_SLOT: usize = KIND_SLOT + 2;

/// The slot `Core\Db\DbError::$constraint` occupies — the constraint the
/// server's condition names, where it names one
/// (`rule:core-classes/db-error`).
///
/// `nvs_db::ServerError::constraint` is where PostgreSQL's comes from, and it
/// is `Option` there for the same reason this is `?string` here: a unique
/// violation names the index it broke, where a syntax error or a permission
/// has no constraint to name. So `null` is the ordinary answer rather than a
/// driver that has not caught up.
/// `nvs_hir::errors::CONSTRAINT_SLOT` is the compiler's copy, held to this one
/// by `nvs-codegen`'s
/// `the_runtime_and_the_compiler_agree_on_every_throwable_slot`.
pub const CONSTRAINT_SLOT: usize = KIND_SLOT + 3;

/// The slot `Core\Db\DbError::$sql` occupies — the statement that was refused,
/// as the program wrote it (`rule:core-classes/db-error`).
///
/// § 8 lets the text ride the throw where it lets no bound value ride it: the
/// SQL is developer-authored and the values are the request's, which is
/// `rule:security/secret-qualifier`'s
/// line and not a judgement made here. It is `?string` because a refusal is not
/// always *of* a statement a caller spelled — § 7's `BEGIN`, `COMMIT` and
/// `SAVEPOINT` are this runtime's own text — and an unwritten slot already
/// reads `null`. `nvs_hir::errors::SQL_SLOT` is the compiler's copy, held to
/// this one by `nvs-codegen`'s
/// `the_runtime_and_the_compiler_agree_on_every_throwable_slot`.
pub const SQL_SLOT: usize = KIND_SLOT + 4;

/// The slot `Core\Db\RolledBack::$reason` occupies —
/// `rule:core-classes/db-transactions`'s abandoned transaction,
/// worded by the program that abandoned it.
///
/// Equal to [`ISSUES_SLOT`] and [`KIND_SLOT`], and derived the same way rather
/// than from either: unrelated classes each declaring one property beyond the
/// root's own is a coincidence of arithmetic, not a rule any of them shares. `nvs_hir::errors::REASON_SLOT` is the compiler's copy, held to this
/// one by `nvs-codegen`'s
/// `the_runtime_and_the_compiler_agree_on_every_throwable_slot`.
///
/// Unlike its siblings this slot's value **is the message**: § 7 gives
/// `rollBack` one string and it is both what the exception says and what the
/// property holds, so [`Thrown::new_as`] seeds it rather than making every
/// thrower pass the same text twice.
pub const REASON_SLOT: usize = SLOT_COUNT;

/// Which of [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
/// § 10's classes a runtime helper's failure lands in.
///
/// A closed enum rather than a `&'static str` a helper writes, for
/// `nvs_stdlib::registry::CoreTy`'s reason: a misspelled class name would be
/// a silent *runtime* miss — the `catch` clause that was meant to handle it
/// simply would not match — rather than a compile error. The roster is
/// `nvs_hir::errors::TREE` minus its root, since a helper that means "anything
/// at all" means [`Self::Runtime`] — spec § 10's tree, plus the classes the
/// rules add to it ([`Self::TestFailure`] among them).
///
/// `nvs-runtime` depends on nothing (see [`crate`]'s own docs), so the names
/// below restate `nvs_hir::errors::TREE`'s; `nvs-codegen`'s
/// `every_thrown_class_is_in_the_compiler_s_exception_tree` is the test that
/// holds the two together, exactly as
/// `the_runtime_and_the_compiler_agree_on_every_throwable_slot` holds the slot
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ThrownClass {
    /// `RuntimeError` — "the world said no", which is what a helper failure
    /// with nothing more specific to say is, and the class
    /// [`Ctx::set_runtime_error_class`] installs.
    #[default]
    Runtime,
    /// `LogicError` — a bug in the program: a bad argument, a bad state, a
    /// bad index.
    Logic,
    /// `IOError` — a file, socket or process failed.
    Io,
    /// `ParseError` — input did not match a format this code declared, which
    /// is what `Core\Json::decode` and `Core\Time::parse` answer with.
    Parse,
    /// `TimeoutError` — a deadline passed.
    Timeout,
    /// `RecursionError` — the call stack passed
    /// `rule:errors/on-limit`'s
    /// *soft* depth. The hard limit beneath it is a `FATAL` and is not in
    /// this roster at all, because no `catch` ever sees one.
    Recursion,
    /// `ArithmeticError` — overflow (`rule:types/declaration`), division by zero.
    Arithmetic,
    /// `Core\Test\Failure` — a failed assertion
    /// (`rule:testing/failure-ledger`), which that section makes an ordinary `Throwable` precisely so a
    /// composite assertion, a retry wrapper or a test *of* an assertion can
    /// intercept one by name.
    ///
    /// `Core\Test\Failure`, [`Self::CliNotInteractive`], [`Self::DbError`] and
    /// [`Self::DbRolledBack`] are the entries in this roster whose names
    /// are namespaced; `nvs_hir::errors::TREE` says why they are classes in the
    /// tree rather than `nvs_stdlib::registry` rows, and nothing here has to
    /// care, the lookup below being by name either way.
    TestFailure,
    /// `Core\Cli\NotInteractive` — a prompt with no controlling terminal to
    /// read and no default to fall back on
    /// (`rule:tooling/a-prompt-is-a-core-member`).
    ///
    /// A throw rather than a block is the whole of that section's second rule,
    /// and it is `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s
    /// "no spelling for an unbounded wait" on a second surface.
    CliNotInteractive,
    /// `Core\Db\DbError` — the database refused a statement, a connection or a
    /// commit (`rule:core-classes/db-error`): every
    /// failure `Core\Db` reports that the program did not itself choose.
    ///
    /// One class rather than ten, because § 8 normalises the condition into a
    /// `kind` instead of naming a type per condition — some of those boundaries
    /// are a driver's rather than the language's. That `kind` is [`KIND_SLOT`]
    /// on the object, beside the raw `sqlState` it was normalised from and the
    /// rest of the row `nvs_hir::errors::OWN_PROPERTIES` declares; the message
    /// next to them is one § 8 requires to carry no bound value.
    DbError,
    /// `Core\Db\RolledBack` — a transaction the program itself rolled back
    /// (`rule:core-classes/db-transactions`), propagated out of
    /// the closure that owned it.
    ///
    /// It is not a driver failure and deliberately not the same class as one:
    /// a `catch` distinguishing "I gave up" from "the database said no" is the
    /// whole reason spec § 18 puts two names in the tree rather than one.
    /// Its `reason` slot is filled by the message the throw carries, which
    /// `nvs_ir::lower::exception`'s synthesized constructor does — there is no
    /// second field for a helper here to write.
    DbRolledBack,
}

impl ThrownClass {
    /// The class's name, as `nvs_hir::errors::TREE` spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Runtime => "RuntimeError",
            Self::Logic => "LogicError",
            Self::Io => "IOError",
            Self::Parse => "ParseError",
            Self::Timeout => "TimeoutError",
            Self::Recursion => "RecursionError",
            Self::Arithmetic => "ArithmeticError",
            Self::TestFailure => "Core\\Test\\Failure",
            Self::CliNotInteractive => "Core\\Cli\\NotInteractive",
            Self::DbError => "Core\\Db\\DbError",
            Self::DbRolledBack => "Core\\Db\\RolledBack",
        }
    }

    /// Every class in the roster — what a guard test iterates.
    pub const ALL: &'static [Self] = &[
        Self::Runtime,
        Self::Logic,
        Self::Io,
        Self::Parse,
        Self::Timeout,
        Self::Recursion,
        Self::Arithmetic,
        Self::TestFailure,
        Self::CliNotInteractive,
        Self::DbError,
        Self::DbRolledBack,
    ];
}

/// One owned reference to a pending exception object.
///
/// Exists so [`crate::Ctx`] can hold a raw `*mut ObjHeader` without either
/// leaking it or hand-writing a release at every early return — the same job
/// [`NvsObj`] does, except that this one may be *null*, which is the state a
/// failure with no object behind it (a helper fault whose class was never
/// installed) leaves.
#[derive(Debug)]
pub struct Thrown {
    /// Null, or one owned reference.
    ptr: *mut ObjHeader,
}

impl Thrown {
    /// Builds a fresh exception of `class` carrying `message`, with an empty
    /// backtrace — what a helper's bare-message failure is promoted to, and
    /// what `nvs run --fault-inject` produces.
    ///
    /// Returns a null [`Thrown`] if `class` is null or describes fewer than
    /// [`SLOT_COUNT`] slots, which is the "no exception class was installed"
    /// case [`Ctx::set_runtime_error_class`] documents.
    ///
    /// # Safety
    ///
    /// `class` must be null or refer to a live class descriptor that outlives
    /// every instance made from it.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness is the caller's obligation and \
                  cannot be expressed in the signature"
    )]
    pub unsafe fn new(class: *const ClassDesc, message: &str) -> Self {
        #[expect(unsafe_code, reason = "forwarding this function's own contract")]
        unsafe {
            Self::new_as(class, ThrownClass::Runtime, message, &[])
        }
    }

    /// [`Self::new`], plus the properties a class below the root declares:
    /// `extra` names each slot and its value, and each value is written there
    /// when the descriptor is wide enough to have it.
    ///
    /// A thrower with nothing to put in a slot passes `&[]`, and the class
    /// then seeds its own. `ParseError::$issues` becomes an empty array, since
    /// the property is declared `array<Issue>` rather than `?array<Issue>` and
    /// reading `null` out of it would be a type the checker ruled out;
    /// `Core\Db\RolledBack::$reason` becomes the message, which is the same
    /// text its synthesized constructor stores (`nvs_ir`'s `ExtraInit::Message`)
    /// and so the only value at which the two ways of building that class
    /// agree. **A seeded default is not a missing value**: every property in
    /// the tree that is not `?T` is written on every path out of here.
    /// `thrown` rather than the descriptor's name decides that: a name compare
    /// on every promotion would put a string equality on the throw path, and a
    /// user's `class ConfigError extends Throwable { public int $code; }` also
    /// has a fifth slot — one that must **not** be seeded here. An `extra` that
    /// names a slot is the caller's own statement about a class it resolved,
    /// so it is written without consulting `thrown` at all.
    ///
    /// Takes over every value's reference; releases the ones with no such slot
    /// to go in (a null or too-narrow descriptor, which is
    /// [`Ctx::set_runtime_error_class`]'s "nothing installed" case).
    ///
    /// # Safety
    ///
    /// `class` must be null or refer to a live class descriptor that outlives
    /// every instance made from it, and each of `extra`'s values must be one
    /// whose reference is being transferred here.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness and the value's reference are both \
                  the caller's obligation and cannot be expressed in the signature"
    )]
    pub unsafe fn new_as(
        class: *const ClassDesc,
        thrown: ThrownClass,
        message: &str,
        extra: &[(usize, Value)],
    ) -> Self {
        let count = if class.is_null() {
            0
        } else {
            #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
            unsafe {
                (*class).field_count()
            }
        };
        if count < SLOT_COUNT {
            for &(_, value) in extra {
                #[expect(
                    unsafe_code,
                    reason = "the caller transferred this reference and there is \
                              no slot to hand it on to"
                )]
                unsafe {
                    value.release();
                }
            }
            return Self::none();
        }
        #[expect(
            unsafe_code,
            reason = "the caller guarantees the descriptor outlives the instance"
        )]
        let obj = unsafe { NvsObj::new(class) };
        obj.set_field(MESSAGE_SLOT, Value::str(NvsStr::new(message.as_bytes())));
        obj.set_field(BACKTRACE_SLOT, Value::array(NvsArray::new()));
        obj.set_field(LOCATION_SLOT, Value::str(NvsStr::new(b"")));
        if extra.is_empty() {
            match thrown {
                ThrownClass::Parse if count > ISSUES_SLOT => {
                    obj.set_field(ISSUES_SLOT, Value::array(NvsArray::new()));
                }
                // The property is the message, so a thrower that has one has
                // both. Without this a `Core\Db\RolledBack` raised here reads
                // `$reason` as `null` while a hand-built one carries the text,
                // which is `nvs_ir`'s `ExtraInit::Message` seeding the
                // synthesized constructor — two spellings of one class
                // answering differently.
                ThrownClass::DbRolledBack if count > REASON_SLOT => {
                    obj.set_field(REASON_SLOT, Value::str(NvsStr::new(message.as_bytes())));
                }
                _ => {}
            }
        } else {
            for &(slot, value) in extra {
                if count > slot {
                    obj.set_field(slot, value);
                } else {
                    #[expect(
                        unsafe_code,
                        reason = "the caller transferred this reference and this \
                                  class declares no slot to hand it on to"
                    )]
                    unsafe {
                        value.release();
                    }
                }
            }
        }
        Self {
            ptr: obj.into_raw(),
        }
    }

    /// The absent exception — what a failure with no object behind it holds.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }

    /// Whether there is no object here at all.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.ptr.is_null()
    }

    /// Takes over one reference to `ptr`, which may be null.
    ///
    /// # Safety
    ///
    /// `ptr` must be null or refer to a live object allocation whose
    /// reference is being transferred here.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "reclaiming a reference is the caller's obligation to state"
    )]
    pub const unsafe fn from_raw(ptr: *mut ObjHeader) -> Self {
        Self { ptr }
    }

    /// Gives the reference back, leaving nothing behind to release.
    #[must_use]
    pub fn into_raw(self) -> *mut ObjHeader {
        let ptr = self.ptr;
        std::mem::forget(self);
        ptr
    }

    /// The exception **as a value**, borrowed — `null` where there is no object
    /// at all.
    ///
    /// No reference is handed over, exactly as [`crate::Ctx::limit_handler`]
    /// hands none over: what keeps the object alive across the borrower's use
    /// of it is the reference this `Thrown` is still holding. Its one caller is
    /// [`crate::Ctx::run_uncaught_handler`], which is
    /// `rule:errors/on-uncaught-throw`'s
    /// "the **real `Throwable` object**, not copied data" — so this is
    /// deliberately not a constructor that copies anything.
    #[must_use]
    pub fn as_value(&self) -> Value {
        if self.ptr.is_null() {
            return Value::null();
        }
        Value::from_obj_ptr(self.ptr)
    }

    /// A borrowed handle on the object, or `None` if there is none.
    ///
    /// [`std::mem::ManuallyDrop`] because [`NvsObj::from_raw`] takes over a
    /// reference this value still owns.
    fn borrow(&self) -> Option<std::mem::ManuallyDrop<NvsObj>> {
        if self.ptr.is_null() {
            return None;
        }
        #[expect(
            unsafe_code,
            reason = "the pointer is non-null and this value owns a reference \
                      to it; the handle is never dropped, so the reference is \
                      not released twice"
        )]
        Some(std::mem::ManuallyDrop::new(unsafe {
            NvsObj::from_raw(self.ptr)
        }))
    }

    /// The descriptor of the class this exception was built from, or null for
    /// an absent one — what a by-name conformance test reads its ancestry off.
    #[must_use]
    pub fn class_desc(&self) -> *const ClassDesc {
        self.borrow().map_or(std::ptr::null(), |obj| obj.class())
    }

    /// The rendered name of that class, or `rule:errors/escalation-ladder`'s
    /// generic `Error` where there is no descriptor to read.
    ///
    /// Here rather than beside either caller: `nvs_host::isolate` names the
    /// class of a failed isolate's throw and [`crate::floor`] names it in the
    /// tier-4 record, and the `unsafe` deref both need is this crate's to
    /// state — its unsafe policy is the reason it owns the raw handle at all.
    #[must_use]
    pub fn class_name(&self) -> String {
        let desc = self.class_desc();
        if desc.is_null() {
            return "Error".to_owned();
        }
        #[expect(
            unsafe_code,
            reason = "a `Thrown`'s descriptor is published by a class table that \
                      outlives every value built from it; see `crate::object`"
        )]
        // SAFETY: non-null here means the class table handed it out, and a
        // descriptor's address is its identity for the life of that table.
        unsafe {
            (*desc).name().to_owned()
        }
    }

    /// The exception's `message` property, as an owned string.
    ///
    /// Empty for an absent exception, or for one whose slot somehow does not
    /// hold a string — a diagnostic path never gets to be the thing that
    /// crashes.
    #[must_use]
    pub fn message(&self) -> String {
        let Some(obj) = self.borrow() else {
            return String::new();
        };
        if obj.field_count() < SLOT_COUNT {
            return String::new();
        }
        let slot = obj.field(MESSAGE_SLOT);
        slot.as_str_bytes()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default()
    }

    /// Field slot `slot` of this exception, or `None` where there is nothing
    /// to read it from — the borrow-shaped read of a property beyond the four
    /// [`SLOT_COUNT`] every `Throwable` has.
    ///
    /// The bound is the **same `count > slot` guard [`Self::new_as`] writes
    /// through**, and for the same reason: a class narrower than the property
    /// being asked about never had it, so the answer is "nothing here" rather
    /// than the panic [`NvsObj::field`] raises on an out-of-range slot. A slot
    /// index is only meaningful against a class that declares it — `KIND_SLOT`
    /// and `ISSUES_SLOT` are the same number — so a caller reaches this
    /// through [`crate::Ctx::pending_slot`], which asks the class question
    /// first.
    ///
    /// **No reference is taken.** The value is good only while this `Thrown`
    /// is alive, exactly as [`NvsObj::field`] states it; a caller keeping one
    /// past that owes a [`Value::retain`] of its own. Handing the reference
    /// out instead would make the one caller that reads an `int` back release
    /// a scalar to stay balanced.
    #[must_use]
    pub fn field(&self, slot: usize) -> Option<Value> {
        let obj = self.borrow()?;
        (obj.field_count() > slot).then(|| obj.field(slot))
    }

    /// The `backtrace` property rendered `#0`-first, the form
    /// `nvs run`'s uncaught report prints.
    ///
    /// The `#N ` prefix is applied here rather than stored, so a frame label
    /// never has to know its own depth at the point it is pushed.
    #[must_use]
    pub fn trace_as_string(&self) -> String {
        let mut out = String::new();
        for (index, frame) in self.frames().iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push('#');
            out.push_str(&index.to_string());
            out.push(' ');
            out.push_str(frame);
        }
        out
    }

    /// Every backtrace frame label, in push order.
    #[must_use]
    pub fn frames(&self) -> Vec<String> {
        let Some(obj) = self.borrow() else {
            return Vec::new();
        };
        if obj.field_count() < SLOT_COUNT {
            return Vec::new();
        }
        let Some(array) = obj.field(BACKTRACE_SLOT).array_ptr() else {
            return Vec::new();
        };
        #[expect(
            unsafe_code,
            reason = "the slot holds one reference the object owns; the handle \
                      is never dropped, so that reference is not released here"
        )]
        let handle = std::mem::ManuallyDrop::new(unsafe { NvsArray::from_raw(array) });
        let mut out = Vec::new();
        let mut slot = 0;
        while let Some(found) = handle.next_slot(slot) {
            if let Some(value) = handle.value_at(found)
                && let Some(bytes) = value.as_str_bytes()
            {
                out.push(String::from_utf8_lossy(bytes).into_owned());
            }
            slot = found + 1;
        }
        out
    }

    /// Appends one frame label to the `backtrace` property, in place.
    ///
    /// The field's own reference is *moved out* of the slot and back in, so
    /// the array's refcount stays at one and copy-on-write never separates —
    /// retaining it first would copy the whole trace once per frame the throw
    /// unwinds through.
    pub fn push_frame(&self, label: &str) {
        let Some(obj) = self.borrow() else {
            return;
        };
        if obj.field_count() < SLOT_COUNT {
            return;
        }
        let held = obj.take_field(BACKTRACE_SLOT);
        let Some(array) = held.array_ptr() else {
            // Not an array: put back exactly what was there, unchanged.
            obj.set_field(BACKTRACE_SLOT, held);
            return;
        };
        #[expect(
            unsafe_code,
            reason = "`take_field` transferred the slot's own reference here, \
                      and `into_raw` hands it straight back to the slot"
        )]
        let mut handle = unsafe { NvsArray::from_raw(array) };
        handle.append(Value::str(NvsStr::new(label.as_bytes())));
        obj.set_field(BACKTRACE_SLOT, Value::from_array_ptr(handle.into_raw()));
    }
}

impl Drop for Thrown {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        #[expect(
            unsafe_code,
            reason = "this value owned exactly one reference to a live \
                      allocation, and this is the one place it is given up"
        )]
        unsafe {
            drop(NvsObj::from_raw(self.ptr));
        }
    }
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// Same split `crate::string`'s own primitives are on, for the same reason:
// none of these can fail, so none of them wears `rule:errors/propagation`'s checked-return
// shape. Every one is `extern "C"` and never `extern "C-unwind"`.

/// Makes `thrown` this request's pending exception — what Novis's `throw`
/// lowers to, immediately before the frame branches to its own cleanup path.
///
/// Takes ownership of the reference it is handed: `nvs_ir::lower` retains an
/// aliasing `throw $e;` operand first, exactly the way it retains any other
/// value copied into a second durable slot.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call, and
/// `thrown` must be null or refer to a live object allocation whose reference
/// is being transferred here.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context and exception pointers; the \
              contract cannot be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_raise(ctx: *mut Ctx, thrown: *mut ObjHeader) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both pointers are valid, and that the \
                  exception's reference is being transferred"
    )]
    unsafe {
        (*ctx).raise(Thrown::from_raw(thrown));
    }
}

/// Builds an exception of `class` carrying `message` and makes it this
/// request's pending one — [`nvs_raise`] for a throw compiled code raises by
/// itself, with no Novis `new` behind it.
///
/// The one caller is `nvs-codegen`'s integer `%`, whose zero divisor
/// must throw spec § 10's `ArithmeticError` rather than trap the process. That
/// site has no Novis expression to construct the exception from and no helper
/// call to carry a [`crate::Fault`] out of — the operator is inline machine
/// code — so it names the class itself: compiled code already knows the
/// descriptor's address as a constant, and passes it. (A helper *can* name a
/// class of its own, [`crate::Fault::thrown_as`]; only the bare-message form
/// is promoted to `RuntimeError`, through [`Ctx::set_runtime_error_class`]'s
/// anchor.)
///
/// A null or too-small `class` leaves the pending failure with no object
/// behind it, exactly as [`Thrown::new`] documents; the status the caller
/// returns is unaffected.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call;
/// `class` must be null or refer to a live class descriptor that outlives the
/// instance made from it; and `message` must be valid for reads of `len`
/// bytes, or `len` must be zero.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context and descriptor pointers plus a \
              pointer and a length into its own data section"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_raise_new(
    ctx: *mut Ctx,
    class: *const ClassDesc,
    message: *const u8,
    len: usize,
) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `message` is valid for `len` bytes; \
                  the zero-length case is split out because `from_raw_parts` \
                  rejects a null pointer even for an empty slice"
    )]
    let bytes = unsafe {
        if len == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(message, len)
        }
    };
    let text = String::from_utf8_lossy(bytes);
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the descriptor outlives the instance"
    )]
    let thrown = unsafe { Thrown::new(class, &text) };
    #[expect(unsafe_code, reason = "the caller guarantees `ctx` is valid")]
    unsafe {
        (*ctx).raise(thrown);
    }
}

/// Records one more frame a pending `THROWN` has unwound out of — the
/// backtrace's whole mechanism, called only from a compiled frame's error
/// path.
///
/// A non-`THROWN` `status` is ignored: a resource-limit or internal failure is
/// not a `Throwable` at all
/// (`rule:errors/escalation-ladder`), so it has
/// no backtrace to grow.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call, and
/// `label` must be valid for reads of `len` bytes, or `len` must be zero.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer plus a pointer and a \
              length into its own data section"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_trace_push(ctx: *mut Ctx, label: *const u8, len: usize, status: i32) {
    if status != crate::THROWN {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid and `label` is valid \
                  for `len` bytes; the zero-length case is split out because \
                  `from_raw_parts` rejects a null pointer even for an empty \
                  slice"
    )]
    unsafe {
        let bytes = if len == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(label, len)
        };
        (*ctx).push_frame(&String::from_utf8_lossy(bytes));
    }
}

/// Hands the pending exception to a `catch` clause's dispatch, clearing it
/// from the context — `nvs_ir::InstKind::TakeThrown`'s entry point.
///
/// The caller owns the returned reference and must eventually release it.
/// It may be **null**: a helper's bare-message failure has no object behind
/// it unless [`Ctx::set_runtime_error_class`] installed a class to build one
/// from. Every operation a `catch` dispatch performs on the result is
/// null-tolerant — `nvs_object_instanceof` answers `false`, so no clause
/// matches and the throw is re-raised unchanged.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer; the contract cannot be \
              expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_take_thrown(ctx: *mut Ctx) -> *mut ObjHeader {
    #[expect(unsafe_code, reason = "the caller guarantees `ctx` is valid")]
    let thrown = unsafe { (*ctx).take_thrown() };
    thrown.into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ClassTable;

    /// The [`SLOT_COUNT`] slots in slot order, spelled the way
    /// `nvs_types::error_lib` declares them.
    const SLOT_NAMES: [&str; SLOT_COUNT] = ["message", "previous", "backtrace", "location"];

    /// A class table shaped like the seeded exception tree: `Throwable` with
    /// its four slots, and one subclass of it with none of its own.
    fn tree() -> (ClassTable, *const ClassDesc, *const ClassDesc) {
        let mut table = ClassTable::new();
        let root = table.define("Throwable", &SLOT_NAMES, &[]);
        let leaf = table.define("LogicError", &SLOT_NAMES, &[root]);
        let (root, leaf) = (table.desc(root), table.desc(leaf));
        (table, root, leaf)
    }

    #[test]
    fn a_fresh_exception_has_its_message_and_an_empty_trace() {
        let (_table, root, _) = tree();
        #[expect(unsafe_code, reason = "the table outlives the instance")]
        let e = unsafe { Thrown::new(root, "boom") };
        assert_eq!(e.message(), "boom");
        assert_eq!(e.trace_as_string(), "");
    }

    #[test]
    fn frames_are_numbered_in_push_order() {
        let (_table, _, leaf) = tree();
        #[expect(unsafe_code, reason = "the table outlives the instance")]
        let e = unsafe { Thrown::new(leaf, "traced") };
        e.push_frame("Deep::inner() at t.nvs:4");
        e.push_frame("Deep::outer() at t.nvs:8");
        assert_eq!(
            e.trace_as_string(),
            "#0 Deep::inner() at t.nvs:4\n#1 Deep::outer() at t.nvs:8"
        );
        assert_eq!(e.frames().len(), 2);
    }

    #[test]
    fn growing_the_backtrace_never_separates_the_array() {
        // The append path moves the slot's own reference out and back, so the
        // array stays uniquely owned — otherwise every frame would copy the
        // whole trace.
        let (_table, root, _) = tree();
        #[expect(unsafe_code, reason = "the table outlives the instance")]
        let e = unsafe { Thrown::new(root, "m") };
        for i in 0..64 {
            e.push_frame(&format!("f{i}"));
        }
        let frames = e.frames();
        assert_eq!(frames.len(), 64);
        assert_eq!(frames[0], "f0");
        assert_eq!(frames[63], "f63");
    }

    #[test]
    fn an_absent_exception_answers_every_query_without_reading_anything() {
        let none = Thrown::none();
        assert!(none.is_none());
        assert_eq!(none.message(), "");
        assert_eq!(none.trace_as_string(), "");
        assert!(none.frames().is_empty());
        none.push_frame("ignored");
    }

    /// [`Thrown::field`] answers under the same bound [`Thrown::new_as`]
    /// writes under, so the two agree about which classes have a fifth slot:
    /// what was written can be read back, and a class that never had one says
    /// so rather than panicking.
    #[test]
    fn a_fifth_slot_reads_back_only_where_the_class_declares_one() {
        const WIDE: [&str; SLOT_COUNT + 1] =
            ["message", "previous", "backtrace", "location", "kind"];
        let mut table = ClassTable::new();
        let root = table.define("Throwable", &SLOT_NAMES, &[]);
        let wide = table.define("Core\\Db\\DbError", &WIDE, &[root]);
        let narrow = table.define("LogicError", &SLOT_NAMES, &[root]);
        let (wide, narrow) = (table.desc(wide), table.desc(narrow));
        #[expect(unsafe_code, reason = "the table outlives both instances")]
        let (refused, plain) = unsafe {
            (
                Thrown::new_as(
                    wide,
                    ThrownClass::DbError,
                    "refused",
                    &[(KIND_SLOT, Value::int(5))],
                ),
                Thrown::new_as(narrow, ThrownClass::Logic, "bad call", &[]),
            )
        };
        assert_eq!(refused.field(KIND_SLOT).and_then(Value::as_int), Some(5));
        assert!(
            plain.field(KIND_SLOT).is_none(),
            "a class with only the four slots has nothing at the fifth"
        );
        assert!(Thrown::none().field(KIND_SLOT).is_none());
        assert_eq!(refused.field(MESSAGE_SLOT).and_then(Value::as_int), None);
    }

    /// A thrower with nothing to hand over still leaves every non-`?T`
    /// property written: `Core\Db\RolledBack::$reason` is the message, which
    /// is what its synthesized constructor stores, and the class next to it
    /// with the same slot number is left alone.
    #[test]
    fn a_class_that_seeds_its_own_slot_gets_it_without_a_thrower_naming_one() {
        const WIDE: [&str; SLOT_COUNT + 1] =
            ["message", "previous", "backtrace", "location", "reason"];
        let mut table = ClassTable::new();
        let root = table.define("Throwable", &SLOT_NAMES, &[]);
        let rolled_back = table.define("Core\\Db\\RolledBack", &WIDE, &[root]);
        let db_error = table.define("Core\\Db\\DbError", &WIDE, &[root]);
        let (rolled_back, db_error) = (table.desc(rolled_back), table.desc(db_error));
        #[expect(unsafe_code, reason = "the table outlives both instances")]
        let (abandoned, refused) = unsafe {
            (
                Thrown::new_as(rolled_back, ThrownClass::DbRolledBack, "no stock", &[]),
                Thrown::new_as(db_error, ThrownClass::DbError, "no stock", &[]),
            )
        };
        let reason = abandoned
            .field(REASON_SLOT)
            .expect("the class declares a fifth slot");
        assert_eq!(reason.as_str_bytes(), Some(&b"no stock"[..]));
        assert!(
            refused
                .field(KIND_SLOT)
                .is_some_and(|kind| kind.as_int().is_none()),
            "a kind is the thrower's to pass; nothing seeds it"
        );
    }

    #[test]
    fn a_class_with_too_few_slots_is_refused_rather_than_written_past() {
        let mut table = ClassTable::new();
        let narrow = table.define("NotAnException", &["message"], &[]);
        #[expect(unsafe_code, reason = "the table outlives the call")]
        let e = unsafe { Thrown::new(table.desc(narrow), "boom") };
        assert!(e.is_none());
    }
}
