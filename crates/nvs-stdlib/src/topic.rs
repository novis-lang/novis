//! `Core\Topic` — `rule:core-classes/topic`'s bus, as the only way two connections meet.
//!
//! § 3 gives a connection one wait over two sources; this is the second one.
//! A connection joins a topic by name, a publish copies a value to everyone who
//! joined it, and the value arrives as the `Core\Socket\Message` whose `topic`
//! is non-`null` — so a program that already writes § 3's loop needs no second
//! control-flow style to receive from the bus.
//!
//! # What is here, and what is not
//!
//! § 4's three rows, the table behind them, and both halves of the crossing
//! between cores: the publish that hands an encoded value to every other core
//! listening, and the drain that reads it back and fans it out where it
//! landed. The transport underneath is [`crate::bus`] — this module is what
//! decides what crosses and what a subscriber is handed.
//!
//! What is **not** here is the wake, and this module is what makes it
//! reachable. The queue carries one — [`nvs_runtime::Inbox`]'s, fired by the
//! fan-out that fills it — and an event stream's `receive()` registers against
//! it, so a publish on this core reaches a stream that is already waiting. A
//! connection parked inside `Core\Socket::receive` is parked on its *socket*
//! and registers nothing here, so a delivery queued while it waits is answered
//! by the next `receive()`; [`nvs_runtime::Ctx::deliver`]'s own known gap is
//! where that is written down. A delivery from another core is behind one
//! bound more on both doors, the drain running at the same `receive()` the
//! local queue is read at.
//!
//! # Decision: the table is per core, and it holds a weak reference
//!
//! One `thread_local!` map from a topic's name to the subscribers on **this**
//! core, which is [`crate::cache`]'s local tier as a representation rather than
//! a rule: the runtime is thread-per-core, so a table another core could reach
//! would need a lock. § 4's "across every core" is the *publish*'s promise and
//! it is kept by a hand-off between cores rather than by one shared map — a
//! bounded message hand-off is what that section says it is, and a shared map
//! with a lock in front of it is the shared state it says it is not.
//!
//! What the table holds per subscriber is a [`Weak`] onto that connection's
//! [`nvs_runtime::Inbox`], whose only strong reference is the connection's own
//! context. **Nothing unsubscribes when a connection ends**, and nothing has
//! to: the isolate's `Ctx` drops, the last strong reference goes with it, and
//! the entry is dead the next time that topic is walked. That is the property
//! worth the indirection, because a connection that ended at a `[limits]`
//! ceiling or a fatal error (§ 1) runs no more of its own code and could not
//! have unsubscribed itself. The alternative — an owned handle plus a
//! teardown hook — is a table that is O(connections served) whenever the hook
//! does not run, which [AGENTS.md](/AGENTS.md)'s memory rule calls a leak
//! rather than a trade-off.
//!
//! **What it spends:** one map entry per topic that has a live subscriber on
//! this core, and one [`Weak`] — two words — per subscription. Both are
//! O(live connections × topics each joined) and neither is O(publishes) or
//! O(connections served). A topic whose last subscriber went away keeps its
//! entry only until the next walk of that name, and an empty one is removed
//! rather than left behind.
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
//! that allocated it needs the per-request provenance M6's arena is what gives.
//!
//! # Decision: a publisher needs no connection, and is not excluded from its own topic
//!
//! `subscribe` and `unsubscribe` refuse a program that is not a connection,
//! because a subscription with no queue behind it is an entry no `receive()`
//! could ever drain. `publish` asks nothing about its host: it hands a value
//! to whoever joined, and the commonest reason to have a bus at all is an
//! ordinary HTTP request telling the connections that something changed. § 4
//! writes its example publish from inside a connection, but nothing in it
//! makes that the rule, and a refusal here would make an application hold a
//! connection open for the sole purpose of being allowed to speak.
//!
//! A connection that joined a topic it also publishes to **is** delivered to,
//! like every other subscriber. The table is keyed by name and holds
//! connections rather than everybody-but-one, so an exclusion would have to be
//! computed per publish, and it would make the count answer something other
//! than "subscribers". § 3's own loop is written that way: the sender sees
//! their own message, which is what every chat does.
//!
//! # Decision: the copy is made before the walk, and whether or not anyone joined
//!
//! `rule:classes/graph-copy`'s
//! graph copy does two jobs here and only one of them scales with the
//! audience. It gives each subscriber a value that shares nothing with the
//! publisher or with any other subscriber, which is § 4's rule and is one copy
//! per subscriber; and it is what **refuses** a value with no meaning on the
//! other side — a resource, or a `secret`, which
//! `rule:security/secret-qualifier` says
//! may never be published.
//!
//! A refusal that only happened once somebody had joined would be a rule that
//! held or did not by timing, and it would be unreachable from the conformance
//! corpus outright, because a `.nvst` case has no connection to subscribe
//! with. So the first copy is made before the walk, the walk hands it to the
//! first subscriber and makes one more for each further one, and a publish to
//! a topic nobody joined releases it unused. **What it spends:** one graph
//! copy on a publish nobody is listening to, which is the price of the refusal
//! being the value's business rather than the topic's.
//!
//! # Decision: what crosses a core is bytes, and the count is what was queued
//!
//! A [`Value`] is refcounted on the core that made it, so the copy a
//! subscriber on another core is handed cannot be made by the publisher. What
//! crosses is `rule:classes/graph-copy`'s *encoding* rather than its copy —
//! [`nvs_runtime::encode`] on the publishing core and `decode` on the
//! receiving one, the same carrier [`crate::cache`]'s shared tier crosses a
//! process with — and the receiving core makes one value per subscriber as it
//! drains. The two halves of that carrier refuse the same graphs, so the
//! crossing decided above is still the one that reports a refusal, and a
//! publish cannot be refused by who happened to be listening elsewhere.
//!
//! **The count is what was queued, on either side.** Locally that is the live
//! subscribers the walk found; elsewhere it is the number that core last
//! reported for the topic, which it refreshes whenever it joins, leaves or
//! walks that row. A remote connection that ended without unsubscribing is
//! therefore counted until its own core next looks at that name — an
//! over-count of the same kind § 4's row already carries locally, where a
//! subscriber counted at the queue may never live to read it. Making it exact
//! would mean waiting for the other core to answer before `publish` returns,
//! which is the publisher blocking on a subscriber, and that is the one thing
//! § 4 says may not happen.
//!
//! **What it spends:** one encoding per publish somebody elsewhere is
//! listening to, held once however many cores take it, and nothing at all on a
//! single-core server or a topic joined only here. [`crate::bus`] owns the
//! rest of that accounting.
//!
//! # Decision: a subscriber that has stopped reading is skipped, not waited for
//!
//! § 4's priority-1 rule, in the fan-out: the walk asks each queue whether it
//! has room *before* it makes that subscriber's copy, and a full one is marked
//! and stepped over. So a connection that stopped reading costs a publish one
//! comparison — not an allocation, not a wait, and not the other nine thousand
//! nine hundred and ninety-nine subscribers' latency.
//!
//! **It is not counted, either.** `publish` answers what it queued, and a value
//! this member declined to copy was never queued for anybody. That keeps the
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
//! parameter is a [`CoreTy::Text`] at [`Qual::Sink`], which is § 4's "a name is
//! built from checked values or it does not compile" — the rule
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

