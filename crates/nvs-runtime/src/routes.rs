//! `rule:routing/matched-once-before-the-handler`
//! 's route table, as a *running* program sees it: the rows the compiler
//! built, and the match the door takes against them once.
//!
//! # Why the table is a runtime value at all
//!
//! [`crate::commands`]' argument, one table along, and the same one § 1 makes
//! for itself: the table is a compile product — `nvs_types::routes::RouteTable`,
//! built by the same `rule:programs/implementing` scan `rule:tooling/terminal-output-is-a-sink`'s commands are — and the
//! question asked of it is a *request's*, which no compile-time answer can
//! hold. So the rows cross, and they cross as **strings and two closed enums**:
//! [`CaptureConv`], which is the one thing a matcher needs that no string
//! spells, and [`Param`], which is what a converted capture became. Nothing
//! below this line learns that a compiler exists — this crate has no
//! `nvs-types` dependency and could not name one of its types if it wanted to.
//!
//! # Where the two halves live
//!
//! The **table** is installed on [`crate::Ctx`] before the program runs, by
//! whoever compiled it, exactly as the command table and the configuration
//! snapshot are. A context with no table is a program that declared no
//! `#[Route]`, which is `rule:routing/table-is-opt-in`'s opt-in rule as a member sees it.
//!
//! The **match** is not on the context: it is on [`crate::Inbound`], because it
//! is a fact about the request rather than about the program, and § 1's whole
//! rule is that it is taken once *before* any application code and travels from
//! there. `nvs_server::route` is the door that takes it and the home of that
//! direction; `Core\Request::route()` is the one reader.
//!
//! # What matching is, and what it is deliberately not
//!
//! [`Routes::match_request`] answers with the row and its typed captures, and
//! **dispatches nothing** — § 1's second rule. A failed conversion is not a
//! match, which is why the walk continues to the next row rather than answering
//! with a row whose capture it could not fill.
//!
//! **A capture typed as a class built from text is the one exception, and it is
//! not a conversion this walk performs.** [`CaptureConv::Parses`] matches on
//! shape and carries the segment with the class that claims it, because this
//! walk runs at the door with no program installed and reaching a compiled
//! `parse` here would put an implementor's body over every request URL ahead of
//! anything that rate-limits it. Such a capture converts where the match
//! crosses into the program, and a segment the class refuses throws there — a
//! `400` over a route that did match, rather than the `404` a failed conversion
//! leads to. `rule:security/route-capture-is-laundered-by-its-type` is the home
//! of the split and of what it costs.
//!
//! Where two rows both match, the one with *more fixed segments earlier* wins:
//! every row carries a rank — one byte per segment, fixed below capture below
//! optional below catch-all — and the smallest rank in load order is the answer.
//! That is `matchit`'s left-to-right precedence, which `rule:routing/path-grammar` names as the
//! model, stated as a comparison rather than grown out of a trie.
//!
//! **The match crosses as four answers, and never as the row.**
//! `Core\Request::route()` answers with `nvs_stdlib::router`'s
//! `Core\Router\Match`, built out of [`Match`] where the match crosses: the
//! name, the captures, the declared verb and the access decision's name, which
//! are what a dispatcher reads. The [`Route`] itself stays on this side, and so
//! does its handler label, because a program that could read `Class::method`
//! off a match is one step from the dispatch
//! `rule:routing/matching-is-not-dispatching` refuses.
//!
//! **What it spends:** one `Arc` clone per request that carries a table, over
//! one `String` per row's verb, path, name, handler and access decision — tens
//! of them for an application, and nothing at all for a program that declares no
//! route. A *matched* request holds one `Arc` bump on the row plus one `String`
//! per capture, which is the segment text it converted, and a second `String`
//! for a capture typed as a class built from text, which is that class's own
//! name. O(in-flight requests), per `rule:programs/memory-priority`.
//!
//! **A capture leaves this crate as the segment arrived, still
//! percent-encoded**, and is decoded once on the far side of the crossing.
//! [`Route::convert`] is why the decode cannot be here: it runs first, so a
//! segment decoded before it would let `%34` reach a `{n: uint}` route as `4`.
//! The decoder is `nvs_stdlib::uri`'s — a second one here would be the
//! two-that-agree-today failure the tainted laundering rules exist to prevent —
//! and `nvs_stdlib::router`'s `capture_value` is both the crossing and the home
//! of that rule, including what a capture whose octets are not UTF-8 becomes. A
//! `uint` capture is unaffected either way: no digit has an encoded spelling.
//!
//! # A trie picks the candidates, and the rank still picks the winner
//!
//! [`Routes::match_request`] does not visit every row. The table keeps a trie
//! keyed by each row's **leading fixed segments**, built once in
//! [`Routes::new`]: a row hangs off the node its fixed prefix ends at, which is
//! the depth of its first capture, or its full length where it has none. A
//! request descends that trie one piece at a time and collects the rows hung
//! along the way — exactly the rows whose fixed prefix equals the request's
//! own — and only those reach the verb comparison and [`Route::fill`]. So what
//! a match costs grows with the path and with the rows sharing its prefix,
//! not with the table.
//!
//! The trie is a filter and nothing more. Every row a request could fill is a
//! candidate, because a fixed segment that is not the request's piece at that
//! depth is one `fill` would refuse; and the candidates are visited in load
//! order, so the smallest rank and its load-order tie-break are the same
//! comparison over the same rows that win it. Precedence is not re-derived
//! out of the trie's shape, which is why the trie needs no wildcard edges.
//!
//! What stays linear is a table whose rows begin with a capture — `/{lang}/…`
//! on every row hangs every row off the root — which is the walk this
//! replaced, and no slower than it.
//!
//! **What it spends:** one node per distinct fixed prefix plus one index per
//! row, built once per table and shared by every request that carries it; and
//! per request one `Vec` of candidate indexes, sorted back into load order.
//!
//! The figure lives in `benches/abi-probe/benches/routing.rs`, which matches
//! one request against tables differing only in how many rows they hold, and
//! the guard beside it in `benches/abi-probe/tests/perf_guards.rs` fails a
//! build when that table size starts to show. `benches/scaling/request/routes.nvs`
//! is the same question asked of a served program.

use std::collections::HashMap;
use std::sync::Arc;

