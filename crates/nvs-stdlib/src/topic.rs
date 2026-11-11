//! `Core\Topic` — `rule:core-classes/topic`'s bus, as the only way two connections meet.
//!
//! `rule:concurrency/receive-is-the-one-wait` gives a connection one wait over
//! two sources; this is the second one. A connection joins a topic by name, a
//! publish encodes a value once for everyone who joined it, and each
//! subscriber decodes it into a value of its own, which arrives as the
//! `Core\Socket\Message` whose `topic` is non-`null` — so a program that
//! already writes `rule:concurrency/a-connection-is-a-loop`'s loop needs no
//! second control-flow style to receive from the bus.
//!
//! # What is here, and what is not
//!
//! The class's three rows, the table behind them, and both halves of the crossing
//! between cores: the publish that hands its encoded value to every other core
//! listening, and the drain that queues it for the subscribers where it
//! landed. The transport underneath is [`crate::bus`] — this module is what
//! decides what crosses and what a subscriber is handed.
//!
//! What is **not** here is the wake, and this module is what makes it
//! reachable. The queue carries one — [`nvs_runtime::Inbox`]'s, fired by the
//! fan-out that fills it — and both doors' `receive()` registers against it,
//! so a publish on this core reaches a connection that is already waiting. A
//! delivery from another core is drained at the same `receive()` the local
//! queue is read at. A socket's wait is also registered with [`crate::bus`],
//! whose hand-off ends it; an event stream's looks again each
//! `CROSS_CORE_TICK` (`crate::sse`).
//!
//! # Decision: the table is per core, and it holds a weak reference
//!
//! One `thread_local!` map from a topic's name to the subscribers on **this**
//! core, as a representation rather than a rule: the runtime is
//! thread-per-core, so a table another core could reach would need a lock. The
//! rule's "across every core" is the *publish*'s promise, and it is kept by a
//! hand-off between cores rather than by one shared map — a bounded message
//! hand-off is what the rule says the bus is, and a shared map with a lock in
//! front of it is the shared state it says the bus is not.
//!
//! What the table holds per subscriber is a [`Weak`] onto that connection's
//! [`nvs_runtime::Inbox`], whose only strong reference is the connection's own
//! context. **Nothing unsubscribes when a connection ends**, and nothing has
//! to: the isolate's `Ctx` drops, the last strong reference goes with it, and
//! the entry is dead the next time that topic is walked. That is the property
//! worth the indirection, because a connection that ended at a `[limits]`
//! ceiling or a fatal error (`rule:concurrency/a-connection-is-a-root-isolate`)
//! runs no more of its own code and could not
//! have unsubscribed itself. The alternative — an owned handle plus a
//! teardown hook — is a table that is O(connections served) whenever the hook
//! does not run, which [AGENTS.md](/AGENTS.md)'s memory rule calls a leak
//! rather than a trade-off.
//!
//! **What it spends:** one map entry per topic that has a live subscriber on
//! this core, and one [`Weak`] — two words — per subscription. Both are
//! O(live connections × topics each joined), and the next decision caps the
//! second factor; neither is O(publishes) or O(connections served). A topic
//! whose last subscriber went away keeps its entry until the next walk of that
//! name or the next sweep, whichever comes first, and an empty one is removed
//! rather than left behind.
//!
//! # Decision: the table has a ceiling per connection, and a sweep for the rows nobody walks
//!
//! The table's bytes are the process's (the next decision), so a connection's
//! own `[limits] memory` does not bound what it puts here. Two fixed numbers
//! do: a connection may be in at most [`nvs_runtime::TOPIC_CAP`] rows at once,
//! and a name is at most [`NAME_CAP`] bytes. Past either, the member throws a
//! `LogicError` the script can catch, and the table is unchanged. **What it
//! spends:** at most `TOPIC_CAP × NAME_CAP` bytes of names, plus a row each,
//! per live connection, which is a quarter of a megabyte and O(live
//! connections). They are numbers rather than settings because nothing an
//! application does on purpose comes near either, and a setting is one more
//! thing an operator can raise past what the table should hold.
//!
//! A ceiling on *live* connections alone is not one on the table: a connection
//! that joins a name nobody else uses and then ends leaves a row of dead
//! handles that no walk of that name will ever prune. So an insert that finds
//! the table at twice the size the last sweep left it walks every row, drops
//! the dead handles and removes the rows left empty. That is O(rows) once per
//! as many inserts, so O(1) amortized per `subscribe`, and it keeps the table
//! within twice its live rows, or [`SWEEP_FLOOR`] rows, whichever is larger.
//!
//! # Decision: the table's own bytes are the process's, and the inbox behind a `Weak` is not
//!
//! A row outlives the request that made it — one connection subscribes, and
//! whichever connection next walks that name is the one that prunes it — so the
//! name and the row it keys are the process's bytes and neither request's:
//! `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`. Without
//! that, a connection subscribing to ten thousand long names is charged for a
//! table it does not own, and the later one that prunes them is credited for
//! memory it never held and runs that much further past its ceiling. Every path
//! that allocates or frees what the table itself holds — the [`Box<str>`](Box)
//! an insert makes, the row it grows, and the removal of a row already emptied
//! — therefore runs inside [`nvs_runtime::budget::Detached`].
//!
//! **The prune is deliberately outside it.** Dropping the last [`Weak`] onto a
//! dead connection's [`Inbox`] frees the allocation behind it, which the
//! *subscribing* request made on its own balance; handing that back to the
//! process's would turn a bounded credit into an unbounded one, which is the
//! case that rule leaves unbracketed. So the two are split where they meet: a
//! `retain` runs outside the bracket, and the `remove` that may follow it runs
//! inside one, by which point the row holds no [`Weak`] left to free. What
//! stays unattributed is that header — two words and a refcount pair per dead
//! subscription, O(live connections) — and charging its release to the request
//! that allocated it needs per-request provenance on the allocation, and a
//! request has no arena of its own to give it.
//!
//! # Decision: a publisher needs no connection, and is not excluded from its own topic
//!
//! `subscribe` and `unsubscribe` refuse a program that is not a connection,
//! because a subscription with no queue behind it is an entry no `receive()`
//! could ever drain. `publish` asks nothing about its host: it hands a value
//! to whoever joined, and the commonest reason to have a bus at all is an
//! ordinary HTTP request telling the connections that something changed. A
//! chat loop publishes from inside a connection, but nothing in
//! `rule:core-classes/topic` makes that a requirement, and a refusal here
//! would make an application hold a connection open for the sole purpose of
//! being allowed to speak.
//!
//! A connection that joined a topic it also publishes to **is** delivered to,
//! like every other subscriber. The table is keyed by name and holds
//! connections rather than everybody-but-one, so an exclusion would have to be
//! computed per publish, and it would make the count answer something other
//! than "subscribers". A chat loop relies on exactly that: the sender sees
//! their own message, which is what every chat does.
//!
//! # Decision: a topic carries bytes on every path, and each subscriber decodes them
//!
//! `publish` encodes the value once with `rule:classes/graph-copy`'s bytes
//! carrier, [`nvs_runtime::encode`], before the walk and whether or not anyone
//! joined. That one encoding is the refusal a publish makes: a resource, a
//! class the compiler made, or a `secret`, which
//! `rule:security/secret-qualifier` says may never be published. A refusal
//! that only happened once somebody had joined would hold or not by timing,
//! and a `.nvst` case, which has no connection to subscribe with, could never
//! reach it.
//!
//! One shared buffer goes into each local subscriber's queue and into the
//! hand-off to other cores ([`crate::bus`]). Each subscriber's own `receive()`
//! decodes it with its own resolver ([`decoded`]), so an object arrives as the
//! class of that name in the **subscriber's** program, checked as
//! `Core\Serialize::decode` checks it. Each decode makes a value that shares
//! nothing with the publisher or with any other subscriber, which is
//! `rule:core-classes/topic`. A live copy per subscriber could not do this:
//! the live carrier keeps the publisher's descriptor, so a program's class
//! would reach a subscriber on another core and never one on the same core.
//!
//! **A subscriber that cannot decode a message gets an error it can catch.**
//! The publisher has returned by then, so the subscriber's `receive()` throws
//! a `ParseError` naming the topic and the class, the error
//! `Core\Cache\Store::get` throws for an entry it cannot decode. The message
//! is off the queue, so the next `receive()` reads the next one, and no other
//! subscriber is affected. After a deploy, a connection that kept its old
//! compiled unit ends on the first message whose class changed, unless it
//! catches, and the client reconnects into the new unit.
//!
//! **What it spends:** one encode per publish, however many joined. Each
//! subscriber pays one decode on its own turn, in its own
//! CPU time and memory balance. The buffer is the process's bytes
//! (`nvs_runtime::Delivery`), held once until the last queue drops it. Each
//! queue counts its full length against its subscriber's limit, so a shared
//! buffer is counted once per queue. That over-counts, which is the safe
//! direction.
//!
//! # Decision: the count is what was queued
//!
//! **The count is what was queued, on either side.** Locally that is the live
//! subscribers the walk found; elsewhere it is the number that core last
//! reported for the topic, which it refreshes whenever it joins, leaves or
//! walks that row. A remote connection that ended without unsubscribing is
//! therefore counted until its own core next looks at that name — an
//! over-count of the same kind `publish` already carries locally, where a
//! subscriber counted at the queue may never live to read it. Making it exact
//! would mean waiting for the other core to answer before `publish` returns,
//! which is the publisher blocking on a subscriber, and that is the one thing
//! `rule:core-classes/topic` says may not happen.
//!
//! The hand-off to other cores is asked for only when another core reported a
//! subscriber, so a single-core server pays one read lock for it.
//! [`crate::bus`] owns the rest of that accounting.
//!
//! # Decision: a subscriber that has stopped reading is skipped, not waited for
//!
//! `rule:core-classes/topic`'s slow-subscriber guarantee, in the fan-out: the
//! walk asks each queue whether it has room *before* it queues the buffer,
//! and a full one is marked and stepped over. So a connection that stopped
//! reading costs a publish one comparison — not an allocation, not a wait,
//! and not the other nine thousand nine hundred and ninety-nine subscribers'
//! latency.
//!
//! **It is not counted, either.** `publish` answers what it queued, and a
//! delivery a full queue declined was never queued for anybody. That keeps the
//! count's meaning the one the card states rather than making it two numbers a
//! caller has to subtract.
//!
//! Where the bound is, what it is, and why the *subscriber* is what performs
//! the close all live in [`nvs_runtime::Inbox`]'s module — this module raises
//! the overflow and `crate::socket`'s `receive()` obeys it, because the peer is
//! a field of that connection's own context and no publisher can reach it.
//!
//! # Decision: the name is checked before the connection is
//!
//! All three members refuse an empty name, and the two that ask about the host
//! do it **before** asking whether this context is a connection's. An empty
//! name is wrong wherever it is written — no publisher can mean it and no
//! subscriber can be reached by it — so it is the call that is wrong rather
//! than the host, and reporting the host's shape first would tell a program
//! running outside a connection the less useful of the two things wrong with
//! its call. It also makes the boundary reachable from a `.nvst` case, which a
//! check behind the connection refusal would not be.
//!
//! `tainted` is refused by the *signature* and not by a body: the name
//! parameter is a [`CoreTy::Text`] at [`Qual::Sink`], so a name built from
//! unchecked input does not compile — the rule
//! `rule:security/metric-label-refuses-tainted` already applies to a
//! metric label, and for the same reason. A name derived from user input is
//! how one tenant subscribes to another's stream.
//!
//! There is no [`crate::registry::CAPABILITIES`] row. The bus reaches no
//! operating-system facility at all — it is a map in this process — and the
//! grant that decides whether a program may be a connection in the first place
//! was asked at the upgrade ([`crate::socket`]).

