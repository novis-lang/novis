//! `rule:errors/diagnostic-record`'s
//! one record model, and the renderings of it.
//!
//! Every developer-facing output in Novis is a [`Record`]: an [`Envelope`]
//! plus a tree of [`Node`]s. The model is **closed** — a node is one of a
//! fixed set of kinds and there is no extension point — and it is **content,
//! not presentation**: it carries no colour, no indentation, no width and no
//! ordering-for-display. A rendering supplies them all. That separation is the
//! whole of `rule:errors/diagnostic-record`, and it is why adding a rendering costs one
//! implementation rather than one per producer.
//!
//! # What is here
//!
//! The model, the plaintext rendering ([`plain`]), § 5's four transformations,
//! and the JSON one ([`json`]) that a log target emits. Beside them, the two
//! sink transforms — [`text::substitute`] and [`html::escape`] — which are
//! peers rather than part of the model: each is what one of
//! `rule:tooling/echo-always-has-a-sink`'s sinks does to text on the way in,
//! and they live here because the record's own transformations already do.
//! `Core\Debug::dump` and `Core\Log::write` are producers in `nvs-stdlib` for
//! the reason § *Where this sits* gives, and an uncaught `Throwable` is one in
//! `nvs_runtime::floor`, which walks a value through `nvs_runtime::record` and
//! carries its frames as [`Node::Frame`] nodes. The rendering and the two
//! producers that do not exist are the block below.
//!
//! **`rule:errors/log-write`'s
//! record-and-write helper renders here**, not in `nvs-runtime` beside the
//! escalation ladder. § 6 asks that ordinary application code and the tier-4
//! floor write one shape through one implementation, and `rule:errors/diagnostic-record` has
//! already put every rendering in this crate; a JSON writer in `nvs-runtime`
//! for the floor plus this one for everything else would be two writers that
//! agree today. The price is the dependency edge below, which § 1 sanctions and
//! this section prices.
//!
//! # Known gaps
//!
//! 1. **The HTML rendering is not written**, so the third of
//!    `rule:errors/renderings`' three renderings has no implementation and
//!    `[debug] inline` has nothing to wire into a response.
//!    — owner: m8-stdlib-depth
//! 2. **A `#[Test]` result is not a producer**, so § 22's three output
//!    formats are the runner's own printing rather than one record rendered
//!    three ways. `docs/decisions/0079.md:871` lands § 22 with the M4S tail,
//!    which the program is already past.
//!    — owner: m8-stdlib-depth
//! 3. **A compiler diagnostic is not a producer**, which
//!    `docs/decisions/0092.md:439` schedules rather than defers.
//!    — owner: M10
//!
//! # § 5's four transformations, and why they are the model's
//!
//! Redaction, control-byte substitution, the bidi rule and elision are
//! properties of the **record**, applied when it is built and before any
//! rendering sees it, so every rendering inherits identical answers and none
//! may weaken one. Three of the four are structural here rather than a
//! convention a producer is asked to follow:
//!
//! * **Control bytes and bidi** are [`Rendered`]'s constructor. That newtype is
//!   the only text a node carries, and the only way to build one is
//!   [`Rendered::new`], which applies [`text::substitute`]. There is no
//!   spelling that puts an un-substituted byte into the model, which is what
//!   makes "decided once" a property of the type rather than of a review.
//! * **Elision** is [`Node::Elided`] carrying an [`Elision`], so a cut is a
//!   node and every rendering renders it as a cut. PHP and Python truncate per
//!   formatter, so the same value is complete in one output and truncated in
//!   another and no reader can tell which.
//! * **A cycle** is [`Node::Cycle`], an identity rather than a `*RECURSION*`
//!   string, so the HTML rendering can link the repeat and the JSON one can
//!   emit a reference.
//!
//! **Redaction is the one a producer must apply**, because what is `secret` is
//! a property of a *declared type* and nothing in this crate can see one:
//! [`Node::Redacted`] stands where the value would have been, and the producer
//! that walks a runtime value is what decides. `nvs_stdlib::debug` is that
//! walk today.
//!
//! # Where this sits
//!
//! `rule:errors/diagnostic-record` puts the model in one crate that both the runtime and the
//! compiler front end depend on, which is why it is not in `nvs-diagnostics`:
//! `nvs-runtime` depends on no `nvs-*` crate but this one, so the dependency
//! has to run the other way.
//!
//! **This crate is a leaf, and that is what a dependent producer costs.** Its
//! dependents are `nvs-runtime` — `rule:errors/log-write`'s tier-4 floor, which renders an uncaught `Throwable` through
//! [`json::line`] — and `nvs-stdlib`, whose `Core\Log::write` is the same
//! render reached from the other caller. Its only dependency is `serde_json`.
//! Depending on `nvs-syntax` for `rule:security/bidi-predicate`'s
//! bidi predicate would close a cycle, since `nvs-syntax` depends on
//! `nvs-diagnostics` and the floor is a dependent here. The predicate lives in
//! [`bidi`] instead and `nvs-syntax` reads it from below, so
//! `rule:security/bidi-predicate`'s one rule is one implementation.
//! `nvs check` rendering from `nvs-diagnostics` needs
//! nothing further: that edge already runs the way round it has to.
//!
//! # What it spends
//!
//! One `Record` per dump, freed with the statement that built it: a `String`
//! per text node, a `Vec` per container. [`Caps`] is what bounds it — a
//! cyclic or merely enormous value cannot make the record larger than the caps
//! allow, which is the property that lets a dump be reached from a request
//! path at all.