use crate::decimal::Decimal;

/// What a capture's text becomes before it reaches the program.
///
/// `rule:routing/a-capture-narrows-to-a-closed-set`'s "a capture narrows to a closed set with a type" as the *one*
/// fact about that type which crosses: not the type, but the conversion the
/// checker already picked for it. [`crate::commands::ArgConv`] is the sibling
/// this is modelled on, and `nvs_types::routes::RouteParam`'s `ty` and
/// `allowed` are where the choice is made.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureConv {
    /// `string` — the segment's own text, unconverted and `tainted`.
    Text,
    /// `int` — a signed decimal, and **no match** where the segment is not one.
    Int,
    /// `uint` — as [`Self::Int`], and no match where the number is negative.
    Uint,
    /// `decimal` — `rule:types/conversion`'s literal, whole, and no match where the
    /// segment is not one. The parse is [`crate::decimal::Decimal::parse`]
    /// itself rather than a grammar written here: a second decimal reader
    /// would agree today and drift tomorrow, which is the `Uuid` arm's reason
    /// one type along.
    Decimal,
    /// `Core\Uuid` — RFC 9562 § 4's canonical form, and no match where the
    /// segment is not one. The parse is [`crate::uuid::read`], which is where
    /// that grammar lives *because* this arm has to reach it: the class is
    /// `nvs_stdlib::uuid`'s, one crate above, and a capture is accepted or
    /// refused here. That module's doc is the home of the placement.
    Uuid,
    /// § 5's closed set: the segment text of each admitted value, in the order
    /// the union declares them. A segment outside the set is no match, which is
    /// what makes the narrowing a property of the *table* rather than a check
    /// the handler was trusted to write.
    OneOf(Vec<String>),
    /// A class a segment reaches through its own `parse` —
    /// `rule:expressions/try-parse`'s pair read as the `Parses` interface —
    /// named here so the crossing can reach that member. Every such class but
    /// `Core\Uuid`, which the arm above reads natively and which therefore
    /// still narrows the match.
    ///
    /// **The segment matches on shape and this walk converts nothing**, which
    /// is the whole of why the class name travels beside it. A match runs at
    /// the door with no armed class table and before anything rate-limits the
    /// request that reached it, so reaching a compiled `parse` from here would
    /// put a body that can loop, allocate and throw over every request URL
    /// including the ones that match no route at all;
    /// [`crate::commands::ArgConv::Parses`] is the one home of what that reach
    /// costs. `nvs_stdlib::router`'s `capture_value` is the binding site that
    /// makes it instead, and a segment the class refuses throws there rather
    /// than falling through to the next route — the class-typed exception
    /// `rule:security/route-capture-is-laundered-by-its-type` carves out of its
    /// own "a failed conversion is not a match".
    Parses(String),
    /// An enum, or a subset of one: every admitted case as the segment text it
    /// matches on and the constant that text becomes, and a segment naming none
    /// of them is no match, exactly as a failed `int` conversion is.
    ///
    /// **Which spelling each case took is decided while compiling** —
    /// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`
    /// takes the written backing value where every admitted case wrote one and
    /// the case name otherwise, and `nvs_types::routes`' `enum_capture` is the
    /// one reading of it. Nothing about that choice crosses, because nothing
    /// here has a second question to ask of it: the match is a lookup in the set
    /// that arrived.
    ///
    /// **The value is the case's own backing integer**, handed over as
    /// [`Param::Int`] or [`Param::Uint`] rather than as an arm of its own. A
    /// case is indistinguishable from that integer by the time it is a
    /// [`Value`](crate::Value) ([`crate::object::EnumCases`] says so), so
    /// constructing anything would be constructing the number twice —
    /// [`crate::commands::ArgConv::Enum`] is the same decision for a
    /// command-line word, and the reason both can make it is that an enum has no
    /// class machinery to reach for.
    Enum {
        /// The enum's declared name, fully qualified — carried for the reason
        /// [`Self::Parses`] carries its class: the compiler resolved it once,
        /// and a [`Match`] hands over the captures and never the row.
        class: String,
        /// Every admitted case: the segment text, and the value it becomes.
        cases: Vec<(String, crate::commands::CaseValue)>,
    },
    /// A type § 5 admits and no arm above turns text into: a `bool`. The
    /// segment matches and its text is handed over — never silently `Text`, so
    /// what is missing stays an arm rather than a behaviour somebody has to
    /// notice.
    Unconverted,
}

/// One capture of a route: the parameter it binds and the conversion its text
/// takes.
///
/// Apart from the path it was written in because the path is text and this is
/// what the *signature* said about it — the two are joined by name, which is
/// `rule:routing/a-query-parameter-is-declared-like-a-capture`'s own rule for relating a capture to a parameter.
#[derive(Clone, Debug)]
pub struct Capture {
    /// The parameter's name, sigil-less, as § 3 compares it.
    pub name: String,
    /// What this capture's text becomes.
    pub conv: CaptureConv,
}

/// A capture as the request filled it — § 1's "typed parameters".
#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    /// The segment's text. `tainted` everywhere above this crate: it is a
    /// request path the peer wrote.
    Text(String),
    /// A `int` capture, converted.
    Int(i64),
    /// A `uint` capture, converted.
    Uint(u64),
    /// A `decimal` capture, converted — the value, not the text it arrived as,
    /// so `19.90` keeps the scale `rule:types/conversion` says it renders with.
    Decimal(Decimal),
    /// A `Core\Uuid` capture, converted: the sixteen octets, in the order the
    /// canonical text writes them. The bytes rather than a type, because the
    /// type is `nvs_stdlib::uuid`'s — this is the argument its `of_octets`
    /// already takes, which is the same seam `rule:core-classes/db-column-types`'s `UUID` column
    /// crosses on.
    Uuid([u8; 16]),
    /// A capture typed as a class built from text, as the two facts the
    /// crossing needs to finish it: the class that claims the segment, and the
    /// segment itself, still exactly as the peer wrote it.
    ///
    /// The one form that is not yet the value it will become, and it is not a
    /// [`Self::Text`] either — [`CaptureConv::Parses`] owns why this walk may
    /// not run a program's `parse`. The class name rides here rather than being
    /// looked back up, because a [`Match`] hands over the captures and never
    /// the row (this module's § 2).
    Parses {
        /// The class whose `parse` the crossing calls, fully qualified.
        class: String,
        /// The segment, still percent-encoded, exactly as [`Self::Text`]
        /// carries one: the decode is `nvs_stdlib::uri`'s and happens once, on
        /// the far side.
        text: String,
    },
}

