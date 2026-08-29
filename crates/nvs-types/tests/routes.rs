//! ADR 0077 § 1's `#[Route]`: the nominal match that keeps a userland spelling
//! out of the route table, and the payload check that is the pass behind the
//! roster entry.
//!
//! The table now has rows, and the two errors that are questions about the
//! whole enumeration — a route declared twice and a `name` claimed twice — are
//! asserted here too, alongside § 2's path grammar and § 3's three questions
//! about the method an attribute is attached to. What it still owes is
//! `nvs_types::routes`' own gap list: nothing reverses the table, and a method's
//! second `#[Route]` is not a second row.

mod common;

use common::{check_src, check_src_table};
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
fn the_collected_table_crosses_on_the_expression_table() {
    // § 5's rows are collected here and reversed in `nvs-ir`, so what makes
    // them reachable at all is `ExprTypeTable::routes` — the channel every
    // other whole-program fact crosses by. Asserted through the table rather
    // than through a `url` call, because nothing reverses it yet.
    let (diags, exprs) = check_src_table(&route_src(
        "  #[Route(path: \"/users\", method: \\Core\\Http\\Method::Get, name: \"Users::index\")]\n  \
         public function index(): string { return \"\"; }\n  \
         #[Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Get, name: \"Users::show\")]\n  \
         public function show(uint $id): string { return \"\"; }\n  \
         #[Route(path: \"/health\", method: \\Core\\Http\\Method::Get)]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    let table = exprs.routes();
    assert_eq!(table.rows().len(), 3);
    // Load order, and declaration order within a file — the order § 3's
    // duplicate errors are reported in, so a consumer walking the rows sees
    // the same program the diagnostics described.
    let paths: Vec<&str> = table.rows().iter().map(|row| row.path.as_str()).collect();
    assert_eq!(paths, ["/users", "/users/{id}", "/health"]);

    // § 4 reverses the table by name, which is the one lookup a row is found
    // by; a route that claimed no `name` is in the table and is reachable by
    // nothing.
    let show = table.named("Users::show").expect("the named route");
    assert_eq!(show.path, "/users/{id}");
    assert_eq!(show.verb, "Get");
    assert_eq!(show.handler, "Users::show");
    assert!(table.named("Users::health").is_none());

    // A program declaring no route pays nothing and reads back as empty
    // rather than as absent.
    let (_, exprs) = check_src_table("<?nvs\necho \"\";\n");
    assert!(exprs.routes().rows().is_empty());
}

#[test]
fn a_path_begins_at_the_root_and_a_capture_is_a_whole_segment() {
    // § 2: everything that is not a capture is a literal segment compared byte
    // for byte, and a capture is a *whole* segment — there is no escape and no
    // partial form, so `u{id}` is one thing or the other and is neither.
    for path in [
        "users",
        "/users/u{id}",
        "/users/{id}.json",
        "/users/{}",
        "/users/{2id}",
        "/users/{id",
    ] {
        let diags = check_src(&route_src(&format!(
            "  #[Route(path: \"{path}\", method: \\Core\\Http\\Method::Get)]\n  \
             public function show(uint $id): string {{ return \"\"; }}\n"
        )));
        assert!(
            reported(&diags, code::E_ROUTE_PATH_GRAMMAR),
            "{path}: {diags:?}"
        );
    }
}

#[test]
fn a_capture_that_may_absorb_the_end_of_a_path_is_written_last() {
    // § 2 permits `{name?}` and `{name...}` in the last position only, and
    // both "at most once" and "never in one path together" fall out of that
    // rather than needing one of their own: one segment is last.
    for path in [
        "/posts/{page?}/comments",
        "/files/{rest...}/raw",
        "/posts/{a?}/{page?}",
        "/files/{a...}/{rest...}",
    ] {
        let diags = check_src(&route_src(&format!(
            "  #[Route(path: \"{path}\", method: \\Core\\Http\\Method::Get)]\n  \
             public function show(string $a = \"\", string $page = \"\", string $rest = \"\"): \
             string {{ return \"\"; }}\n"
        )));
        assert!(
            reported(&diags, code::E_ROUTE_PATH_GRAMMAR),
            "{path}: {diags:?}"
        );
    }
}

#[test]
fn a_capture_arrives_as_the_parameter_it_is_named_after() {
    // § 3: the comparison is exact, per ADR 0029, so `{userId}` and `$userid`
    // are two names and the capture has nowhere to arrive.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{userId}\", method: \\Core\\Http\\Method::Get)]\n  \
         public function show(uint $userid): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_ROUTE_CAPTURE_UNBOUND), "{diags:?}");

    // The reverse is deliberately fine: a parameter the path does not name is
    // simply not the router's.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Get)]\n  \
         public function show(uint $id, string $note = \"\"): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_captures_type_is_one_a_segment_converts_to() {
    // § 3's roster is `nvs_types::commands::converts_from_string` and there is
    // no second copy of it here, so an `array<int>` is refused at a capture for
    // the reason it is refused at an `#[Option]`.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{id}\", method: \\Core\\Http\\Method::Get)]\n  \
         public function show(array<int> $id): string { return \"\"; }\n",
    ));
    assert!(
        reported(&diags, code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION),
        "{diags:?}"
    );

    // A catch-all is the one capture that roster does not answer for: nothing
    // about it was checked, so § 3 hands it over as one `tainted string` and a
    // `uint` is not a type it can arrive at, however well `uint` converts.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/files/{rest...}\", method: \\Core\\Http\\Method::Get)]\n  \
         public function raw(uint $rest): string { return \"\"; }\n",
    ));
    assert!(
        reported(&diags, code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION),
        "{diags:?}"
    );

    let diags = check_src(&route_src(
        "  #[Route(path: \"/files/{rest...}\", method: \\Core\\Http\\Method::Get)]\n  \
         public function raw(string $rest): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_optional_capture_needs_a_default_to_be_absent_at() {
    // § 2: `{page?}` matches one whole segment or none, and the default is what
    // makes the absent case well-typed rather than nullable by accident.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/posts/{page?}\", method: \\Core\\Http\\Method::Get)]\n  \
         public function page(uint $page): string { return \"\"; }\n",
    ));
    assert!(
        reported(&diags, code::E_OPTIONAL_CAPTURE_NEEDS_DEFAULT),
        "{diags:?}"
    );

    let diags = check_src(&route_src(
        "  #[Route(path: \"/posts/{page?}\", method: \\Core\\Http\\Method::Get)]\n  \
         public function page(uint $page = 1): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_shape_keeps_the_capture_form_it_erases_the_name_of() {
    // § 2's precedence is structural — a `{name}` beats a `{name?}` — so the
    // two are two nodes of the trie and the pair has an answer that does not
    // depend on declaration order. A shape erasing the form would report them
    // as the duplicate they are not.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/posts/{page}\", method: \\Core\\Http\\Method::Get, name: \"a\")]\n  \
         public function one(uint $page): string { return \"\"; }\n  \
         #[Route(path: \"/posts/{page?}\", method: \\Core\\Http\\Method::Get, name: \"b\")]\n  \
         public function two(uint $page = 1): string { return \"\"; }\n",
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
