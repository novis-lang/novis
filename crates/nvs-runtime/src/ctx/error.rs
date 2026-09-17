//! The pending failure, the class it names, and the assertions a test records.
//!
//! `rule:errors/propagation`'s checked return says a
//! helper reports a failure by leaving it *pending* on the context and
//! answering its error value; every question a caller can then ask about that
//! failure is here — its message, its class, whether it conforms to a name, a
//! slot off it — as is [`Ctx::raise`], which is how a full [`Thrown`] becomes
//! one.
//!
//! [`ErrorClass`] is the descriptor table that makes those questions
//! answerable at all: without a class table installed a pending failure is a
//! message and nothing more, and
//! `rule:errors/escalation-ladder`'s ladder needs it to
//! tell a catchable class from a fatal one.

use super::*;

/// One entry of `rule:testing/failure-ledger`'s ledger: an assertion that ran, and how it came out.
///
/// The outcome is the *message*, not a `bool`, because the ledger is what the
/// runner reports from — reading the exception state instead is exactly the
/// silently-passing test § 5 abolishes, and a `bool` would send it back there
/// for the wording.
#[derive(Clone, Debug)]
pub struct AssertionOutcome {
    /// Which `Core\Test` member ran — a literal the member itself names, so a
    /// passing assertion costs no allocation.
    pub member: &'static str,
    /// `None` where the assertion held; the failure's message where it did
    /// not.
    pub failure: Option<String>,
}

/// One `Core\Test::assertMatchesInline` that did not hold — the material
/// `rule:testing/inline-snapshots`'s
/// `nvs test --update` splices from.
///
/// Both halves are the *text*, because both halves of the join are: the
/// snapshot the author wrote is what identifies the literal in the compiler's
/// own table of written call sites, and the rendering is what replaces it. The
/// runtime holds no span for either — a helper is called with a value, never
/// with the expression that built it — so this record carries no location at
/// all and the runner joins it to one.
///
/// Recorded on every mismatch rather than only under `--update`, for
/// [`AssertionOutcome`]'s reason: what a run recorded may not depend on a flag
/// the run was started with, or the flag becomes a second thing that decides
/// whether a test failed. What `--update` gates is the *writing*, which is the
/// runner's and happens after the whole suite.
#[derive(Clone, Debug)]
pub struct SnapshotMismatch {
    /// The snapshot as the source holds it — the join key.
    pub expected: String,
    /// What `Core\Debug::render` made of the value instead.
    pub produced: String,
}

/// One class descriptor, plus the table that owns it.
///
/// The safe way to hand a [`Ctx`] a descriptor that must outlive it: a
/// [`ClassDesc`]'s *address* is its identity (`crate::object`), so a context
/// cannot make one up and still match a `catch` clause's — it has to be handed
/// the compiled unit's. Carrying the [`Arc`](std::sync::Arc)-shared
/// [`ClassTable`] along with the id is what turns "the table must outlive the
/// context" from a contract into a fact, which is why
/// [`Ctx::set_runtime_error_class`] needs no `unsafe` at all.
///
/// The share is atomic because the table's one owner is a compiled unit that
/// every core reads (`nvs_codegen::Unit`), and a handle taken out of it has to
/// be able to cross with it. One atomic increment per install is what that
/// costs, per `rule:programs/memory-priority`.
#[derive(Clone, Debug)]
pub struct ErrorClass {
    table: std::sync::Arc<ClassTable>,
    id: ClassId,
}

impl ErrorClass {
    /// A handle on `id` within `table`.
    #[must_use]
    pub fn new(table: std::sync::Arc<ClassTable>, id: ClassId) -> Self {
        Self { table, id }
    }

    /// The descriptor's address, live for as long as this handle is.
    #[must_use]
    pub fn desc(&self) -> *const ClassDesc {
        self.table.desc(self.id)
    }

    /// A handle on another class of the *same* table, by name — how
    /// [`Ctx::error_desc`] reaches spec § 10's `ParseError` from the
    /// `RuntimeError` the embedder installed.
    ///
    /// Sharing the table is the point: the returned handle keeps it alive by
    /// itself, so a `catch` clause's descriptor and this one are two addresses
    /// in the same table and compare by pointer the way `crate::object`'s
    /// conformance test requires.
    #[must_use]
    pub fn sibling(&self, name: &str) -> Option<Self> {
        Some(Self::new(
            std::sync::Arc::clone(&self.table),
            self.table.id_of(name)?,
        ))
    }
}