use nvs_runtime::{Ctx, Delivery, Fault, Inbox, ThrownClass, Value, budget, copy_graph};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};
use crate::socket::{release_crossed, retained};

/// `Core\Topic`'s fully-qualified name, in one place so the row and every
/// message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Topic";

/// `Core\Topic`'s registry rows — `rule:core-classes/topic`'s three, and see
/// [`crate::registry::CLASSES`].
///
/// A namespace class with no instance members and no slots, because a
/// subscription is not a value a program holds: the subscriber is the
/// *connection*, which § 1 makes the isolate, and the isolate is already the
/// thing `Core\Socket::current()` answers for. A handle here would be a second
/// name for the same connection, and the first thing it could get wrong is
/// being used from a different one.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "subscribe",
            names: &["topic"],
            // [`Qual::Sink`] is § 4's refusal of a `tainted` name, and the
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
            // the copy rather than by the signature.
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
        desc: "`$topic` is empty, or this script is not a WebSocket connection or an event \
               stream. Only a script that `Core\\Socket::upgrade` or `Core\\Sse::upgrade` \
               started can subscribe.",
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
                   subscriber makes is not seen by any other. A `tainted` value is still \
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
        desc: "`$topic` is empty, or `$value` cannot be copied to another connection, such as an \
               object with a `secret` property. The error is thrown even when nobody joined the \
               topic.",
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
        desc: "`$topic` is empty, or this script is not a WebSocket connection or an event \
               stream. These are the same two errors `subscribe` throws.",
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