use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use nvs_runtime::{Ctx, Delivery, Fault, Inbox, ThrownClass, Value, budget};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};
use crate::socket::retained;

/// `Core\Topic`'s fully-qualified name, in one place so the row and every
/// message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Topic";

/// `Core\Topic`'s registry rows — `rule:core-classes/topic`'s three, and see
/// [`crate::registry::CLASSES`].
///
/// A namespace class with no instance members and no slots, because a
/// subscription is not a value a program holds: the subscriber is the
/// *connection*, which `rule:concurrency/a-connection-is-a-root-isolate`
/// makes the isolate, and the isolate is already the thing
/// `Core\Socket::current()` answers for. A handle here would be a second name
/// for the same connection, and the first thing it could get wrong is being
/// used from a different one.
///
/// Ranked beside `Core\Socket`, whose `receive` is the only thing a topic
/// delivery arrives through: the bus is the *second* of the two sources
/// `rule:concurrency/receive-is-the-one-wait` waits on, so it is not a
/// facility of its own but the far end of a member `Core\Socket` already
/// registers. [`crate::topic`] owns why the subscriber table is per core and
/// holds a weak reference.
// rank: 810
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "subscribe",
            names: &["topic"],
            // [`Qual::Sink`] is `rule:core-classes/topic`'s refusal of a
            // `tainted` name, and the
            // module doc owns why it is the signature's job rather than a
            // body's.
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: SUBSCRIBE_SYMBOL,
            doc: Some(&SUBSCRIBE_DOC),
        },
        CoreMethod {
            name: "publish",
            names: &["topic", "value"],
            // The name is a sink for `subscribe`'s reason. The value is not
            // one: a `tainted` payload crosses and stays `tainted` where it
            // arrives (`rule:security/tainted-qualifier`), and what may not cross at all is refused by
            // the encoding rather than by the signature.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: PUBLISH_SYMBOL,
            doc: Some(&PUBLISH_DOC),
        },
        CoreMethod {
            name: "unsubscribe",
            names: &["topic"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: UNSUBSCRIBE_SYMBOL,
            doc: Some(&UNSUBSCRIBE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The symbol [`CLASS`]'s `subscribe` row is reached through.
const SUBSCRIBE_SYMBOL: &str = "nvs_core_topic_subscribe";

/// The symbol [`CLASS`]'s `publish` row is reached through.
const PUBLISH_SYMBOL: &str = "nvs_core_topic_publish";

/// The symbol [`CLASS`]'s `unsubscribe` row is reached through.
const UNSUBSCRIBE_SYMBOL: &str = "nvs_core_topic_unsubscribe";

/// `Core\Topic`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Sends values between WebSocket connections and event streams. A connection joins a \
            topic by name with `subscribe`. Any script can send a value to everyone who joined \
            with `publish`, and the value arrives at their next `receive()`.",
};

/// `Core\Topic::subscribe`'s reference card — `rule:core-api/reference-card`.
const SUBSCRIBE_DOC: MethodDoc = MethodDoc {
    short: "Joins this connection to `$topic`. A value published to that topic then arrives at \
            the next `receive()`, and the message's `topic()` returns the name.",
    params: &[ParamDoc {
        name: "topic",
        desc: "The name of the topic. It may not be `tainted` (come from user input), so you \
               build it from values your program checked. Otherwise the call does not compile. \
               This stops one user from joining the topic of another user.",
        shape: &[],
    }],
    ret: "Nothing. If a connection subscribes twice to the same topic, a published value still \
          arrives only once.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$topic` is empty or longer than 256 bytes, or this script is not a WebSocket \
               connection or an event stream. Only a script that `Core\\Socket::upgrade` or \
               `Core\\Sse::upgrade` started can subscribe. One connection can join at most 1024 \
               topics at the same time, so joining one more also throws this error.",
    }],
};

/// `Core\Topic::publish`'s reference card — `rule:core-api/reference-card`.
const PUBLISH_DOC: MethodDoc = MethodDoc {
    short: "Sends a copy of `$value` to every connection that subscribed to `$topic`, and returns \
            how many connections it was sent to.",
    params: &[
        ParamDoc {
            name: "topic",
            desc: "The name of the topic. As with `subscribe`, it may not be `tainted`, so a name \
                   built from user input does not compile.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value to send. Each subscriber gets its own copy, so a change one \
                   subscriber makes is not seen by any other. An object arrives as the class \
                   with the same name in the subscriber's program. A `tainted` value is still \
                   `tainted` when it arrives. A value that contains a `secret` cannot be \
                   published.",
            shape: &[],
        },
    ],
    ret: "The number of subscribers the value was sent to. It is `0` for a topic nobody joined. \
          Any script can publish, so an ordinary web request can tell the connections that \
          something changed. A connection that publishes to a topic it joined also receives \
          the value. A subscriber with too many unread messages is skipped and not counted. It \
          is being closed, and `publish` never waits for it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$topic` is empty or longer than 256 bytes, or `$value` cannot be sent to \
               another connection. Examples are an object with a `secret` property, a \
               function, a generator and an open file. The error is thrown even when nobody \
               joined the topic.",
    }],
};