pub mod bidi;
pub mod html;
pub mod json;
pub mod plain;
pub mod text;

/// `rule:errors/log-level`'s five levels, with the fixed syslog mapping that section's table gives.
///
/// The mapping is fixed because `rule:errors/engine-floor` names `syslog` as a target and a severity is not optional there.
///
/// This is the Rust side. The *Novis* enum `Log\Level` that `Core\Log::write`
/// takes is `nvs_stdlib::registry`'s: its cases are these and its integer
/// backings are [`Self::syslog_severity`], read from here rather than written
/// again.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum Level {
    /// `Core\Debug::dump`'s destination — `rule:errors/debug-dump`.
    #[default]
    Debug,
    /// Ordinary progress.
    Info,
    /// `rule:http-server/a-development-server-on-a-public-interface-warns-and-serves`
    /// 's public-bind record.
    Warn,
    /// An uncaught `Throwable`.
    Error,
    /// `rule:errors/escalation-ladder`'s tier-3
    /// and tier-4 floor. It exists so the escalation ladder has a level of its
    /// own rather than a parallel channel.
    Critical,
}

impl Level {
    /// Every level, quietest first — the roster [`Self::syslog_severity`] and
    /// [`Self::name`] are total over, and the one
    /// [`Self::from_syslog_severity`] searches.
    pub const ALL: [Self; 5] = [
        Self::Debug,
        Self::Info,
        Self::Warn,
        Self::Error,
        Self::Critical,
    ];

    /// The level whose [`Self::syslog_severity`] is `severity`, or `None` for a
    /// number `rule:errors/log-level`'s roster does not carry — syslog's `Notice` (5) and
    /// `Alert` (1) among them.
    ///
    /// It reads the mapping back by *searching* [`Self::ALL`] rather than by
    /// matching the five integers a second time, so the severities are written
    /// once and this direction cannot come to disagree with the other. The
    /// caller that needs it is `Core\Log::write`, whose `Core\Log\Level` cases
    /// are valued by these severities, so what arrives at the member is one of
    /// these integers and nothing else.
    #[must_use]
    pub fn from_syslog_severity(severity: u8) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.syslog_severity() == severity)
    }

    /// The syslog severity `rule:errors/log-level`'s table pairs with this level.
    #[must_use]
    pub const fn syslog_severity(self) -> u8 {
        match self {
            Self::Debug => 7,
            Self::Info => 6,
            Self::Warn => 4,
            Self::Error => 3,
            Self::Critical => 2,
        }
    }

    /// The level's own name, lower case, as every rendering spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }

    /// The level's own case name, as `rule:errors/log-level`'s roster writes it —
    /// `Log\Level::Debug` — and as [`Self::of`] reads `[log] level`.
    ///
    /// Another match over the roster rather than a case fold of [`Self::name`],
    /// because there is no `const` fold and because these are two spellings of
    /// one level rather than one spelling seen twice.
    #[must_use]
    pub const fn case_name(self) -> &'static str {
        match self {
            Self::Debug => "Debug",
            Self::Info => "Info",
            Self::Warn => "Warn",
            Self::Error => "Error",
            Self::Critical => "Critical",
        }
    }

    /// The level `written` names, or `None` for a word `rule:errors/log-level`'s roster
    /// does not carry.
    ///
    /// **Two spellings, and both of them the documentation's own.** § 2 writes
    /// the roster as `Log\Level::Debug`, and so does
    /// `rule:config/a-mode-is-five-defaults`
    /// 's per-mode default column for `[log] level`; a record renders the
    /// same level as [`Self::name`]'s `debug`. An operator has read one of the
    /// two and writes back what they read, so both resolve. Neither is a fold
    /// of the other: `DEBUG` is refused like any other word, the way
    /// `nvs_config::log::Target` refuses `STDERR`.
    ///
    /// The caller is `[log] level` — `nvs_config::log::validate` asks whether a
    /// tree boots, and `nvs_runtime::Ctx::write_log_record` asks what the
    /// quietest written level is — the two-reader split `[log] target`'s own
    /// grammar is in `nvs-config` for. Searched over [`Self::ALL`] rather than
    /// matched again, for [`Self::from_syslog_severity`]'s reason.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.case_name() == written || level.name() == written)
    }
}