/// What is behind a pending non-[`crate::OK`] status.
///
/// One field on [`Ctx`], not two: the exception object **subsumes** the
/// message a [`crate::Fault`] leaves behind, rather than sitting beside it.
/// Two fields would mean two places to ask "what failed", and every read would
/// have to state which one wins.
///
/// The two variants are not two kinds of failure — they are the same failure
/// at two levels of detail:
///
/// * [`Pending::Message`] is what a runtime helper's [`crate::Fault`] and
///   every [`crate::FATAL`] produce. It allocates nothing when the message is
///   `'static`, which is the property
///   `rule:errors/throw-is-not-slower`
///   depends on: `benches/abi-probe` measures a throw against a normal return,
///   and where the message allocates a throw costs a multiple of one while a
///   `'static` message makes it *cheaper* than one — and PHP code throws on
///   ordinary control-flow paths.
/// * [`Pending::Thrown`] is what Novis's own `throw` produces, and the only one
///   carrying a backtrace. A `Message` is promoted to one on demand — by
///   [`Ctx::take_thrown`] when a `catch` dispatch takes it, or by
///   [`Ctx::push_frame`] when a `THROWN` unwinds a compiled frame — so a
///   helper-raised throw is catchable and traceable without every helper
///   paying for an allocation it usually does not need.
///
/// Promotion needs a class to build the object from, which only the compiled
/// unit has — see [`Ctx::set_runtime_error_class`]. With none installed, a
/// `Message` stays a message: it is still reported, and still ends the
/// request, but no `catch` clause matches it and it grows no backtrace.
///
/// A [`crate::FATAL`] never becomes a `Thrown`: compiled code only ever pushes
/// a frame for a `THROWN` status, and no `catch` is ever entered for a
/// `FATAL` (`rule:errors/escalation-ladder`).
///
/// [`Pending::Thrown`] is a slot for an **object**, not for a `Throwable`:
/// [`Thrown`] is one nullable reference with no class bound on it, and
/// [`Ctx::raise`] records whatever it is handed. Every read of one is
/// width-guarded rather than ancestry-guarded — [`Thrown::message`],
/// [`Thrown::field`] and [`Thrown::push_frame`] each answer emptily for a class
/// declaring fewer than [`crate::SLOT_COUNT`] slots instead of reaching past
/// its end — while [`Ctx::pending_conforms_to`], which is the reading a `catch`
/// clause binds by, answers off the descriptor's own ancestry. So an object of
/// a class outside the `Throwable` tree rides the `THROWN` path through every
/// landing pad and matches no `catch` naming a class in that tree, and neither
/// property costs a widening here.
#[derive(Debug)]
pub(super) enum Pending {
    /// A message alone, with no exception object behind it yet, plus the
    /// class it will be promoted to — see [`ThrownClass`].
    Message(ThrownClass, Cow<'static, str>),
    /// Novis's own exception object.
    Thrown(Thrown),
}

impl Pending {
    /// The message, whichever shape this is.
    fn message(&self) -> Cow<'_, str> {
        match self {
            Self::Message(_, message) => Cow::Borrowed(message),
            Self::Thrown(thrown) => Cow::Owned(thrown.message()),
        }
    }

    /// The class a promotion would build, or `None` for a failure that is
    /// already an object.
    fn class(&self) -> Option<ThrownClass> {
        match self {
            Self::Message(class, _) => Some(*class),
            Self::Thrown(_) => None,
        }
    }

    /// This failure as an exception object, allocating one of `class` around a
    /// bare message if that is all there is.
    ///
    /// # Safety
    ///
    /// `class` must be null or refer to a live class descriptor that outlives
    /// every instance made from it.
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness is the caller's obligation and \
                  cannot be expressed in the signature"
    )]
    unsafe fn into_thrown(self, class: *const ClassDesc) -> Thrown {
        match self {
            // The class is passed through rather than dropped: it is what
            // decides which slot this promotion seeds — `rule:core-classes/derive-reports-every-field`'s
            // `issues`, `rule:core-classes/db-transactions`'s `reason` — see `Thrown::new_as`.
            #[expect(unsafe_code, reason = "forwarding this function's own contract")]
            Self::Message(thrown, message) => unsafe {
                Thrown::new_as(class, thrown, &message, &[])
            },
            Self::Thrown(thrown) => thrown,
        }
    }
}

impl Ctx {
    /// Records the message behind a `THROWN` or `FATAL` status, as spec
    /// § 10's `RuntimeError`.
    pub fn set_pending(&mut self, message: impl Into<Cow<'static, str>>) {
        self.set_pending_as(ThrownClass::Runtime, message);
    }

    /// Records the message behind a `THROWN`, naming which of spec § 10's
    /// classes a `catch` will see — [`ThrownClass`] owns the roster.
    pub fn set_pending_as(&mut self, class: ThrownClass, message: impl Into<Cow<'static, str>>) {
        self.pending = Some(Pending::Message(class, message.into()));
        // A message has no object yet and so no frame to supersede: whatever
        // failure [`Self::raise_sited`] marked one for is the one this replaces.
        self.site_frame_pending = false;
    }

    /// Installs the class a bare-message failure is promoted to — spec
    /// § 10's `RuntimeError`, "the world said no", which is what a runtime
    /// helper's failure is.
    ///
    /// It is also this context's *anchor into the compiled unit's class
    /// table*: every other § 10 class is reached from it by name
    /// ([`ErrorClass::sibling`]), so a helper that throws a
    /// [`ThrownClass::Parse`] needs no second installation call. One handle
    /// rather than one per class because they are not independent — they all
    /// come from the one table a `Unit` owns, and installing a subset would
    /// make "which classes can this request throw" a property of the embedder.
    ///
    /// A caller that never installs one gets the degraded behaviour
    /// [`Pending`] describes, never a crash.
    pub fn set_runtime_error_class(&mut self, class: ErrorClass) {
        self.runtime_error_class = Some(class);
    }