/// § 2's three capture forms and the fixed segment that is none of them, as the
/// matcher walks a path rather than as an author wrote one.
///
/// Parsed once, when the row is built, because a path is fixed for the life of
/// the program and re-reading its braces per request would be the same answer
/// bought again at every hop.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Seg {
    /// Compared byte for byte and case-sensitively
    /// (`rule:classes/names-resolve-case-sensitively`).
    Fixed(String),
    /// `{name}` — one whole segment, which may not be empty.
    One(String),
    /// `{name?}` — one whole segment or none.
    Optional(String),
    /// `{name...}` — every remaining segment, as one value.
    Rest(String),
}

impl Seg {
    /// This form's place in § 2's precedence: lower binds tighter.
    fn rank(&self) -> u8 {
        match self {
            Self::Fixed(_) => 0,
            Self::One(_) => 1,
            Self::Optional(_) => 2,
            Self::Rest(_) => 3,
        }
    }
}

/// One row of § 1's table: a declared route, and what a match against it
/// answers with.
///
/// `nvs_types::routes::Route`'s fields that survive the crossing — the verb,
/// the path, § 1's name, the handler label and `rule:attributes/access-is-a-required-sibling`'s access decision —
/// plus the two things derived from the path once at boot: its parsed segments
/// and its rank. The OpenAPI half of the compiler's row (summary, tags,
/// security, errors, example) does not cross: nothing a *request* asks reads
/// it, and `rule:routing/api-document-is-generated-from-the-route-table`'s document is generated from the compiler's own table.
///
/// Built through [`Route::new`] rather than as a literal, because those two
/// derived fields are not the caller's to state — two fields that must agree
/// are two fields that can disagree.
#[derive(Clone, Debug)]
pub struct Route {
    verb: String,
    path: String,
    name: Option<String>,
    handler: String,
    access: Option<String>,
    csrf: bool,
    slotted: bool,
    captures: Vec<Capture>,
    segments: Vec<Seg>,
    rank: Vec<u8>,
}

/// `rule:security/csrf-is-on-by-default`'s four verbs — the ones a CSRF check covers — spelled as the
/// `Core\Http\Method` cases a row's [`Route::verb`] is written in.
///
/// A second reading of that section's list; `nvs_types::routes::UNSAFE_VERBS`
/// is the compiler's, for the diagnostic that refuses a pointless opt-out. The
/// two crates do not meet — nothing below this one depends on the checker — and
/// they are asking different questions of the same rule, whose one home is the
/// ADR. What crosses between them is [`Route::csrf`]'s answer and not this
/// list.
const UNSAFE_VERBS: [&str; 4] = ["Post", "Put", "Patch", "Delete"];

impl Route {
    /// The row a compiled `#[Route]` becomes, with its path read as § 2's
    /// grammar.
    ///
    /// A segment that is neither fixed text nor a well-formed capture is taken
    /// as a **fixed segment**, which is the fail-closed reading: it then matches its
    /// own text and absorbs nothing. Such a path does not compile
    /// (`nvs_types::routes::parse_path` refuses it), so this is only reachable
    /// from a table built by hand.
    #[must_use]
    pub fn new(
        verb: impl Into<String>,
        path: impl Into<String>,
        name: Option<String>,
        handler: impl Into<String>,
        access: Option<String>,
        captures: Vec<Capture>,
    ) -> Self {
        let verb = verb.into();
        let path = path.into();
        let segments = segments_of(&path);
        let rank = segments.iter().map(Seg::rank).collect();
        // `rule:security/csrf-is-on-by-default`'s default, derived rather than passed: on for the four
        // unsafe verbs and off for every other, so a table built by hand is
        // fail-closed by construction. § 1a's opt-out is the rare half and is
        // [`Self::without_csrf`].
        let csrf = UNSAFE_VERBS.contains(&verb.as_str());
        Self {
            verb,
            path,
            name,
            handler: handler.into(),
            access,
            csrf,
            slotted: false,
            captures,
            segments,
            rank,
        }
    }

    /// The same row with `rule:attributes/access-payload`'s opt-out recorded — the `csrf: false`
    /// its `#[Access]` wrote — so a request that matches it is not checked.
    ///
    /// A builder rather than another parameter to [`Self::new`], because the
    /// answer it changes is one every other row takes from its verb: a
    /// parameter would put the safe value in every call site that has nothing
    /// to say about it, and would make forgetting it fail open.
    #[must_use]
    pub fn without_csrf(mut self) -> Self {
        self.csrf = false;
        self
    }

    /// `rule:security/csrf-is-on-by-default`'s answer for this row: whether a request that matched it is
    /// CSRF-checked.
    ///
    /// The verb's half and § 1a's opt-out already folded together, because a
    /// reader of a match is asking one question and the two halves are decided
    /// at two different times — the verb here at boot, the opt-out by the
    /// compiler that wrote the row. `nvs_server::route::csrf_required` is the
    /// door's reader and the only one.
    #[must_use]
    pub fn csrf(&self) -> bool {
        self.csrf
    }

    /// The same row with `rule:core-classes/html-later`'s `slotted: true`
    /// recorded, so a response to a request that matched it is slotted.
    ///
    /// A builder for [`Self::without_csrf`]'s reason: almost every row is not
    /// slotted, and a parameter would put `false` in every call site.
    #[must_use]
    pub fn with_slotted(mut self) -> Self {
        self.slotted = true;
        self
    }

    /// Whether the route was declared `slotted: true`.
    #[must_use]
    pub fn slotted(&self) -> bool {
        self.slotted
    }

    /// The `Core\Http\Method` case this route is declared under, by its own
    /// name — `Get`, `Post`.
    #[must_use]
    pub fn verb(&self) -> &str {
        &self.verb
    }

    /// The path exactly as written, captures and all.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// § 1's `name`, or `None` where the route declares none.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// `Class::method` the attribute was attached to.
    #[must_use]
    pub fn handler(&self) -> &str {
        &self.handler
    }

