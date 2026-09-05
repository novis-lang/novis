//! ADR 0083's qualifiers at the connection boundary — what a peer sent arrives
//! as, and what a topic name may be built from.
//!
//! Both are ordinary checker questions rather than runtime ones, which is the
//! point: § 3's payload and § 4's name are decided by two registry rows
//! (`nvs_stdlib::socket`'s `Core\Socket\Message` and `nvs_stdlib::topic`'s
//! `Core\Topic`), so a program that would leak either does not compile. The
//! third qualifier rule at this boundary — a `secret` crossing § 2's `args:` or
//! § 4's publish — is in `isolates.rs`, beside the graph copy it shares with
//! `spawn script`.
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

/// ADR 0083 § 3: "a received frame's payload is `tainted` — it is untrusted
/// input arriving over a network, exactly like a request body".
///
/// The claim is a property of the *payload* and not of the member that answered
/// it, so it is asserted on both sides of one value: the same expression is
/// refused where a plain `string` is expected and accepted where ADR 0024 § 2's
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

    // ADR 0024 § 2's checked conversion is what laundering is spelled as
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

/// ADR 0083 § 4: "a topic name refuses `tainted` … a name derived from user
/// input is how one tenant subscribes to another's stream. A name is built from
/// checked values or it does not compile."
///
/// Asked of all three members that take a name, and asserted by **counting**
/// rather than off one line: the rule is a property of `Core\Topic`'s surface,
/// so a member that grew a neutral parameter of its own would still look right
/// on its own row. What carries it is `Qual::Sink` on each row, which reports
/// the ordinary mismatch a sink does — ADR 0088 § 1's classification, not a
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