/// Text a record carries, with `rule:errors/record-transformations`'s control-byte and bidi
/// transformations already applied.
///
/// The only way to build one is [`Rendered::new`], and it is the only text
/// shape a [`Node`] holds — which is what makes § 5's *"decided once, in the
/// model"* a property of the type rather than a rule each producer is asked to
/// remember. A producer that wants the original bytes back does not get them:
/// the substitution is deliberately one-way, because the record exists to be
/// looked at.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rendered(String);

impl Rendered {
    /// `text` with [`text::substitute`] applied — the one constructor.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(text::substitute(text).into_owned())
    }

    /// The substituted text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Rendered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One scalar value, tagged with the Novis type it *is* rather than with how
/// it prints — so `"1"` and `1` are never confusable, which is the one thing
/// `print_r` cannot do.
#[derive(Clone, PartialEq, Debug)]
pub enum Scalar {
    /// `null`.
    Null,
    /// `bool`.
    Bool(bool),
    /// `int`.
    Int(i64),
    /// `uint` — `rule:types/arithmetic`.
    Uint(u64),
    /// `float`.
    Float(f64),
    /// `decimal` — `rule:types/decimal`,
    /// carried as the exact text the value renders as rather than as an `f64`,
    /// which is the whole reason that type exists.
    Decimal(String),
    /// `string`, with its length in **bytes** — the length is carried rather
    /// than taken from the text because § 5's substitution changes it and the
    /// reader wants the value's own.
    Str {
        /// The substituted text.
        text: Rendered,
        /// The value's own length in bytes, before substitution changed it.
        bytes: usize,
    },
    /// `bytes` — `rule:types/bytes`'s
    /// binary scalar, held raw. A rendering decides how to show them; the
    /// model does not, because they are not text and § 5's substitution is
    /// about text.
    Bytes(Vec<u8>),
}

/// What an [`Node::Elided`] node says was cut, and how much of it — `rule:errors/record-transformations`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Elision {
    /// The subtree below [`Caps::depth`] was cut.
    Depth,
    /// A container held more entries than [`Caps::entries`]: this node stands
    /// after the ones that were kept and names how many were not.
    Entries {
        /// How many entries the container has in all.
        total: usize,
        /// How many of them this record does not carry.
        cut: usize,
    },
    /// A text or `bytes` scalar was longer than [`Caps::text`]: this node
    /// carries the prefix that was kept and names how many bytes were not.
    ///
    /// It replaces the scalar node rather than sitting beside it, because a
    /// scalar has no children for it to be one of. That is the one asymmetry
    /// in the roster, and it is what keeps a cut a *node* in every rendering.
    Text {
        /// The kept prefix, substituted like any other text.
        kept: Rendered,
        /// How many bytes of the original this record does not carry.
        cut: usize,
    },
}

