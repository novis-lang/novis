//! `rule:routing/route-attribute`'s `#[Route]`: the nominal match that keeps a userland spelling
//! out of the route table, and the payload check that is the pass behind the
//! roster entry.
//!
//! The table now has rows, and the two errors that are questions about the
//! whole enumeration — a route declared twice and a `name` claimed twice — are
//! asserted here too, alongside § 2's path grammar and § 3's three questions
//! about the method an attribute is attached to. Those enumeration questions
//! are asked over **two files** through `common::check_program_table`, which is
//! the only shape that can tell "the program's table" apart from "this file's".
//!
//! `rule:routing/a-query-parameter-is-declared-like-a-capture`'s `#[Query]` is here too, and it is asserted from both ends: the
//! type list it shares with a capture, and § 6's `$params` key that names
//! neither one nor the other — the refusal the marker exists to make writable.
//!
//! `rule:security/access-is-checked-for-presence-not-meaning`'s `#[Access]` is asserted here from all three ends: the payload —
//! the decision is required, and it is a name rather than a value, wherever the
//! attribute is attached — § 1's *presence* rule, and § 1a's one-per-method
//! rule. Because presence is landed, every fixture above goes through
//! `with_access`, which supplies the one line those fixtures do not vary.
//!
//! The decision itself is asserted on the row rather than only as a refusal:
//! `rule:security/access-is-checked-for-presence-not-meaning` leaves enforcement to the dispatcher, so what the row carries
//! is the name the declaration resolved to.
//!
//! `rule:security/access-is-checked-for-presence-not-meaning` is asserted here whole: § 1's presence rule, § 1a's payload and
//! one-per-method rules, and § 4's opt-out held to a method that has something
//! to opt out of.

mod common;

use common::{check_program_table, check_src, check_src_table};
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
    format!(
        "<?nvs\nuse Core\\Route;\nclass Users {{\n{}\n}}\n",
        with_access(attributes)
    )
}

/// `rule:attributes/access-is-a-required-sibling`'s decision, supplied for every fixture here that is about
/// something else.
///
/// § 1 makes `#[Access]` a required sibling of `#[Route]`, so without this each
/// fixture below would carry one line it does not vary and one error it is not
/// asking about. `Core\Audience::Public` is the case § 1a names for a route
/// that is genuinely open. The presence rule itself is asserted by
/// `a_route_without_a_sibling_access_does_not_compile`, which is the one test
/// here that does not go through this.
fn with_access(members: &str) -> String {
    members.replace(
        "public function",
        "#[Core\\Access(allow: Core\\Audience::Public)]\n  public function",
    )
}

/// A user class implementing `Parses`, in the shape
/// `nvs_hir::interfaces::PARSES` spells the contract: one required
/// `parse(tainted string $s): static`, and the text kept in a field declared
/// `tainted` because a class carries no qualifier of its own
/// (`rule:security/tainted-qualifier`).
///
/// Every fixture using this writes its own `#[Core\Access]` rather than going
/// through [`with_access`], which would decorate this class's constructor too.
const SLUG: &str = "class Slug implements Parses {\n  \
                    public tainted string $text = \"\";\n  \
                    public function constructor(tainted string $text) { $this->text = $text; }\n  \
                    public static function parse(tainted string $s): static \
                    { return new static($s); }\n}\n";

