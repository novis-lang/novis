//! A check that walks the bodies of some files and folds every other file's
//! cached output reports and records what a whole check does.
//!
//! Each test checks one program twice or more through `common::Program`,
//! which keeps each file's id across checks, and with one interner, as the
//! editor does. A whole check is the oracle: its diagnostics and its table,
//! as `ExprTypeTable::canonical_text`, are what every partial check must
//! give.

mod common;

use common::Program;
use nvs_diagnostics::code;
use nvs_types::CheckCache;
use nvs_types::ty::TypeInterner;

/// The entry: a route, a link to a route in `blog.nvs`, and an anonymous
/// function at file scope.
const MAIN: &str = "<?nvs\nrequire 'blog.nvs';\nrequire 'shop.nvs';\n\
    class Health {\n  \
    #[Core\\Route(path: \"/health\", method: Core\\Http\\Method::Get)]\n  \
    #[Core\\Access(allow: Core\\Audience::Public)]\n  \
    public function show(): string { return Blog::title() . Shop::name(); }\n}\n\
    echo Core\\Router::url(Blog::post, [\"id\" => 1]), \"\\n\";\n\
    var $twice = fn(int $n): int => $n * 2;\n";

/// A route, a method another file calls, and an anonymous function at file
/// scope, so its label depends on the files before it.
const BLOG: &str = "<?nvs\nclass Blog {\n  \
    #[Core\\Route(path: \"/posts/{id}\", method: Core\\Http\\Method::Get)]\n  \
    #[Core\\Access(allow: Core\\Audience::Public)]\n  \
    public function post(uint $id): string { return Blog::title(); }\n  \
    public static function title(): string { return \"Blog\"; }\n}\n\
    var $shout = fn(string $s): string => $s . \"!\";\n";

/// A class that calls into `blog.nvs`, with a body the tests edit.
const SHOP: &str = "<?nvs\nclass Shop {\n  \
    public static function name(): string { return \"Shop\"; }\n  \
    public static function both(): string { return Shop::name() . Blog::title(); }\n}\n";

fn program(name: &str) -> Program {
    Program::new(
        name,
        &[("main.nvs", MAIN), ("blog.nvs", BLOG), ("shop.nvs", SHOP)],
    )
}

#[test]
fn a_partial_check_with_cached_outputs_matches_a_whole_check() {
    let mut program = program("partial-matches-whole");
    let mut interner = TypeInterner::new();
    let mut cache = CheckCache::new();
    let filled = program.check(
        "main.nvs",
        &mut interner,
        Some((&["main.nvs", "blog.nvs", "shop.nvs"], &mut cache)),
    );
    assert!(filled.diags.is_empty(), "{:?}", filled.diags);
    assert_eq!(cache.len(), 3);
    let whole = program.check("main.nvs", &mut interner, None);
    assert_eq!(filled.diags, whole.diags);
    assert_eq!(filled.table, whole.table);

    // No body walked, and one body walked: both fold to the whole check.
    for bodies in [&[][..], &["shop.nvs"][..], &["main.nvs"][..]] {
        let partial = program.check("main.nvs", &mut interner, Some((bodies, &mut cache)));
        assert_eq!(partial.diags, whole.diags, "{bodies:?}");
        assert_eq!(partial.table, whole.table, "{bodies:?}");
    }

    // A body edit in one file, walked alone, gives what a whole check of the
    // edited program gives, its new error included.
    program.write(
        "shop.nvs",
        &SHOP.replace(
            "return Shop::name() . Blog::title();",
            "int $wrong = \"text\"; return Shop::name();",
        ),
    );
    let partial = program.check("main.nvs", &mut interner, Some((&["shop.nvs"], &mut cache)));
    let whole = program.check("main.nvs", &mut interner, None);
    assert!(!whole.diags.is_empty(), "the edit reports an error");
    assert_eq!(partial.diags, whole.diags);
    assert_eq!(partial.table, whole.table);
}

#[test]
fn a_route_collision_with_an_unchecked_file_is_still_reported() {
    let mut program = program("partial-route-collision");
    program.write(
        "shop.nvs",
        &SHOP.replace(
            "class Shop {\n",
            "class Shop {\n  \
         #[Core\\Route(path: \"/posts/{id}\", method: Core\\Http\\Method::Get)]\n  \
         #[Core\\Access(allow: Core\\Audience::Public)]\n  \
         public function post(uint $id): string { return \"\"; }\n",
        ),
    );
    let mut interner = TypeInterner::new();
    let mut cache = CheckCache::new();
    let filled = program.check(
        "main.nvs",
        &mut interner,
        Some((&["main.nvs", "blog.nvs", "shop.nvs"], &mut cache)),
    );
    let collides = |diags: &[String]| {
        diags
            .iter()
            .filter(|d| d.contains(code::E_DUPLICATE_ROUTE.as_str()))
            .count()
    };
    assert_eq!(collides(&filled.diags), 1, "{:?}", filled.diags);

    // Neither file that declares the route is walked.
    let partial = program.check("main.nvs", &mut interner, Some((&["main.nvs"], &mut cache)));
    assert_eq!(collides(&partial.diags), 1, "{:?}", partial.diags);
    assert_eq!(partial.diags, filled.diags);
}

