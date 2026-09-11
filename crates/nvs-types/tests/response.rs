//! `rule:security/response-body-is-one-typed-member`'s sixth row: `echo` and a `Core\Response` body member on one
//! response is a compile error (`E0801`).
//!
//! Asserted from both ends, because the rule is as much about what it leaves
//! alone as about what it refuses: inside a `#[Route]` handler the mix is
//! refused whichever order it is written in and wherever in the body it is
//! written, and outside one — a CLI program, where a body member's declaration
//! is inert — nothing here fires at all. `nvs_types::response`'s module doc
//! owns that scope and the entry-script gap it leaves.

mod common;

use common::check_src;
use nvs_diagnostics::{Diagnostics, code};

/// `rule:routing/matched-once-before-the-handler`'s request body, as small as it goes: one `#[Route]` handler
/// with `rule:attributes/access-is-a-required-sibling`'s required sibling decision.
fn handler(body: &str) -> String {
    format!(
        "<?nvs\nclass Pages {{\n  \
         #[Core\\Route(path: \"/p\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function show(): void {{\n{body}\n  }}\n}}\n"
    )
}

/// The same body with no attributes on it — a method a request never reaches.
fn plain_method(body: &str) -> String {
    format!("<?nvs\nclass Pages {{\n  public function show(): void {{\n{body}\n  }}\n}}\n")
}

/// Whether the refusal was reported. By code rather than `has_errors`, because
/// a fixture that stopped compiling for some other reason satisfies "an error
/// was reported" without asserting anything about this rule.
fn refused(diags: &Diagnostics) -> bool {
    diags
        .iter()
        .any(|d| d.code == Some(code::E_ECHO_BESIDE_A_BODY_MEMBER))
}

#[test]
fn mixing_echo_with_a_typed_body_member_is_a_compile_error() {
    // § 4's sixth row, in the order the ADR's own sentence writes it: the
    // handler has echoed a prelude and then declared the body a JSON document.
    let diags = check_src(&handler(
        "    echo \"prelude\";\n    Core\\Response::json(1);\n",
    ));
    assert!(refused(&diags), "{diags:?}");

    // The other order is the same mistake and is refused the same way — the
    // rule is that the body has two writers, not that one of them came second.
    let diags = check_src(&handler(
        "    Core\\Response::json(1);\n    echo \"epilogue\";\n",
    ));
    assert!(refused(&diags), "{diags:?}");

    // Each of § 4's three landed members, so the roster is asserted rather than
    // one member standing in for it.
    for call in [
        "Core\\Response::text(\"t\");",
        "Core\\Response::json(1);",
        "Core\\Response::bytes(\"b\" as bytes, \"application/octet-stream\");",
    ] {
        let diags = check_src(&handler(&format!("    echo \"prelude\";\n    {call}\n")));
        assert!(refused(&diags), "{call}: {diags:?}");
    }

    // A closure written in the handler writes the same response, and its body
    // is checked inline, so the two writers meet with nothing added for it.
    let diags = check_src(&handler(
        "    echo \"prelude\";\n    \
         var $f = fn (): void => { Core\\Response::text(\"t\"); };\n",
    ));
    assert!(refused(&diags), "{diags:?}");
}

/// A body written **over time** is still one body, so `echo` beside the member
/// that opens one is the same compile error — reached through the roster
/// already here rather than through a second rule
/// (`rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`).
///
/// Asserted in both orders and through the handle as well as the opener,
/// because the two are what a reader would expect to differ: the disagreement
/// is about the body the *handler* writes, and a member reached on a value
/// rather than on the class is the shape this rule had never met before.
#[test]
fn echo_beside_a_response_stream_is_a_diagnostic() {
    // The opener alone is enough: it declares the content type, which is the
    // half `echo` contradicts.
    let diags = check_src(&handler(
        "    echo \"prelude\";\n    var $s = Core\\Response::stream(\"text/csv\");\n",
    ));
    assert!(refused(&diags), "{diags:?}");

    // The other order is the same mistake, as it is for every other row.
    let diags = check_src(&handler(
        "    var $s = Core\\Response::stream(\"text/csv\");\n    echo \"epilogue\";\n",
    ));
    assert!(refused(&diags), "{diags:?}");

    // And a handler that writes chunks through the handle is refused for the
    // opener it must have called to get one — the write is not a second writer
    // to report, it is the same body arriving in pieces.
    let diags = check_src(&handler(
        "    echo \"prelude\";\n    \
         var $s = Core\\Response::stream(\"text/csv\");\n    $s->write(\"row\");\n",
    ));
    assert!(refused(&diags), "{diags:?}");

    // Outside a handler nothing fires, for the reason every other row leaves a
    // CLI program alone: there is no response for two writers to disagree over.
    let diags = check_src(&plain_method(
        "    echo \"prelude\";\n    var $s = Core\\Response::stream(\"text/csv\");\n",
    ));
    assert!(!refused(&diags), "{diags:?}");
}

#[test]
fn a_handler_that_writes_its_body_one_way_is_accepted() {
    // `echo` alone is § 4's last bullet — the inline-HTML page, which is the
    // shape this rule must not make harder to write.
    let diags = check_src(&handler("    echo \"a\";\n    echo \"b\";\n"));
    assert!(!diags.has_errors(), "{diags:?}");

    // One member alone, and then the same member twice: what § 4's row names is
    // `echo` beside a typed writer, so a handler writing its whole body through
    // the members is not this diagnostic's business however often it writes.
    let diags = check_src(&handler("    Core\\Response::text(\"a\");\n"));
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_src(&handler(
        "    Core\\Response::text(\"a\");\n    Core\\Response::text(\"b\");\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // A `Core\Response` member that is not one of § 4's body members says
    // nothing about the body — `isDraining` is a reader, and a reader beside an
    // `echo` is an ordinary handler.
    let diags = check_src(&handler(
        "    echo \"a\";\n    bool $d = Core\\Server::isDraining();\n    echo $d;\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_method_no_request_reaches_may_write_both() {
    // The scope decision, pinned: `rule:tooling/echo-always-has-a-sink` binds `echo` by context, so a
    // method with no `#[Route]` on it is a CLI body until something says
    // otherwise, and there is no response for two writers to disagree over.
    // The corpus depends on this — every `.nvst` case that observes a body
    // member is a script that also echoes.
    let diags = check_src(&plain_method(
        "    echo \"a\";\n    Core\\Response::text(\"b\");\n    echo \"c\";\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // And the state does not leak out of the handler that armed it: the same
    // pair written in a plain method *after* a handler in the same class is
    // still accepted, and the handler's own refusal is still made.
    let diags = check_src(
        "<?nvs\nclass Pages {\n  \
         #[Core\\Route(path: \"/p\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function show(): void {\n    \
         echo \"a\";\n    Core\\Response::text(\"b\");\n  }\n  \
         public function helper(): void {\n    \
         echo \"a\";\n    Core\\Response::text(\"b\");\n  }\n}\n",
    );
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_ECHO_BESIDE_A_BODY_MEMBER))
            .count(),
        1,
        "{diags:?}"
    );
}