/// One node of a record's tree. The roster is `rule:errors/diagnostic-record`'s, closed.
#[derive(Clone, PartialEq, Debug)]
pub enum Node {
    /// A scalar, tagged with its Novis type.
    Scalar(Scalar),
    /// An `array` whose keys are `"0"`, `"1"`, … in order — the list shape.
    Sequence(Vec<Node>),
    /// An `array` read by key — the map shape.
    Map(Vec<(Rendered, Node)>),
    /// A class instance: the class name and its **declared** properties, per
    /// `rule:classes/no-debug-hook`. Never a `toString` result, and never a customization hook — ADR
    /// 0092 § 7.
    Object {
        /// The class's rendered name.
        class: String,
        /// The instance's identity within this record, numbered from `1` in
        /// the order the walk first met each object, or `None` for a producer
        /// that has no identity to give.
        ///
        /// It is carried on every object rather than only on a repeated one
        /// because a single-pass walk cannot know that a node will be revisited
        /// until it already has, and a second pass to erase the unused ones
        /// would buy nothing: a [`Node::Cycle`] names an id, so every id has to
        /// be findable for the reference to resolve.
        id: Option<usize>,
        /// Its declared properties, in slot order.
        properties: Vec<(String, Node)>,
    },
    /// An enum case: the enum's name and the case's, never its underlying
    /// integer (`rule:enums/closed-integer-type`).
    EnumCase {
        /// The enum's rendered name.
        enum_name: String,
        /// The case's own name.
        case: String,
    },
    /// A closure, by the signature it declares — never a body, and never
    /// captured state (`rule:types/closure-literal`).
    Closure {
        /// How many parameters it declares.
        parameters: usize,
    },
    /// Stands where a `secret`-typed value would have been — `rule:errors/record-transformations`,
    /// `rule:security/secret-sinks-refuse`.
    Redacted,
    /// Stands where content was cut, naming what and how much.
    Elided(Elision),
    /// A repeat of a node already in this record, by the identity
    /// `nvs_runtime::identity` gives it — not a `*RECURSION*` string, so a
    /// rendering can link or reference it.
    Cycle {
        /// The repeated node's identity within this record, numbered from `1`
        /// in the order the walk first met each object.
        id: usize,
    },
    /// A source range with a label — what a compiler diagnostic is made of.
    Span {
        /// The file the range is in.
        file: String,
        /// Its one-based first line.
        line: u32,
        /// What the range is being pointed at for.
        label: Rendered,
    },
    /// One frame of a `Throwable`'s backtrace — `rule:errors/record-producers`'s
    /// throw producer carries one of these per frame, rather than one string
    /// for the whole stack summary.
    ///
    /// Its own kind rather than an [`Self::Object`] because a frame is not a
    /// class instance: no `Throwable\Frame` exists for a program to name, and
    /// the two readers of a trace want two spellings the shape already has —
    /// the `#0`-first line a person greps in the plaintext rendering, and
    /// `{function, file, line}` in the JSON one, which an object's
    /// `$class`/`$properties` envelope would bury. Novis's frames carry a label
    /// and nothing else (`rule:errors/propagation` builds the trace as the
    /// throw unwinds), so these three parts are the whole of one.
    Frame {
        /// How far the frame is from the throw: `0` is the one it was raised
        /// in. Carried on the node rather than read off its position, so a
        /// frame says how deep it is wherever a rendering puts it.
        depth: usize,
        /// The member the frame was running — `Class::member`, or a script's
        /// own label — without the `()` a rendering adds.
        function: Rendered,
        /// The file it was written in, absent for a label that carries no site.
        file: Option<String>,
        /// Its one-based line, on the same terms.
        line: Option<u32>,
    },
}

/// The depth and length bounds `rule:errors/record-transformations` owes the model, past which content
/// becomes an [`Elision`].
///
/// The numbers are this crate's, taken under AGENTS.md's priority ordering
/// rather than from the ADR, which names the caps without fixing them. They
/// are chosen for a **reader**: a dump is looked at by a person, so the bound
/// that matters is what stays legible rather than what stays cheap.
///
/// * `depth` — eight levels. Deeper than that is not read; it is scrolled
///   past, and an ORM entity graph reaches it in one hop.
/// * `entries` — a hundred. Several screens, so a container that is merely
///   large still shows what it holds and one that is unbounded does not run
///   the terminal off.
/// * `text` — 1024 bytes. Long enough to recognise a rendered SQL statement or
///   a JSON body, short enough that a megabyte upload does not become the
///   whole output.
///
/// **They bound the record, not the rendering.** A cut is decided once and
/// every rendering shows the same cut, which is the property § 5 exists for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Caps {
    /// How many levels of container the record carries before the rest
    /// becomes [`Elision::Depth`].
    pub depth: usize,
    /// How many entries of one container it carries before the rest becomes
    /// [`Elision::Entries`].
    pub entries: usize,
    /// How many bytes of one text or `bytes` scalar it carries before the rest
    /// becomes [`Elision::Text`].
    pub text: usize,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            depth: 8,
            entries: 100,
            text: 1024,
        }
    }
}