    /// `rule:attributes/access-is-a-required-sibling`'s access decision as the name it resolved to, which
    /// `rule:security/access-is-checked-for-presence-not-meaning`
    /// leaves to whoever dispatches. `None` only for a program that was
    /// already refused.
    #[must_use]
    pub fn access(&self) -> Option<&str> {
        self.access.as_deref()
    }

    /// Every capture this route declares, by name.
    #[must_use]
    pub fn captures(&self) -> &[Capture] {
        &self.captures
    }

    /// The captures `request` fills, or `None` where this row does not match it
    /// at all — including the row that matches shape-wise and whose conversion
    /// failed, which § 1's measured table counts as a miss.
    fn fill(&self, request: &[&str]) -> Option<Vec<(String, Param)>> {
        let mut filled: Vec<(String, Param)> = Vec::new();
        let mut index = 0usize;
        for segment in &self.segments {
            match segment {
                Seg::Fixed(text) => {
                    if *request.get(index)? != text.as_str() {
                        return None;
                    }
                    index += 1;
                }
                Seg::One(name) => {
                    let text = *request.get(index)?;
                    // A capture is "one whole segment", and `//` carries none:
                    // admitting it would bind an empty string to a parameter
                    // whose declaration says a segment was there.
                    if text.is_empty() {
                        return None;
                    }
                    filled.push((name.clone(), self.convert(name, text)?));
                    index += 1;
                }
                Seg::Optional(name) => match request.get(index) {
                    // `/posts` against `/posts/{page?}`: the segment is absent
                    // and so is the parameter.
                    None => {}
                    // `/posts/` — a trailing slash is the same absence written
                    // with one more byte, and not an empty value.
                    Some(&"") => index += 1,
                    Some(&text) => {
                        filled.push((name.clone(), self.convert(name, text)?));
                        index += 1;
                    }
                },
                Seg::Rest(name) => {
                    if index >= request.len() {
                        return None;
                    }
                    let joined = request[index..].join("/");
                    if joined.is_empty() {
                        return None;
                    }
                    filled.push((name.clone(), self.convert(name, &joined)?));
                    index = request.len();
                }
            }
        }
        // Every segment of the request has to have been claimed: a route is
        // matched whole, and a prefix of one is a different path.
        (index == request.len()).then_some(filled)
    }

    /// `text` as the parameter `name` is declared to take it, or `None` where
    /// it is not one of those values.
    ///
    /// A capture no declaration names is [`CaptureConv::Text`]: it cannot occur
    /// in a program that compiles (an unbound capture is
    /// `code::E_ROUTE_CAPTURE_UNBOUND`), and text is what the segment already
    /// is.
    fn convert(&self, name: &str, text: &str) -> Option<Param> {
        let conv = self
            .captures
            .iter()
            .find(|capture| capture.name == name)
            .map_or(&CaptureConv::Text, |capture| &capture.conv);
        match conv {
            CaptureConv::Text | CaptureConv::Unconverted => Some(Param::Text(text.to_owned())),
            // The segment and the class that claims it, neither of which this
            // walk may act on and both of which the crossing needs.
            CaptureConv::Parses(class) => Some(Param::Parses {
                class: class.clone(),
                text: text.to_owned(),
            }),
            CaptureConv::Int => text.parse::<i64>().ok().map(Param::Int),
            CaptureConv::Uint => text.parse::<u64>().ok().map(Param::Uint),
            CaptureConv::Decimal => Decimal::parse(text).map(Param::Decimal),
            CaptureConv::Uuid => crate::uuid::read(text).map(Param::Uuid),
            CaptureConv::OneOf(admitted) => admitted
                .iter()
                .any(|value| value == text)
                .then(|| Param::Text(text.to_owned())),
            // Compared byte for byte and case-sensitively under either
            // spelling, as `Seg::Fixed` is: a case name is a name
            // (`rule:classes/names-resolve-case-sensitively`), and a backing
            // value has one decimal spelling and no other.
            CaptureConv::Enum { cases, .. } => cases
                .iter()
                .find(|(spelling, _)| spelling == text)
                .map(|(_, value)| match value {
                    crate::commands::CaseValue::Int(number) => Param::Int(*number),
                    crate::commands::CaseValue::Uint(number) => Param::Uint(*number),
                }),
        }
    }
}

/// § 2's grammar over one path, as the matcher needs it.
///
/// The leading empty piece `"/users"` splits into is kept rather than skipped,
/// so that a request path — which also begins at the root — is compared piece
/// for piece with no offset to remember.
fn segments_of(path: &str) -> Vec<Seg> {
    path.split('/')
        .map(|segment| {
            let Some(inner) = segment
                .strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
            else {
                return Seg::Fixed(segment.to_owned());
            };
            if let Some(name) = inner.strip_suffix("...") {
                Seg::Rest(name.to_owned())
            } else if let Some(name) = inner.strip_suffix('?') {
                Seg::Optional(name.to_owned())
            } else if inner.is_empty() {
                Seg::Fixed(segment.to_owned())
            } else {
                Seg::One(inner.to_owned())
            }
        })
        .collect()
}

/// One node of the module doc's trie: the rows whose fixed prefix ends here,
/// and the nodes one fixed segment further on.
///
/// A node at depth `d` stands for the request's first `d` pieces. A row whose
/// segments before `d` are all [`Seg::Fixed`] has consumed exactly one piece
/// each to get here, so its segment `d` — a capture, or nothing — is compared
/// against piece `d` by [`Route::fill`] with no offset to account for.
#[derive(Clone, Debug, Default)]
struct Prefix {
    here: Vec<usize>,
    next: HashMap<String, Prefix>,
}

impl Prefix {
    /// Hangs row `index` off the node its leading fixed segments lead to.
    fn insert(&mut self, segments: &[Seg], index: usize) {
        let mut node = self;
        for segment in segments {
            let Seg::Fixed(text) = segment else {
                break;
            };
            node = node.next.entry(text.clone()).or_default();
        }
        node.here.push(index);
    }
}