/// The topic name an argument carries, checked.
///
/// # Errors
///
/// A `LogicError` for the empty name — the module doc owns why that is the
/// call's fault and why it is asked first.
fn topic_of(argument: &Value, member: &str) -> Result<String, Fault> {
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
    Ok(name.to_owned())
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
fn join(topic: &str, inbox: &Rc<Inbox>) {
    let live = SUBSCRIBERS.with_borrow_mut(|table| {
        let Some(row) = table.get_mut(topic) else {
            let _bracket = budget::Detached::begin();
            table.insert(Box::from(topic), vec![Rc::downgrade(inbox)]);
            return 1;
        };
        // Outside the bracket on purpose: what this drops is the last handle
        // onto a dead connection's inbox, and that allocation is its own
        // request's. The module doc's accounting decision owns the split.
        row.retain(|held| held.strong_count() > 0);
        if !row
            .iter()
            .any(|held| held.upgrade().is_some_and(|live| Rc::ptr_eq(&live, inbox)))
        {
            let _bracket = budget::Detached::begin();
            row.push(Rc::downgrade(inbox));
        }
        row.len()
    });
    crate::bus::note_subscribers(topic, live);
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
        row.retain(|held| held.upgrade().is_some_and(|live| !Rc::ptr_eq(&live, inbox)));
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
/// a graph copy per subscriber, and a table borrow held across an allocation
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

/// `rule:classes/graph-copy`'s copy of the value being published, which is also the one
/// refusal `publish` makes about it.
///
/// **Answers one owned reference**, which the delivery it is put in takes over.
/// The argument is only borrowed by this frame, so the retain is what
/// reconciles the two conventions — [`crate::socket::retained`] owns why.
///
/// # Errors
///
/// A `LogicError` naming what has no meaning on the other side of a copy
/// boundary: a resource, or a `secret`, which `rule:security/secret-qualifier` says may never be
/// published.
fn cross(value: Value) -> Result<Value, Fault> {
    copy_graph(retained(value)).map_err(|refused| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!("`Core\\Topic::publish` cannot publish this value: {refused}"),
        )
    })
}

/// `rule:classes/graph-copy`'s *encoding* of the value being published, which is what
/// crosses to another core in place of a copy.
///
/// **Borrows the argument**, on [`retained`]'s convention, and answers bytes
/// that own nothing.
///
/// # Errors
///
/// The same refusal [`cross`] makes, in the same words: `encode` and
/// `copy_graph` share the walk that decides, so this cannot refuse a value the
/// copy above already accepted. It is mapped rather than asserted because a
/// carrier answering `Result` is not a place to write an `expect`.
fn externalize(value: Value) -> Result<Vec<u8>, Fault> {
    nvs_runtime::encode(retained(value)).map_err(|refused| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!("`Core\\Topic::publish` cannot publish this value: {refused}"),
        )
    })
}

/// Fans out everything other cores have published to this one, into the
/// subscribers that joined here.
///
/// Called by `Core\Socket::receive()` before it reads either source, which is
/// where a connection is about to wait anyway — [`crate::socket`] owns that
/// ordering, and this module's docs own why a delivery from another core
/// inherits the wake gap rather than adding one.
///
/// One `decode` **per subscriber**, so each is handed a graph it shares with
/// nobody, which is § 4's rule and is the same guarantee the local walk's
/// copy-per-subscriber gives. An envelope naming a class this program cannot
/// resolve is dropped rather than thrown: the connection running this drain
/// did not publish it and has no answer to give, and the resolver is the
/// draining program's own class table (`nvs_runtime::graph`'s known gap 2).
pub(crate) fn deliver_from_other_cores(ctx: &Ctx) {
    // The drain is held for the whole fan-out and freed by the bus, on the
    // balance the publishing core allocated it on — `crate::bus`'s accounting
    // decision, which is why this is not a `for` over an owned `Vec`.
    let drained = crate::bus::take_all();
    for envelope in drained.envelopes() {
        let subscribers = subscribers_of(envelope.topic());
        let resolve = |name: &str| ctx.class_desc(name);
        for inbox in &subscribers {
            // The same bound the local walk keeps, asked before the decode for
            // the same reason it is asked before the copy.
            if !inbox.has_room() {
                inbox.note_overflow();
                continue;
            }
            let Ok(value) = nvs_runtime::decode(envelope.payload(), &resolve) else {
                break;
            };
            if let Some(refused) = inbox.push(Delivery::new(envelope.topic(), value)) {
                release_crossed(refused.into_value());
            }
        }
    }
}