    /// The descriptor for the class `name` spells in **this program's** table,
    /// or `None` for a name it does not declare.
    ///
    /// The one route from a `Core` member to a class by name, and it exists for
    /// [`crate::graph::decode`]: `rule:classes/serialize-is-a-closed-format` refuses a payload
    /// naming a class the receiving side cannot resolve, and the two tables
    /// that can resolve one are the compiled unit's and the `Core` library's.
    ///
    /// **The program's own table is asked first.** It is the one
    /// [`Self::set_runtime_error_class`] installed rather than a second
    /// registration, for that method's own reason — § 10's classes and every
    /// class the program declares are all rows of one table, and a second
    /// handle on it would be a second thing to keep in step. A name it does not
    /// hold is put to [`Self::set_core_classes`]'s resolver, which is what lets
    /// an encoded `Core\Time\Instant` arrive back as one rather than as a
    /// refusal naming it.
    ///
    /// The pointer is live for as long as this context is: the program's handle
    /// shares ownership of its table ([`ErrorClass`]), and the `Core`
    /// descriptors are one table leaked for the process.
    #[must_use]
    pub fn class_desc(&self, name: &str) -> Option<*const ClassDesc> {
        self.runtime_error_class
            .as_ref()
            .and_then(|class| class.sibling(name))
            .map(|class| class.desc())
            .or_else(|| (self.core_classes?)(name))
    }

    /// The same table as a **handle that keeps it alive by itself** — one
    /// [`ErrorClass`] is a handle on the whole table, since `sibling` reaches
    /// any class in it by name.
    ///
    /// [`Self::class_desc`] answers for a caller holding this context; this
    /// answers for one that will still be asking after it has let go of it.
    /// Its caller is the isolate boundary: a child's answer is copied out on
    /// the child's own stack, where the *parent's* context is borrowed by the
    /// frame parked on the join, so the receiving side has to have been taken
    /// before the child started. Cloning it is an `Rc` bump and holding it
    /// keeps one class table alive for the length of one isolate, which is
    /// bounded by what is in flight.
    #[must_use]
    pub fn class_table(&self) -> Option<ErrorClass> {
        self.runtime_error_class.clone()
    }

    /// The descriptor `class` names, or the installed `RuntimeError`'s if the
    /// table holds no such class, or null if none was installed at all.
    ///
    /// Falling back rather than failing is deliberate: a compiled unit always
    /// carries the whole seeded tree (`nvs_hir::errors::TREE`), so a miss here
    /// means an embedder built a table by hand — and a failure that arrives as
    /// a `RuntimeError` is strictly better than one that arrives as no object
    /// at all.
    fn error_desc(&self, class: ThrownClass) -> *const ClassDesc {
        let Some(installed) = self.runtime_error_class.as_ref() else {
            return std::ptr::null();
        };
        installed
            .sibling(class.name())
            .map_or_else(|| installed.desc(), |found| found.desc())
    }

    /// Runs `body` with this context's pending failure set aside, **reporting**
    /// anything `body` raised and putting the saved one back.
    ///
    /// # A throw out of a dying generator's `finally` is reported where it stands
    ///
    /// Its one caller is [`crate::object::dismantle`], and a release has no
    /// error edge to propagate on: `nvs_object_release` answers nothing, and
    /// the release itself is very often *already* running under an exception —
    /// a landing pad dropping its locals on the way out. Leaving the throw in
    /// the pending slot would therefore either replace the exception actually
    /// in flight with one raised by a `finally` the program never resumed into
    /// by hand, or attach itself to whatever call returns next; the first loses
    /// the original as well. So the `finally` **runs** — which is what PHP
    /// compatibility asks for (`rule:iteration/generators`) — and the throw
    /// escaping it is **reported rather than carried**: it goes to
    /// `rule:errors/escalation-ladder`'s tier 3 through
    /// [`crate::floor::escalate`], and to the floor beneath it when no handler
    /// answered, which is every tier the ladder has left once the throw cannot
    /// be propagated. Nothing it does replaces the failure in flight; the
    /// restore below is what holds that.
    ///
    /// **Tier 2 is skipped, and that is the divergence from PHP that remains.**
    /// `rule:errors/on-uncaught-throw`'s handler belongs to a request that
    /// reached its root uncaught, and this throw reached no root at all — it was
    /// raised under a refcount hitting zero. Running the program's own handler
    /// from there is user code re-entered from inside a release with another
    /// failure already in flight, which is the thing this whole function exists
    /// to prevent. PHP reports such a throw as uncaught, so the message, the
    /// class and the backtrace are the same and the *tier* is not.
    ///
    /// **What it spends:** one [`crate::floor::uncaught`] record — a node per
    /// frame, bounded by [`nvs_render::Caps`] — per throw that escapes, and
    /// nothing at all when none does.
    pub(crate) fn with_pending_set_aside<R>(&mut self, body: impl FnOnce(&mut Self) -> R) -> R {
        let saved = self.pending.take();
        // The mark travels with the failure it describes, or the throw put back
        // below would have `body`'s answer to "is the innermost frame still
        // provisional" rather than its own.
        let saved_site_frame = std::mem::take(&mut self.site_frame_pending);
        let out = body(self);
        if self.pending.is_some() {
            // Taken rather than borrowed: the report needs this context, and
            // the escaped throw is an object whose reference is released as
            // `escaped` drops at the end of the block.
            let escaped = self.take_thrown();
            let record = crate::floor::uncaught(&escaped);
            if !crate::floor::escalate(self, &record) {
                crate::floor::report(self, &record);
            }
        }
        // Dropping a `Pending::Thrown` releases the exception object's own
        // reference, which is why this is a replace rather than an assignment.
        // It covers the report as well: a tier-3 handler that left a failure of
        // its own on this context does not get to be the one that comes back.
        drop(std::mem::replace(&mut self.pending, saved));
        self.site_frame_pending = saved_site_frame;
        out
    }