/// `Core\Topic::unsubscribe`'s reference card — `rule:core-api/reference-card`.
const UNSUBSCRIBE_DOC: MethodDoc = MethodDoc {
    short: "Removes this connection from `$topic`. A value published to that topic after this \
            call does not reach this connection.",
    params: &[ParamDoc {
        name: "topic",
        desc: "The name of the topic. As with `subscribe`, it may not be `tainted`.",
        shape: &[],
    }],
    ret: "Nothing. Leaving a topic this connection never joined is not an error.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$topic` is empty or longer than 256 bytes, or this script is not a WebSocket \
               connection or an event stream. `subscribe` throws the same errors.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See `crate::address`.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        SUBSCRIBE_SYMBOL => (nvs_core_topic_subscribe as *const ()).cast(),
        PUBLISH_SYMBOL => (nvs_core_topic_publish as *const ()).cast(),
        UNSUBSCRIBE_SYMBOL => (nvs_core_topic_unsubscribe as *const ()).cast(),
        _ => return None,
    })
}

thread_local! {
    /// This core's subscriber table: each topic that has been joined here, and
    /// the queues of the connections that joined it.
    ///
    /// The module doc owns why it is per core, why the reference is weak, and
    /// what it spends.
    static SUBSCRIBERS: std::cell::RefCell<HashMap<Box<str>, Vec<Weak<Inbox>>>> =
        std::cell::RefCell::new(HashMap::new());
}

/// The longest topic name, in bytes, any of the three members takes — the
/// other half of the table's ceiling, beside [`nvs_runtime::TOPIC_CAP`]. The
/// module doc's ceiling decision owns why both are fixed numbers, and why all
/// three members check it: a name `subscribe` cannot join is one `publish`
/// could never reach anybody with.
const NAME_CAP: usize = 256;

/// The topic name an argument carries, checked.
///
/// # Errors
///
/// A `LogicError` for the empty name and for one longer than [`NAME_CAP`] —
/// the module doc owns why that is the call's fault and why it is asked first.
fn topic_of<'a>(argument: &'a Value, member: &str) -> Result<&'a str, Fault> {
    // Unreachable from source: the row declares `string`, so `E0401` refuses
    // every other spelling before this body runs.
    let name = argument.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "`Core\\Topic::{member}` expected a `string`, got tag {}",
            argument.tag_byte()
        ))
    })?;
    if name.is_empty() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("`Core\\Topic::{member}` needs a topic name and was given an empty one"),
        ));
    }
    if name.len() > NAME_CAP {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "`Core\\Topic::{member}` takes a topic name of at most {NAME_CAP} bytes and was \
                 given one of {}",
                name.len()
            ),
        ));
    }
    Ok(name)
}

/// This connection's delivery queue, for the member that is about to put it in
/// the table or take it out.
///
/// **Either of `rule:concurrency/two-doors-one-isolate`'s doors**, the socket
/// and the event stream, because what a subscription needs is not a peer but a
/// wait to be drained on — and a connection isolate has one whichever hand-over
/// opened it. The predicates are two because the facts are two: a peer is a
/// descriptor this isolate owns, and an event stream's door is what it was told
/// its body means.
///
/// # Errors
///
/// A `LogicError` on a context that is not a connection's, in
/// `crate::socket`'s wording for the same fact: a topic is how two
/// *connections* meet, so a subscription made by anything else would be an
/// entry in the table that no `receive()` could ever drain. A request streaming
/// its own events is refused by that same reading — it answers now, and its
/// response is over before a publish could reach it.
fn connection_inbox(ctx: &mut Ctx, member: &str) -> Result<Rc<Inbox>, Fault> {
    if !ctx.has_peer() && !ctx.has_event_stream_connection() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "`Core\\Topic::{member}` needs a connection and this program is not one: only a \
                 script `Core\\Socket::upgrade` or `Core\\Sse::upgrade` opened has one"
            ),
        ));
    }
    Ok(ctx.inbox())
}

/// Puts `inbox` in `topic`'s row, once however often it is asked.
///
/// The walk prunes every subscriber whose connection has ended, which is the
/// whole of how a subscription is undone by an isolate that never got to
/// unsubscribe itself — see this module's docs.
///
/// What the row then holds is reported to [`crate::bus`], because a publisher
/// on another core counts and reaches this topic through that number and
/// through nothing else.
///
/// # Errors
///
/// A `LogicError` when this would be the connection's topic past
/// [`nvs_runtime::TOPIC_CAP`]. A topic it already joined is never refused.
fn join(topic: &str, inbox: &Rc<Inbox>) -> Result<(), Fault> {
    let live = SUBSCRIBERS.with_borrow_mut(|table| {
        let Some(row) = table.get_mut(topic) else {
            if !inbox.may_join() {
                return Err(too_many_topics());
            }
            if table.len() >= NEXT_SWEEP.get() {
                sweep(table);
                NEXT_SWEEP.set(table.len().saturating_mul(2).max(SWEEP_FLOOR));
            }
            let _bracket = budget::Detached::begin();
            table.insert(Box::from(topic), vec![Rc::downgrade(inbox)]);
            inbox.joined();
            return Ok(1);
        };
        // Outside the bracket on purpose: what this drops is the last handle
        // onto a dead connection's inbox, and that allocation is its own
        // request's. The module doc's accounting decision owns the split.
        row.retain(|held| held.strong_count() > 0);
        if !row
            .iter()
            .any(|held| held.upgrade().is_some_and(|live| Rc::ptr_eq(&live, inbox)))
        {
            if !inbox.may_join() {
                return Err(too_many_topics());
            }
            let _bracket = budget::Detached::begin();
            row.push(Rc::downgrade(inbox));
            inbox.joined();
        }
        Ok(row.len())
    })?;
    crate::bus::note_subscribers(topic, live);
    Ok(())
}

/// `subscribe`'s refusal past [`nvs_runtime::TOPIC_CAP`].
fn too_many_topics() -> Fault {
    // no case can reach this: a `.nvst` case is not a connection, so its
    // `subscribe` throws before the table is reached. The `#[test]`
    // `a_connection_joins_at_most_its_cap_of_topics` asserts it instead.
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "`Core\\Topic::subscribe` cannot join more than {} topics on one connection: leave \
             one with `Core\\Topic::unsubscribe` first",
            nvs_runtime::TOPIC_CAP
        ),
    )
}

/// The table size below which a new row never sweeps — small enough that a
/// sweep is cheap, large enough that a server with a few topics never runs one.
const SWEEP_FLOOR: usize = 64;

thread_local! {
    /// How many rows the table may reach before the next insert sweeps it:
    /// twice what the last sweep left, and never under [`SWEEP_FLOOR`].
    static NEXT_SWEEP: std::cell::Cell<usize> = const { std::cell::Cell::new(SWEEP_FLOOR) };
}

/// Drops every subscriber whose connection has ended, in every row, and every
/// row that is left with nobody — the module doc's ceiling decision, for the
/// rows nobody walks again.
///
/// Split the way [`subscribers_of`] splits it, and for the same reason: the
/// dead handles go back outside the bracket and only the names and rows are
/// freed inside one.
fn sweep(table: &mut HashMap<Box<str>, Vec<Weak<Inbox>>>) {
    for (name, row) in table.iter_mut() {
        let before = row.len();
        row.retain(|held| held.strong_count() > 0);
        if row.len() != before {
            crate::bus::note_subscribers(name, row.len());
        }
    }
    let _bracket = budget::Detached::begin();
    table.retain(|_, row| !row.is_empty());
}

/// Takes `inbox` out of `topic`'s row, and the row out of the table once it
/// holds nobody.
///
/// A name that is not in the table at all is the state the caller asked for,
/// so there is nothing here to report.
fn leave(topic: &str, inbox: &Rc<Inbox>) {
    let live = SUBSCRIBERS.with_borrow_mut(|table| {
        let Some(row) = table.get_mut(topic) else {
            return 0;
        };
        let mut found = false;
        row.retain(|held| match held.upgrade() {
            Some(live) if Rc::ptr_eq(&live, inbox) => {
                found = true;
                false
            }
            Some(_) => true,
            None => false,
        });
        if found {
            inbox.left();
        }
        let live = row.len();
        if row.is_empty() {
            // The row holds nothing by now, so what the removal frees is the
            // name and the row itself — the table's own bytes, given back to
            // the balance they were taken from.
            let _bracket = budget::Detached::begin();
            table.remove(topic);
        }
        live
    });
    crate::bus::note_subscribers(topic, live);
}