/// Publishes `text` on `topic` as `Core\Topic::publish($topic, $text)` does,
/// and answers how many subscribers it reached.
///
/// For a publisher with no program behind it: `nvs run --events` reads its
/// values off a file and puts them on the bus through this, so the program it
/// runs meets the same fan-out, the same copy and the same bound as a
/// connection published to by another. A name the member would refuse reaches
/// nobody and answers `0`.
#[must_use]
pub fn publish_text(topic: &str, text: &str) -> u64 {
    let mut publisher = Ctx::buffered();
    let name = Value::str(nvs_runtime::NvsStr::new(topic.as_bytes()));
    let payload = Value::str(nvs_runtime::NvsStr::new(text.as_bytes()));
    let reached = nvs_runtime::call(nvs_core_topic_publish, &mut publisher, &[name, payload]);
    #[expect(
        unsafe_code,
        reason = "this frame owns the two references it built for the arguments, \
                  and the member borrows rather than takes them"
    )]
    // SAFETY: nothing else points at either value — each subscriber was queued
    // a copy of its own.
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
        join(&topic, &inbox);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Topic::publish(string $topic, mixed $value): uint` — `rule:core-classes/topic`'s second row, and the only member of this class that asks nothing
    /// about the program calling it.
    ///
    /// The name, then the crossing, then the walk. The crossing stands before
    /// the walk and is made once however many joined, which is this module's
    /// third decision; the walk hands that first copy to the first subscriber
    /// and makes one more for each further one, because § 4 says subscribers
    /// share nothing with each other or with the publisher.
    ///
    /// The count is what reached a queue, not what a subscriber has read: a
    /// delivery waits until that connection's own `receive()` drains it (§ 3),
    /// and this member never blocks on one — on this core or on any other.
    ///
    /// **The walk of the other cores comes last**, and it is asked before the
    /// value is encoded so that a topic nobody joined elsewhere costs no
    /// carrier at all. What it hands over is bytes and what it counts is what
    /// those cores reported, which is this module's fourth decision.
    fn nvs_core_topic_publish(_ctx, args: [2]) {
        let topic = topic_of(&args[0], "publish")?;
        let subscribers = subscribers_of(&topic);
        let mut first = Some(cross(args[1])?);
        let mut delivered: u64 = 0;
        for inbox in &subscribers {
            // § 4's bound, asked before the copy is made rather than after:
            // a subscriber that is already being closed costs this publisher
            // nothing but the test. `nvs_runtime::peer` owns the rest of it.
            if !inbox.has_room() {
                inbox.note_overflow();
                continue;
            }
            // A refusal is a property of the graph rather than of the copy, so
            // the one above is the one that reports it: a further copy of the
            // same unchanged value cannot decide differently, and this `?` is
            // unreachable in the same sense the module doc's decision is.
            let copy = match first.take() {
                Some(made) => made,
                None => cross(args[1])?,
            };
            // Nothing between `has_room` and here can have filled the queue —
            // this core is the only one that pushes into its own subscribers'
            // queues, and neither call suspends — so the refusal is written for
            // the ownership rule rather than for a path a publish reaches.
            if let Some(refused) = inbox.push(Delivery::new(topic.as_str(), copy)) {
                release_crossed(refused.into_value());
                continue;
            }
            delivered += 1;
        }
        if let Some(unused) = first {
            release_crossed(unused);
        }
        if crate::bus::subscribers_elsewhere(&topic) > 0 {
            delivered += crate::bus::hand_off(&topic, externalize(args[1])?);
        }
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
        leave(&topic, &inbox);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{
        Closing, Ctx, EventStreamDoor, INBOX_CAP, NvsFn, NvsStr, PeerError, PeerFrame, PeerSocket,
        Value, budget,
    };

    use super::{
        nvs_core_topic_publish, nvs_core_topic_subscribe, nvs_core_topic_unsubscribe,
        subscribers_of,
    };

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
    /// from every other kind of context. § 4's slow subscriber needs one thing
    /// more — the code the peer was closed with — and a handle onto the same
    /// cell is how a case reads it back after the context took the socket.
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
        let name = Value::str(NvsStr::new(topic.as_bytes()));
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
    /// of this module's half of the bus. The name is long enough that a charge
    /// for it cannot hide in the noise of a call: the process's balance takes it
    /// and the connection's does not, and the unsubscribe gives it back to the
    /// same balance rather than crediting whichever connection happened to be
    /// the last one out. The module doc's accounting decision is why the prune
    /// beside it is deliberately outside the bracket.
    #[test]
    fn a_subscription_is_charged_to_the_process_and_not_to_the_connection() {
        /// A name long enough to show through what a call allocates around it.
        const NAME: usize = 256 * 1024;

        let name = format!("room:{}", "n".repeat(NAME));
        let mut joining = connected();
        let live = budget::live_bytes();
        let held = budget::detached_bytes();
        call(nvs_core_topic_subscribe, &mut joining, &name).expect("it joins");

        assert!(
            budget::detached_bytes() - held >= NAME.cast_signed(),
            "the row's name reached no balance at all, so nothing holds it to the process"
        );
        assert!(
            budget::live_bytes() - live < NAME.cast_signed(),
            "the connection was charged for a table row that outlives it"
        );

        let stored = budget::detached_bytes();
        call(nvs_core_topic_unsubscribe, &mut joining, &name).expect("and leaves");
        assert!(
            budget::detached_bytes() <= stored - NAME.cast_signed(),
            "the row was freed on some balance other than the one it was allocated on, so a \
             connection that prunes a name is credited for bytes it never held"
        );
        assert_eq!(subscriber_count(&name), 0, "and the row is gone");
    }

    /// One `publish` call, with the name and a text payload as arguments, and
    /// the count it answered.
    fn publish(ctx: &mut Ctx, topic: &str, text: &str) -> Result<u64, i32> {
        let name = Value::str(NvsStr::new(topic.as_bytes()));
        let payload = Value::str(NvsStr::new(text.as_bytes()));
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

    /// Takes the one delivery `topic` should have queued on `ctx`, releasing
    /// the reference it hands over.
    ///
    /// The release is the case's rather than `Ctx`'s own `Drop` on purpose:
    /// two independent releases of what a fan-out handed two subscribers is
    /// exactly what a walk that shared one copy would fail at.
    fn drained(ctx: &mut Ctx, topic: &str) {
        let delivery = ctx.take_delivery().expect("this subscriber was queued one");
        assert_eq!(delivery.topic(), topic);
        #[expect(
            unsafe_code,
            reason = "the delivery handed this frame the only reference to its value"
        )]
        // SAFETY: nothing else points at the copy this subscriber was queued.
        unsafe {
            delivery.into_value().release();
        }
        assert!(ctx.take_delivery().is_none(), "and exactly one");
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

            super::deliver_from_other_cores(&ctx);
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
    /// asserted as the two payloads not being one allocation.
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

        let mine = first
            .take_delivery()
            .expect("the first subscriber was queued one")
            .into_value();
        let yours = second
            .take_delivery()
            .expect("and so was the second")
            .into_value();
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

        #[expect(
            unsafe_code,
            reason = "each delivery handed this frame the only reference to its own copy"
        )]
        // SAFETY: nothing else points at either copy — the fan-out made one per
        // subscriber and both queues have been drained.
        unsafe {
            mine.release();
            yours.release();
        }
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
    /// like every other subscriber — this module's second decision, and § 3's
    /// own loop, where the sender sees their own message.
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
    /// one connection twice, and § 4's copy makes that two objects rather than
    /// one noticed twice.
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

    /// `rule:core-classes/topic`'s priority-1 rule, on both halves at once: a subscriber
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
    /// where § 4's rule is performed: the publisher cannot reach another
    /// isolate's peer, so a case asserting the close on the publish would be
    /// asserting a design this tree deliberately does not have.
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
            "§ 4's metric counts the subscriber once, not once per publish"
        );

        // The subscriber's own next wait is what performs the close, and it
        // answers § 3's `null` — the condition its `while` loop ends on.
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
}
