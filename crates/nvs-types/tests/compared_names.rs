//! `rule:classes/a-compared-property-name-is-a-use`'s warning `W1039`: a string a
//! property callback compares with its name parameter, where no property of the
//! class, of a class above it or of a class below it has that name.
//!
//! `tests/lsp/diagnostics/a-compared-name-no-property-has-is-a-warning.lspt`
//! pins the same warning as the editor shows it.

mod common;

use common::check_src;
use nvs_diagnostics::code;

/// `Page` with the callback body `body`, one property `views` and one
/// subclass that declares `likes`.
fn page(body: &str) -> String {
    format!(
        "<?nvs
class Page implements LateInit {{
    public lateinit int $views;
    public function initProperty(string $name): void {{
{body}
    }}
}}
class Post extends Page {{
    public lateinit int $likes;
}}
"
    )
}

/// The `W1039` warnings in the check of `src`, each as its message.
fn warnings(src: &str) -> Vec<String> {
    let diags = check_src(src);
    assert!(!diags.has_errors(), "{diags:?}");
    diags
        .iter()
        .filter(|d| d.code == Some(code::W_COMPARED_NAME_IS_NO_PROPERTY))
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn a_compared_name_no_property_has_is_a_warning() {
    let src = page("if ($name == 'veiws') { $this->views = 0; }");
    assert_eq!(warnings(&src), ["`'veiws'` is not a property of `Page`"]);
}

#[test]
fn a_property_of_the_class_or_of_a_class_below_is_no_warning() {
    let src = page(
        "if ($name == 'views') { $this->views = 0; }
         if ($name == 'likes') { return; }",
    );
    assert_eq!(warnings(&src), Vec::<String>::new());
}

#[test]
fn a_callback_that_writes_its_name_is_not_read() {
    let src = page(
        "$name = 'views';
         if ($name == 'veiws') { $this->views = 0; }",
    );
    assert_eq!(warnings(&src), Vec::<String>::new());
}