    /// Records an already-built exception as the pending `THROWN`, taking
    /// ownership of the reference it was handed.
    ///
    /// The exception is one nothing rendered a raise site for — what a
    /// helper's [`crate::Fault`] produces, and what [`crate::nvs_raise_new`]
    /// falls back to when it is handed no site — so any frame
    /// [`Self::raise_sited`] marked as provisional belongs to a failure this
    /// one replaces, and the mark goes with it.
    pub fn raise(&mut self, thrown: Thrown) {
        self.pending = Some(Pending::Thrown(thrown));
        self.site_frame_pending = false;
    }

    /// [`Self::raise`] for Novis's own `throw`: `seeded` is
    /// [`Thrown::capture_site`]'s answer, and says the exception's innermost
    /// backtrace frame is the label the raise rendered rather than one a
    /// compiled frame pushed — so the first frame the throw unwinds out of
    /// replaces it instead of naming that frame a second time.
    ///
    /// A raise carrying **no** site leaves the mark exactly as it found it.
    /// The one compiled shape that raises without one is a `catch` matching no
    /// clause, handing the very same object onward, so the frame this context
    /// seeded at the original throw is still the innermost one and is still
    /// the one the compiler's own rendering supersedes.
    pub(crate) fn raise_sited(&mut self, thrown: Thrown, seeded: bool) {
        self.pending = Some(Pending::Thrown(thrown));
        self.site_frame_pending |= seeded;
    }

