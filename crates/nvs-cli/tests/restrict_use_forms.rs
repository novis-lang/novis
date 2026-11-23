//! `rule:packaging/a-restriction-limits-where-a-declaration-is-used`: one row per expression kind
//! that reaches a declaration and had no arm in the checker's reader before.
//!
//! Each row builds a program folder by hand: `package.toml` is the application's file, `main.nvs`
//! autoloads `Framework` from `framework/` and `Shop` from `shop/`, and calls into `Shop`. Every
//! refused row has a control beside it, the same layout with the restriction lifted, so a
//! refusal that fires on every program cannot pass.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The entry: both autoloads, then `body`.
fn entry(body: &str) -> String {
    format!("<?nvs\nautoload 'Framework' from 'framework';\nautoload 'Shop' from 'shop';\n\n{body}")
}

/// A fresh program folder with `files` written under it, each `(path, text)`.
fn layout(files: &[(&str, &str)]) -> (nvs_repo::Scratch, PathBuf) {
    let root = nvs_repo::scratch("restrict-use-forms");
    let app = root.join("app");
    for (path, text) in files {
        let path = app.join(path);
        std::fs::create_dir_all(path.parent().expect("every file is in a directory"))
            .expect("the test makes its directories");
        std::fs::write(&path, text).expect("the test writes its files");
    }
    (root, app)
}