/// § 1's match: the row the request selected, and the captures it filled.
///
/// It holds the row rather than a copy of its fields — one atomic bump against
/// a clone of every `String` in it — and the capture names it does copy are the
/// few a path declares. The row outliving the table is what makes the match
/// *travel*: a request carries this from the door to `Core\Request::route()`
/// with nothing left to look up, which is § 1's rule stated as an ownership.
#[derive(Clone, Debug)]
pub struct Match {
    route: Arc<Route>,
    params: Vec<(String, Param)>,
}

impl Match {
    /// The row this request matched.
    #[must_use]
    pub fn route(&self) -> &Route {
        &self.route
    }

    /// § 1's declared name, which
    /// `rule:observability/route-label-is-the-declared-name`'s `route`
    /// label reads. `None` where the route declares none.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.route.name()
    }

    /// Every capture the path filled, in path order.
    #[must_use]
    pub fn params(&self) -> &[(String, Param)] {
        &self.params
    }

    /// One capture by the parameter name it binds.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params
            .iter()
            .find(|(bound, _)| bound == name)
            .map(|(_, value)| value)
    }
}

/// Every route the program declares, in the compiler's own load order.
///
/// A `Vec` rather than a map, which is the compiler-side table's shape and its
/// reason: a path key is a *shape* rather than the written text, so nothing
/// could be looked up by one. Order is also what breaks a precedence tie —
/// [`Self::match_request`] takes the first row of the best rank. Beside it, the
/// module doc's trie over the rows' fixed prefixes, which only narrows which
/// rows that comparison visits.
#[derive(Clone, Debug, Default)]
pub struct Routes {
    rows: Vec<Arc<Route>>,
    /// The rows by their leading fixed segments, as indexes into `rows`.
    prefixes: Prefix,
    /// Whether the unit that declared these rows also **links** to one
    /// absolutely — [`Self::absolute_links`], and the one fact here that is
    /// about the calls rather than about the declarations.
    absolute_links: bool,
}

impl Routes {
    /// The table holding `rows`, in load order.
    #[must_use]
    pub fn new(rows: Vec<Route>) -> Self {
        let mut prefixes = Prefix::default();
        for (index, row) in rows.iter().enumerate() {
            prefixes.insert(&row.segments, index);
        }
        Self {
            rows: rows.into_iter().map(Arc::new).collect(),
            prefixes,
            absolute_links: false,
        }
    }

    /// Every row whose leading fixed segments are `request`'s own, as indexes
    /// in load order — the only rows [`Route::fill`] could fill.
    fn candidates(&self, request: &[&str]) -> Vec<usize> {
        let mut found: Vec<usize> = Vec::new();
        let mut node = &self.prefixes;
        let mut pieces = request.iter();
        loop {
            found.extend_from_slice(&node.here);
            let Some(next) = pieces.next().and_then(|piece| node.next.get(*piece)) else {
                break;
            };
            node = next;
        }
        found.sort_unstable();
        found
    }

    /// The same table, declared by a unit that builds an absolute link.
    ///
    /// A builder rather than an argument, because it is the one fact here a
    /// route declaration does not carry: the compiler reads it off the call
    /// sites it resolved (`nvs_types::ExprTypeTable::links_absolutely`), and it
    /// rides on the table because a link resolves against exactly this table
    /// and crosses by exactly this channel.
    #[must_use]
    pub fn linking_absolutely(mut self) -> Self {
        self.absolute_links = true;
        self
    }

    /// Every row, in load order.
    #[must_use]
    pub fn rows(&self) -> &[Arc<Route>] {
        &self.rows
    }

    /// Whether the unit that declared this table builds an absolute link over a
    /// route name given as a string literal.
    ///
    /// Read at boot and never per request: it is
    /// `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s question,
    /// and the whole point of asking it at a mount's expansion is that nothing
    /// asks it while a request is being answered.
    #[must_use]
    pub fn absolute_links(&self) -> bool {
        self.absolute_links
    }

    /// § 1's match: this method and this path against the whole table, once.
    ///
    /// `None` is "nothing matched", which is a served request like any other —
    /// nothing here dispatches and nothing here refuses. § 2's `404`/`405` is
    /// the *other* question, asked only once this one has answered `None`.
    ///
    /// **`HEAD` matches a route declared `Get`**, which is RFC 9110 § 9.3.2's
    /// own reading of the verb and is what `Core\Request::method` already
    /// answers with; a table that made them different would have the door and
    /// the program disagreeing about which handler a `HEAD` is for.
    #[must_use]
    pub fn match_request(&self, method: &str, path: &str) -> Option<Match> {
        let verb = if method.eq_ignore_ascii_case("HEAD") {
            "GET"
        } else {
            method
        };
        let request: Vec<&str> = path.split('/').collect();
        let mut best: Option<(&Vec<u8>, Match)> = None;
        for index in self.candidates(&request) {
            let row = &self.rows[index];
            if !row.verb.eq_ignore_ascii_case(verb) {
                continue;
            }
            let Some(params) = row.fill(&request) else {
                continue;
            };
            // Strictly better, so a tie is the row that was loaded first.
            if best
                .as_ref()
                .is_none_or(|(best_rank, _)| row.rank < **best_rank)
            {
                best = Some((
                    &row.rank,
                    Match {
                        route: Arc::clone(row),
                        params,
                    },
                ));
            }
        }
        best.map(|(_, matched)| matched)
    }