/// The live subscribers `topic` has on this core, with the row left holding
/// only them.
///
/// Handed back as owned handles rather than walked in place: the fan-out makes
/// a delivery per subscriber, and a table borrow held across an allocation
/// is a re-entrancy nobody needs — `subscribe` runs on a connection's own task
/// and reaches the same map.
fn subscribers_of(topic: &str) -> Vec<Rc<Inbox>> {
    let live = SUBSCRIBERS.with_borrow_mut(|table| {
        let Some(row) = table.get_mut(topic) else {
            return Vec::new();
        };
        let live: Vec<Rc<Inbox>> = row.iter().filter_map(Weak::upgrade).collect();
        if live.is_empty() {
            // Emptied first and removed second, so that the dead handles go
            // back to the requests that made them and only the name and the
            // row reach the bracket. The two steps are one `remove` without
            // that split.
            row.clear();
            let _bracket = budget::Detached::begin();
            table.remove(topic);
        } else {
            row.retain(|held| held.strong_count() > 0);
        }
        live
    });
    // The prune above is the only thing that lowers this core's count, so it is
    // also the freshest a publisher on another core ever sees — the module
    // doc's fourth decision is what that costs.
    crate::bus::note_subscribers(topic, live.len());
    live
}

/// `rule:classes/graph-copy`'s encoding of the value being published, made once
/// per publish. It is also the one refusal `publish` makes about the value.
///
/// **Borrows the argument**, on [`retained`]'s convention. The bytes are
/// written on the publishing request's balance and copied once into the shared
/// buffer, which is the process's (`nvs_runtime::Delivery`). The request frees
/// its own `Vec` here, outside the bracket, so each block is allocated and
/// freed on one balance.
///
/// # Errors
///
/// A `LogicError` naming what has no meaning on the other side of a copy
/// boundary: a resource, a class the compiler made, or a `secret`, which
/// `rule:security/secret-qualifier` says may never be published.
fn encoded(value: Value) -> Result<Arc<[u8]>, Fault> {
    let bytes = nvs_runtime::encode(retained(value)).map_err(|refused| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!("`Core\\Topic::publish` cannot publish this value: {refused}"),
        )
    })?;
    #[cfg(test)]
    tests::ENCODES.with(|count| count.set(count.get() + 1));
    let _bracket = budget::Detached::begin();
    Ok(Arc::from(&bytes[..]))
}

/// A delivery on `topic` carrying `value`, encoded as `publish` encodes it,
/// for a test that queues one by hand. Borrows `value`.
#[cfg(test)]
pub(crate) fn delivery_of(topic: &str, value: Value) -> Delivery {
    Delivery::new(
        topic,
        encoded(value).expect("a test delivers a value that crosses"),
    )
}

/// The value a delivery carries, decoded with the receiving program's own
/// classes — `receive()`'s half of the bus, for both doors.
///
/// An object arrives as the class of that name in the subscriber's program,
/// checked as `Core\Serialize::decode` checks it. The delivery is already off
/// the queue, so the next `receive()` reads the next message whatever this one
/// answers.
///
/// # Errors
///
/// A `ParseError` naming the topic and what the payload holds that this
/// program cannot make: a class it does not declare, or one whose properties
/// differ. `member` is the `receive` that asked.
pub(crate) fn decoded(ctx: &Ctx, delivery: &Delivery, member: &str) -> Result<Value, Fault> {
    nvs_runtime::decode(delivery.payload(), &ctx.receiving()).map_err(|why| {
        Fault::thrown_as(
            ThrownClass::Parse,
            format!(
                "`{member}` cannot read the message on topic `{}`: {why}",
                delivery.topic()
            ),
        )
    })
}

/// Moves everything other cores have published to this one into the queues of
/// the subscribers that joined here.
///
/// Called by both doors' `receive()` before it reads either source, which is
/// where a connection is about to wait anyway — [`crate::socket`] owns that
/// ordering, and this module's docs own why a delivery from another core
/// inherits the wake gap rather than adding one.
///
/// It decodes nothing. Each subscriber's queue gets the envelope's own shared
/// buffer, and each subscriber's `receive()` decodes it with its own classes
/// ([`decoded`]). So a message one subscriber cannot read still reaches the
/// others.
pub(crate) fn deliver_from_other_cores() {
    // The drain is held for the whole fan-out and freed by the bus, on the
    // balance the publishing core allocated it on — `crate::bus`'s accounting
    // decision, which is why this is not a `for` over an owned `Vec`.
    let drained = crate::bus::take_all();
    for envelope in drained.envelopes() {
        for inbox in &subscribers_of(envelope.topic()) {
            queue_into(inbox, envelope.topic(), envelope.payload());
        }
    }
}

/// Queues `payload` for one subscriber, and answers whether it was taken.
///
/// The count bound is asked first, so a subscriber already being closed costs
/// no allocation. [`Inbox::push`] asks the byte bound with the buffer's full
/// length.
fn queue_into(inbox: &Inbox, topic: &str, payload: &Arc<[u8]>) -> bool {
    if !inbox.has_room() {
        inbox.note_overflow();
        return false;
    }
    inbox.push(Delivery::new(topic, Arc::clone(payload)))
}

/// Publishes `text` on `topic` as `Core\Topic::publish($topic, $text)` does,
/// and answers how many subscribers it reached.
///
/// For a publisher with no program behind it: `nvs run --events` reads its
/// values off a file and puts them on the bus through this, so the program it
/// runs meets the same fan-out, the same encoding and the same bound as a
/// connection published to by another. A name the member would refuse reaches
/// nobody and answers `0`.
#[must_use]
pub fn publish_text(topic: &str, text: &str) -> u64 {
    let mut publisher = Ctx::buffered();
    let name = Value::text(topic);
    let payload = Value::text(text);
    let reached = nvs_runtime::call(nvs_core_topic_publish, &mut publisher, &[name, payload]);
    #[expect(
        unsafe_code,
        reason = "this frame owns the two references it built for the arguments, \
                  and the member borrows rather than takes them"
    )]
    // SAFETY: nothing else points at either value — each subscriber was queued
    // the encoded bytes, not the value.
    unsafe {
        name.release();
        payload.release();
    }
    reached.ok().and_then(|count| count.as_uint()).unwrap_or(0)
}

