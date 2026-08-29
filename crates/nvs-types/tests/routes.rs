//! ADR 0077 § 1's `#[Route]`: the nominal match that keeps a userland spelling
//! out of the route table, and the payload check that is the pass behind the
//! roster entry.
//!
//! The table now has rows, and the two errors that are questions about the
//! whole enumeration — a route declared twice and a `name` claimed twice — are
//! asserted here too. What it still owes is `nvs_types::routes`' own gap list:
//! nothing reverses the table, and nothing reads the method an attribute is
//! attached to.

mod common;

use common::check_src;
use nvs_diagnostics::{Code, Diagnostics, code};

/// Whether `diags` reported `want`. Asserted by code rather than by
/// `has_errors`, because every fixture below writes a payload the roster walk
/// also has an opinion about, and "some error was reported" is satisfied by
/// the wrong one.
fn reported(diags: &Diagnostics, want: Code) -> bool {
    diags.iter().any(|d| d.code == Some(want))
}

/// § 1's own example, reduced to one controller method. The placing import is
/// per *name* — `use Core\Route;` aliases `Route`, and it is not the
/// `use Core\Router;` that reaches the member reversing the table.
fn route_src(attributes: &str) -> String {
    format!("<?nvs\nuse Core\\Route;\nclass Users {{\n{attributes}\n}}\n")
}

#[test]
fn a_route_is_matched_nominally_rather_than_as_a_shape() {
    // Fully qualified needs no import at all. Before the name joined
    // `nvs_types::derive::ATTRIBUTES` this was `E0726` — `Core\Route` is not a
    // `type` alias and was never going to be one, because § 1 builds a table
    // from it and a userland alias must not contribute a route.
    let diags = check_src(
        "<?nvs\nclass Users {\n  \
         #[\\Core\\Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Get, \
         name: \"Users::show\")]\n  \
         public function show(uint $id): string { return \"\"; }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    // The `use`d bare spelling is the same attribute, and the bare one with
    // nothing importing it resolves to `\Route` — no declaration at all, so it
    // is the ordinary undeclared-name refusal rather than a silently ignored
    // attribute.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", method: \\Core\\Http\\Method::Get)]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_src(
        "<?nvs\nclass Users {\n  #[Route(path: \"/health\", method: \\Core\\Http\\Method::Get)]\n  \
         public function health(): string { return \"\"; }\n}\n",
    );
    assert!(diags.has_errors());
}

#[test]
fn the_attribute_repeats_so_one_method_serves_two_verbs() {
    // § 1: no `methods: array<Method>` field and no union — ADR 0046 § 3's
    // repeatability is what already covers it, so two attributes on one method
    // are two payloads each checked on its own, and two rows of the table.
    // Two *verbs*, because the rows are what § 3's duplicate-route error is
    // over and one path served twice by the same verb is that error.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", method: \\Core\\Http\\Method::Get, name: \"health.get\")]\n  \
         #[Route(path: \"/health\", method: \\Core\\Http\\Method::Head, name: \"health.head\")]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn only_the_three_options_section_1_names_are_admitted() {
    // `methods` is the field a reader arriving from another framework writes,
    // and `route` is the same mistake one step along: the roster is what § 1
    // writes and nothing else, so a plausible field is still a typo.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", methods: \"GET\")]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(diags.has_errors());

    // `short` belongs to `#[Option]` and is no field of this one — the rosters
    // are per attribute rather than one pooled set.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", short: \"h\")]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(diags.has_errors());
}

#[test]
fn a_path_and_a_name_are_strings_and_each_is_written_once() {
    // A path that reads plausibly — `path: 1` — is refused at the value rather
    // than accepted and folded to something no route table could match.
    let diags = check_src(&route_src(
        "  #[Route(path: 1)]\n  public function health(): string { return \"\"; }\n",
    ));
    assert!(diags.has_errors());

    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", name: true)]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(diags.has_errors());

    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", path: \"/healthz\")]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(diags.has_errors());
}