/// Where a record was produced — `rule:errors/diagnostic-record`'s `source` field.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Source {
    /// The file's path as the program named it.
    pub file: String,
    /// Its one-based line.
    pub line: u32,
    /// The enclosing member, `Class::member`, or `None` at file scope.
    pub member: Option<String>,
}

/// What is true of a whole record and nothing about how it looks — `rule:errors/diagnostic-record`'s table.
///
/// Every field but [`Self::level`] is optional, and an absent one is **omitted**
/// by a rendering rather than rendered empty — `rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`
/// 's rule for `trace_id`/`span_id`, applied to the whole envelope because a
/// producer that has no request to name should not have to invent one.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Envelope {
    /// RFC 3339, as `rule:errors/log-write` already fixes.
    pub ts: Option<String>,
    /// § 2's level.
    pub level: Level,
    /// A plain `string`, never a qualified one — `rule:security/secret-sinks-refuse`
    /// 's `Throwable`-message rule, on the same argument.
    pub message: Option<Rendered>,
    /// The request this record belongs to.
    pub request_id: Option<String>,
    /// Present only when a trace is active, omitted rather than empty.
    pub trace_id: Option<String>,
    /// The span within that trace, on the same terms.
    pub span_id: Option<String>,
    /// Where the record was produced.
    pub source: Option<Source>,
    /// How many identical records this one stands for —
    /// `rule:http-server/the-floor-cannot-fill-the-disk`
    /// 's coalescing counter, absent for the ordinary record that stands
    /// only for itself.
    ///
    /// **Written by the sink, not by the producer**, which is why it is an
    /// envelope key rather than an entry in [`Self::fields`]: a producer's bag
    /// is the call site's own vocabulary, and a `count` the rate limiter added
    /// would collide with one a program had named. `nvs_runtime::floor` is the
    /// only writer today.
    pub count: Option<u64>,
    /// Named fields, each carrying a node — **not** a stringly bag, which is
    /// what lets the compile-time field schema `rule:errors/log-fields` keeps possible
    /// arrive without changing the model.
    pub fields: Vec<(String, Node)>,
}

/// One developer-facing output: `rule:errors/diagnostic-record`'s envelope plus a tree of nodes.
///
/// [`Self::nodes`] is what the producer had to say beyond the envelope —
/// `Core\Debug::dump`'s one node per argument, a `Throwable`'s frames, a
/// `#[Test]` result's expected and actual. A producer with nothing but fields
/// leaves it empty.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Record {
    /// What is true of the whole record.
    pub envelope: Envelope,
    /// The record's own nodes, in the order the producer wrote them.
    pub nodes: Vec<Node>,
}

impl Record {
    /// An empty record at `level`.
    #[must_use]
    pub fn at(level: Level) -> Self {
        Self {
            envelope: Envelope {
                level,
                ..Envelope::default()
            },
            nodes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:errors/log-level`'s table, which is fixed because `rule:errors/engine-floor` names
    /// `syslog` as a target and a severity is not optional there.
    #[test]
    fn every_level_carries_its_syslog_severity() {
        assert_eq!(Level::Debug.syslog_severity(), 7);
        assert_eq!(Level::Info.syslog_severity(), 6);
        assert_eq!(Level::Warn.syslog_severity(), 4);
        assert_eq!(Level::Error.syslog_severity(), 3);
        assert_eq!(Level::Critical.syslog_severity(), 2);
    }

    /// The one constructor substitutes, so there is no spelling that puts an
    /// un-substituted byte into the model — § 5 as a property of the type.
    #[test]
    fn text_in_the_model_is_already_substituted() {
        assert_eq!(Rendered::new("a\u{1B}b").as_str(), "a\u{241B}b");
        assert_eq!(Rendered::new("a\rb").as_str(), "a\u{240D}b");
        assert_eq!(Rendered::new("a\nb").as_str(), "a\nb");
    }
}