    /// § 2's other question: every verb this path claims, in load order.
    ///
    /// Empty is "no route claims this path at all" — the `404`. Non-empty is
    /// the `405`, and the list is the `Allow:` header RFC 9110 requires beside
    /// it. It is asked **only once [`Self::match_request`] has answered
    /// `None`**, so a served request never walks this.
    ///
    /// The same walk, with the verb filter dropped and the rank ignored: a row
    /// claims the path iff [`Route::fill`] fills it, which is what makes § 2's
    /// `{name?}` rule fall out rather than needing a case — `/posts` and
    /// `/posts/3` against `/posts/{page?}` are both fills, so the shorter form
    /// cannot `404` while the longer one `405`s. A failed conversion is not a
    /// claim, on § 1's own reading: a `uint` capture that the segment is not
    /// leaves the path unclaimed by that row, exactly as it leaves it unmatched.
    ///
    /// **`HEAD` is never synthesized into the answer**, although
    /// [`Self::match_request`] serves one from a row declared `Get`. The two
    /// agree without it: a `HEAD` against a path with a `Get` row *matched*, so
    /// this is never asked; and a path with no `Get` row does not serve `HEAD`
    /// either, so naming it in `Allow:` would advertise a method the door
    /// refuses. What the member answers is what the table declares.
    ///
    /// **What it spends:** the discarded capture `Vec` of every row whose shape
    /// fits, because it reuses `fill` rather than growing a second shape-only
    /// walk that would have to agree with it forever. That is on the `405` path
    /// only, where a response is being built regardless.
    #[must_use]
    pub fn methods_for(&self, path: &str) -> Vec<&str> {
        let request: Vec<&str> = path.split('/').collect();
        let mut verbs: Vec<&str> = Vec::new();
        for index in self.candidates(&request) {
            let row = &self.rows[index];
            if row.fill(&request).is_none() {
                continue;
            }
            // Two rows may claim one path under one verb — a fixed segment and the
            // capture it beat — and `Allow: GET, GET` is not a header.
            if !verbs
                .iter()
                .any(|verb| verb.eq_ignore_ascii_case(&row.verb))
            {
                verbs.push(&row.verb);
            }
        }
        verbs
    }
}

#[cfg(test)]
mod tests {
    use super::{Capture, CaptureConv, Decimal, Param, Routes};

    /// A table of the shapes § 2's grammar admits, in a deliberately
    /// unhelpful load order: the capture rows come before the fixed segments
    /// they have to lose to.
    fn table() -> Routes {
        Routes::new(vec![
            super::Route::new(
                "Get",
                "/users/{id}",
                Some("user.show".to_owned()),
                "App\\Users::show",
                Some("Core\\Audience::Public".to_owned()),
                vec![Capture {
                    name: "id".to_owned(),
                    conv: CaptureConv::Uint,
                }],
            ),
            super::Route::new("Get", "/users/new", None, "App\\Users::new", None, vec![]),
            super::Route::new("Post", "/users", None, "App\\Users::create", None, vec![]),
            super::Route::new(
                "Get",
                "/posts/{page?}",
                None,
                "App\\Posts::list",
                None,
                vec![Capture {
                    name: "page".to_owned(),
                    conv: CaptureConv::Uint,
                }],
            ),
            super::Route::new(
                "Get",
                "/files/{rest...}",
                None,
                "App\\Files::send",
                None,
                vec![Capture {
                    name: "rest".to_owned(),
                    conv: CaptureConv::Text,
                }],
            ),
            super::Route::new(
                "Get",
                "/{lang}/about",
                None,
                "App\\Pages::about",
                None,
                vec![Capture {
                    name: "lang".to_owned(),
                    conv: CaptureConv::OneOf(vec!["en".to_owned(), "de".to_owned()]),
                }],
            ),
        ])
    }

    /// Rows hung at different depths of the prefix trie still compete on rank
    /// and then on load order, as one comparison over every row would have it.
    #[test]
    fn the_prefix_trie_keeps_precedence_across_its_depths() {
        let text = |name: &str| Capture {
            name: name.to_owned(),
            conv: CaptureConv::Text,
        };
        let routes = Routes::new(vec![
            super::Route::new(
                "Get",
                "/{lang}/posts",
                None,
                "A::lang",
                None,
                vec![text("lang")],
            ),
            super::Route::new(
                "Get",
                "/en/{page}",
                None,
                "A::page",
                None,
                vec![text("page")],
            ),
            super::Route::new(
                "Get",
                "/en/{slug}",
                None,
                "A::slug",
                None,
                vec![text("slug")],
            ),
            super::Route::new(
                "Get",
                "/{rest...}",
                None,
                "A::rest",
                None,
                vec![text("rest")],
            ),
        ]);
        let handler = |path: &str| {
            routes
                .match_request("GET", path)
                .map(|matched| matched.route().handler().to_owned())
        };
        // `/en/{page}` has its fixed segment earlier than `/{lang}/posts`.
        assert_eq!(handler("/en/posts").as_deref(), Some("A::page"));
        // A tie in rank goes to the row loaded first.
        assert_eq!(handler("/en/hello").as_deref(), Some("A::page"));
        // A row hung off the root still matches a request no fixed row claims.
        assert_eq!(handler("/de/posts").as_deref(), Some("A::lang"));
        assert_eq!(handler("/de/x/y").as_deref(), Some("A::rest"));
        assert_eq!(routes.methods_for("/en/posts"), vec!["Get"]);
    }

    /// § 2's forms each match what they claim, and the verb selects among
    /// rows sharing a path.
    #[test]
    fn every_capture_form_matches_the_shape_its_grammar_declares() {
        let routes = table();
        let matched = routes.match_request("GET", "/users/42").expect("a match");
        assert_eq!(matched.name(), Some("user.show"));
        assert_eq!(matched.param("id"), Some(&Param::Uint(42)));

        assert_eq!(
            routes
                .match_request("GET", "/posts")
                .expect("an absent optional")
                .params(),
            &[]
        );
        assert_eq!(
            routes
                .match_request("GET", "/posts/3")
                .expect("a filled optional")
                .param("page"),
            Some(&Param::Uint(3))
        );
        assert_eq!(
            routes
                .match_request("GET", "/files/a/b/c.png")
                .expect("a catch-all")
                .param("rest"),
            Some(&Param::Text("a/b/c.png".to_owned()))
        );
        assert_eq!(
            routes
                .match_request("POST", "/users")
                .expect("the verb selects")
                .route()
                .handler(),
            "App\\Users::create"
        );
        assert!(routes.match_request("DELETE", "/users").is_none());
    }

    /// § 2's precedence, and § 1's "a failed conversion is not a match" — both
    /// asserted where the answer is a *different* row rather than nothing, so a
    /// matcher that took the first shape-wise hit fails here.
    #[test]
    fn a_fixed_segment_beats_a_capture_and_a_failed_conversion_is_a_miss() {
        let routes = table();
        assert_eq!(
            routes
                .match_request("GET", "/users/new")
                .expect("the fixed-segment row")
                .route()
                .handler(),
            "App\\Users::new"
        );
        // `/users/{id}` is `uint` and `/users/new` is spelled differently, so
        // nothing in the table claims this path.
        assert!(routes.match_request("GET", "/users/-1").is_none());
        assert!(routes.match_request("GET", "/en/about").is_some());
        // § 5's closed set: `fr` is not a value the union declares.
        assert!(routes.match_request("GET", "/fr/about").is_none());
    }

