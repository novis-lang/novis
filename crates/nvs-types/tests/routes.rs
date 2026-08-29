//! ADR 0077 § 1's `#[Route]`: the nominal match that keeps a userland spelling
//! out of the route table, and the payload check that is the pass behind the
//! roster entry.
//!
//! The table itself has no rows yet — `nvs_types::routes`' gaps own what §§ 1-3
//! still cannot report — so what is asserted here is exactly what the name *is*
//! today: recognized after `nvs_hir::resolve_ref`, and checked against a roster
//! of options rather than against a shape.

mod common;

use common::check_src;

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
        "<?nvs\nclass Users {\n  #[\\Core\\Route(path: \"/users/{id}\", name: \"Users::show\")]\n  \
         public function show(uint $id): string { return \"\"; }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    // The `use`d bare spelling is the same attribute, and the bare one with
    // nothing importing it resolves to `\Route` — no declaration at all, so it
    // is the ordinary undeclared-name refusal rather than a silently ignored
    // attribute.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\")]\n  public function health(): string { return \"\"; }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_src(
        "<?nvs\nclass Users {\n  #[Route(path: \"/health\")]\n  \
         public function health(): string { return \"\"; }\n}\n",
    );
    assert!(diags.has_errors());
}

#[test]
fn the_attribute_repeats_so_one_method_serves_two_verbs() {
    // § 1: no `methods: array<Method>` field and no union — ADR 0046 § 3's
    // repeatability is what already covers it, so two attributes on one method
    // are two payloads each checked on its own.
    let diags = check_src(&route_src(
        "  #[Route(path: \"/health\", name: \"health.get\")]\n  \
         #[Route(path: \"/health\", name: \"health.head\")]\n  \
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
    // in a payload — so the fixture's own spelling passes the payload walk. The
    // enum it names is declared nowhere yet, so the row interns to nothing and
    // the case is checked as a constant and placed at no type;
    // `nvs_types::routes`' gap 1 is that this admits a case of the wrong enum
    // until `Core\Http\Method` lands.
    let diags = check_src(
        "<?nvs\nclass Users {\n  \
         #[\\Core\\Route(path: \"/users\", method: \\Core\\Http\\Method::Get, name: \"Users::index\")]\n  \
         public function index(): string { return \"\"; }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}
