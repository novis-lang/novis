//! `Core\Topic` — [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md)
//! § 4's bus, as the only way two connections meet.
//!
//! § 3 gives a connection one wait over two sources; this is the second one.
//! A connection joins a topic by name, a publish copies a value to everyone who
//! joined it, and the value arrives as the `Core\Socket\Message` whose `topic`
//! is non-`null` — so a program that already writes § 3's loop needs no second
//! control-flow style to receive from the bus.
//!
//! # What is here, and what is not
//!
//! § 4's three rows and the table behind them. What is **not** here is the
//! hand-off between cores: `publish` copies to the subscribers that joined on
//! the core it runs on, and § 4's "a publish from a connection on core 3
//! reaches subscribers on core 0" waits on the bounded queue a neighbouring
//! core is handed. Until that lands a publish is whole only where the
//! publisher and the subscriber landed on the same core, and that is this
//! module's first known gap.
//!
//! The second one is under the seam rather than here, and this row is what
//! makes it reachable: a delivery queued while its connection is already
//! parked inside `receive()` is answered by the *next* `receive()` rather than
//! waking the parked one. [`nvs_runtime::Ctx::deliver`]'s own known gap is
//! where that is written down.
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
//! [ADR 0023](/docs/adr/0023-clone-serialize-and-cross-boundary-copy.md) § 2's
//! graph copy does two jobs here and only one of them scales with the
//! audience. It gives each subscriber a value that shares nothing with the
//! publisher or with any other subscriber, which is § 4's rule and is one copy
//! per subscriber; and it is what **refuses** a value with no meaning on the
//! other side — a resource, or a `secret`, which
//! [ADR 0033](/docs/adr/0033-secret-qualifier-for-confidential-values.md) says
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
//! [ADR 0076](/docs/adr/0076-observability-export.md) already applies to a
//! metric label, and for the same reason. A name derived from user input is
//! how one tenant subscribes to another's stream.
//!
//! There is no [`crate::registry::CAPABILITIES`] row. The bus reaches no
//! operating-system facility at all — it is a map in this process — and the
//! grant that decides whether a program may be a connection in the first place
//! was asked at the upgrade ([`crate::socket`]).

use std::collections::HashMap;
use std::rc::{Rc, Weak};

use nvs_runtime::{Ctx, Delivery, Fault, Inbox, ThrownClass, Value, copy_graph};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};
use crate::socket::{release_crossed, retained};

/// `Core\Topic`'s fully-qualified name, in one place so the row and every
/// message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Topic";

/// `Core\Topic`'s registry rows — ADR 0083 § 4's three, and see
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
            // arrives (ADR 0024), and what may not cross at all is refused by
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

/// `Core\Topic::subscribe`'s reference card — ADR 0117.
const SUBSCRIBE_DOC: MethodDoc = MethodDoc {
    short: "Joins this connection to `$topic`, so that a value published to it arrives at the \
            next `receive()` as a message whose `topic()` is that name.",
    params: &[ParamDoc {
        name: "topic",
        desc: "The topic's name. It may not come from outside the program — a name derived from \
               user input is how one tenant subscribes to another's stream — so it is built from \
               checked values or it does not compile.",
        shape: &[],
    }],
    ret: "Nothing. Subscribing twice to one name is one subscription, so a published value \
          arrives once however many times the connection joined.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "An empty `$topic`, which no publisher can mean; and a call from a program that is \
               not a connection, which has nothing to deliver to.",
    }],
};