    /// A route is matched whole: neither a prefix of one nor a path with an
    /// empty segment where a capture is declared.
    #[test]
    fn a_partial_path_and_an_empty_segment_are_both_misses() {
        let routes = table();
        assert!(routes.match_request("GET", "/users").is_none());
        assert!(routes.match_request("GET", "/users/42/edit").is_none());
        assert!(routes.match_request("GET", "/users/").is_none());
        assert!(routes.match_request("GET", "//about").is_none());
    }

    /// RFC 9110 § 9.3.2, and the reason `Core\Request::method` answers `Get`
    /// for one: a `HEAD` is a `GET` that stops at the head.
    #[test]
    fn head_matches_a_route_declared_get() {
        let matched = table().match_request("HEAD", "/users/7").expect("a match");
        assert_eq!(matched.param("id"), Some(&Param::Uint(7)));
    }

    /// § 2's two answers, asked of the same table § 1's match walks: empty is
    /// the `404` and non-empty is the `405`'s `Allow:`, with both bounds named
    /// together so a member that claimed everything or nothing fails here.
    #[test]
    fn methods_for_answers_the_verbs_a_path_claims_and_nothing_where_none_does() {
        let routes = table();
        // The `405`: the path is claimed, just not under the verb asked for.
        assert!(routes.match_request("GET", "/users").is_none());
        assert_eq!(routes.methods_for("/users"), vec!["Post"]);
        // The `404`: nothing in the table claims this path at all…
        assert!(routes.methods_for("/nothing/here").is_empty());
        // …including the path whose *shape* fits and whose conversion does
        // not, which § 1 counts as a miss and so does this.
        assert!(routes.methods_for("/users/-1").is_empty());
        // No `HEAD` is invented beside the `Get` it would be served from.
        assert_eq!(routes.methods_for("/users/42"), vec!["Get"]);
    }

    /// § 2's `{name?}` rule, stated as the agreement it exists to force: the
    /// two forms of a terminal node answer the same verbs, so the shorter one
    /// cannot `404` while the longer one `405`s.
    #[test]
    fn both_forms_of_an_optional_capture_claim_the_same_verbs() {
        let routes = table();
        assert_eq!(routes.methods_for("/posts"), vec!["Get"]);
        assert_eq!(routes.methods_for("/posts"), routes.methods_for("/posts/3"));
    }

    /// The answer is the *verbs*, once each and in load order — not one entry
    /// per row, which the fixed-segment-beats-a-capture table would otherwise make
    /// into `Allow: GET, GET`.
    #[test]
    fn a_paths_verbs_are_reported_once_each_in_load_order() {
        let text = |name: &str| {
            vec![Capture {
                name: name.to_owned(),
                conv: CaptureConv::Text,
            }]
        };
        let routes = Routes::new(vec![
            super::Route::new(
                "Delete",
                "/users/{id}",
                None,
                "App\\Users::destroy",
                None,
                text("id"),
            ),
            super::Route::new(
                "Get",
                "/users/{id}",
                None,
                "App\\Users::show",
                None,
                text("id"),
            ),
            super::Route::new("Get", "/users/me", None, "App\\Users::me", None, vec![]),
        ]);
        assert_eq!(routes.methods_for("/users/me"), vec!["Delete", "Get"]);
    }

    /// The class-typed capture's half of § 5, on the side that runs at the
    /// door: the segment matches on **shape**, and the class name travels
    /// beside its text rather than this walk reaching a `parse`.
    ///
    /// Both halves named together, because a matcher that converted here would
    /// pass the first assertion and fail the second — and converting here is
    /// what `rule:security/route-capture-is-laundered-by-its-type` forbids,
    /// since a match runs at the door ahead of everything that rate-limits the
    /// request that reached it. `nvs_stdlib::router`'s `capture_value` is the
    /// binding site that converts instead.
    #[test]
    fn a_parses_capture_matches_on_shape_and_crosses_as_the_class_and_its_text() {
        let routes = Routes::new(vec![super::Route::new(
            "Get",
            "/deploy/{target}",
            None,
            "App\\Deploys::show",
            None,
            vec![Capture {
                name: "target".to_owned(),
                conv: CaptureConv::Parses("App\\Slug".to_owned()),
            }],
        )]);
        let matched = |path: &str| routes.match_request("GET", path);
        let crossed = |text: &str| {
            Some(Param::Parses {
                class: "App\\Slug".to_owned(),
                text: text.to_owned(),
            })
        };

        assert_eq!(
            matched("/deploy/prod").expect("a match").param("target"),
            crossed("prod").as_ref()
        );

        // A segment no slug could be matches too: this walk holds no class
        // table to ask, so shape is the whole of what it decides. The segment
        // also crosses undecoded, which is the second thing the binding site
        // owns — a converted capture is read out of the raw segment here.
        assert_eq!(
            matched("/deploy/%20!!").expect("a match").param("target"),
            crossed("%20!!").as_ref(),
            "the segment crosses as it arrived, converted by nothing"
        );

        // Shape is what it narrows on, and it still narrows: one capture is
        // one segment, and a path with another segment on it is a different
        // route rather than a longer value.
        assert!(matched("/deploy/prod/eu").is_none());
        assert!(matched("/deploy").is_none());
    }