#[test]
fn a_method_case_is_admitted_at_the_enum_the_roster_names() {
    // § 1's `method` is an enum case (ADR 0063 R11), which ADR 0046 § 2 admits
    // in a payload — so the fixture's own spelling passes the payload walk.
    // `Core\Http\Method` is `nvs_stdlib::router::METHOD`, seeded into the enum
    // table like any other `Core` enum, so the roster row interns to that type
    // and the case is placed at it rather than at nothing.
    let diags = check_src(
        "<?nvs\nclass Users {\n  \
         #[\\Core\\Route(path: \"/users\", method: \\Core\\Http\\Method::Get, name: \"Users::index\")]\n  \
         public function index(): string { return \"\"; }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_case_of_another_enum_is_refused_at_the_method_option() {
    // The half the row interning to a real type buys: before `Core\Http\Method`
    // existed, `method` was checked as a compile-time constant and placed at no
    // type, so any enum case at all passed. `Core\Digest` is an unrelated `Core`
    // enum, so this is that same walk with the type restored.
    let diags = check_src(
        "<?nvs\nclass Users {\n  \
         #[\\Core\\Route(path: \"/users\", method: \\Core\\Digest::Md5)]\n  \
         public function index(): string { return \"\"; }\n}\n",
    );
    assert!(diags.has_errors());
}

#[test]
fn a_row_needs_a_path_and_a_verb_and_says_which_is_missing() {
    // § 1 marks only `name` optional. The empty attribute is the shape the
    // userland alias it replaces would have refused, and it is refused by the
    // pass that would have to build a row out of it.
    let diags = check_src(&route_src(
        "  #[Route]\n  public function health(): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_ROUTE_INCOMPLETE), "{diags:?}");
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // One diagnostic for the empty attribute above, and one here — never one
    // per missing field, which is the same attribute reported twice.
    let diags = check_src(&route_src(
        "  #[Route(name: \"health\")]\n  public function health(): string { return \"\"; }\n",
    ));
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // A `path` written at the wrong type is that error and not this one: the
    // field is there, and naming it missing would report the author's second
    // problem ahead of their first.
    let diags = check_src(&route_src(
        "  #[Route(path: 1, method: \\Core\\Http\\Method::Get)]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(diags.has_errors());
    assert!(!reported(&diags, code::E_ROUTE_INCOMPLETE), "{diags:?}");
}

#[test]
fn one_route_shape_is_served_once_per_verb() {
    // § 2 matches by shape, so two captures differing only in name are the one
    // route both would match — the pair the written text would call different.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Get, name: \"a\")]\n  \
         public function show(uint $id): string { return \"\"; }\n  \
         #[Route(path: \"/users/{userId}\", method: \\Core\\Http\\Method::Get, name: \"b\")]\n  \
         public function other(uint $userId): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_DUPLICATE_ROUTE), "{diags:?}");

    // The same path under a different verb is a different route, and a
    // different literal segment is a different shape — the two halves that
    // stop this from being a rule about the path alone.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Get, name: \"a\")]\n  \
         public function show(uint $id): string { return \"\"; }\n  \
         #[Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Delete, name: \"b\")]\n  \
         public function drop(uint $id): string { return \"\"; }\n  \
         #[Route(path: \"/users/new\", method: \\Core\\Http\\Method::Get, name: \"c\")]\n  \
         public function fresh(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_route_name_names_one_route() {
    // § 4's `url` reverses the table by name, so a name meaning two routes is
    // a link with no answer — a question about the enumeration, which is why
    // it is asked once every file has been walked rather than per declaration.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users\", method: \\Core\\Http\\Method::Get, name: \"Users::show\")]\n  \
         public function index(): string { return \"\"; }\n  \
         #[Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Get, name: \"Users::show\")]\n  \
         public function show(uint $id): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_DUPLICATE_ROUTE_NAME), "{diags:?}");

    // Two rows with no `name` at all collide over nothing: § 1 leaves it
    // optional, and an absent name is not a name two routes share.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users\", method: \\Core\\Http\\Method::Get)]\n  \
         public function index(): string { return \"\"; }\n  \
         #[Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Get)]\n  \
         public function show(uint $id): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}