/// `Core\Topic::publish`'s reference card — ADR 0117.
const PUBLISH_DOC: MethodDoc = MethodDoc {
    short: "Copies `$value` to every connection subscribed to `$topic`, and answers how many were \
            reached.",
    params: &[
        ParamDoc {
            name: "topic",
            desc: "The topic's name, under the rule `subscribe` reads it under: it is built from \
                   checked values or it does not compile.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "What to publish. Every subscriber is handed its own copy, so nothing is shared \
                   with the publisher or between subscribers; a `tainted` value is still \
                   `tainted` where it arrives, and a `secret` may not be published at all.",
            shape: &[],
        },
    ],
    ret: "How many subscribers the value was queued for, which is `0` for a topic nobody has \
          joined. Publishing needs no connection of its own — an ordinary request may tell the \
          connections that something changed — and a connection publishing to a topic it joined \
          itself is delivered to like any other subscriber.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "An empty `$topic`, which no subscriber can be reached by; and a `$value` with no \
               meaning on the other side of a copy boundary — a resource, or a `secret` — which \
               is refused whether or not anybody has joined.",
    }],
};

/// `Core\Topic::unsubscribe`'s reference card — ADR 0117.
const UNSUBSCRIBE_DOC: MethodDoc = MethodDoc {
    short: "Leaves `$topic`, so nothing published to it reaches this connection again.",
    params: &[ParamDoc {
        name: "topic",
        desc: "The topic's name, under the same rule `subscribe` reads it under.",
        shape: &[],
    }],
    ret: "Nothing. Leaving a topic this connection never joined is not an error — the state it \
          asks for is the state that already holds.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "An empty `$topic`, and a call from a program that is not a connection — the same \
               two `subscribe` refuses, so the pair cannot disagree about what a call means.",
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
/// # Errors
///
/// A `LogicError` on a context that is not a connection's, in
/// `crate::socket`'s wording for the same fact: a topic is how two
/// *connections* meet, so a subscription made by anything else would be an
/// entry in the table that no `receive()` could ever drain.
fn connection_inbox(ctx: &mut Ctx, member: &str) -> Result<Rc<Inbox>, Fault> {
    if !ctx.has_peer() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "`Core\\Topic::{member}` needs a connection and this program is not one: only a \
                 script `Core\\Socket::upgrade` opened runs inside a connection isolate"
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
fn join(topic: &str, inbox: &Rc<Inbox>) {
    SUBSCRIBERS.with_borrow_mut(|table| {
        let Some(row) = table.get_mut(topic) else {
            table.insert(Box::from(topic), vec![Rc::downgrade(inbox)]);
            return;
        };
        row.retain(|held| held.strong_count() > 0);
        if !row
            .iter()
            .any(|held| held.upgrade().is_some_and(|live| Rc::ptr_eq(&live, inbox)))
        {
            row.push(Rc::downgrade(inbox));
        }
    });
}

/// Takes `inbox` out of `topic`'s row, and the row out of the table once it
/// holds nobody.
///
/// A name that is not in the table at all is the state the caller asked for,
/// so there is nothing here to report.
fn leave(topic: &str, inbox: &Rc<Inbox>) {
    SUBSCRIBERS.with_borrow_mut(|table| {
        let Some(row) = table.get_mut(topic) else {
            return;
        };
        row.retain(|held| held.upgrade().is_some_and(|live| !Rc::ptr_eq(&live, inbox)));
        if row.is_empty() {
            table.remove(topic);
        }
    });
}

/// The live subscribers `topic` has on this core, with the row left holding
/// only them.
///
/// Handed back as owned handles rather than walked in place: the fan-out makes
/// a graph copy per subscriber, and a table borrow held across an allocation
/// is a re-entrancy nobody needs — `subscribe` runs on a connection's own task
/// and reaches the same map.
fn subscribers_of(topic: &str) -> Vec<Rc<Inbox>> {
    SUBSCRIBERS.with_borrow_mut(|table| {
        let Some(row) = table.get_mut(topic) else {
            return Vec::new();
        };
        let live: Vec<Rc<Inbox>> = row.iter().filter_map(Weak::upgrade).collect();
        if live.is_empty() {
            table.remove(topic);
        } else {
            row.retain(|held| held.strong_count() > 0);
        }
        live
    })
}

/// ADR 0023 § 2's copy of the value being published, which is also the one
/// refusal `publish` makes about it.
///
/// **Answers one owned reference**, which the delivery it is put in takes over.
/// The argument is only borrowed by this frame, so the retain is what
/// reconciles the two conventions — [`crate::socket::retained`] owns why.
///
/// # Errors
///
/// A `LogicError` naming what has no meaning on the other side of a copy
/// boundary: a resource, or a `secret`, which ADR 0033 says may never be
/// published.
fn cross(value: Value) -> Result<Value, Fault> {
    copy_graph(retained(value)).map_err(|refused| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!("`Core\\Topic::publish` cannot publish this value: {refused}"),
        )
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Topic::subscribe(string $topic): void` — ADR 0083 § 4's first row.
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
    /// `Core\Topic::publish(string $topic, mixed $value): uint` — ADR 0083
    /// § 4's second row, and the only member of this class that asks nothing
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
    /// and this member never blocks on one.
    fn nvs_core_topic_publish(_ctx, args: [2]) {
        let topic = topic_of(&args[0], "publish")?;
        let subscribers = subscribers_of(&topic);
        let mut first = Some(cross(args[1])?);
        let mut delivered: u64 = 0;
        for inbox in &subscribers {
            // A refusal is a property of the graph rather than of the copy, so
            // the one above is the one that reports it: a further copy of the
            // same unchanged value cannot decide differently, and this `?` is
            // unreachable in the same sense the module doc's decision is.
            let copy = match first.take() {
                Some(made) => made,
                None => cross(args[1])?,
            };
            inbox.push(Delivery::new(topic.as_str(), copy));
            delivered += 1;
        }
        if let Some(unused) = first {
            release_crossed(unused);
        }
        Ok(Value::uint(delivered))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Topic::unsubscribe(string $topic): void` — ADR 0083 § 4's third
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
    use nvs_runtime::{Ctx, NvsFn, NvsStr, PeerError, PeerFrame, PeerSocket, Value};

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

    /// A peer that says nothing and is never read: what these cases need from
    /// a socket is only that the context *has* one, which is the whole of what
    /// separates a connection isolate from every other kind of context.
    #[derive(Debug)]
    struct Silent;

    impl PeerSocket for Silent {
        fn receive(&mut self) -> Result<Option<PeerFrame>, PeerError> {
            Ok(None)
        }

        fn send(&mut self, _frame: PeerFrame) -> Result<(), PeerError> {
            Ok(())
        }

        fn close(&mut self) {}
    }

    /// A context that is a connection's — ADR 0083 § 1's isolate with a socket
    /// already moved onto it.
    fn connected() -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_peer(Box::new(Silent));
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
    /// and hands each one a copy of its own — ADR 0083 § 4.
    ///
    /// The publisher here is not a connection, which is this module's second
    /// decision: a topic is how two connections meet, and an ordinary program
    /// is allowed to be what tells them so.
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
    /// subscribing twice is still one — ADR 0083 § 4.
    ///
    /// The second half is what a publish would otherwise get wrong: a
    /// fan-out over a row with a duplicate in it delivers the same value to
    /// one connection twice, and § 4's copy makes that two objects rather than
    /// one noticed twice.
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
    /// than leaving an empty one — ADR 0083 § 4's queue is per subscriber, so
    /// a topic with no subscriber is nothing at all.
    #[test]
    fn unsubscribing_from_a_topic_that_was_never_joined_is_the_state_it_asks_for() {
        let mut ctx = connected();
        call(nvs_core_topic_unsubscribe, &mut ctx, "room:never").expect("it is not an error");
        assert_eq!(subscriber_count("room:never"), 0);

        call(nvs_core_topic_subscribe, &mut ctx, "room:brief").expect("joined");
        call(nvs_core_topic_unsubscribe, &mut ctx, "room:brief").expect("and left");
        assert_eq!(subscriber_count("room:brief"), 0);
    }
}