#[test]
fn a_route_is_matched_nominally_rather_than_as_a_shape() {
    // Fully qualified needs no import at all. Before the name joined
    // `nvs_types::derive::ATTRIBUTES` this was `E0726` — `Core\Route` is not a
    // `type` alias and was never going to be one, because § 1 builds a table
    // from it and a userland alias must not contribute a route.
    let diags = check_src(&with_access(
        "<?nvs\nclass Users {\n  \
         #[Core\\Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get, \
         name: \"Users::show\")]\n  \
         public function show(uint $id): string { return \"\"; }\n}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // The `use`d bare spelling is the same attribute, and the bare one with
    // nothing importing it resolves to `\Route` — no declaration at all, so it
    // is the ordinary undeclared-name refusal rather than a silently ignored
    // attribute.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", method: Core\\Http\\Method::Get)]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_src(
        "<?nvs\nclass Users {\n  #[Route(path: \"/health\", method: Core\\Http\\Method::Get)]\n  \
         public function health(): string { return \"\"; }\n}\n",
    );
    assert!(diags.has_errors());
}

#[test]
fn the_attribute_repeats_so_one_method_serves_two_verbs() {
    // § 1: no `methods: array<Method>` field and no union — `rule:attributes/repeatable`'s
    // repeatability is what already covers it, so two attributes on one method
    // are two payloads each checked on its own, and two rows of the table.
    // Two *verbs*, because the rows are what § 3's duplicate-route error is
    // over and one path served twice by the same verb is that error.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", method: Core\\Http\\Method::Get, name: \"health.get\")]\n  \
         #[Route(path: \"/health\", method: Core\\Http\\Method::Head, name: \"health.head\")]\n  \
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
    // § 1's `method` is an enum case (`rule:core-api/shape-rules` R11), which `rule:attributes/payload-is-a-compile-time-constant` admits
    // in a payload — so the fixture's own spelling passes the payload walk.
    // `Core\Http\Method` is `nvs_stdlib::router::METHOD`, seeded into the enum
    // table like any other `Core` enum, so the roster row interns to that type
    // and the case is placed at it rather than at nothing.
    let diags = check_src(&with_access(
        "<?nvs\nclass Users {\n  \
         #[Core\\Route(path: \"/users\", method: Core\\Http\\Method::Get, name: \"Users::index\")]\n  \
         public function index(): string { return \"\"; }\n}\n",
    ));
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
         #[Core\\Route(path: \"/users\", method: Core\\Digest::Md5)]\n  \
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
        "  #[Route(path: 1, method: Core\\Http\\Method::Get)]\n  \
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
        "  #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get, name: \"a\")]\n  \
         public function show(uint $id): string { return \"\"; }\n  \
         #[Route(path: \"/users/{userId}\", method: Core\\Http\\Method::Get, name: \"b\")]\n  \
         public function other(uint $userId): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_DUPLICATE_ROUTE), "{diags:?}");

    // The same path under a different verb is a different route, and a
    // different literal segment is a different shape — the two halves that
    // stop this from being a rule about the path alone.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get, name: \"a\")]\n  \
         public function show(uint $id): string { return \"\"; }\n  \
         #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Delete, name: \"b\")]\n  \
         public function drop(uint $id): string { return \"\"; }\n  \
         #[Route(path: \"/users/new\", method: Core\\Http\\Method::Get, name: \"c\")]\n  \
         public function fresh(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn the_route_attribute_takes_slotted() {
    // `rule:core-classes/html-later`'s route field is an optional `bool`.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/blog\", method: Core\\Http\\Method::Get, slotted: true)]\n  \
         public function blog(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // A string is refused at the value, the way `name: true` is.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/blog\", method: Core\\Http\\Method::Get, slotted: \"yes\")]\n  \
         public function blog(): string { return \"\"; }\n",
    ));
    assert!(diags.has_errors());
}

#[test]
fn slotted_is_carried_to_the_route_table() {
    let (diags, exprs) = check_src_table(&route_src(
        "  #[Route(path: \"/blog\", method: Core\\Http\\Method::Get, slotted: true)]\n  \
         public function blog(): string { return \"\"; }\n  \
         #[Route(path: \"/shop\", method: Core\\Http\\Method::Get, slotted: false)]\n  \
         public function shop(): string { return \"\"; }\n  \
         #[Route(path: \"/health\", method: Core\\Http\\Method::Get)]\n  \
         public function health(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let slotted: Vec<(&str, bool)> = exprs
        .routes()
        .rows()
        .iter()
        .map(|row| (row.path.as_str(), row.slotted))
        .collect();
    // An attribute that writes no `slotted` is a normal route.
    assert_eq!(
        slotted,
        [("/blog", true), ("/shop", false), ("/health", false)]
    );
}

#[test]
fn the_collected_table_crosses_on_the_expression_table() {
    // § 5's rows are collected here and reversed in `nvs-ir`, so what makes
    // them reachable at all is `ExprTypeTable::routes` — the channel every
    // other whole-program fact crosses by. Asserted through the table rather
    // than through a `url` call, because nothing reverses it yet.
    let (diags, exprs) = check_src_table(&route_src(
        "  #[Route(path: \"/users\", method: Core\\Http\\Method::Get, name: \"Users::index\")]\n  \
         public function index(): string { return \"\"; }\n  \
         #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get, name: \"Users::show\")]\n  \
         public function show(uint $id): string { return \"\"; }\n  \
         #[Route(path: \"/health\", method: Core\\Http\\Method::Get)]\n  \
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
fn every_route_on_one_method_becomes_a_row_and_they_may_share_a_name() {
    // `rule:attributes/repeatable`'s repetition, read as `rule:routing/route-attribute` writes it: one method
    // serving three verbs declares three routes rather than one. `rule:routing/repeated-routes-share-a-name-when-they-share-a-path`
    // then lets them carry the same `name`, because they carry the same path
    // and `url()` therefore has one answer to give.
    let webhook = |name: &str| {
        format!(
            "  #[Route(path: \"/webhook\", method: Core\\Http\\Method::Post, name: \"{name}\")]\n  \
             #[Route(path: \"/webhook\", method: Core\\Http\\Method::Put, name: \"{name}\")]\n  \
             #[Route(path: \"/webhook\", method: Core\\Http\\Method::Delete, name: \"{name}\")]\n  \
             public function receive(): string {{ return \"\"; }}\n"
        )
    };
    let (diags, exprs) = check_src_table(&route_src(&webhook("webhook")));
    assert!(!diags.has_errors(), "{diags:?}");
    let table = exprs.routes();
    assert_eq!(table.rows().len(), 3);
    let verbs: Vec<&str> = table.rows().iter().map(|row| row.verb.as_str()).collect();
    assert_eq!(verbs, ["Post", "Put", "Delete"]);

    // Every row is the same method's, and every one carries `rule:attributes/access-is-a-required-sibling`'s
    // decision: the sibling `#[Access]` is asked once and answers for all of
    // them, which is why one attribute covering three routes is not a hole.
    for row in table.rows() {
        assert_eq!(row.handler, "Users::receive");
        assert_eq!(row.access.as_deref(), Some("Core\\Audience::Public"));
    }

    // § 1's exception has two halves and needs both. The same name on two
    // *methods* is the copy-paste the original rule was written for.
    let (diags, _) = check_src_table(&route_src(&format!(
        "{}  #[Route(path: \"/other\", method: Core\\Http\\Method::Get, name: \"webhook\")]\n  \
         public function other(): string {{ return \"\"; }}\n",
        webhook("webhook")
    )));
    assert!(reported(&diags, code::E_DUPLICATE_ROUTE_NAME), "{diags:?}");

    // And one method whose repetitions carry different paths is the ambiguity
    // itself: `url()` would have two answers and no ground to prefer one.
    let (diags, _) = check_src_table(&route_src(
        "  #[Route(path: \"/webhook\", method: Core\\Http\\Method::Post, name: \"webhook\")]\n  \
         #[Route(path: \"/hook\", method: Core\\Http\\Method::Post, name: \"webhook\")]\n  \
         public function receive(): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_DUPLICATE_ROUTE_NAME), "{diags:?}");

    // The duplicate-*route* rule is untouched by that exception: two
    // attributes sharing both `path` and `method` are one route however they
    // are grouped, so nothing enters through the door a shared name opens.
    let (diags, _) = check_src_table(&route_src(
        "  #[Route(path: \"/webhook\", method: Core\\Http\\Method::Post)]\n  \
         #[Route(path: \"/webhook\", method: Core\\Http\\Method::Post)]\n  \
         public function receive(): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_DUPLICATE_ROUTE), "{diags:?}");
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
            "  #[Route(path: \"{path}\", method: Core\\Http\\Method::Get)]\n  \
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
            "  #[Route(path: \"{path}\", method: Core\\Http\\Method::Get)]\n  \
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
    // § 3: the comparison is exact, per `rule:core-api/identifier-casing`, so `{userId}` and `$userid`
    // are two names and the capture has nowhere to arrive.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{userId}\", method: Core\\Http\\Method::Get)]\n  \
         public function show(uint $userid): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_ROUTE_CAPTURE_UNBOUND), "{diags:?}");

    // The reverse is deliberately fine: a parameter the path does not name is
    // simply not the router's.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get)]\n  \
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
        "  #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get)]\n  \
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
        "  #[Route(path: \"/files/{rest...}\", method: Core\\Http\\Method::Get)]\n  \
         public function raw(uint $rest): string { return \"\"; }\n",
    ));
    assert!(
        reported(&diags, code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION),
        "{diags:?}"
    );

    let diags = check_src(&route_src(
        "  #[Route(path: \"/files/{rest...}\", method: Core\\Http\\Method::Get)]\n  \
         public function raw(string $rest): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_optional_capture_needs_a_default_to_be_absent_at() {
    // § 2: `{page?}` matches one whole segment or none, and the default is what
    // makes the absent case well-typed rather than nullable by accident.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/posts/{page?}\", method: Core\\Http\\Method::Get)]\n  \
         public function page(uint $page): string { return \"\"; }\n",
    ));
    assert!(
        reported(&diags, code::E_OPTIONAL_CAPTURE_NEEDS_DEFAULT),
        "{diags:?}"
    );

    let diags = check_src(&route_src(
        "  #[Route(path: \"/posts/{page?}\", method: Core\\Http\\Method::Get)]\n  \
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
        "  #[Route(path: \"/posts/{page}\", method: Core\\Http\\Method::Get, name: \"a\")]\n  \
         public function one(uint $page): string { return \"\"; }\n  \
         #[Route(path: \"/posts/{page?}\", method: Core\\Http\\Method::Get, name: \"b\")]\n  \
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
        "  #[Route(path: \"/users\", method: Core\\Http\\Method::Get, name: \"Users::show\")]\n  \
         public function index(): string { return \"\"; }\n  \
         #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get, name: \"Users::show\")]\n  \
         public function show(uint $id): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_DUPLICATE_ROUTE_NAME), "{diags:?}");

    // Two rows with no `name` at all collide over nothing: § 1 leaves it
    // optional, and an absent name is not a name two routes share.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users\", method: Core\\Http\\Method::Get)]\n  \
         public function index(): string { return \"\"; }\n  \
         #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get)]\n  \
         public function show(uint $id): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// How many diagnostics reported `want` — the count, where "at all" is
/// satisfied by a pass that reports the same collision once per row it walks
/// afterwards.
fn count(diags: &Diagnostics, want: Code) -> usize {
    diags.iter().filter(|d| d.code == Some(want)).count()
}

#[test]
fn a_route_table_is_built_from_the_program_enumeration() {
    // § 5: the table is the *program*'s, filled by the walk over every file
    // the entry reaches, so a link written in one file resolves against a row
    // declared in another. Asserted over two files rather than one, because a
    // single source cannot tell "the table is the program's" apart from "the
    // table is this file's".
    let (diags, exprs) = check_program_table(&[
        (
            "table-main.nvs",
            "<?nvs\nrequire 'table-users.nvs';\nclass Health {\n  \
             #[Core\\Route(path: \"/health\", method: Core\\Http\\Method::Get, \
             name: \"Health::show\")]\n  \
             #[Core\\Access(allow: Core\\Audience::Public)]\n  \
             public function show(): string { return \"\"; }\n}\n\
             echo Core\\Router::url(\"Users::show\", [\"id\" => 1]), \"\\n\";\n",
        ),
        (
            "table-users.nvs",
            "<?nvs\nclass Users {\n  \
             #[Core\\Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get, \
             name: \"Users::show\")]\n  \
             #[Core\\Access(allow: Core\\Audience::Public)]\n  \
             public function show(uint $id): string { return \"\"; }\n}\n",
        ),
    ]);
    // The link in the entry file names a route declared in the file it
    // requires: no `E0754`, which is the whole-program half of § 4 asserted
    // from the side that would fail if the walk stopped at the entry.
    assert!(!diags.has_errors(), "{diags:?}");

    let table = exprs.routes();
    // Entry first, then each required file — `nvs_hir::resolve_program`'s load
    // order, which is what makes a duplicate report at a deterministic row.
    let paths: Vec<&str> = table.rows().iter().map(|row| row.path.as_str()).collect();
    assert_eq!(paths, ["/health", "/users/{id}"]);
    let show = table
        .named("Users::show")
        .expect("the required file's route");
    assert_eq!(show.handler, "Users::show");
}

#[test]
fn a_route_parameter_takes_its_type_from_the_method_that_declares_it() {
    // § 3: a capture is typed by the parameter it names, so the roster of
    // types a segment converts to is asked of the *declaration* rather than of
    // the path — the four spellings below are one path checked four ways.
    for ty in ["uint", "int", "string", "bool"] {
        let diags = check_src(&route_src(&format!(
            "  #[Route(path: \"/users/{{id}}\", method: Core\\Http\\Method::Get)]\n  \
             public function show({ty} $id): string {{ return \"\"; }}\n"
        )));
        assert!(!diags.has_errors(), "{ty}: {diags:?}");
    }

    // And it is the method's own parameter, not the capture's name pooled
    // across the program: two routes capturing `{id}` are two questions, and
    // only the one whose own parameter is outside the roster is refused.
    let diags = check_src(&with_access(
        "<?nvs\nuse Core\\Route;\nclass Users {\n  \
         #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get)]\n  \
         public function show(uint $id): string { return \"\"; }\n}\n\
         class Orders {\n  \
         #[Route(path: \"/orders/{id}\", method: Core\\Http\\Method::Get)]\n  \
         public function show(float $id): string { return \"\"; }\n}\n",
    ));
    assert_eq!(
        count(&diags, code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION),
        1,
        "{diags:?}"
    );
}

#[test]
fn a_duplicate_route_is_a_diagnostic() {
    // § 3's duplicate is a question about the whole enumeration, so two
    // classes in two *files* collide exactly as two methods of one class do —
    // the case one written in a single source cannot make.
    let (diags, _) = check_program_table(&[
        (
            "dup-main.nvs",
            "<?nvs\nrequire 'dup-other.nvs';\nclass Users {\n  \
             #[Core\\Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get, \
             name: \"a\")]\n  \
             #[Core\\Access(allow: Core\\Audience::Public)]\n  \
             public function show(uint $id): string { return \"\"; }\n}\n",
        ),
        (
            "dup-other.nvs",
            "<?nvs\nclass Admin {\n  \
             #[Core\\Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get, \
             name: \"b\")]\n  \
             #[Core\\Access(allow: Core\\Audience::Public)]\n  \
             public function show(uint $id): string { return \"\"; }\n}\n",
        ),
    ]);
    // Once, not once per row walked afterwards: the collision is one fact,
    // reported against the row that arrived second.
    assert_eq!(count(&diags, code::E_DUPLICATE_ROUTE), 1, "{diags:?}");
}

#[test]
fn a_path_capture_with_no_matching_method_parameter_is_a_diagnostic() {
    // § 3: every capture has somewhere to arrive, so a method declaring no
    // parameters at all is the shortest way to have nowhere.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get)]\n  \
         public function show(): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_ROUTE_CAPTURE_UNBOUND), "{diags:?}");

    // Each capture is asked on its own rather than the path being asked once:
    // a method binding the first of two is still missing the second.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/orders/{orderId}/lines/{lineId}\", \
         method: Core\\Http\\Method::Get)]\n  \
         public function line(uint $orderId): string { return \"\"; }\n",
    ));
    assert_eq!(count(&diags, code::E_ROUTE_CAPTURE_UNBOUND), 1, "{diags:?}");

    let diags = check_src(&route_src(
        "  #[Route(path: \"/orders/{orderId}/lines/{lineId}\", \
         method: Core\\Http\\Method::Get)]\n  \
         public function line(uint $orderId, uint $lineId): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_unknown_literal_url_name_is_a_diagnostic() {
    // § 4's first refusal, asked here rather than only in the reject case,
    // because it is the one link error that fires with no `$params` question
    // in front of it.
    let src = |name: &str| {
        with_access(&format!(
            "<?nvs\nclass Users {{\n  \
             #[Core\\Route(path: \"/users/{{id}}\", method: Core\\Http\\Method::Get, \
             name: \"Users::show\")]\n  \
             public function show(uint $id): string {{ return \"\"; }}\n}}\n\
             echo Core\\Router::url({name}, [\"id\" => 1]), \"\\n\";\n"
        ))
    };
    let diags = check_src(&src("\"Users::missing\""));
    assert!(reported(&diags, code::E_UNKNOWN_ROUTE_NAME), "{diags:?}");

    // The name the table does claim resolves, and is the fold rather than a
    // call left standing.
    let diags = check_src(&src("\"Users::show\""));
    assert!(!diags.has_errors(), "{diags:?}");

    // A *computed* name is not this error: nothing is read, so nothing is
    // checked and the member's own body throws at run time. The pair is what
    // keeps § 4's literal/computed split from being a claim about one half.
    let diags = check_src(&format!(
        "<?nvs\nstring $name = \"Users::missing\";\n{}",
        src("$name").trim_start_matches("<?nvs\n")
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_named_link_argument_is_folded_like_a_positional_one() {
    // `rule:core-api/parameters-are-callable-by-name`: `$name` and `$params`
    // are filled by the names the spec publishes as surely as by position, and
    // the call's own argument mapping is what says so — so § 4's refusals are
    // the same refusals when the link is written that way, and a link that
    // throws at run time is a computed name and nothing else.
    let src = |link: &str| {
        with_access(&format!(
            "<?nvs\nclass Users {{\n  \
             #[Core\\Route(path: \"/users/{{id}}\", method: Core\\Http\\Method::Get, \
             name: \"Users::show\")]\n  \
             public function show(uint $id): string {{ return \"\"; }}\n}}\n\
             echo Core\\Router::url({link}), \"\\n\";\n"
        ))
    };
    let diags = check_src(&src("name: \"Users::missing\", params: [\"id\" => 1]"));
    assert!(reported(&diags, code::E_UNKNOWN_ROUTE_NAME), "{diags:?}");

    // The `$params` half through the same mapping, and written *first*: a key
    // that is neither a capture nor a `#[Query]` parameter is
    // `rule:routing/a-leftover-link-key-is-a-query-string`'s refusal, and
    // reaching it means both arguments were read at the parameters they filled
    // rather than at the positions they were written in.
    let diags = check_src(&src(
        "params: [\"id\" => 1, \"nope\" => 2], name: \"Users::show\"",
    ));
    assert!(
        reported(&diags, code::E_ROUTE_LINK_UNKNOWN_PARAM),
        "{diags:?}"
    );

    // And the link that is right, written entirely by name: no diagnostic, so
    // the pair above is a refusal of the *name* and not of the spelling.
    let diags = check_src(&src("name: \"Users::show\", params: [\"id\" => 1]"));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_optional_capture_outside_the_last_position_is_a_diagnostic() {
    // `rule:routing/a-trailing-segment-may-be-absent`: `{name?}` matches a segment or none, and "or none" only
    // has an answer where nothing follows it — a literal segment after one is
    // as unreachable as another capture.
    for path in ["/posts/{page?}/comments", "/posts/{page?}/{id}"] {
        let diags = check_src(&route_src(&format!(
            "  #[Route(path: \"{path}\", method: Core\\Http\\Method::Get)]\n  \
             public function show(uint $page = 1, uint $id = 0): string {{ return \"\"; }}\n"
        )));
        assert!(
            reported(&diags, code::E_ROUTE_PATH_GRAMMAR),
            "{path}: {diags:?}"
        );
    }

    // The other side of the same bound: the identical capture, last, is the
    // form § 4 admits.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/posts/comments/{page?}\", method: Core\\Http\\Method::Get)]\n  \
         public function show(uint $page = 1): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_optional_capture_bound_to_a_parameter_with_no_default_is_a_diagnostic() {
    // `rule:routing/a-trailing-segment-may-be-absent`: what makes the absent segment well-typed is the parameter's
    // *default*, so the refusal is about the default and not about the type —
    // both spellings below convert from a segment and both are still refused.
    for ty in ["uint", "string"] {
        let diags = check_src(&route_src(&format!(
            "  #[Route(path: \"/posts/{{page?}}\", method: Core\\Http\\Method::Get)]\n  \
             public function page({ty} $page): string {{ return \"\"; }}\n"
        )));
        assert!(
            reported(&diags, code::E_OPTIONAL_CAPTURE_NEEDS_DEFAULT),
            "{ty}: {diags:?}"
        );
    }

    let diags = check_src(&route_src(
        "  #[Route(path: \"/posts/{page?}\", method: Core\\Http\\Method::Get)]\n  \
         public function page(string $page = \"1\"): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_capture_or_query_parameter_outside_the_type_list_is_a_diagnostic() {
    // `rule:routing/a-query-parameter-is-declared-like-a-capture` is a closed list, so the refusal is written from the first
    // type *outside* it rather than from an implausible one: a `float`
    // converts from a segment in every language that guesses, and § 3 does not
    // guess (`rule:errors/ambiguous-input-refused`). A nullable is the other near miss — `?uint` is not
    // `uint`, and an absent segment is § 4's question rather than this one.
    //
    for ty in ["float", "?uint", "array<int>"] {
        let diags = check_src(&route_src(&format!(
            "  #[Route(path: \"/users/{{id}}\", method: Core\\Http\\Method::Get)]\n  \
             public function show({ty} $id): string {{ return \"\"; }}\n"
        )));
        assert!(
            reported(&diags, code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION),
            "{ty}: {diags:?}"
        );
    }

    let diags = check_src(&route_src(
        "  #[Route(path: \"/users/{id}\", method: Core\\Http\\Method::Get)]\n  \
         public function show(uint $id): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // The `#[Query]` half of the same rule, and the reason this test is named
    // for both: § 3 gives a query parameter the *same* list, so the same three
    // types are refused at the same code. The route below declares no capture
    // at all, which is what makes this the attribute's question rather than the
    // path's.
    for ty in ["float", "?uint", "array<int>"] {
        let diags = check_src(&route_src(&format!(
            "  #[Route(path: \"/users\", method: Core\\Http\\Method::Get)]\n  \
             public function index(#[Core\\Query] {ty} $page): string {{ return \"\"; }}\n"
        )));
        assert!(
            reported(&diags, code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION),
            "{ty}: {diags:?}"
        );
    }

    // § 3's own example, spelled bare: the name is matched nominally after the
    // `use` that places it, exactly as `#[Route]` is, and both of its declared
    // types are on the list.
    let diags = check_src(&with_access(
        "<?nvs\nuse Core\\Route;\nuse Core\\Query;\nclass Orders {\n  \
         #[Route(path: \"/orders\", method: Core\\Http\\Method::Get)]\n  \
         public function index(#[Query] uint $page = 1, #[Query] string $sort = \"asc\"): \
         string { return \"\"; }\n}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_url_key_that_is_neither_a_capture_nor_a_query_parameter_is_a_diagnostic() {
    // `rule:routing/a-leftover-link-key-is-a-query-string`: every key that is not a capture *becomes* the link's query
    // string, so a key naming nothing at all is not inert — it ships as
    // `?pge=2` and nothing says so. That is the whole reason the refusal exists,
    // and it is why it could not be written before `#[Query]` did: refusing
    // every non-capture key would refuse the query strings the same sentence
    // requires.
    let src = |params: &str| {
        with_access(&format!(
            "<?nvs\nuse Core\\Query;\nclass Orders {{\n  \
             #[Core\\Route(path: \"/orders/{{id}}\", method: Core\\Http\\Method::Get, \
             name: \"orders.show\")]\n  \
             public function show(uint $id, #[Query] uint $page = 1, string $sort = \"asc\"): \
             string {{ return \"\"; }}\n\
             }}\necho Core\\Router::url(\"orders.show\", {params}), \"\\n\";\n"
        ))
    };
    let diags = check_src(&src("[\"id\" => 1, \"pge\" => 2]"));
    assert!(
        reported(&diags, code::E_ROUTE_LINK_UNKNOWN_PARAM),
        "{diags:?}"
    );

    // The two things a key may be, asked in one link: the capture, and the
    // declared `#[Query]` parameter that is the whole point of the rule.
    let diags = check_src(&src("[\"id\" => 1, \"page\" => 2]"));
    assert!(!diags.has_errors(), "{diags:?}");

    // A key naming a parameter the handler declares but did not mark is not a
    // query parameter — the marker is the declaration, and nothing is inferred
    // from the parameter list.
    let diags = check_src(&src("[\"id\" => 1, \"sort\" => \"asc\"]"));
    assert!(
        reported(&diags, code::E_ROUTE_LINK_UNKNOWN_PARAM),
        "{diags:?}"
    );

    // Only the first of the two link errors is reported for one call: a
    // misspelled capture is *both* a capture with no key and a key with nothing
    // to be, and the missing name is the half that says what to write.
    let diags = check_src(&src("[\"idd\" => 1]"));
    assert!(
        reported(&diags, code::E_ROUTE_LINK_MISSING_PARAM),
        "{diags:?}"
    );
    assert!(
        !reported(&diags, code::E_ROUTE_LINK_UNKNOWN_PARAM),
        "{diags:?}"
    );
}

/// `rule:attributes/access-is-a-required-sibling`'s own pairing with a payload left to vary: the attribute on the
/// same method as the `#[Route]` it is the sibling of, over an unsafe verb so
/// that § 4's `csrf` opt-out has something to opt out of.
///
/// The decisions are a userland enum and a userland class constant, which is
/// what § 1a says every application writes. `Core\Audience::Public` — the one
/// decision `Core` names, and the fixture `with_access` supplies — is the other
/// half, and it resolves like any other name.
fn access_src(payload: &str) -> String {
    format!(
        "<?nvs\nenum Role {{ Admin, Owner }}\nclass Policy {{\n  \
         public const string ADMIN = \"admin\";\n}}\nclass Users {{\n  \
         #[Core\\Route(path: \"/admin\", method: Core\\Http\\Method::Post)]\n  \
         #[Core\\Access({payload})]\n  \
         public function admin(): string {{ return \"\"; }}\n}}\n"
    )
}

#[test]
fn an_access_decision_is_required_and_is_a_name() {
    // § 1a marks only `csrf` optional. An enum case is what the section's own
    // examples write, and a class constant is the other half of its narrowing —
    // one syntactic form, so the check asks one question rather than two.
    let diags = check_src(&access_src("allow: Role::Admin"));
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_src(&access_src("allow: Policy::ADMIN"));
    assert!(!diags.has_errors(), "{diags:?}");

    // A payload carrying only the opt-out declares nothing to opt out of: the
    // attribute is there and the decision is not, which is § 3's omission one
    // step along rather than a different mistake.
    let diags = check_src(&access_src("csrf: false"));
    assert!(reported(&diags, code::E_ACCESS_INCOMPLETE), "{diags:?}");
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // The literal is the whole point of the rule: `mixed` gives the value no
    // type to fail against, so a string that reads exactly like a role would
    // otherwise be a decision nothing resolves.
    let diags = check_src(&access_src("allow: \"admin\""));
    assert!(
        reported(&diags, code::E_ACCESS_ALLOW_NOT_A_NAME),
        "{diags:?}"
    );
    let diags = check_src(&access_src("allow: 1"));
    assert!(
        reported(&diags, code::E_ACCESS_ALLOW_NOT_A_NAME),
        "{diags:?}"
    );
}

#[test]
fn csrf_false_on_a_route_whose_every_verb_is_safe_does_not_compile() {
    // §§ 1a and 4: CSRF is on for the four unsafe verbs and for no other, so
    // beside a `Get` the opt-out turns nothing off and is refused rather than
    // ignored.
    let safe = "<?nvs\nclass Users {\n  \
                #[Core\\Route(path: \"/users\", method: Core\\Http\\Method::Get)]\n  \
                #[Core\\Access(allow: Core\\Audience::Public, csrf: false)]\n  \
                public function index(): string { return \"\"; }\n}\n";
    let diags = check_src(safe);
    assert!(
        reported(&diags, code::E_CSRF_WITHOUT_UNSAFE_VERB),
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // The same payload over an unsafe verb is § 4's whole point — `access_src`
    // writes `Post` for exactly this reason.
    let diags = check_src(&access_src("allow: Role::Admin, csrf: false"));
    assert!(!diags.has_errors(), "{diags:?}");

    // Only `false` is refused: `csrf: true` on a safe route restates the
    // default rather than claiming anything untrue.
    let diags = check_src(&safe.replace("csrf: false", "csrf: true"));
    assert!(!diags.has_errors(), "{diags:?}");

    // And an `#[Access]` that never mentions the field is the ordinary case.
    let diags = check_src(&safe.replace(", csrf: false", ""));
    assert!(!diags.has_errors(), "{diags:?}");

    // The verbs are the *method's*, not the row's: one unsafe `#[Route]` among
    // several is a check to opt out of, and the refusal is not repeated once
    // per row either.
    let diags = check_src(
        "<?nvs\nclass Users {\n  \
         #[Core\\Route(path: \"/users\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Route(path: \"/users\", method: Core\\Http\\Method::Post)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public, csrf: false)]\n  \
         public function index(): string { return \"\"; }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_second_access_on_one_method_is_refused_naming_both() {
    // § 1a's last rule, over `rule:attributes/repeatable`'s general repeatability: two
    // decisions are two readings — every one of them, or any one of them — and
    // the compiler refuses to pick one silently.
    let two = |members: &str| {
        format!("<?nvs\nenum Role {{ Admin, Owner }}\nclass Users {{\n{members}}}\n")
    };
    let route = "  #[Core\\Route(path: \"/admin\", method: Core\\Http\\Method::Get)]\n  \
                 #[Core\\Access(allow: Role::Admin)]\n  \
                 #[Core\\Access(allow: Role::Owner)]\n  \
                 public function admin(): string { return \"\"; }\n";
    let diags = check_src(&two(route));
    assert!(reported(&diags, code::E_ACCESS_REPEATED), "{diags:?}");
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // `rule:attributes/attach-sites-and-forms`'s group form is the same two attributes written with one
    // pair of brackets, so the rule is read off the flattened list rather than
    // off the groups.
    let diags = check_src(&two(
        "  #[Core\\Route(path: \"/admin\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Role::Admin), Core\\Access(allow: Role::Owner)]\n  \
         public function admin(): string { return \"\"; }\n",
    ));
    assert!(reported(&diags, code::E_ACCESS_REPEATED), "{diags:?}");

    // A third is reported too, each against the first, so an author deleting
    // the extras is told about all of them in one build.
    let diags = check_src(&two(
        "  #[Core\\Route(path: \"/admin\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Role::Admin)]\n  \
         #[Core\\Access(allow: Role::Owner)]\n  \
         #[Core\\Access(allow: Role::Admin)]\n  \
         public function admin(): string { return \"\"; }\n",
    ));
    assert_eq!(diags.error_count(), 2, "{diags:?}");

    // The rule is per method, not per class: two methods each declaring one
    // decision is the ordinary program § 1 asks for.
    let diags = check_src(&two(&format!(
        "{route_one}{route_two}",
        route_one = "  #[Core\\Route(path: \"/admin\", method: Core\\Http\\Method::Get)]\n  \
                     #[Core\\Access(allow: Role::Admin)]\n  \
                     public function admin(): string { return \"\"; }\n",
        route_two = "  #[Core\\Route(path: \"/owner\", method: Core\\Http\\Method::Get)]\n  \
                     #[Core\\Access(allow: Role::Owner)]\n  \
                     public function owner(): string { return \"\"; }\n",
    )));
    assert!(!diags.has_errors(), "{diags:?}");

    // And it is asked of every method rather than only of a route's: an
    // `#[Access]` on a method with no `#[Route]` is still a decision something
    // will read, so two of them there are the same two readings.
    let diags = check_src(&two("  #[Core\\Access(allow: Role::Admin)]\n  \
         #[Core\\Access(allow: Role::Owner)]\n  \
         public function plain(): string { return \"\"; }\n"));
    assert!(reported(&diags, code::E_ACCESS_REPEATED), "{diags:?}");
}

#[test]
fn the_access_decision_rides_on_the_row_as_the_name_it_resolves_to() {
    // `rule:security/access-is-checked-for-presence-not-meaning` puts enforcement in the dispatcher, so the decision the
    // compiler guarantees was *written* has to reach it — on the row, the way
    // `query` does, because by the time anything dispatches, the attribute is
    // in a file this walk has long moved past.
    let (diags, exprs) = check_src_table(&access_src("allow: Role::Admin"));
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(
        exprs.routes().rows()[0].access.as_deref(),
        Some("Role::Admin")
    );

    // A class constant is the other half of § 1a's narrowing and rides
    // identically, because § 2 never asks which of the two forms a name is.
    let (diags, exprs) = check_src_table(&access_src("allow: Policy::ADMIN"));
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(
        exprs.routes().rows()[0].access.as_deref(),
        Some("Policy::ADMIN")
    );

    // Resolved rather than as written: `Audience` is the `use`d spelling, and
    // a consumer holding rows from several files cannot re-read the imports
    // each of them was written under.
    let (diags, exprs) = check_src_table(
        "<?nvs\nuse Core\\Audience;\nclass Home {\n  \
         #[Core\\Route(path: \"/\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Audience::Public)]\n  \
         public function home(): string { return \"\"; }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(
        exprs.routes().rows()[0].access.as_deref(),
        Some("Core\\Audience::Public")
    );

    // A decision that was refused leaves the row carrying none: `None` is the
    // shape of "already reported", and a program that compiles has no row in
    // it.
    let (diags, exprs) = check_src_table(&access_src("allow: \"admin\""));
    assert!(
        reported(&diags, code::E_ACCESS_ALLOW_NOT_A_NAME),
        "{diags:?}"
    );
    assert!(exprs.routes().rows()[0].access.is_none());
}

#[test]
fn an_empty_decision_is_refused_wherever_the_attribute_is_written() {
    // The payload walk is `rule:attributes/attach-sites-and-forms`'s per-attach-site one, so an `#[Access]`
    // declaring nothing is refused with no `#[Route]` in front of it. The two
    // halves of § 1 are separable that way round: this is the attribute failing
    // on its own terms, not a route missing its sibling.
    let diags = check_src(
        "<?nvs\nclass Users {\n  #[Core\\Access]\n  \
         public function admin(): string { return \"\"; }\n}\n",
    );
    assert!(reported(&diags, code::E_ACCESS_INCOMPLETE), "{diags:?}");
}

#[test]
fn only_the_two_options_section_1a_names_are_admitted() {
    // `role` is the field a reader writes when they read the attribute as
    // naming who is allowed rather than what the decision is. § 2 refuses to
    // grow a roster of those, so a plausible field is still a typo.
    let diags = check_src(&access_src("allow: Role::Admin, role: Role::Owner"));
    assert!(reported(&diags, code::E_UNKNOWN_OPTION), "{diags:?}");

    // `csrf` is § 4's opt-out and the roster does place it at a type — the
    // half `allow` gives up by being `mixed`, kept here because a `bool` says
    // everything a per-route opt-out means.
    let diags = check_src(&access_src("allow: Role::Admin, csrf: false"));
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_src(&access_src("allow: Role::Admin, csrf: \"no\""));
    assert!(diags.has_errors(), "{diags:?}");

    // Given twice is the third answer every roster gives, and being required
    // does not exempt `allow` from it.
    let diags = check_src(&access_src("allow: Role::Admin, allow: Role::Owner"));
    assert!(reported(&diags, code::E_DUPLICATE_DECLARATION), "{diags:?}");
}

#[test]
fn a_route_without_a_sibling_access_does_not_compile() {
    // § 1: two attributes rather than one field is the whole mechanism, so the
    // fixture is every other test in this file with the decision taken away —
    // `route_src` supplies it, and this is what happens where nothing does.
    let bare = "<?nvs\nclass Users {\n  \
                #[Core\\Route(path: \"/users\", method: Core\\Http\\Method::Get)]\n  \
                public function index(): string { return \"\"; }\n}\n";
    let diags = check_src(bare);
    assert!(reported(&diags, code::E_ROUTE_WITHOUT_ACCESS), "{diags:?}");
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // § 3: what closes it is the decision *written*. `Core\Audience::Public` is
    // the case § 1a puts in `Core` for exactly this, so the fix for the error
    // is a name that already resolves rather than one an application has to
    // invent before it can compile a public route.
    let diags = check_src(&with_access(bare));
    assert!(!diags.has_errors(), "{diags:?}");

    // A method carrying neither attribute owes nothing: the `#[Route]` is what
    // makes a decision required, which is why the diagnostic points at it.
    let diags =
        check_src("<?nvs\nclass Users {\n  public function index(): string { return \"\"; }\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");

    // The sibling is found by name resolution and not by text, so § 1a's own
    // spelling — both names placed by a `use` — is the same attribute the
    // fully-qualified fixtures write.
    let diags = check_src(
        "<?nvs\nuse Core\\Access;\nuse Core\\Audience;\nclass Users {\n  \
         #[Core\\Route(path: \"/users\", method: Core\\Http\\Method::Get)]\n  \
         #[Access(allow: Audience::Public)]\n  \
         public function index(): string { return \"\"; }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_query_marker_outside_a_route_method_is_refused() {
    // `rule:routing/a-query-parameter-is-declared-like-a-capture` gives `#[Query]` its meaning on a route method's parameter,
    // so the same declaration with the `#[Route]` taken away binds nothing —
    // and it is the pass that walks *every* method, not the route pass, that
    // can see it: the route pass by construction visits only the methods a
    // `#[Route]` marks.
    let stray = "<?nvs\nclass Users {\n  \
                 public function index(#[Core\\Query] string $sort): string { return $sort; }\n}\n";
    let diags = check_src(stray);
    assert!(reported(&diags, code::E_QUERY_WITHOUT_ROUTE), "{diags:?}");
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // The same parameter under a route is § 3's ordinary case, which is what
    // makes the refusal above about the *sibling* and not about the marker.
    let diags = check_src(&with_access(
        "<?nvs\nclass Users {\n  \
         #[Core\\Route(path: \"/users\", method: Core\\Http\\Method::Get)]\n  \
         public function index(#[Core\\Query] string $sort): string { return $sort; }\n}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // One report per marker rather than one per method: each is its own
    // mistake with its own span, so an author who wrote two is told about both.
    let diags = check_src(
        "<?nvs\nclass Users {\n  \
         public function index(#[Core\\Query] string $sort, #[Core\\Query] uint $page): string \
         { return $sort; }\n}\n",
    );
    assert_eq!(count(&diags, code::E_QUERY_WITHOUT_ROUTE), 2, "{diags:?}");

    // Matched nominally like every other name on the closed roster, so the
    // placed spelling is the same attribute the qualified fixtures write, and
    // a userland `Query` is not it.
    let diags = check_src(
        "<?nvs\nuse Core\\Query;\nclass Users {\n  \
         public function index(#[Query] string $sort): string { return $sort; }\n}\n",
    );
    assert!(reported(&diags, code::E_QUERY_WITHOUT_ROUTE), "{diags:?}");
    let diags = check_src(
        "<?nvs\ntype Query = {};\nclass Users {\n  \
         public function index(#[Query] string $sort): string { return $sort; }\n}\n",
    );
    assert!(!reported(&diags, code::E_QUERY_WITHOUT_ROUTE), "{diags:?}");
}

#[test]
fn a_user_class_implementing_parses_may_be_a_route_capture() {
    // The roster a capture narrows to ends in a *predicate* rather than in one
    // more name: `Core\Uuid` is admitted because it is a class built from text,
    // and so is any other class that says so by implementing `Parses`
    // (`rule:security/route-capture-is-laundered-by-its-type`). Nothing about
    // this class is registered anywhere — it is named by the parameter's own
    // declared type and reached structurally from there.
    let (diags, exprs) = check_src_table(&format!(
        "<?nvs\n{SLUG}class Posts {{\n  \
         #[Core\\Route(path: \"/posts/{{slug}}\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function show(Slug $slug): string {{ return \"\"; }}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // The row says which of the two kinds of class it holds, because a class
    // and an enum render identically as their qualified name — so `parses` is
    // what tells the type whose wire form is a bare string from the one whose
    // case spellings are still undecided (`RouteParam::parses`).
    let row = &exprs.routes().rows()[0];
    let param = &row.params[0];
    assert_eq!(param.name, "slug");
    assert_eq!(param.source, nvs_types::ParamIn::Path);
    assert_eq!(param.ty.as_deref(), Some("Slug"));
    assert!(param.parses, "{param:?}");

    // The other bound, at the one capture the roster does not answer for: a
    // catch-all is every remaining segment as one unchecked value, so it
    // arrives as a `tainted string` and at no other type — implementing
    // `Parses` buys a class nothing there.
    let diags = check_src(&format!(
        "<?nvs\n{SLUG}class Posts {{\n  \
         #[Core\\Route(path: \"/posts/{{rest...}}\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function raw(Slug $rest): string {{ return \"\"; }}\n}}\n"
    ));
    assert!(
        reported(&diags, code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION),
        "{diags:?}"
    );
}

#[test]
fn a_user_class_implementing_parses_may_be_a_query_parameter() {
    // `rule:routing/a-query-parameter-is-declared-like-a-capture` gives a
    // `#[Query]` parameter the same type list as a capture, so the roster's
    // last entry reaches the query half by the same predicate and nothing is
    // written twice. The route below declares no capture at all, which is what
    // makes this the marker's question rather than the path's.
    let (diags, exprs) = check_src_table(&format!(
        "<?nvs\n{SLUG}class Posts {{\n  \
         #[Core\\Route(path: \"/posts\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function index(#[Core\\Query] Slug $slug): string {{ return \"\"; }}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    let param = &exprs.routes().rows()[0].params[0];
    assert_eq!(param.name, "slug");
    assert_eq!(param.source, nvs_types::ParamIn::Query);
    assert_eq!(param.ty.as_deref(), Some("Slug"));
    assert!(param.parses, "{param:?}");
    // A default is what makes a query key optional, and this parameter carries
    // none, so the row asks the request for it.
    assert!(param.required, "{param:?}");
}

#[test]
fn a_class_without_the_interface_is_refused_naming_parses_as_the_fix() {
    // The interface is the whole of what admits a class, so the same class
    // written twice — once with the contract and once without — is the pair
    // that says so. A class is not admitted for being a class.
    let refused = check_src(&format!(
        "<?nvs\nclass Plain {{\n  public tainted string $text = \"\";\n}}\n\
         class Posts {{\n  \
         #[Core\\Route(path: \"/posts/{{slug}}\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function show(Plain $slug): string {{ return \"\"; }}\n}}\n{SLUG}"
    ));
    assert!(
        reported(&refused, code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION),
        "{refused:?}"
    );

    // And the refusal names the fix rather than only the roster: an author who
    // wrote a class is told which contract to implement, which is the half a
    // roster listing `Core\Uuid` by name could not have said.
    assert!(
        refused.iter().any(|d| {
            d.code == Some(code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION)
                && d.notes
                    .iter()
                    .any(|note| note.contains("class implementing `Parses`"))
        }),
        "{refused:?}"
    );
}

#[test]
fn core_uuid_reaches_the_roster_through_the_interface_and_not_its_name() {
    // The engine's own class is on the roster by the same predicate a user's is
    // — there is no arm matching it by name — so the two rows agree in kind and
    // a reader cannot tell which class the library shipped.
    let (diags, exprs) = check_src_table(&format!(
        "<?nvs\n{SLUG}class Posts {{\n  \
         #[Core\\Route(path: \"/posts/{{slug}}\", method: Core\\Http\\Method::Get, \
         name: \"Posts::show\")]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function show(Slug $slug): string {{ return \"\"; }}\n  \
         #[Core\\Route(path: \"/posts/by-id/{{id}}\", method: Core\\Http\\Method::Get, \
         name: \"Posts::byId\")]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function byId(Core\\Uuid $id): string {{ return \"\"; }}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    let table = exprs.routes();
    let mine = &table.named("Posts::show").expect("the user's route").params[0];
    let engines = &table
        .named("Posts::byId")
        .expect("the engine's route")
        .params[0];
    assert_eq!(mine.ty.as_deref(), Some("Slug"));
    assert_eq!(engines.ty.as_deref(), Some("Core\\Uuid"));
    assert!(mine.parses && engines.parses, "{mine:?} {engines:?}");
    assert_eq!(mine.allowed, engines.allowed);
}

#[test]
fn a_parses_capture_names_no_closed_set_and_changes_no_route_rank() {
    // A `Parses` capture is `converts_from_string`'s business and never
    // `closed_set`'s: the contract says the text either parses or does not and
    // never *which* texts do, so there is nothing a segment could be checked
    // against before the class runs. The union beside it is the bound on the
    // other side — a type that does name its set still names it.
    let (diags, exprs) = check_src_table(&format!(
        "<?nvs\n{SLUG}class Posts {{\n  \
         #[Core\\Route(path: \"/posts/{{slug}}/{{lang}}\", \
         method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function show(Slug $slug, \"en\"|\"de\" $lang): string {{ return \"\"; }}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let params = &exprs.routes().rows()[0].params;
    assert_eq!(params[0].allowed, None, "{params:?}");
    assert_eq!(
        params[1].allowed.as_deref(),
        Some(&["en".to_owned(), "de".to_owned()][..]),
        "{params:?}"
    );

    // And it changes no rank: two routes distinguished only by a `Parses`
    // capture's *content* are the one route both would match, exactly as two
    // `{id}` captures are — the trie is built from the shape, and the class is
    // not part of it.
    let diags = check_src(&format!(
        "<?nvs\n{SLUG}class Posts {{\n  \
         #[Core\\Route(path: \"/posts/{{slug}}\", method: Core\\Http\\Method::Get, \
         name: \"a\")]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function show(Slug $slug): string {{ return \"\"; }}\n  \
         #[Core\\Route(path: \"/posts/{{id}}\", method: Core\\Http\\Method::Get, name: \"b\")]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function other(string $id): string {{ return \"\"; }}\n}}\n"
    ));
    assert!(reported(&diags, code::E_DUPLICATE_ROUTE), "{diags:?}");
}
