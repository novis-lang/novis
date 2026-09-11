//! `rule:concurrency/a-connection-is-a-root-isolate`'s qualifiers at the connection boundary — what a peer sent arrives
//! as, what a topic name may be built from, and which of an event's fields a
//! client may have chosen.
//!
//! Each is an ordinary checker question rather than a runtime one, which is the
//! point: § 3's payload, § 4's name and § 5's event fields are decided by
//! registry rows (`nvs_stdlib::socket`'s `Core\Socket\Message`,
//! `nvs_stdlib::topic`'s `Core\Topic` and `nvs_stdlib::sse`'s `Core\Sse`), so a
//! program that would leak any of them does not compile. The qualifier rule at
//! this boundary that is not here — a `secret` crossing § 2's `args:` or § 4's
//! publish — is in `isolates.rs`, beside the graph copy it shares with `spawn
//! script`.
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

/// `rule:concurrency/a-connection-is-a-loop`: "a received frame's payload is `tainted` — it is untrusted
/// input arriving over a network, exactly like a request body".
///
/// The claim is a property of the *payload* and not of the member that answered
/// it, so it is asserted on both sides of one value: the same expression is
/// refused where a plain `string` is expected and accepted where `rule:security/taint-propagation`'s
/// checked conversion has proven its shape. Reading `text()` and then narrowing
/// away the `null` is the whole of what § 3's loop does before it uses the
/// value, so the fixture is the shape a program actually writes.
#[test]
fn a_received_frames_payload_is_tainted_until_laundered() {
    let refused = check_src(
        "<?nvs\n\
         class T {\n\
           function m(Core\\Socket\\Message $msg): void {\n\
             var $payload = $msg->text();\n\
             if ($payload != null) {\n\
               string $plain = $payload;\n\
             }\n\
           }\n\
         }\n",
    );
    assert!(
        refused
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{refused:?}"
    );

    // `rule:security/taint-propagation`'s checked conversion is what laundering is spelled as
    // today: `as uint` already throws on a malformed shape, so a value that
    // survives it is proven safe. § 3 names `Core\Validate` as the launderer a
    // program reaches for, and that class has no text member yet
    // (`nvs_stdlib::validate`); when it grows one it launders through this same
    // rule rather than a second one.
    let laundered = check_src(
        "<?nvs\n\
         class T {\n\
           function m(Core\\Socket\\Message $msg): void {\n\
             var $payload = $msg->text();\n\
             if ($payload != null) {\n\
               uint $n = $payload as uint;\n\
             }\n\
           }\n\
         }\n",
    );
    assert!(!laundered.has_errors(), "{laundered:?}");
}

/// `rule:core-classes/topic`: "a topic name refuses `tainted` … a name derived from user
/// input is how one tenant subscribes to another's stream. A name is built from
/// checked values or it does not compile."
///
/// Asked of all three members that take a name, and asserted by **counting**
/// rather than off one line: the rule is a property of `Core\Topic`'s surface,
/// so a member that grew a neutral parameter of its own would still look right
/// on its own row. What carries it is `Qual::Sink` on each row, which reports
/// the ordinary mismatch a sink does — `rule:security/sink-predicate`'s classification, not a
/// rule of this ADR's own.
#[test]
fn a_tainted_topic_name_fails_to_compile() {
    let each = ["subscribe($t)", "publish($t, 1)", "unsubscribe($t)"];
    for call in each {
        let diags = check_in_method(&format!(
            "tainted string $t = \"room:lobby\" as tainted string;\n\
             Core\\Topic::{call};"
        ));
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "`{call}` accepted a tainted name: {diags:?}"
        );
    }

    // The other side of the bound: the same three calls with a name built from
    // checked values compile, so what was refused is the qualifier and not the
    // shape of the argument.
    let accepted = check_in_method(
        "string $room = \"lobby\";\n\
         Core\\Topic::subscribe(\"room:\" . $room);\n\
         Core\\Topic::publish(\"room:\" . $room, 1);\n\
         Core\\Topic::unsubscribe(\"room:\" . $room);",
    );
    assert!(!accepted.has_errors(), "{accepted:?}");
}

/// `rule:security/unclassified-parameter-refuses-tainted` over `Core\Sse->send`'s
/// event name, which is `rule:security/sink-predicate`'s own test read against a
/// reader rather than a parser: a client dispatches on the name it was sent, so
/// a name an attacker chose is an instruction and not a value.
///
/// Asserted on both sides of the bound, as the topic name above is: a name built
/// from checked values compiles through the same call, so what was refused is
/// the qualifier and not the shape of the argument.
#[test]
fn a_tainted_event_name_is_refused_at_the_call_site() {
    let refused = check_in_method(
        "tainted string $name = \"order\" as tainted string;\n\
         var $sse = Core\\Sse::stream();\n\
         $sse->send(\"sold\", event: $name);",
    );
    assert!(
        refused
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "`event:` accepted a tainted name: {refused:?}"
    );

    let accepted = check_in_method(
        "string $kind = \"order\";\n\
         var $sse = Core\\Sse::stream();\n\
         $sse->send(\"sold\", event: $kind);",
    );
    assert!(!accepted.has_errors(), "{accepted:?}");
}

/// The same rule over `Core\Sse->send`'s id, and it is a second test rather than
/// a second line in the one above because the *reason* is its own: a client
/// echoes the id back in `Last-Event-ID` and the handler that reads it resumes
/// from it, so an attacker-chosen id is a request for someone else's history.
#[test]
fn a_tainted_id_is_refused_at_the_call_site() {
    let refused = check_in_method(
        "tainted string $last = \"42\" as tainted string;\n\
         var $sse = Core\\Sse::stream();\n\
         $sse->send(\"sold\", id: $last);",
    );
    assert!(
        refused
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "`id:` accepted a tainted id: {refused:?}"
    );

    let accepted = check_in_method(
        "uint $seq = 42;\n\
         var $sse = Core\\Sse::stream();\n\
         $sse->send(\"sold\", id: \"e-\" . $seq);",
    );
    assert!(!accepted.has_errors(), "{accepted:?}");
}

/// The payload is the other side of that predicate, on the argument
/// `Core\Response::json` already makes: the framing belongs to the framer, which
/// normalizes a payload before it splits it into `data:` lines, so nothing a
/// program supplies can reach a field a client acts on.
///
/// Accepting it launders nothing, and that is the half a test asserting only the
/// acceptance would miss: `rule:security/taint-propagation` keeps the value
/// `tainted` across the call, so the same expression is still refused where a
/// plain `string` is expected.
#[test]
fn a_tainted_data_payload_is_accepted_and_stays_tainted() {
    let accepted = check_in_method(
        "tainted string $body = \"<b>sold</b>\" as tainted string;\n\
         var $sse = Core\\Sse::stream();\n\
         $sse->send($body);",
    );
    assert!(!accepted.has_errors(), "{accepted:?}");

    let still_tainted = check_in_method(
        "tainted string $body = \"<b>sold</b>\" as tainted string;\n\
         var $sse = Core\\Sse::stream();\n\
         $sse->send($body);\n\
         string $plain = $body;",
    );
    assert!(
        still_tainted
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "sending a payload laundered it: {still_tainted:?}"
    );
}