/// `nvs check <entry>`, as `(stderr, success)`.
fn check(entry: &Path) -> (String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("check")
        .arg(entry)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// The application's file with one entry on `target`, allowed only in `Framework`.
fn restricting(target: &str) -> String {
    format!("[[restrict]]\nuse   = '{target}'\nallow = ['Framework']\n")
}

/// Checks `files` under the restriction on `target`, and under none, and asserts the first is one
/// refused use of `target` in `at` and the second compiles.
fn refused_only_when_restricted(files: &[(&str, &str)], target: &str, at: &str) {
    let restricted = restricting(target);
    let mut with = vec![("package.toml", restricted.as_str())];
    with.extend_from_slice(files);
    let (_root, app) = layout(&with);
    let (stderr, ok) = check(&app.join("main.nvs"));
    assert!(
        !ok && stderr.contains("error[E0664]")
            && stderr.contains(&format!("`{target}` cannot be used here"))
            && stderr.contains(at),
        "the use of `{target}` in {at} is refused:\n{stderr}"
    );

    let mut without = vec![("package.toml", "[grants]\n")];
    without.extend_from_slice(files);
    let (_root, app) = layout(&without);
    let (stderr, ok) = check(&app.join("main.nvs"));
    assert!(ok, "with no restriction the program compiles:\n{stderr}");
}

#[test]
fn a_union_receiver_uses_the_member_on_every_arm() {
    let a = "<?nvs\nnamespace Framework;\n\nclass A\n{\n    public function run(): int\n    {\n        return 1;\n    }\n}\n";
    let b = "<?nvs\nnamespace Framework;\n\nclass B\n{\n    public function run(): int\n    {\n        return 2;\n    }\n}\n";
    let cart = "<?nvs\nnamespace Shop;\n\nclass Cart\n{\n    public static function total(Framework\\A|Framework\\B $x): int\n    {\n        return $x->run();\n    }\n}\n";
    let main = entry("echo Shop\\Cart::total(Framework\\Make::b()), \"\\n\";\n");
    let make = "<?nvs\nnamespace Framework;\n\nclass Make\n{\n    public static function b(): B\n    {\n        return new B();\n    }\n}\n";
    refused_only_when_restricted(
        &[
            ("main.nvs", &main),
            ("framework/A.nvs", a),
            ("framework/B.nvs", b),
            ("framework/Make.nvs", make),
            ("shop/Cart.nvs", cart),
        ],
        "Framework\\B::run",
        "Cart.nvs:8",
    );
}

#[test]
fn an_enumeration_of_implementors_uses_every_class_it_constructs() {
    let plugin = "<?nvs\nnamespace Framework;\n\ninterface Plugin\n{\n}\n";
    let secret = "<?nvs\nnamespace Framework;\n\nclass Secret implements Plugin\n{\n}\n";
    let list = "<?nvs\nnamespace Shop;\n\nclass Plugins\n{\n    public static function count(): uint\n    {\n        return Core\\Arr::count(Core\\Program::implementing<Framework\\Plugin>());\n    }\n}\n";
    let main = entry("echo Shop\\Plugins::count(), \"\\n\";\n");
    refused_only_when_restricted(
        &[
            ("main.nvs", &main),
            ("framework/Plugin.nvs", plugin),
            ("framework/Secret.nvs", secret),
            ("shop/Plugins.nvs", list),
        ],
        "Framework\\Secret",
        "Plugins.nvs:8",
    );
}

#[test]
fn a_late_static_constant_read_uses_the_constant() {
    let base = "<?nvs\nnamespace Framework;\n\nclass Base\n{\n    public const int LIMIT = 3;\n}\n";
    let child = "<?nvs\nnamespace Shop;\n\nclass Child extends Framework\\Base\n{\n    public static function limit(): int\n    {\n        return static::LIMIT;\n    }\n}\n";
    let main = entry("echo Shop\\Child::limit(), \"\\n\";\n");
    refused_only_when_restricted(
        &[
            ("main.nvs", &main),
            ("framework/Base.nvs", base),
            ("shop/Child.nvs", child),
        ],
        "Framework\\Base::LIMIT",
        "Child.nvs:8",
    );
}

#[test]
fn a_property_default_uses_the_constant_it_reads() {
    let base = "<?nvs\nnamespace Framework;\n\nclass Base\n{\n    public const int LIMIT = 3;\n}\n";
    let cart =
        "<?nvs\nnamespace Shop;\n\nclass Cart\n{\n    public int $n = Framework\\Base::LIMIT;\n}\n";
    let main = entry("echo new Shop\\Cart()->n, \"\\n\";\n");
    refused_only_when_restricted(
        &[
            ("main.nvs", &main),
            ("framework/Base.nvs", base),
            ("shop/Cart.nvs", cart),
        ],
        "Framework\\Base::LIMIT",
        "Cart.nvs:6",
    );
}

#[test]
fn a_parameter_default_uses_the_constant_it_reads() {
    let base = "<?nvs\nnamespace Framework;\n\nclass Base\n{\n    public const int LIMIT = 3;\n}\n";
    let cart = "<?nvs\nnamespace Shop;\n\nclass Cart\n{\n    public static function take(int $n = Framework\\Base::LIMIT): int\n    {\n        return $n;\n    }\n}\n";
    let main = entry("echo Shop\\Cart::take(), \"\\n\";\n");
    refused_only_when_restricted(
        &[
            ("main.nvs", &main),
            ("framework/Base.nvs", base),
            ("shop/Cart.nvs", cart),
        ],
        "Framework\\Base::LIMIT",
        "Cart.nvs:6",
    );
}

#[test]
fn an_attribute_payload_uses_the_constant_it_reads() {
    let base = "<?nvs\nnamespace Framework;\n\nclass Base\n{\n    public const int LIMIT = 3;\n}\n";
    let size = "<?nvs\nnamespace Shop;\n\nattribute Size on method { n: int }\n";
    let cart = "<?nvs\nnamespace Shop;\n\nclass Cart\n{\n    #[Size(n: Framework\\Base::LIMIT)]\n    public static function take(): int\n    {\n        return 1;\n    }\n}\n";
    let main = entry("echo Shop\\Cart::take(), \"\\n\";\n");
    refused_only_when_restricted(
        &[
            ("main.nvs", &main),
            ("framework/Base.nvs", base),
            ("shop/Size.nvs", size),
            ("shop/Cart.nvs", cart),
        ],
        "Framework\\Base::LIMIT",
        "Cart.nvs:6",
    );
}

#[test]
fn a_route_link_uses_the_method_it_calls() {
    let pages = "<?nvs\nnamespace Framework;\n\nclass Pages\n{\n    #[Core\\Route(path: \"/home\", method: Core\\Http\\Method::Get)]\n    #[Core\\Access(allow: Core\\Audience::Public)]\n    public static function home(): string\n    {\n        return \"home\";\n    }\n}\n";
    let cart = "<?nvs\nnamespace Shop;\n\nclass Cart\n{\n    public static function link(): string\n    {\n        return Core\\Router::url(Framework\\Pages::home);\n    }\n}\n";
    let main = entry("echo Shop\\Cart::link(), \"\\n\";\n");
    refused_only_when_restricted(
        &[
            ("main.nvs", &main),
            ("framework/Pages.nvs", pages),
            ("shop/Cart.nvs", cart),
        ],
        "Core\\Router::url",
        "Cart.nvs:8",
    );
}

#[test]
fn an_allowed_class_does_not_allow_a_namespace_of_the_same_name() {
    let tax = "<?nvs\nnamespace Framework;\n\nclass Tax\n{\n    public static function rate(): int\n    {\n        return 20;\n    }\n}\n";
    let cart = "<?nvs\nnamespace Shop;\n\nclass Cart\n{\n    public static function rate(): int\n    {\n        return Framework\\Tax::rate();\n    }\n}\n";
    let item = "<?nvs\nnamespace Shop\\Cart;\n\nclass Item\n{\n    public static function rate(): int\n    {\n        return Framework\\Tax::rate();\n    }\n}\n";
    let package = "[[restrict]]\nuse   = 'Framework\\Tax::rate'\nallow = ['Shop\\Cart']\n";

    let main = entry("echo Shop\\Cart::rate(), \"\\n\";\n");
    let (_root, app) = layout(&[
        ("package.toml", package),
        ("main.nvs", &main),
        ("framework/Tax.nvs", tax),
        ("shop/Cart.nvs", cart),
    ]);
    let (stderr, ok) = check(&app.join("main.nvs"));
    assert!(ok, "the class `Shop\\Cart` is allowed:\n{stderr}");

    let main = entry("echo Shop\\Cart::rate(), Shop\\Cart\\Item::rate(), \"\\n\";\n");
    let (_root, app) = layout(&[
        ("package.toml", package),
        ("main.nvs", &main),
        ("framework/Tax.nvs", tax),
        ("shop/Cart.nvs", cart),
        ("shop/Cart/Item.nvs", item),
    ]);
    let (stderr, ok) = check(&app.join("main.nvs"));
    assert!(
        !ok && stderr.contains("error[E0664]") && stderr.contains("Item.nvs:8"),
        "the namespace `Shop\\Cart` is not the class `Shop\\Cart`:\n{stderr}"
    );
    assert!(
        !stderr.contains("Cart.nvs:8"),
        "the class itself is still allowed:\n{stderr}"
    );
}