    /// Records a `THROWN` of `class` carrying `message`, with each pair in
    /// `slots` written into that slot of the object it builds —
    /// [`crate::Fault::ThrownWithSlots`]' one destination, and so the whole of
    /// how a native member fills a property of a subclass's own, below the
    /// ones `Throwable` declares (`ParseError::$issues`,
    /// `Core\Db\DbError::$kind` and the raw `$sqlState` beside it).
    ///
    /// The object is built **here** rather than left as a [`Pending::Message`]
    /// to be promoted later, which is what keeps the pending state free of an
    /// owned reference: every path that replaces or discards a pending failure
    /// would otherwise have to release one, and exactly one of those paths
    /// being missed is the shape a refcount leak takes. Only a member with
    /// something to put in the slot reaches this, so the eager allocation is on
    /// a path that has already allocated.
    ///
    /// A `class` whose descriptor is too narrow for a slot releases that value
    /// and throws without it — [`Thrown::new_as`] owns that, and it is the same
    /// "nothing installed" case a null descriptor is.
    ///
    /// # Safety
    ///
    /// Each of `slots`' values must be one whose reference is being transferred
    /// here.
    #[expect(
        unsafe_code,
        reason = "the values' references and the installed descriptor's liveness \
                  are both obligations the signature cannot express"
    )]
    pub unsafe fn raise_with_slots(
        &mut self,
        class: ThrownClass,
        message: &str,
        slots: &[(usize, crate::Value)],
    ) {
        let desc = self.error_desc(class);
        #[expect(
            unsafe_code,
            reason = "the descriptor comes from the `Rc`-shared table this \
                      context holds, so it outlives the instance; the values' \
                      references are forwarded"
        )]
        let thrown = unsafe { Thrown::new_as(desc, class, message, slots) };
        self.raise(thrown);
    }

    /// Renders the site of a failure raised in this frame and caught in it —
    /// [`crate::nvs_raise_site`]'s whole body, and the only place a frame is
    /// recorded for an exception that unwinds out of nothing.
    ///
    /// A failure that already names a frame keeps it. A `throw` or a checked
    /// operator rendered its own through [`Thrown::capture_site`], and one
    /// that arrived from a callee names that callee's frame, so this writes
    /// only for the case the compiled raises leave open: a helper's bare
    /// [`crate::Fault`], whose site is the statement whose call failed.
    ///
    /// The promotion to an object is [`Self::push_frame`]'s, on the same
    /// terms — a class the driver never installed leaves the message pending
    /// and unsited rather than failing.
    ///
    /// The frame this writes is **provisional**, exactly as a `throw`'s own
    /// is: a `catch` matching no clause hands the exception onward, and the
    /// label that frame pushes on its way out replaces this one rather than
    /// naming the frame twice ([`Self::raise_sited`]).
    ///
    /// # Safety
    ///
    /// `blob` must be null or an address [`crate::source::encode`]'s bytes
    /// were baked at.
    #[expect(
        unsafe_code,
        reason = "the carrier's liveness is the caller's obligation and cannot \
                  be expressed in the signature"
    )]
    pub unsafe fn seed_raise_site(&mut self, blob: *const u8) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        // Captured before promotion, for the one case promotion cannot produce
        // an object for — see `Pending`'s note on an uninstalled class.
        let message = pending.message().into_owned();
        let class = pending.class();
        let desc = class.map_or(std::ptr::null(), |class| self.error_desc(class));
        #[expect(
            unsafe_code,
            reason = "`set_runtime_error_class`'s own contract makes the \
                      installed descriptor outlive every instance built here"
        )]
        let thrown = unsafe { pending.into_thrown(desc) };
        if thrown.is_none() {
            self.pending = Some(Pending::Message(
                class.unwrap_or_default(),
                Cow::Owned(message),
            ));
            return;
        }
        if !thrown.has_frame() {
            #[expect(unsafe_code, reason = "forwarding this function's own contract")]
            let seeded = unsafe { thrown.capture_site(blob) };
            self.site_frame_pending |= seeded;
        }
        self.pending = Some(Pending::Thrown(thrown));
    }

    /// The pending message, if any, without clearing it.
    #[must_use]
    pub fn pending(&self) -> Option<Cow<'_, str>> {
        self.pending.as_ref().map(Pending::message)
    }

    /// The class descriptor a pending failure would be caught through, or
    /// `None` where nothing is pending.
    ///
    /// The pointer may still be null: a [`Pending::Message`] raised before
    /// [`Self::set_runtime_error_class`] installed anything has no class to
    /// resolve against, which is that method's documented "no exception class
    /// installed" state.
    fn pending_desc(&self) -> Option<*const ClassDesc> {
        Some(match self.pending.as_ref()? {
            Pending::Message(class, _) => self.error_desc(*class),
            Pending::Thrown(thrown) => thrown.class_desc(),
        })
    }

    /// The name of the class a pending failure would be caught as, without
    /// clearing it — [`Self::pending`]'s companion, for a reader that has to
    /// report *what* was thrown rather than what it said.
    #[must_use]
    pub fn pending_class(&self) -> Option<String> {
        let desc = self.pending_desc()?;
        if desc.is_null() {
            return self
                .pending
                .as_ref()?
                .class()
                .map(ThrownClass::name)
                .map(str::to_owned);
        }
        #[expect(
            unsafe_code,
            reason = "the descriptor came out of the `Rc`-shared table this \
                      context holds, or off a live exception object's own \
                      header, so it outlives this borrow"
        )]
        Some(unsafe { (*desc).name() }.to_owned())
    }

    /// Whether a pending failure is an instance of the class `name` spells —
    /// **its own class or any ancestor**, which is exactly what a `catch`
    /// clause naming that class would bind.
    ///
    /// # Decision: an ancestor matches
    ///
    /// `rule:testing/assertions-are-typed`'s `Core\Test::assertThrows` is the caller, and a test
    /// naming `RuntimeError` is claiming no more than that the failure is one
    /// — matching a subclass is what PHP's own `expectException` does, and it
    /// is the reading under which the assertion agrees with the `catch` a
    /// reader would have written by hand instead. The narrower "this exact
    /// class" reading is available to a test that wants it, by asserting the
    /// name: there is no spelling of the wider one if this answers narrowly.
    ///
    /// The ancestry itself is read off the descriptor
    /// ([`crate::ClassDesc::conforms_to_name`]) rather than off a second copy
    /// of `nvs_hir::errors::TREE` here, so the two cannot disagree about what
    /// `ParseError` descends from. `false` where nothing is pending, and where
    /// no exception class was ever installed to resolve one against.
    #[must_use]
    pub fn pending_conforms_to(&self, name: &str) -> bool {
        let Some(desc) = self.pending_desc() else {
            return false;
        };
        if desc.is_null() {
            return false;
        }
        #[expect(
            unsafe_code,
            reason = "the descriptor came out of the `Rc`-shared table this \
                      context holds, or off a live exception object's own \
                      header, so it outlives this borrow"
        )]
        unsafe {
            (*desc).conforms_to_name(name)
        }
    }

    /// One extra slot of the pending failure's exception object, read
    /// **without clearing it** — [`Self::take_thrown`]'s borrowing half, for a
    /// caller that has to decide something about a throw it may still re-raise
    /// unchanged.
    ///
    /// `rule:core-classes/db-error`'s retry loop is why
    /// this exists: it has to know whether the closure's own refusal was a
    /// deadlock or a serialization failure before it decides to run the
    /// closure again, and `take_thrown` would clear the very failure it is
    /// still deciding about — a decision that came out "do not retry" would
    /// then have to re-raise a throw it had already consumed.
    ///
    /// `class` is not decoration. `KIND_SLOT` and `ISSUES_SLOT` are the same
    /// number, so a slot index alone would read a `ParseError`'s issue array
    /// as a `Core\Db\ErrorKind`; the answer is `None` unless the pending
    /// failure is an instance of `class`, by [`Self::pending_conforms_to`]'s
    /// reading of that word — the one a `catch` naming it would bind.
    ///
    /// `None` also where nothing is pending, where the failure is a bare
    /// [`Pending::Message`] with no object behind it at all, and where the
    /// class declares too few fields to hold that slot
    /// ([`Thrown::field`]). The value is borrowed, not retained: it is good
    /// only while the failure is still pending.
    #[must_use]
    pub fn pending_slot(&self, class: &str, slot: usize) -> Option<Value> {
        if !self.pending_conforms_to(class) {
            return None;
        }
        match self.pending.as_ref()? {
            Pending::Message(_, _) => None,
            Pending::Thrown(thrown) => thrown.field(slot),
        }
    }

    /// Takes the pending message, clearing it — and dropping the exception
    /// object behind it, if there was one.
    #[must_use]
    pub fn take_pending(&mut self) -> Option<Cow<'static, str>> {
        Some(match self.pending.take()? {
            Pending::Message(_, message) => message,
            Pending::Thrown(thrown) => Cow::Owned(thrown.message()),
        })
    }

    /// Appends one entry to
    /// `rule:testing/failure-ledger`'s ledger — what every `Core\Test` assertion does, whether it held
    /// or not.
    ///
    /// Recording the *passing* ones as well is not bookkeeping for its own
    /// sake: § 20's "a test that asserts nothing fails" is a question about
    /// how many entries a test produced, and a ledger holding only failures
    /// cannot answer it.
    pub fn record_assertion(&mut self, member: &'static str, failure: Option<String>) {
        self.assertions.push(AssertionOutcome { member, failure });
    }

    /// How many entries the ledger holds — the mark
    /// [`Self::discharge_failures_from`] is given.
    #[must_use]
    pub fn assertion_count(&self) -> usize {
        self.assertions.len()
    }

    /// Removes every **failed** entry recorded at or after `mark`, answering
    /// how many there were — `Core\Test::expectFailure(callable)`'s half of
    /// `rule:testing/failure-ledger`.
    ///
    /// The passing entries in that range stay: they are assertions that really
    /// ran, and § 20 counts them. Only the failure is discharged, and only
    /// where the caller has said it expected one.
    pub fn discharge_failures_from(&mut self, mark: usize) -> usize {
        let mark = mark.min(self.assertions.len());
        let mut discharged = 0;
        let mut index = mark;
        while index < self.assertions.len() {
            if self.assertions[index].failure.is_some() {
                self.assertions.remove(index);
                discharged += 1;
            } else {
                index += 1;
            }
        }
        discharged
    }

    /// The whole ledger, taken — what the runner reads at the end of a test.
    ///
    /// Taking rather than borrowing is what makes one test's ledger that
    /// test's: the runner clears it between tests by consuming it, so a
    /// context reused across a file cannot report an earlier test's entries
    /// against a later one.
    #[must_use]
    pub fn take_assertions(&mut self) -> Vec<AssertionOutcome> {
        std::mem::take(&mut self.assertions)
    }

    /// Records `rule:testing/inline-snapshots`'s mismatch — `Core\Test::assertMatchesInline` is
    /// the one caller, and it calls this beside the failed ledger entry rather
    /// than instead of it.
    pub fn record_snapshot_mismatch(&mut self, expected: String, produced: String) {
        self.snapshot_mismatches
            .push(SnapshotMismatch { expected, produced });
    }

    /// Every snapshot mismatch this context recorded, taken — read once, on the
    /// test's own stack, exactly as [`Self::take_assertions`] is and for its
    /// reason: the record is one test's, and a context that outlived the test
    /// must not offer it to the next.
    #[must_use]
    pub fn take_snapshot_mismatches(&mut self) -> Vec<SnapshotMismatch> {
        std::mem::take(&mut self.snapshot_mismatches)
    }

    /// Takes the pending failure as an exception object, clearing it — what a
    /// `catch` binds to its variable, and what `nvs run` reports a backtrace
    /// from.
    ///
    /// Promotes a bare [`Pending::Message`] rather than returning `None` for
    /// one: a helper-raised `THROWN` is as catchable as Novis's own, it just has
    /// no backtrace to show.
    #[must_use]
    pub fn take_thrown(&mut self) -> Thrown {
        let Some(pending) = self.pending.take() else {
            return Thrown::none();
        };
        let desc = pending
            .class()
            .map_or(std::ptr::null(), |class| self.error_desc(class));
        #[expect(
            unsafe_code,
            reason = "`set_runtime_error_class`'s own contract makes the \
                      installed descriptor outlive every instance built here"
        )]
        unsafe {
            pending.into_thrown(desc)
        }
    }

    /// Records one more frame a pending `THROWN` has unwound out of.
    ///
    /// A bare message is promoted to a real exception here, so a helper-raised
    /// throw accumulates a backtrace from the first compiled frame it leaves.
    ///
    /// The first frame after a raise that rendered its own site **replaces**
    /// that rendering rather than following it: both name the frame the throw
    /// is leaving, and this one is the compiler's, which knows the label a
    /// script frame carries and a site blob does not
    /// ([`Self::raise_sited`]).
    pub fn push_frame(&mut self, label: &str) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        // Captured before promotion, so the message survives the one case
        // promotion cannot produce an object for — see `Pending`'s note on an
        // uninstalled class.
        let message = pending.message().into_owned();
        let class = pending.class();
        let desc = class.map_or(std::ptr::null(), |class| self.error_desc(class));
        #[expect(
            unsafe_code,
            reason = "`set_runtime_error_class`'s own contract makes the \
                      installed descriptor outlive every instance built here"
        )]
        let thrown = unsafe { pending.into_thrown(desc) };
        if thrown.is_none() {
            self.pending = Some(Pending::Message(
                class.unwrap_or_default(),
                Cow::Owned(message),
            ));
            return;
        }
        if std::mem::take(&mut self.site_frame_pending) {
            thrown.replace_innermost_frame(label);
        } else {
            thrown.push_frame(label);
        }
        self.pending = Some(Pending::Thrown(thrown));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::throwable::Frame;

    /// `rule:core-classes/db-error`'s retry loop reads a refusal's `kind` off a failure it has
    /// not decided about yet, so the read leaves the pending exactly as it
    /// found it — and the class it names is the whole of what keeps
    /// `KIND_SLOT` from reading a `ParseError`'s `issues` back as an
    /// `ErrorKind`, the two being the same slot number.
    #[test]
    fn a_pending_refusals_kind_reads_back_without_disturbing_it() {
        const WIDE: [&str; 5] = ["message", "previous", "backtrace", "location", "kind"];
        const NARROW: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut table = ClassTable::new();
        let root = table.define("RuntimeError", &NARROW, &[]);
        table.define("Core\\Db\\DbError", &WIDE, &[root]);
        table.define("ParseError", &WIDE, &[root]);

        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(table), root));
        #[expect(
            unsafe_code,
            reason = "an `int` carries no reference for the slot to take over"
        )]
        unsafe {
            ctx.raise_with_slots(
                ThrownClass::DbError,
                "refused",
                &[(crate::KIND_SLOT, Value::int(5))],
            );
        }

        assert_eq!(
            ctx.pending_slot("Core\\Db\\DbError", crate::KIND_SLOT)
                .and_then(Value::as_int),
            Some(5)
        );
        assert!(
            ctx.pending_slot("ParseError", crate::KIND_SLOT).is_none(),
            "the slot number is the same one; only the class tells them apart"
        );
        assert_eq!(
            ctx.pending_class().as_deref(),
            Some("Core\\Db\\DbError"),
            "reading a slot decides nothing and clears nothing"
        );

        let taken = ctx.take_thrown();
        assert_eq!(taken.message(), "refused");
        assert!(
            ctx.pending_slot("Core\\Db\\DbError", crate::KIND_SLOT)
                .is_none(),
            "nothing is pending once it has been taken"
        );
    }

    /// A helper's fault reaches a `catch` in the frame that called the helper
    /// having unwound out of nothing, so the site seeded on that edge is the
    /// only thing that ever names where it happened. The other half is the
    /// skip: a failure carrying a frame already names its own origin, and a
    /// second label there would name a frame twice
    /// (`rule:errors/throw-is-not-slower`).
    #[test]
    fn a_seeded_site_names_the_raising_frame_and_leaves_a_named_one_alone() {
        const NARROW: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut table = ClassTable::new();
        let root = table.define("RuntimeError", &NARROW, &[]);
        let shared = std::sync::Arc::new(table);

        let site = crate::source::encode(&nvs_render::Source {
            file: "case.nvs".to_owned(),
            line: 7,
            member: Some("Relay::go".to_owned()),
        });

        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::clone(&shared), root));
        ctx.set_pending("the world said no");
        #[expect(
            unsafe_code,
            reason = "the carrier is a live `Vec` for the whole of the call"
        )]
        unsafe {
            ctx.seed_raise_site(site.as_ptr());
        }
        let seeded = ctx.take_thrown();
        assert_eq!(seeded.message(), "the world said no");
        assert_eq!(
            seeded
                .frames()
                .iter()
                .map(Frame::label)
                .collect::<Vec<String>>(),
            vec!["Relay::go() at case.nvs:7"],
            "a bare-message fault is promoted and the seeded frame is the whole trace"
        );

        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(shared, root));
        ctx.set_pending("raised further down");
        // The frame a callee pushed on its way out: this failure already says
        // where it came from.
        ctx.push_frame("Inner::deeper() at case.nvs:21");
        #[expect(
            unsafe_code,
            reason = "the carrier is a live `Vec` for the whole of the call"
        )]
        unsafe {
            ctx.seed_raise_site(site.as_ptr());
        }
        assert_eq!(
            ctx.take_thrown()
                .frames()
                .iter()
                .map(Frame::label)
                .collect::<Vec<String>>(),
            vec!["Inner::deeper() at case.nvs:21"],
            "an exception arriving from a callee passes through the seed untouched"
        );
    }

    /// The property `Pending`'s own doc states, pinned: the pending slot is a
    /// place for an object rather than for a `Throwable`, so a marker class
    /// outside the tree can travel the `THROWN` path — which is what runs every
    /// `finally` between the raise and the root — while matching no `catch`
    /// clause naming a class in that tree. Both halves are load-bearing for
    /// `Core\Script::finish`, and neither is asserted anywhere the compiled
    /// side can see; a widening of either read here is what would silently
    /// admit one to a `catch (Throwable $e)`.
    #[test]
    fn a_pending_object_outside_the_throwable_tree_unwinds_intact_and_matches_no_catch() {
        const THROWABLE: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut table = ClassTable::new();
        let root = table.define("Throwable", &THROWABLE, &[]);
        table.define("RuntimeError", &THROWABLE, &[root]);
        // No parents and no slots: outside the tree, and not shaped like it
        // either — the narrowest class a promotion or a backtrace push can meet.
        table.define("Core\\Script\\Finish", &[] as &[&str], &[]);

        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(table), root));
        let desc = ctx
            .class_desc("Core\\Script\\Finish")
            .expect("the installed table defines it");
        #[expect(
            unsafe_code,
            reason = "the descriptor comes out of the table this context holds, \
                      so it outlives the object, whose one reference is handed \
                      to the `Thrown`"
        )]
        let marker = unsafe { Thrown::from_raw(crate::NvsObj::new(desc).into_raw()) };
        ctx.raise(marker);

        assert_eq!(ctx.pending_class().as_deref(), Some("Core\\Script\\Finish"));
        assert!(
            !ctx.pending_conforms_to("Throwable"),
            "a `catch (Throwable $e)` binds by this and must not admit the marker"
        );
        assert!(!ctx.pending_conforms_to("RuntimeError"));
        assert!(
            ctx.pending_conforms_to("Core\\Script\\Finish"),
            "it is still an instance of its own class"
        );

        // One compiled frame reporting itself on the way out. A class narrower
        // than `SLOT_COUNT` grows no backtrace, and stays the very object that
        // was raised rather than being promoted to a `RuntimeError`.
        ctx.push_frame("main");
        assert_eq!(ctx.pending_class().as_deref(), Some("Core\\Script\\Finish"));
        assert_eq!(
            ctx.pending().as_deref(),
            Some(""),
            "there is no message slot to read one out of"
        );

        let taken = ctx.take_thrown();
        assert_eq!(taken.class_name(), "Core\\Script\\Finish");
        assert!(
            taken.field(crate::MESSAGE_SLOT).is_none(),
            "a read past the end of the class answers nothing rather than panicking"
        );
        assert!(ctx.pending().is_none());
    }

    #[test]
    fn a_pending_message_is_taken_once() {
        let mut ctx = Ctx::buffered();
        ctx.set_pending("boom");
        assert_eq!(ctx.pending().as_deref(), Some("boom"));
        assert_eq!(ctx.take_pending().as_deref(), Some("boom"));
        assert!(ctx.pending().is_none());
    }

    #[test]
    fn expect_failure_discharges_only_the_failures_inside_its_own_body() {
        // `rule:testing/failure-ledger`: the discharge is what `Core\Test::expectFailure` does,
        // and it is deliberately narrow in both directions — a failure recorded
        // *before* the mark is another test's problem and stays, and a passing
        // assertion inside the body really ran, so § 20 still counts it.
        let mut ctx = Ctx::buffered();
        ctx.record_assertion("assertSame", Some("an earlier failure".to_owned()));
        let mark = ctx.assertion_count();
        ctx.record_assertion("assertSame", None);
        ctx.record_assertion("assertEquals", Some("the expected one".to_owned()));
        ctx.record_assertion("assertEqualsDeep", Some("and a second".to_owned()));

        assert_eq!(ctx.discharge_failures_from(mark), 2);
        let left = ctx.take_assertions();
        assert_eq!(left.len(), 2);
        assert_eq!(left[0].failure.as_deref(), Some("an earlier failure"));
        assert_eq!(left[1].member, "assertSame");
        assert!(left[1].failure.is_none());
        // Taken rather than borrowed: the next test starts empty.
        assert_eq!(ctx.assertion_count(), 0);
        assert_eq!(ctx.discharge_failures_from(mark), 0);
    }

    /// Serialises the two tests below against each other. The installed tier 3
    /// is process-wide, so one test's handler would otherwise answer the
    /// other's escalation — and the pair exist precisely to tell an answered
    /// escalation from an unanswered one.
    static LADDER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Every record [`claims`] was handed, rendered.
    static ESCALATED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

    /// A tier 3 that reports everything, which is the answer that leaves the
    /// floor nothing to write.
    fn claims(_ctx: &mut Ctx, record: &nvs_render::Record) -> bool {
        ESCALATED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(nvs_render::json::render(record));
        true
    }

    /// Installs `ladder` for the length of the test and puts back what it
    /// displaced, however the test ends.
    struct Installed(Option<crate::floor::Ladder>);

    impl Drop for Installed {
        fn drop(&mut self) {
            crate::floor::swap_ladder(self.0);
        }
    }

    /// A context whose failures can be read back, with the seeded error tree a
    /// promotion needs — the pair every test of a reported throw wants.
    fn reporting_ctx() -> Ctx {
        const NARROW: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut table = ClassTable::new();
        let root = table.define("RuntimeError", &NARROW, &[]);
        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(table), root));
        ctx.set_diagnostic_sink(crate::OutputSink::Buffer(Vec::new()));
        ctx
    }

    /// `rule:errors/escalation-ladder`'s first sentence, on the one path that
    /// has no request root to report at: a throw escaping an abandoned
    /// generator's `finally` reaches tier 3, and the exception actually in
    /// flight is still the pending one afterwards.
    #[test]
    fn a_throw_escaping_a_set_aside_region_reaches_the_ladder_and_replaces_nothing() {
        let _serialised = LADDER_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        ESCALATED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        let _installed = Installed(crate::floor::swap_ladder(Some(claims)));

        let mut ctx = reporting_ctx();
        ctx.set_pending("the exception the release is unwinding under");
        ctx.with_pending_set_aside(|ctx| ctx.set_pending("thrown out of an abandoned finally"));

        let escalated = ESCALATED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(escalated.len(), 1, "one escalation, and no retry of it");
        assert!(
            escalated[0].contains("thrown out of an abandoned finally"),
            "the escaped throw is what tier 3 is handed: {}",
            escalated[0]
        );
        assert_eq!(
            ctx.take_thrown().message(),
            "the exception the release is unwinding under",
            "the failure in flight is the one that comes back"
        );
        assert!(
            ctx.take_buffered_diagnostic()
                .expect("the diagnostic channel was given a buffer")
                .is_empty(),
            "tier 4 is the floor beneath tier 3, not a second line beside it"
        );
    }

    /// The other half of that answer: nothing reported, so the floor owes the
    /// line — which is also every process that installed no handler at all.
    #[test]
    fn a_throw_escaping_a_set_aside_region_falls_to_the_floor_when_no_tier_3_answers() {
        let _serialised = LADDER_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _installed = Installed(crate::floor::swap_ladder(None));

        let mut ctx = reporting_ctx();
        ctx.with_pending_set_aside(|ctx| ctx.set_pending("thrown out of an abandoned finally"));

        assert!(ctx.pending_class().is_none(), "nothing is carried onward");
        let written = String::from_utf8(
            ctx.take_buffered_diagnostic()
                .expect("the diagnostic channel was given a buffer"),
        )
        .expect("a record renders as UTF-8");
        assert!(
            written.contains("thrown out of an abandoned finally"),
            "the floor writes the throw nothing else reported: {written}"
        );
    }
}