    /// § 5's narrowing, for the type whose reader this crate owns: a `decimal`
    /// capture converts, and a segment that is not one is **no match** rather
    /// than text handed to a handler that declared a number.
    ///
    /// Both sides named together, because a conversion that refused everything
    /// would pass either half alone. The refused half is the point: a matcher
    /// that handed every segment over as text passes the first assertion and
    /// fails the rest.
    #[test]
    fn a_decimal_capture_converts_and_refuses_what_is_not_one() {
        let routes = Routes::new(vec![super::Route::new(
            "Get",
            "/orders/{total}",
            None,
            "App\\Orders::show",
            None,
            vec![Capture {
                name: "total".to_owned(),
                conv: CaptureConv::Decimal,
            }],
        )]);
        let matched = |path: &str| routes.match_request("GET", path);

        // The value, not the text: `19.90` keeps its scale, and an integral
        // segment is a decimal too.
        assert_eq!(
            matched("/orders/19.90").expect("a match").param("total"),
            Some(&Param::Decimal(Decimal::parse("19.90").expect("a decimal")))
        );
        assert_eq!(
            matched("/orders/-7").expect("a match").param("total"),
            Some(&Param::Decimal(Decimal::parse("-7").expect("a decimal")))
        );

        // The grammar admitted is `rule:types/conversion`'s whole literal and not a
        // narrower one this arm picked: an exponent is a decimal literal, so a
        // segment written that way matches. Asserted because delegating is the
        // rule here — a hand-written `[0-9.]` check would pass every other line
        // of this test and fail this one.
        assert!(matched("/orders/1e3").is_some());

        // Refused, and the row is the only one in the table, so a refusal is a
        // miss: the segment is not a decimal literal, is one with something
        // stuck to it, or is empty. `parse` decides all of them — this arm
        // never spells the grammar out a second time.
        for segment in ["abc", "19.90usd", "", " 1"] {
            assert!(
                matched(&format!("/orders/{segment}")).is_none(),
                "`{segment}` is not a decimal literal and must not match"
            );
        }
    }

    /// § 5's narrowing for the type whose class this crate does *not* own: a
    /// `Core\Uuid` capture converts to the sixteen octets, and a segment that is
    /// not RFC 9562 § 4's canonical form is **no match** rather than text handed
    /// to a handler that declared an identifier.
    ///
    /// Both sides named together, for the `decimal` case's reason. The refused
    /// half is what the placement buys: [`crate::uuid`] puts the parse on this
    /// side of the walk, so a segment that is not one is a miss here rather
    /// than text handed to the crate above.
    #[test]
    fn a_uuid_capture_converts_and_refuses_what_is_not_one() {
        let routes = Routes::new(vec![super::Route::new(
            "Get",
            "/tenants/{tenant}",
            None,
            "App\\Tenants::show",
            None,
            vec![Capture {
                name: "tenant".to_owned(),
                conv: CaptureConv::Uuid,
            }],
        )]);
        let matched = |path: &str| routes.match_request("GET", path);

        // The octets, not the text: what crosses is what
        // `nvs_stdlib::uuid::of_octets` takes, so the program is handed a value
        // with a type rather than 36 characters it would read a second time.
        assert_eq!(
            matched("/tenants/0fb0bc5c-4e24-4e4e-8e1e-6a6b8a0e1b2c")
                .expect("a match")
                .param("tenant"),
            Some(&Param::Uuid([
                0x0f, 0xb0, 0xbc, 0x5c, 0x4e, 0x24, 0x4e, 0x4e, 0x8e, 0x1e, 0x6a, 0x6b, 0x8a, 0x0e,
                0x1b, 0x2c
            ]))
        );

        // Refused, and the row is the only one in the table, so a refusal is a
        // miss. The non-canonical spellings of a *valid* UUID are named among
        // them because a route that accepted those would key on two strings
        // that are one identity, which is the whole of why `Core\Uuid`'s reader
        // is strict.
        for segment in [
            "not-a-uuid",
            "0fb0bc5c4e244e4e8e1e6a6b8a0e1b2c",
            "urn:uuid:0fb0bc5c-4e24-4e4e-8e1e-6a6b8a0e1b2c",
            "",
        ] {
            assert!(
                matched(&format!("/tenants/{segment}")).is_none(),
                "`{segment}` is not a canonical UUID and must not match"
            );
        }
    }

    /// § 2's two answers as the *decision* taken over them: a miss is the `404`
    /// exactly where nothing claims the path, and the `405` otherwise, with the
    /// answer spelling the `Allow:` RFC 9110 requires beside it.
    ///
    /// **Filed here rather than in `nvs-server`.** § 1 forbids the door to send
    /// either status — "the program may still serve the request however it
    /// likes, because nothing here dispatches" — and a door that refused a miss
    /// would refuse every request of a program declaring no `#[Route]` at all,
    /// since an empty table claims no path. So both answers are a computation
    /// *this* table performs and the program sends, and this is the only crate
    /// where the claim can be asserted against something that exists.
    #[test]
    fn no_methods_for_a_path_is_404_and_some_is_405_with_allow() {
        let routes = table();
        // What a sender does with the answer, written out once here because
        // nothing in this crate does it: the status, and the field value that
        // rides beside a `405`. The `expect` is § 2's calling rule — the walk
        // is asked *only* once the match has answered `None`.
        let answer = |verb: &str, path: &str| {
            assert!(
                routes.match_request(verb, path).is_none(),
                "{verb} {path} matched; § 2 is asked only after a miss"
            );
            let verbs = routes.methods_for(path);
            if verbs.is_empty() {
                (404, String::new())
            } else {
                (405, verbs.join(", "))
            }
        };

        // The `405`: the path is claimed, under verbs the header now names.
        assert_eq!(answer("DELETE", "/users/42"), (405, "Get".to_owned()));
        assert_eq!(answer("GET", "/users"), (405, "Post".to_owned()));
        assert_eq!(answer("PUT", "/files/a/b.png"), (405, "Get".to_owned()));
        assert_eq!(answer("POST", "/en/about"), (405, "Get".to_owned()));

        // The `404`, and each way a path goes unclaimed: no row's shape
        // fits it, or a shape fits under a conversion the segment fails —
        // whether that conversion is a number or a closed set.
        assert_eq!(answer("GET", "/nothing/here"), (404, String::new()));
        assert_eq!(answer("GET", "/users/-1"), (404, String::new()));
        assert_eq!(answer("GET", "/fr/about"), (404, String::new()));

        // The other side of the bound: a `405` is a *refused* verb rather than
        // an unknown path, so each of those paths is served under the verb it
        // declares. A member answering `405` for everything passes the block
        // above and fails here.
        assert!(routes.match_request("GET", "/users/42").is_some());
        assert!(routes.match_request("POST", "/users").is_some());
        assert!(routes.match_request("GET", "/files/a/b.png").is_some());
        assert!(routes.match_request("GET", "/en/about").is_some());
    }
}