#[test]
fn a_link_to_a_route_in_an_unchecked_file_still_resolves() {
    let mut program = program("partial-link");
    let mut interner = TypeInterner::new();
    let mut cache = CheckCache::new();
    program.check(
        "main.nvs",
        &mut interner,
        Some((&["main.nvs", "blog.nvs", "shop.nvs"], &mut cache)),
    );

    // The link in `main.nvs` is walked, and the route it names is in the
    // cached `blog.nvs`.
    let partial = program.check("main.nvs", &mut interner, Some((&["main.nvs"], &mut cache)));
    assert!(partial.diags.is_empty(), "{:?}", partial.diags);
    let route = partial
        .exprs
        .routes()
        .handled_by("Blog::post")
        .expect("the cached file's route");
    assert_eq!(route.path, "/posts/{id}");

    // A link to a method with no route is still found from the cached table.
    program.write("main.nvs", &MAIN.replace("Blog::post", "Blog::title"));
    let partial = program.check("main.nvs", &mut interner, Some((&["main.nvs"], &mut cache)));
    assert!(
        partial
            .diags
            .iter()
            .any(|d| d.contains(code::E_LINK_METHOD_HAS_NO_ROUTE.as_str())),
        "{:?}",
        partial.diags
    );
}

#[test]
fn a_cached_output_is_valid_in_a_second_source_map_with_the_same_pins() {
    let mut program = program("partial-pins");
    // No anonymous function at file scope, so every file starts at the count
    // `0` in both programs, and every cached output is folded.
    program.write(
        "main.nvs",
        &MAIN.replace("var $twice = fn(int $n): int => $n * 2;\n", ""),
    );
    program.write(
        "blog.nvs",
        &BLOG.replace("var $shout = fn(string $s): string => $s . \"!\";\n", ""),
    );
    // A second entry that loads the same two files. Without the pins, the
    // files would take other ids, and the cached spans would name the wrong
    // files.
    program.write(
        "other.nvs",
        "<?nvs\nrequire 'shop.nvs';\nrequire 'blog.nvs';\necho Shop::both(), \"\\n\";\n",
    );
    let mut interner = TypeInterner::new();
    let mut cache = CheckCache::new();
    let first = program.check(
        "main.nvs",
        &mut interner,
        Some((&["main.nvs", "blog.nvs", "shop.nvs"], &mut cache)),
    );
    assert!(first.diags.is_empty(), "{:?}", first.diags);

    let partial = program.check(
        "other.nvs",
        &mut interner,
        Some((&["other.nvs"], &mut cache)),
    );
    assert_eq!(partial.id("blog.nvs"), first.id("blog.nvs"));
    assert_eq!(partial.id("shop.nvs"), first.id("shop.nvs"));
    assert_ne!(partial.id("other.nvs"), first.id("main.nvs"));
    for name in ["blog.nvs", "shop.nvs"] {
        let start = cache
            .get(first.id(name))
            .map(|output| output.anon_fn_start());
        assert_eq!(start, Some(0), "{name}");
    }
    let whole = program.check("other.nvs", &mut interner, None);
    assert!(whole.diags.is_empty(), "{:?}", whole.diags);
    assert_eq!(partial.diags, whole.diags);
    assert_eq!(partial.table, whole.table);
}

#[test]
fn a_file_whose_starting_anonymous_function_count_moved_is_checked_again() {
    let mut program = program("partial-anon-count");
    let mut interner = TypeInterner::new();
    let mut cache = CheckCache::new();
    let first = program.check(
        "main.nvs",
        &mut interner,
        Some((&["main.nvs", "blog.nvs", "shop.nvs"], &mut cache)),
    );
    let blog = first.id("blog.nvs");
    let before = cache
        .get(blog)
        .expect("blog.nvs was walked")
        .anon_fn_start();

    // `main.nvs` comes before `blog.nvs`, and now has one more anonymous
    // function at file scope.
    program.write(
        "main.nvs",
        &format!("{MAIN}var $thrice = fn(int $n): int => $n * 3;\n"),
    );
    let partial = program.check("main.nvs", &mut interner, Some((&["main.nvs"], &mut cache)));
    let after = cache.get(blog).expect("blog.nvs is cached").anon_fn_start();
    assert_eq!(
        after,
        before + 1,
        "blog.nvs was walked again from the new count"
    );

    let whole = program.check("main.nvs", &mut interner, None);
    let labels = |checked: &common::Checked| -> Vec<String> {
        let mut labels: Vec<String> = checked
            .exprs
            .anon_fns()
            .map(|(class, _, _)| class.to_owned())
            .collect();
        labels.sort();
        labels
    };
    assert_eq!(labels(&partial), labels(&whole));
    assert_eq!(partial.diags, whole.diags);
    assert_eq!(partial.table, whole.table);
}