nvs_runtime::nvs_helper! {
    /// `Core\Topic::subscribe(string $topic): void` — `rule:core-classes/topic`'s first row.
    ///
    /// The name is checked, then the host, then the table — and that order is
    /// this module's own decision rather than an accident of writing.
    fn nvs_core_topic_subscribe(ctx, args: [1]) {
        let topic = topic_of(&args[0], "subscribe")?;
        let inbox = connection_inbox(ctx, "subscribe")?;
        join(topic, &inbox)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Topic::publish(string $topic, mixed $value): uint` — `rule:core-classes/topic`'s second row, and the only member of this class that asks nothing
    /// about the program calling it.
    ///
    /// The name, then the encoding, then the walk. The encoding stands before
    /// the walk and is made once however many joined, and the walk queues that
    /// one buffer for every subscriber. Each subscriber decodes it into a value
    /// of its own, which is this module's decision on the carrier.
    ///
    /// The count is what reached a queue, not what a subscriber has read: a
    /// delivery waits until that connection's own `receive()` drains it
    /// (`rule:concurrency/receive-is-the-one-wait`), and this member never
    /// blocks on one — on this core or on any other.
    ///
    /// **The walk of the other cores comes last.** It hands over the same
    /// buffer, and what it counts is what those cores reported, which is this
    /// module's decision on the count.
    fn nvs_core_topic_publish(ctx, args: [2]) {
        let topic = topic_of(&args[0], "publish")?;
        let payload = encoded(args[1])?;
        let mut delivered: u64 = 0;
        for inbox in &subscribers_of(topic) {
            if queue_into(inbox, topic, &payload) {
                delivered += 1;
            }
        }
        if crate::bus::subscribers_elsewhere(topic) > 0 {
            delivered += crate::bus::hand_off(topic, &payload, ctx.memory_limit());
        }
        // The last reference this request holds goes back to the balance the
        // buffer was made on.
        let _bracket = budget::Detached::begin();
        drop(payload);
        Ok(Value::uint(delivered))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Topic::unsubscribe(string $topic): void` — `rule:core-classes/topic`'s third
    /// row, and [`nvs_core_topic_subscribe`]'s exact undoing.
    ///
    /// It refuses what its twin refuses and nothing more: a name it does not
    /// find is the state the call asked for, so there is no third outcome
    /// between "left" and "was never there".
    fn nvs_core_topic_unsubscribe(ctx, args: [1]) {
        let topic = topic_of(&args[0], "unsubscribe")?;
        let inbox = connection_inbox(ctx, "unsubscribe")?;
        leave(topic, &inbox);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nvs_runtime::{
        ClassTable, Closing, Ctx, ErrorClass, EventStreamDoor, INBOX_CAP, NvsFn, NvsObj, PeerError,
        PeerFrame, PeerSocket, Value, budget,
    };

    use super::{
        nvs_core_topic_publish, nvs_core_topic_subscribe, nvs_core_topic_unsubscribe,
        subscribers_of,
    };

    thread_local! {
        /// How many times `publish` encoded a value on this thread — read by
        /// `a_publish_encodes_once_however_many_subscribed`.
        pub(super) static ENCODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    /// How many live connections on this core have joined `topic`.
    ///
    /// The fan-out's own walk, asked for its length — so a case counting
    /// subscribers and a publish reaching them cannot disagree about which
    /// entries are still alive.
    fn subscriber_count(topic: &str) -> usize {
        subscribers_of(topic).len()
    }

    /// A peer that says nothing and records the close it was told to send.
    ///
    /// What most of these cases need from a socket is only that the context
    /// *has* one, which is the whole of what separates a connection isolate
    /// from every other kind of context. The slow-subscriber case needs one
    /// thing more — the code the peer was closed with — and a handle onto the
    /// same cell is how a case reads it back after the context took the socket.
    #[derive(Clone, Debug, Default)]
    struct Watched(std::rc::Rc<std::cell::Cell<Option<Closing>>>);

    impl PeerSocket for Watched {
        fn receive(&mut self) -> Result<Option<PeerFrame>, PeerError> {
            Ok(None)
        }

        fn send(&mut self, _frame: PeerFrame) -> Result<(), PeerError> {
            Ok(())
        }

        fn close(&mut self, why: Closing) {
            self.0.set(Some(why));
        }
    }

    /// A context that is a connection's — `rule:concurrency/a-connection-is-a-root-isolate`'s isolate with a socket
    /// already moved onto it.
    fn connected() -> Ctx {
        connected_to(&Watched::default())
    }

    /// The same, over a socket the case kept a handle onto.
    fn connected_to(peer: &Watched) -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_peer(Box::new(peer.clone()));
        ctx
    }

    /// A context writing an event stream through `door` and handed no socket —
    /// `rule:concurrency/two-doors-one-isolate`'s other hand-over, which takes
    /// nothing, so the mark is the whole of what a case has to arrange.
    fn over_events(door: EventStreamDoor) -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.mark_event_stream(door);
        ctx
    }

    /// One `subscribe`/`unsubscribe` call, with the name as an argument.
    fn call(member: NvsFn, ctx: &mut Ctx, topic: &str) -> Result<Value, i32> {
        let name = Value::text(topic);
        let answered = nvs_runtime::call(member, ctx, &[name]);
        #[expect(
            unsafe_code,
            reason = "the case owns the reference it made for the argument, and \
                      the call borrows rather than takes it"
        )]
        // SAFETY: nothing else points at the name this case built.
        unsafe {
            name.release();
        }
        answered
    }

    /// The table's bytes are the process's at both ends, and no connection is
    /// measured by a row it did not allocate and did not free.
    ///
    /// `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance` asked
    /// of this module's half of the bus. The names add up to enough that a
    /// charge for them cannot hide in the noise of the calls: the process's
    /// balance takes them and the connection's does not, and the unsubscribes
    /// give them back to the same balance rather than crediting whichever
    /// connection happened to be the last one out. The module doc's accounting
    /// decision is why the prune beside it is deliberately outside the bracket.
    #[test]
    fn a_subscription_is_charged_to_the_process_and_not_to_the_connection() {
        /// How many names, each as long as a name may be: 512, or 64 under
        /// Miri, where each call is slow. Every bound below is per name.
        const NAMES: usize = if cfg!(miri) { 64 } else { 512 };
        const TOTAL: usize = NAMES * super::NAME_CAP;

        let names: Vec<String> = (0..NAMES)
            .map(|i| format!("{i:05}:{}", "n".repeat(super::NAME_CAP - 6)))
            .collect();
        let mut joining = connected();
        let live = budget::live_bytes();
        let held = budget::detached_bytes();
        for name in &names {
            call(nvs_core_topic_subscribe, &mut joining, name).expect("it joins");
        }

        assert!(
            budget::detached_bytes() - held >= TOTAL.cast_signed(),
            "the rows' names reached no balance at all, so nothing holds them to the process"
        );
        assert!(
            budget::live_bytes() - live < TOTAL.cast_signed(),
            "the connection was charged for table rows that outlive it"
        );

        let stored = budget::detached_bytes();
        for name in &names {
            call(nvs_core_topic_unsubscribe, &mut joining, name).expect("and leaves");
        }
        assert!(
            budget::detached_bytes() <= stored - TOTAL.cast_signed(),
            "the rows were freed on some balance other than the one they were allocated on, so \
             a connection that prunes a name is credited for bytes it never held"
        );
        assert_eq!(subscriber_count(&names[0]), 0, "and the rows are gone");
    }

    /// `nvs_runtime::TOPIC_CAP` topics join, the next one throws, a topic
    /// already joined still joins, and leaving one makes room for another.
    #[test]
    #[cfg_attr(miri, ignore = "joins 1,024 topics, which Miri takes minutes for")]
    fn a_connection_joins_at_most_its_cap_of_topics() {
        let mut ctx = connected();
        for i in 0..nvs_runtime::TOPIC_CAP {
            call(nvs_core_topic_subscribe, &mut ctx, &format!("cap:{i}")).expect("under the cap");
        }
        assert!(
            call(nvs_core_topic_subscribe, &mut ctx, "cap:one-more").is_err(),
            "one topic past the cap joined"
        );
        assert_eq!(
            subscriber_count("cap:one-more"),
            0,
            "and the refusal left no row"
        );
        call(nvs_core_topic_subscribe, &mut ctx, "cap:0").expect("a topic already joined");

        call(nvs_core_topic_unsubscribe, &mut ctx, "cap:0").expect("it leaves");
        call(nvs_core_topic_subscribe, &mut ctx, "cap:one-more").expect("and that makes room");

        // A second connection has a cap of its own.
        let mut other = connected();
        call(nvs_core_topic_subscribe, &mut other, "cap:1").expect("another connection joins");
        for i in 0..nvs_runtime::TOPIC_CAP {
            call(nvs_core_topic_unsubscribe, &mut ctx, &format!("cap:{i}")).expect("it leaves");
        }
        call(nvs_core_topic_unsubscribe, &mut ctx, "cap:one-more").expect("it leaves");
    }

    /// A name of `NAME_CAP` bytes is taken by all three members, and one byte
    /// more is refused by each of them.
    #[test]
    fn a_name_past_the_cap_is_refused_by_every_member() {
        let fits = "f".repeat(super::NAME_CAP);
        let long = "l".repeat(super::NAME_CAP + 1);
        let mut ctx = connected();

        call(nvs_core_topic_subscribe, &mut ctx, &fits).expect("the longest name joins");
        assert_eq!(publish(&mut ctx, &fits, "hi"), Ok(1), "and is published to");
        call(nvs_core_topic_unsubscribe, &mut ctx, &fits).expect("and is left");
        drained(&mut ctx, &fits);

        assert!(
            call(nvs_core_topic_subscribe, &mut ctx, &long).is_err(),
            "subscribe took it"
        );
        assert!(publish(&mut ctx, &long, "hi").is_err(), "publish took it");
        assert!(
            call(nvs_core_topic_unsubscribe, &mut ctx, &long).is_err(),
            "unsubscribe took it"
        );
        assert_eq!(subscriber_count(&long), 0, "and no row was made");
    }

    /// A row whose only subscriber ended, under a name nobody walks again, is
    /// removed by a later insert's sweep — so the table stays O(live
    /// connections) and not O(connections served).
    #[test]
    fn a_row_nobody_walks_again_is_swept() {
        for i in 0..(super::SWEEP_FLOOR * 4) {
            let mut gone = connected();
            call(nvs_core_topic_subscribe, &mut gone, &format!("swept:{i}")).expect("it joins");
        }
        let rows = super::SUBSCRIBERS.with_borrow(|table| {
            table
                .keys()
                .filter(|name| name.starts_with("swept:"))
                .count()
        });
        assert!(
            rows <= super::SWEEP_FLOOR * 2,
            "{rows} rows of ended connections are still in the table"
        );
    }

    /// One `publish` call, with the name and a text payload as arguments, and
    /// the count it answered.
    fn publish(ctx: &mut Ctx, topic: &str, text: &str) -> Result<u64, i32> {
        let name = Value::text(topic);
        let payload = Value::text(text);
        let answered = nvs_runtime::call(nvs_core_topic_publish, ctx, &[name, payload]);
        #[expect(
            unsafe_code,
            reason = "the case owns the two references it made for the arguments, and \
                      the call borrows rather than takes them"
        )]
        // SAFETY: nothing else points at the values this case built — the copy
        // each subscriber was queued is its own allocation.
        unsafe {
            name.release();
            payload.release();
        }
        answered.map(|count| count.as_uint().expect("`publish` answers a `uint`"))
    }

    /// Takes the one delivery `topic` should have queued on `ctx`, decodes it
    /// as `receive()` does, and releases the value.
    fn drained(ctx: &mut Ctx, topic: &str) {
        let value = read(ctx, topic).expect("the subscriber reads what it was sent");
        release(value);
        assert!(ctx.take_delivery().is_none(), "and exactly one");
    }

    /// Takes the next delivery on `ctx`, which must be on `topic`, and decodes
    /// it with `ctx`'s own classes, as both doors' `receive()` does.
    fn read(ctx: &mut Ctx, topic: &str) -> Result<Value, nvs_runtime::Fault> {
        let delivery = ctx.take_delivery().expect("this subscriber was queued one");
        assert_eq!(delivery.topic(), topic);
        super::decoded(ctx, &delivery, "receive")
    }

    /// Releases a value this frame owns one reference to.
    fn release(value: Value) {
        #[expect(unsafe_code, reason = "the caller owns the one reference it passes")]
        // SAFETY: nothing else points at a value a decode just made.
        unsafe {
            value.release();
        }
    }

    /// A class table that declares `class` with one property, `text`, anchored
    /// on its own `RuntimeError` as a compiled unit's table is, with the
    /// `ParseError` a refused message throws.
    fn declaring(class: &str) -> Arc<ClassTable> {
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = ClassTable::new();
        classes.define("RuntimeError", &SLOTS, &[]);
        classes.define("ParseError", &SLOTS, &[]);
        classes.define(class, &["text"], &[]);
        Arc::new(classes)
    }

    /// A connection whose program is `classes`.
    fn connected_with(classes: &Arc<ClassTable>) -> Ctx {
        let mut ctx = connected();
        let root = classes
            .id_of("RuntimeError")
            .expect("the table anchors on it");
        ctx.set_runtime_error_class(ErrorClass::new(Arc::clone(classes), root));
        ctx
    }

    /// An instance of `class` from `classes`, its `text` set to `text`.
    fn note(classes: &Arc<ClassTable>, class: &str, text: &str) -> Value {
        let id = classes.id_of(class).expect("the table declares the class");
        #[expect(unsafe_code, reason = "the table outlives the object")]
        // SAFETY: the case holds the table for as long as the object lives.
        let object = unsafe { NvsObj::new(classes.desc(id)) };
        object.set_field(0, Value::text(text));
        Value::object(object)
    }

    /// One `publish` of `value`, which the call borrows.
    fn publish_value(ctx: &mut Ctx, topic: &str, value: Value) -> Result<u64, i32> {
        let name = Value::text(topic);
        let answered = nvs_runtime::call(nvs_core_topic_publish, ctx, &[name, value]);
        release(name);
        answered.map(|count| count.as_uint().expect("`publish` answers a `uint`"))
    }

    /// The class descriptor of the object `value` is.
    fn class_of(value: Value) -> *const nvs_runtime::ClassDesc {
        let header = value.obj_ptr().expect("an object arrived");
        #[expect(unsafe_code, reason = "the value is a live object this frame holds")]
        // SAFETY: the header belongs to a live object the case owns.
        unsafe {
            NvsObj::class_of(header)
        }
    }

    /// `rule:core-classes/topic`: an object published by one program arrives
    /// as the class of the same name in the subscriber's program. The two
    /// tables declare one name, and the subscriber's object has the
    /// subscriber table's descriptor.
    #[test]
    fn a_published_object_arrives_as_the_subscribers_own_class() {
        let theirs = declaring("Note");
        let mine = declaring("Note");
        let mut subscriber = connected_with(&mine);
        call(nvs_core_topic_subscribe, &mut subscriber, "room:own-class").expect("it joins");

        let mut publisher = Ctx::buffered();
        let sent = note(&theirs, "Note", "hello");
        assert_eq!(publish_value(&mut publisher, "room:own-class", sent), Ok(1));
        release(sent);

        let arrived = read(&mut subscriber, "room:own-class").expect("the subscriber declares it");
        let id = mine.id_of("Note").expect("declared");
        assert!(
            std::ptr::eq(class_of(arrived), mine.desc(id)),
            "the object arrived with a descriptor other than the subscriber's own"
        );
        release(arrived);
    }

    /// A subscriber whose program does not declare the class gets a
    /// `ParseError` it can catch, and its next `receive()` reads the next
    /// message.
    #[test]
    fn a_subscriber_that_lacks_the_class_catches_an_error_and_reads_the_next_message() {
        let theirs = declaring("Note");
        let mine = declaring("Other");
        let mut subscriber = connected_with(&mine);
        call(nvs_core_topic_subscribe, &mut subscriber, "room:lacks").expect("it joins");

        let mut publisher = Ctx::buffered();
        let sent = note(&theirs, "Note", "hello");
        assert_eq!(publish_value(&mut publisher, "room:lacks", sent), Ok(1));
        release(sent);
        assert_eq!(publish(&mut publisher, "room:lacks", "next"), Ok(1));

        let conn = nvs_runtime::call(crate::socket::nvs_core_socket_current, &mut subscriber, &[])
            .expect("a connection isolate answers `current()`");
        let refused = nvs_runtime::call(
            crate::socket::nvs_core_socket_receive,
            &mut subscriber,
            &[conn],
        );
        assert!(refused.is_err(), "a class the program lacks was read");
        let thrown = subscriber.take_thrown();
        assert!(
            !thrown.is_none(),
            "the refusal is a throw the program can catch"
        );
        assert_eq!(thrown.class_name(), "ParseError");
        assert!(
            thrown.message().contains("room:lacks") && thrown.message().contains("Note"),
            "the error names the topic and the class: {}",
            thrown.message()
        );
        drop(thrown);

        let next = nvs_runtime::call(
            crate::socket::nvs_core_socket_receive,
            &mut subscriber,
            &[conn],
        )
        .expect("the next message is read");
        let value = nvs_runtime::call(
            crate::socket::nvs_core_socket_message_value,
            &mut subscriber,
            &[next],
        )
        .expect("a delivery carries its value");
        assert_eq!(value.as_text(), Some("next"));
        release(value);
        release(next);
        release(conn);
    }

    /// The same message reaches a subscriber that can read it and one that
    /// cannot, and only the second one's `receive()` throws. On this core.
    #[test]
    fn a_message_one_subscriber_cannot_read_still_reaches_the_others() {
        let theirs = declaring("Note");
        let reads = declaring("Note");
        let lacks = declaring("Other");
        let mut reader = connected_with(&reads);
        let mut stranger = connected_with(&lacks);
        call(nvs_core_topic_subscribe, &mut stranger, "room:mixed").expect("one joins");
        call(nvs_core_topic_subscribe, &mut reader, "room:mixed").expect("and another");

        let mut publisher = Ctx::buffered();
        let sent = note(&theirs, "Note", "hello");
        assert_eq!(publish_value(&mut publisher, "room:mixed", sent), Ok(2));
        release(sent);

        assert!(
            read(&mut stranger, "room:mixed").is_err(),
            "the stranger read it"
        );
        release(read(&mut reader, "room:mixed").expect("the reader reads it"));
    }

    /// The same, through the drain: the message comes from another core, and
    /// both subscribers on this one are queued it.
    #[test]
    fn a_message_one_subscriber_cannot_read_still_reaches_the_others_through_the_drain() {
        let (joined, listening) = std::sync::mpsc::channel();
        let (published, sent) = std::sync::mpsc::channel::<()>();
        let subscribers = std::thread::spawn(move || {
            let reads = declaring("Note");
            let lacks = declaring("Other");
            let mut reader = connected_with(&reads);
            let mut stranger = connected_with(&lacks);
            call(nvs_core_topic_subscribe, &mut stranger, "room:mixed-far").expect("one joins");
            call(nvs_core_topic_subscribe, &mut reader, "room:mixed-far").expect("and another");
            joined.send(()).expect("the publisher is waiting");
            sent.recv()
                .expect("the publisher says when it has published");

            super::deliver_from_other_cores();
            let stranger_read = read(&mut stranger, "room:mixed-far").is_ok();
            let reader_read = read(&mut reader, "room:mixed-far").map(release).is_ok();
            (stranger_read, reader_read)
        });

        listening.recv().expect("the subscribers have joined");
        let theirs = declaring("Note");
        let mut publisher = Ctx::buffered();
        let sent_note = note(&theirs, "Note", "hello");
        assert_eq!(
            publish_value(&mut publisher, "room:mixed-far", sent_note),
            Ok(2)
        );
        release(sent_note);
        published.send(()).expect("the subscribers are waiting");
        let (stranger_read, reader_read) = subscribers.join().expect("the subscribers finished");
        assert!(
            !stranger_read,
            "the stranger read a class it does not declare"
        );
        assert!(
            reader_read,
            "the reader missed a message the stranger could not read"
        );
    }

    /// A publish encodes the value once, and every subscriber's queue holds
    /// that one buffer: the count of encodes and the buffers queued are the
    /// same for two subscribers and for eight.
    #[test]
    fn a_publish_encodes_once_however_many_subscribed() {
        let measure = |count: usize, topic: &str| {
            let mut subscribers: Vec<Ctx> = (0..count).map(|_| connected()).collect();
            for ctx in &mut subscribers {
                call(nvs_core_topic_subscribe, ctx, topic).expect("it joins");
            }
            let before = ENCODES.with(std::cell::Cell::get);
            let mut publisher = Ctx::buffered();
            assert_eq!(
                publish(&mut publisher, topic, "one payload for everyone"),
                Ok(u64::try_from(count).expect("a small count"))
            );
            let encodes = ENCODES.with(std::cell::Cell::get) - before;
            let buffers: std::collections::HashSet<usize> = subscribers
                .iter_mut()
                .map(|ctx| {
                    let delivery = ctx.take_delivery().expect("each subscriber was queued one");
                    delivery.payload().as_ptr() as usize
                })
                .collect();
            (encodes, buffers.len())
        };
        assert_eq!(measure(2, "room:two"), (1, 1));
        assert_eq!(measure(8, "room:eight"), (1, 1));
    }

    /// A publish reaches every live subscriber on this core, answers how many,
    /// and hands each one a copy of its own — `rule:core-classes/topic`.
    ///
    /// The publisher here is not a connection, which is this module's second
    /// decision: a topic is how two connections meet, and an ordinary program
    /// is allowed to be what tells them so.
    // covers: Core\Topic::publish
    #[test]
    fn a_publish_reaches_every_subscriber_on_this_core_and_answers_how_many() {
        let mut first = connected();
        let mut second = connected();
        call(nvs_core_topic_subscribe, &mut first, "room:fanout").expect("one joins");
        call(nvs_core_topic_subscribe, &mut second, "room:fanout").expect("and another");

        let mut publisher = Ctx::buffered();
        let reached = publish(&mut publisher, "room:fanout", "hello").expect("a publish");
        assert_eq!(reached, 2);

        drained(&mut first, "room:fanout");
        drained(&mut second, "room:fanout");
    }

    /// A publish from one core reaches the subscribers on another and answers a
    /// count that includes them — `rule:core-classes/topic`'s "a publish from a connection
    /// on core 3 reaches subscribers on core 0".
    ///
    /// The two threads are what a server's two cores are, and the channels
    /// stand in for the ordering a running server gets from its own accept
    /// loop. What crosses between them is bytes: the subscriber's value is made
    /// by the `decode` its own core runs at the drain `Core\Socket::receive()`
    /// performs, which is this module's fourth decision. The last assertion is
    /// the half that would still pass if the table had quietly become shared —
    /// the publisher's core joined nothing, so a subscriber found there would
    /// be one core reading another's map.
    #[test]
    fn a_publish_reaches_a_subscriber_on_another_core() {
        let (joined, listening) = std::sync::mpsc::channel();
        let (published, sent) = std::sync::mpsc::channel::<()>();
        let subscriber = std::thread::spawn(move || {
            let mut ctx = connected();
            call(nvs_core_topic_subscribe, &mut ctx, "room:crosscore").expect("it joins");
            joined.send(()).expect("the publisher is waiting");
            sent.recv()
                .expect("the publisher says when it has published");

            super::deliver_from_other_cores();
            drained(&mut ctx, "room:crosscore");
        });

        listening.recv().expect("the subscriber has joined");
        let mut publisher = Ctx::buffered();
        let reached =
            publish(&mut publisher, "room:crosscore", "from another core").expect("a publish");
        published.send(()).expect("the subscriber is waiting");
        subscriber.join().expect("the subscriber finished");

        assert_eq!(reached, 1, "the subscriber on the other core is counted");
        assert_eq!(
            subscriber_count("room:crosscore"),
            0,
            "and it joined that core's table rather than this one's"
        );
    }

    /// Each subscriber is handed a value of its own — `rule:core-classes/topic`'s
    /// "subscribers share nothing with the publisher or with each other",
    /// asserted as the two decoded values not being one allocation, though the
    /// two queues share one encoded buffer.
    ///
    /// Read off the pointers rather than off the contents, because two
    /// subscribers holding one refcounted string compare equal on every
    /// content test there is and differ only in whether a write by one is seen
    /// by the other.
    #[test]
    fn a_published_value_is_a_copy_shared_with_nobody() {
        let mut first = connected();
        let mut second = connected();
        call(nvs_core_topic_subscribe, &mut first, "room:unshared").expect("one joins");
        call(nvs_core_topic_subscribe, &mut second, "room:unshared").expect("and another");

        let text = "a payload long enough to be an allocation rather than a few bytes in a value";
        let mut publisher = Ctx::buffered();
        assert_eq!(
            publish(&mut publisher, "room:unshared", text).expect("a publish"),
            2
        );

        let mine = read(&mut first, "room:unshared").expect("the first subscriber reads it");
        let yours = read(&mut second, "room:unshared").expect("and so does the second");
        let held = |value: &Value| {
            value
                .as_str_bytes()
                .expect("a published string arrives as one")
                .as_ptr()
        };
        assert_eq!(mine.as_str_bytes(), Some(text.as_bytes()));
        assert_eq!(yours.as_str_bytes(), Some(text.as_bytes()));
        assert!(
            !std::ptr::eq(held(&mine), held(&yours)),
            "one copy per subscriber, so the two payloads are two allocations"
        );

        release(mine);
        release(yours);
    }

    /// A topic nobody joined is reached by nobody, and a connection that ended
    /// is not a subscriber a publish counts — the weak reference in the table,
    /// read from the fan-out's side rather than from the walk that prunes it.
    #[test]
    fn a_publish_counts_only_the_connections_that_are_still_there() {
        let mut publisher = Ctx::buffered();
        assert_eq!(
            publish(&mut publisher, "room:empty", "nobody").expect("not an error"),
            0
        );

        let mut staying = connected();
        let mut leaving = connected();
        call(nvs_core_topic_subscribe, &mut staying, "room:thinning").expect("one joins");
        call(nvs_core_topic_subscribe, &mut leaving, "room:thinning").expect("and another");
        drop(leaving);

        assert_eq!(
            publish(&mut publisher, "room:thinning", "still here").expect("a publish"),
            1
        );
        drained(&mut staying, "room:thinning");
    }

    /// A connection that publishes to a topic it joined itself is delivered to
    /// like every other subscriber — this module's second decision, and what a
    /// chat loop relies on, where the sender sees their own message.
    #[test]
    fn a_connection_publishing_to_its_own_topic_is_one_of_the_subscribers() {
        let mut talking = connected();
        call(nvs_core_topic_subscribe, &mut talking, "room:echo").expect("it joins");

        assert_eq!(
            publish(&mut talking, "room:echo", "said").expect("and speaks"),
            1
        );
        drained(&mut talking, "room:echo");
    }

    /// The table is keyed by name and holds one entry per live connection, so
    /// two connections on one topic are two subscribers and one connection
    /// subscribing twice is still one — `rule:core-classes/topic`.
    ///
    /// The second half is what a publish would otherwise get wrong: a
    /// fan-out over a row with a duplicate in it delivers the same value to
    /// one connection twice, and the copy per subscriber makes that two objects
    /// rather than one noticed twice.
    // covers: Core\Topic::subscribe
    #[test]
    fn a_topic_holds_one_entry_per_connection_however_often_it_subscribed() {
        let mut first = connected();
        let mut second = connected();

        call(nvs_core_topic_subscribe, &mut first, "room:lobby").expect("a connection subscribes");
        call(nvs_core_topic_subscribe, &mut first, "room:lobby").expect("and may say so twice");
        assert_eq!(subscriber_count("room:lobby"), 1);

        call(nvs_core_topic_subscribe, &mut second, "room:lobby").expect("so does another");
        assert_eq!(subscriber_count("room:lobby"), 2);

        call(nvs_core_topic_unsubscribe, &mut first, "room:lobby").expect("one leaves");
        assert_eq!(subscriber_count("room:lobby"), 1);
    }

    /// A connection that ends without unsubscribing leaves no entry behind,
    /// because the table's reference is weak and the context holds the only
    /// strong one — the module doc's reason for the indirection, asserted.
    ///
    /// This is the case that says the table is O(live connections) rather than
    /// O(connections served), which is the difference between a bound and a
    /// leak.
    #[test]
    fn a_connection_that_ended_is_no_longer_a_subscriber() {
        let mut ended = connected();
        call(nvs_core_topic_subscribe, &mut ended, "room:closing").expect("it subscribes");
        assert_eq!(subscriber_count("room:closing"), 1);

        drop(ended);

        assert_eq!(subscriber_count("room:closing"), 0);
    }

    /// Leaving a topic this connection never joined is not an error, and
    /// leaving one nobody else is on takes the row out of the table rather
    /// than leaving an empty one — `rule:core-classes/topic`'s queue is per subscriber, so
    /// a topic with no subscriber is nothing at all.
    // covers: Core\Topic::unsubscribe
    #[test]
    fn unsubscribing_from_a_topic_that_was_never_joined_is_the_state_it_asks_for() {
        let mut ctx = connected();
        call(nvs_core_topic_unsubscribe, &mut ctx, "room:never").expect("it is not an error");
        assert_eq!(subscriber_count("room:never"), 0);

        call(nvs_core_topic_subscribe, &mut ctx, "room:brief").expect("joined");
        call(nvs_core_topic_unsubscribe, &mut ctx, "room:brief").expect("and left");
        assert_eq!(subscriber_count("room:brief"), 0);
    }

    /// `rule:concurrency/two-doors-one-isolate`'s doors, asked the one question
    /// `Core\Topic` asks of a context: the isolate an event stream outlives its
    /// request on is a connection and joins, and a request answering with its
    /// own events is not one and is refused.
    ///
    /// The refusal is why the door is recorded and not merely the fact that one
    /// was opened: both contexts here are writing an event stream and neither
    /// holds a socket, so a member reading `has_event_stream` would let a
    /// response that is already ending into a table no wait of its own could
    /// ever drain.
    #[test]
    fn an_event_stream_connection_subscribes_and_a_streaming_response_is_refused() {
        let mut connection = over_events(EventStreamDoor::Connection);
        call(nvs_core_topic_subscribe, &mut connection, "room:door-one").expect("joined");
        assert_eq!(subscriber_count("room:door-one"), 1);

        let mut response = over_events(EventStreamDoor::Response);
        assert!(
            call(nvs_core_topic_subscribe, &mut response, "room:door-one").is_err(),
            "a streaming response is not a connection"
        );
        assert_eq!(subscriber_count("room:door-one"), 1);
    }

    /// `rule:core-classes/topic`'s security rule, on both halves at once: a subscriber
    /// that never reads is **closed**, and the publisher fanning out to it is
    /// unaffected — not blocked, not failed, and still reaching everybody else.
    ///
    /// The two connections here differ in exactly one thing: one drains its
    /// queue at every publish and the other never does. So what the case pins
    /// is the bound and nothing beside it — the slow one stops being counted at
    /// [`INBOX_CAP`], the fast one keeps being delivered to afterwards, and the
    /// close carries [`Closing::SlowSubscriber`]'s code rather than the one an
    /// orderly end would.
    ///
    /// The close is asserted at the *subscriber's* own `receive()`, which is
    /// where `rule:core-classes/topic`'s close is performed: the publisher
    /// cannot reach another isolate's peer, so a case asserting the close on the
    /// publish would be asserting a design this tree deliberately does not have.
    #[test]
    fn a_subscriber_that_never_reads_is_closed_and_the_publisher_is_unaffected() {
        let socket = Watched::default();
        let mut sleeper = connected_to(&socket);
        let mut reader = connected();
        call(nvs_core_topic_subscribe, &mut sleeper, "room:slow").expect("the slow one joins");
        call(nvs_core_topic_subscribe, &mut reader, "room:slow").expect("and one that reads");

        let closed_before = nvs_runtime::slow_subscribers_closed();
        let mut publisher = Ctx::buffered();
        for _ in 0..INBOX_CAP {
            let reached = publish(&mut publisher, "room:slow", "tick").expect("a publish");
            assert_eq!(reached, 2, "both are still being queued");
            drained(&mut reader, "room:slow");
        }

        let reached = publish(&mut publisher, "room:slow", "one too many").expect("a publish");
        assert_eq!(
            reached, 1,
            "the full queue is stepped over rather than waited for"
        );
        drained(&mut reader, "room:slow");
        assert_eq!(
            nvs_runtime::slow_subscribers_closed(),
            closed_before + 1,
            "the slow-subscriber metric counts the subscriber once, not once per publish"
        );

        // The subscriber's own next wait is what performs the close, and it
        // answers `null` — the condition `rule:concurrency/a-connection-is-a-loop`'s
        // `while` loop ends on.
        let conn = nvs_runtime::call(crate::socket::nvs_core_socket_current, &mut sleeper, &[])
            .expect("a connection isolate answers `current()`");
        let answered = nvs_runtime::call(
            crate::socket::nvs_core_socket_receive,
            &mut sleeper,
            &[conn],
        )
        .expect("the wait answered");
        assert!(
            answered.as_text().is_none() && answered.obj_ptr().is_none(),
            "a connection being closed was handed a message"
        );
        assert_eq!(socket.0.get(), Some(Closing::SlowSubscriber));
        #[expect(
            unsafe_code,
            reason = "the case owns the reference `current()` answered with, and \
                      `receive` borrowed rather than took it"
        )]
        // SAFETY: nothing else points at the connection value this case built.
        unsafe {
            conn.release();
        }

        // And the publisher carries on to everybody else, which is the half of
        // the rule a bound alone would not give.
        let reached = publish(&mut publisher, "room:slow", "and on").expect("a publish");
        assert_eq!(reached, 1, "the reader is still a subscriber");
        drained(&mut reader, "room:slow");
    }

    /// The queue's byte bound: a subscriber that never reads holds at most its
    /// own memory limit in deliveries, however far below the count bound the
    /// publishes stop, and the next one past it takes the slow-subscriber path.
    #[test]
    fn a_subscriber_that_never_reads_holds_at_most_its_memory_limit() {
        const LIMIT: usize = 64 * 1024;
        let mut sleeper = connected();
        sleeper.set_memory_limit(LIMIT);
        call(nvs_core_topic_subscribe, &mut sleeper, "room:large").expect("the slow one joins");
        let inbox = sleeper.deliveries().expect("a subscribe makes the queue");

        let large = "x".repeat(16 * 1024);
        let mut publisher = Ctx::buffered();
        let mut reached = 0;
        for _ in 0..16 {
            reached += publish(&mut publisher, "room:large", &large).expect("a publish");
        }

        assert!(
            (1..16).contains(&reached),
            "the byte bound stopped the queue after {reached} of 16 deliveries"
        );
        assert!(
            inbox.held_bytes() <= LIMIT,
            "the queue holds {} bytes against a {LIMIT}-byte limit",
            inbox.held_bytes()
        );
        assert!(
            inbox.len() < INBOX_CAP,
            "the count bound is not what stopped it"
        );
        assert!(
            inbox.overflowed(),
            "a refusal past the bytes closes the subscriber"
        );
    }
}
